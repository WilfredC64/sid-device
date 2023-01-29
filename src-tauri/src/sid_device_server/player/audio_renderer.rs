// Copyright (C) 2022 Wilfred Bos
// Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.

use parking_lot::Mutex;
use std::cmp::min;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::{thread, time::{Duration, Instant}};

use atomicring::AtomicRingBuffer;
use cpal::{Device, OutputCallbackInfo, Sample, SampleFormat, StreamConfig};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{Sender, Receiver, bounded};
use rand::Rng;
use typed_builder::TypedBuilder;

use resid::{chip_model, sampling_method, Sid};
use thread_priority::{set_current_thread_priority, ThreadPriority};

pub static AUDIO_ERROR: AtomicBool = AtomicBool::new(false);

const AUDIO_BUFFER_SIZE: usize = 65_536;
const SAMPLE_BUFFER_SIZE: usize = 8_192;

const AUDIO_STREAM_LIMIT: usize = 10_000;
const AUDIO_STREAM_MAX_LIMIT: usize = 55_000;

const PAL_CLOCK: u32 = 985_248;
const NTSC_CLOCK: u32 = 1_022_727;

const DEFAULT_FILTER_BIAS_6581: f64 = 0.24;

const PAUSE_AUDIO_IDLE_TIME_IN_SEC: u64 = 2;

const CYCLES_PER_SAMPLE: u32 = 5_000;

const DEFAULT_SAMPLE_RATE: u32 = 48_000;

const CYCLES_IN_BUFFER_THRESHOLD: u32 = 10_000;
const SOUND_BUFFER_SIZE_THRESHOLD: usize = 5_000;

const STOP_PAUSE_LATENCY_IN_MILLIS: u64 = 10;

#[derive(Copy, Clone)]
pub struct SidWrite {
    pub reg: u8,
    pub data: u8,
    pub cycles: u16,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PlayerCommand {
    SetClock,
    SetModel,
    SetSidCount,
    SetPosition,
    SetSamplingMethod,
    EnableDigiboost,
    DisableDigiboost,
    SetFilterBias6581,
    SetSamplingFrequency,
    Reset,
    Read
}

struct DeviceState {
    should_stop: Arc<AtomicBool>,
    should_pause: Arc<AtomicBool>,
    queue_started: Arc<AtomicBool>,
    aborted: Arc<AtomicBool>,
    cycles_in_buffer: Arc<AtomicU32>
}

#[derive(TypedBuilder)]
pub struct Config {
    pub sample_rate: u32,
    pub sampling_method: sampling_method,
    pub clock: u32,
    pub sid_count: i32,
    pub chip_model: Vec<chip_model>,
    pub position_left: Vec<i32>,
    pub position_right: Vec<i32>,
    pub digiboost: bool,
    pub filter_bias_6581: f64,

    #[builder(default=false)]
    pub config_changed: bool
}

pub struct AudioRenderer {
    in_cmd_sender: Sender<(PlayerCommand, Option<i32>)>,
    in_cmd_receiver: Receiver<(PlayerCommand, Option<i32>)>,
    out_sid_read_sender: Sender<u8>,
    out_sid_read_receiver: Receiver<u8>,
    queue: Arc<AtomicRingBuffer<SidWrite>>,
    queue_started: Arc<AtomicBool>,
    aborted: Arc<AtomicBool>,
    cycles_in_buffer: Arc<AtomicU32>,
    audio_device_number: Option<i32>,
    should_stop_audio_producer: Arc<AtomicBool>,
    should_stop_audio_generator: Arc<AtomicBool>,
    should_pause: Arc<AtomicBool>,
    emulation_thread: Option<thread::JoinHandle<()>>,
    audio_thread: Option<thread::JoinHandle<()>>,
    config: Arc<Mutex<Config>>,
    sound_buffer: Arc<AtomicRingBuffer<i16>>
}

impl Drop for AudioRenderer {
    fn drop(&mut self) {
        self.stop_threads();
    }
}

impl AudioRenderer {
    pub fn new(
        queue: Arc<AtomicRingBuffer<SidWrite>>,
        queue_started: Arc<AtomicBool>,
        aborted: Arc<AtomicBool>,
        cycles_in_buffer: Arc<AtomicU32>
    ) -> AudioRenderer {
        let (in_cmd_sender, in_cmd_receiver) = bounded(0);
        let (out_sid_read_sender, out_sid_read_receiver) = bounded(0);
        let should_stop_audio_producer = Arc::new(AtomicBool::new(false));
        let should_stop_audio_generator = Arc::new(AtomicBool::new(false));
        let should_pause = Arc::new(AtomicBool::new(false));
        let config = Self::create_default_config(DEFAULT_SAMPLE_RATE);
        let sound_buffer = Arc::new(AtomicRingBuffer::<i16>::with_capacity(AUDIO_BUFFER_SIZE));

        AudioRenderer {
            in_cmd_sender,
            in_cmd_receiver,
            out_sid_read_sender,
            out_sid_read_receiver,
            queue,
            queue_started,
            aborted,
            cycles_in_buffer,
            audio_device_number: None,
            should_stop_audio_producer,
            should_stop_audio_generator,
            should_pause,
            emulation_thread: None,
            audio_thread: None,
            config: Arc::new(Mutex::new(config)),
            sound_buffer
        }
    }

    fn stop_threads(&mut self) {
        self.stop_audio_generator_thread();
        self.stop_audio_producer_thread();
    }

    fn stop_audio_generator_thread(&mut self) {
        self.should_stop_audio_generator.store(true, Ordering::SeqCst);

        if self.emulation_thread.is_some() {
            let _ = self.emulation_thread.take().unwrap().join().ok();
        }

        self.should_stop_audio_generator.store(false, Ordering::SeqCst);
    }


    fn stop_audio_producer_thread(&mut self) {
        self.should_stop_audio_producer.store(true, Ordering::SeqCst);

        if self.audio_thread.is_some() {
            let _ = self.audio_thread.take().unwrap().join().ok();
        }

        self.should_stop_audio_producer.store(false, Ordering::SeqCst);
    }

    pub fn start(&mut self, audio_device_number: Option<i32>) {
        if audio_device_number.is_some() {
            self.audio_device_number = audio_device_number;
        }

        let mut restart = self.audio_thread.is_some() || self.emulation_thread.is_some();
        self.stop_threads();

        if AUDIO_ERROR.load(Ordering::SeqCst) {
            AUDIO_ERROR.store(false, Ordering::SeqCst);
            restart = false;
        }

        self.sound_buffer.clear();

        self.start_audio_thread(audio_device_number, !restart);

        let mut config = self.config.clone();

        let mut sound_buffer_clone = self.sound_buffer.clone();
        let should_stop_audio_generator_clone = self.should_stop_audio_generator.clone();
        let should_pause_clone = self.should_pause.clone();
        let aborted = self.aborted.clone();
        let mut queue = self.queue.clone();
        let cycles_in_buffer = self.cycles_in_buffer.clone();

        let in_cmd_receiver = self.in_cmd_receiver.clone();
        let out_sid_read_sender = self.out_sid_read_sender.clone();

        let queue_started = self.queue_started.clone();

        let device_state = DeviceState {
            should_stop: should_stop_audio_generator_clone,
            should_pause: should_pause_clone,
            queue_started,
            aborted,
            cycles_in_buffer
        };

        self.emulation_thread = Some(thread::spawn(move || {
            Self::sid_emulation_thread(
                &mut queue,
                &in_cmd_receiver,
                &out_sid_read_sender,
                &mut config,
                &mut sound_buffer_clone,
                device_state
            )
        }));
    }

    fn start_audio_thread(&mut self, audio_device_number: Option<i32>, log_device_name: bool) {
        let device = Self::get_audio_device(audio_device_number);
        let device_config = device.default_output_config().unwrap();
        let sample_rate = device_config.sample_rate();

        let mut config = self.config.lock();
        config.sample_rate = sample_rate.0;

        let should_stop_audio_producer_clone = self.should_stop_audio_producer.clone();
        let should_pause = self.should_pause.clone();
        let sound_buffer_clone = self.sound_buffer.clone();

        if log_device_name && audio_device_number.is_some() {
            println!("Using audio device: \"{}\" (sample rate: {})\r", device.name().unwrap(), sample_rate.0);
        }

        self.audio_thread = Some(thread::spawn(move || {
            let _ = match device_config.sample_format() {
                SampleFormat::F32 => run::<f32>(&device, &device_config.into(), sound_buffer_clone, should_stop_audio_producer_clone, should_pause),
                SampleFormat::I16 => run::<i16>(&device, &device_config.into(), sound_buffer_clone, should_stop_audio_producer_clone, should_pause),
                SampleFormat::U16 => run::<u16>(&device, &device_config.into(), sound_buffer_clone, should_stop_audio_producer_clone, should_pause)
            };
        }));
    }

    pub fn restart(&mut self, audio_device_number: Option<i32>) {
        if audio_device_number.is_some() {
            self.audio_device_number = audio_device_number;
        }
        self.start(self.audio_device_number);
    }

    pub fn set_audio_device(&mut self, audio_device_number: Option<i32>) {
        self.audio_device_number = audio_device_number;

        self.stop_audio_producer_thread();
        self.sound_buffer.clear();
        self.start_audio_thread(self.audio_device_number, true);

        let sample_rate = self.config.lock().sample_rate;
        let _ = self.in_cmd_sender.send((PlayerCommand::SetSamplingFrequency, Some(sample_rate as i32)));
    }

    fn get_audio_device(audio_device_number: Option<i32>) -> Device {
        let host = cpal::default_host();

        if let Some(audio_device_number) = audio_device_number {
            let devices = host.output_devices();
            if let Ok(devices) = devices {
                let device = devices.enumerate().find(|(index, _device)| *index == audio_device_number as usize);
                if let Some(device) = device {
                    return device.1
                }
            }
        }

        host.default_output_device().expect("Failed to find a default output device")
    }

    fn sid_emulation_thread(
        queue: &mut Arc<AtomicRingBuffer<SidWrite>>,
        in_cmd_receiver_clone: &Receiver<(PlayerCommand, Option<i32>)>,
        out_sid_read_sender: &Sender<u8>,
        config: &mut Arc<Mutex<Config>>,
        sound_buffer: &mut Arc<AtomicRingBuffer<i16>>,
        device_state: DeviceState
    ) {
        let _ = set_current_thread_priority(ThreadPriority::Max);

        let mut sids: Vec<Sid> = vec![];

        {
            let mut config = config.lock();
            configure_sids(&mut sids, &mut config);
        }

        let mut last_activity = Instant::now();
        loop {
            let mut config = config.lock();

            if device_state.should_stop.load(Ordering::SeqCst) {
                break;
            }
            if device_state.aborted.load(Ordering::SeqCst) {
                sound_buffer.clear();
                device_state.aborted.store(false, Ordering::SeqCst);
            }

            if !queue.is_empty() && device_state.queue_started.load(Ordering::SeqCst) {
                last_activity = Instant::now();
                device_state.should_pause.store(false, Ordering::SeqCst);
            } else if !device_state.should_pause.load(Ordering::SeqCst) && last_activity.elapsed().as_secs() > PAUSE_AUDIO_IDLE_TIME_IN_SEC {
                device_state.should_pause.store(true, Ordering::SeqCst);
            }

            let cmd = process_player_command(in_cmd_receiver_clone, &mut config, &mut sids);

            if let Some((command, param1)) = cmd {
                if command == PlayerCommand::Read {
                    while !queue.is_empty() {
                        generate_sample(sound_buffer, queue, &mut sids, &device_state.cycles_in_buffer, &mut config);
                    }

                    let reg = param1.unwrap_or(0);
                    let sid_num = min(reg >> 5, config.sid_count - 1) as usize;

                    let sid_env_out = sids[sid_num].read(reg as u32 & 0x1f) as u8;
                    let _ = out_sid_read_sender.send(sid_env_out);
                }
            } else {
                if !device_state.queue_started.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(5));
                    continue;
                }

                try_generate_sample(sound_buffer, queue, &mut sids, &device_state.cycles_in_buffer, &mut config);
                if Self::has_enough_data(sound_buffer, &device_state) {
                    thread::sleep(Duration::from_millis(1));
                }
            }
        }
    }

    #[inline]
    fn has_enough_data(sound_buffer: &mut Arc<AtomicRingBuffer<i16>>, device_state: &DeviceState) -> bool {
        device_state.cycles_in_buffer.load(Ordering::SeqCst) > CYCLES_IN_BUFFER_THRESHOLD && sound_buffer.len() > SOUND_BUFFER_SIZE_THRESHOLD
    }

    fn create_default_config(sample_rate: u32) -> Config {
        Config::builder()
            .sample_rate(sample_rate)
            .sampling_method(sampling_method::SAMPLE_RESAMPLE)
            .clock(PAL_CLOCK)
            .sid_count(1)
            .chip_model(vec![chip_model::MOS6581])
            .position_left(vec![0])
            .position_right(vec![0])
            .digiboost(false)
            .filter_bias_6581(DEFAULT_FILTER_BIAS_6581)
            .build()
    }

    pub fn get_channel_sender(&self) -> Sender<(PlayerCommand, Option<i32>)> {
        self.in_cmd_sender.clone()
    }

    pub fn get_sid_read_receiver(&self) -> Receiver<u8> {
        self.out_sid_read_receiver.clone()
    }
}

#[inline]
fn process_player_command(in_cmd_receiver: &Receiver<(PlayerCommand, Option<i32>)>, config: &mut Config, sids: &mut [Sid]) -> Option<(PlayerCommand, Option<i32>)> {
    let recv_result = in_cmd_receiver.try_recv();

    if let Ok((command, param1)) = recv_result {
        match command {
            PlayerCommand::SetModel => {
                if let Some(param1) = param1 {
                    let model = param1 & 0xff;
                    let sid_number = param1 >> 8;
                    if sid_number >= 0 && sid_number < config.sid_count {
                        config.chip_model[sid_number as usize] = if model == 0 {
                            chip_model::MOS6581
                        } else {
                            chip_model::MOS8580
                        };
                    }

                    config.config_changed = true;
                }
            }
            PlayerCommand::SetClock => {
                let clock = param1.unwrap();
                config.clock = if clock == 0 {
                    PAL_CLOCK
                } else {
                    NTSC_CLOCK
                };

                config.config_changed = true;
            }
            PlayerCommand::SetSidCount => {
                let count = param1.unwrap() as usize;
                config.sid_count = count as i32;
                config.chip_model = vec![config.chip_model[0]; count];
                config.position_left = vec![0; count];
                config.position_right = vec![0; count];

                config.config_changed = true;
            }
            PlayerCommand::SetPosition => {
                if let Some(param1) = param1 {
                    let position = ((param1 & 0xff) as i8) as i32;
                    let sid_number = param1 >> 8;
                    if sid_number >= 0 && sid_number < config.sid_count {
                        config.position_left[sid_number as usize] = if position <= 0 { 100 } else { 100 - position };
                        config.position_right[sid_number as usize] = if position >= 0 { 100 } else { 100 + position };
                    }
                }
            }
            PlayerCommand::SetSamplingMethod => {
                let sampling_method = param1.unwrap();
                config.sampling_method = if sampling_method == 1 {
                    sampling_method::SAMPLE_RESAMPLE
                } else {
                    sampling_method::SAMPLE_INTERPOLATE
                };

                config.config_changed = true;
            }
            PlayerCommand::EnableDigiboost => {
                config.digiboost = true;

                for (i, sid) in sids.iter_mut().enumerate() {
                    if config.chip_model[i] == chip_model::MOS8580 {
                        sid.set_voice_mask(0x0f_u32);
                        sid.input(i16::MIN);
                    }
                }
            }
            PlayerCommand::DisableDigiboost => {
                config.digiboost = false;

                for (i, sid) in sids.iter_mut().enumerate() {
                    if config.chip_model[i] == chip_model::MOS8580 {
                        sid.set_voice_mask(0x07_u32);
                        sid.input(0);
                    }
                }
            }
            PlayerCommand::SetFilterBias6581 => {
                if let Some(param1) = param1 {
                    let filter_bias = param1;
                    config.filter_bias_6581 = filter_bias as f64 / 100.0;

                    for (i, sid) in sids.iter_mut().enumerate() {
                        if config.chip_model[i] == chip_model::MOS6581 {
                            sid.adjust_filter_bias(config.filter_bias_6581);
                        }
                    }
                }
            }
            PlayerCommand::SetSamplingFrequency => {
                if let Some(param1) = param1 {
                    for sid in &mut sids.iter_mut() {
                        sid.adjust_sampling_frequency(param1 as f64);
                    }
                }
            }
            PlayerCommand::Reset => {
                config.config_changed = true;
            }
            _ => {}
        }
        return Some((command, param1));
    }
    None
}

fn configure_sids(sids: &mut Vec<Sid>, config: &mut Config) {
    sids.clear();

    for i in 0..config.sid_count {
        let mut sid = Sid::new();

        sid.set_chip_model(config.chip_model[i as usize]);

        let _ = sid.set_sampling_parameters(config.clock as f64, config.sampling_method, config.sample_rate as f64);

        sid.enable_filter(true);

        let mut voice_mask = 0x07u32;
        let mut input_sample = 0;

        if config.chip_model[i as usize] == chip_model::MOS8580 {
            if config.digiboost {
                voice_mask |= 0x08;
                input_sample = i16::MIN;
            }
        } else {
            sid.adjust_filter_bias(config.filter_bias_6581);
        }

        sid.set_voice_mask(voice_mask);
        sid.input(input_sample);

        sid.clock_delta(0xffff);

        sids.push(sid);
    }

    config.config_changed = false;
}

fn try_generate_sample(audio_output_stream: &mut Arc<AtomicRingBuffer<i16>>, sid_write_queue: &mut Arc<AtomicRingBuffer<SidWrite>>, sids: &mut Vec<Sid>, cycles_in_buffer: &Arc<AtomicU32>, config: &mut Config) {
    if sid_write_queue.len() > 0 && audio_output_stream.len() < AUDIO_STREAM_LIMIT {
        generate_sample(audio_output_stream, sid_write_queue, sids, cycles_in_buffer, config);
    }
}

fn generate_sample(audio_output_stream: &mut Arc<AtomicRingBuffer<i16>>, sid_write_queue: &mut Arc<AtomicRingBuffer<SidWrite>>, sids: &mut Vec<Sid>, cycles_in_buffer: &Arc<AtomicU32>, config: &mut Config) {
    if audio_output_stream.len() > AUDIO_STREAM_MAX_LIMIT {
        return;
    }

    if config.config_changed {
        configure_sids(sids, config);
    }

    let mut total_cycles = 0;
    let mut sample_buffers = vec![[0i16; SAMPLE_BUFFER_SIZE]; sids.len()];

    let mut audio_buffer = [0i16; SAMPLE_BUFFER_SIZE * 2];    // for left and right channel

    let mut rng = rand::thread_rng();
    let mut prev_dithering = 0;
    let mut generate_next_dithering_value = || -> i32 {
        let tmp_value = prev_dithering;
        prev_dithering = rng.gen::<i32>() & 1;
        prev_dithering - tmp_value
    };

    let mut store_audio = |audio_buffer: &mut [i16; SAMPLE_BUFFER_SIZE * 2], i: usize, left, right| {
        let dithering = generate_next_dithering_value();
        audio_buffer[i * 2] = add_dithering_and_limit_output(left, dithering);
        audio_buffer[i * 2 + 1] = add_dithering_and_limit_output(right, dithering);
    };

    while total_cycles < CYCLES_PER_SAMPLE {
        let sid_write = sid_write_queue.try_pop();
        if let Some(sid_write) = sid_write {

            let cycles = sid_write.cycles as u32;
            total_cycles += cycles;

            if cycles > 0 {
                let mut cycles = cycles;

                while cycles > 0 {
                    let mut total_sample_length = 0;
                    let mut total_cycles_left = 0;

                    for sid_num in 0..config.sid_count as usize {
                        let (sample_length, cycles_left) = sids[sid_num].sample(cycles, &mut sample_buffers[sid_num], 1);

                        total_sample_length = sample_length;
                        total_cycles_left = cycles_left;
                    }

                    if config.sid_count == 1 {
                        for i in 0..total_sample_length {
                            let sample = sample_buffers[0][i] as i32;
                            store_audio(&mut audio_buffer, i, sample, sample);
                        }
                    } else {
                        for i in 0..total_sample_length {
                            let mut left = 0;
                            let mut right = 0;

                            for (j, sid_sample_buffer) in sample_buffers.iter().enumerate().take(config.sid_count as usize) {
                                let panning_left = config.position_left[j];
                                let panning_right = config.position_right[j];
                                left += sid_sample_buffer[i] as i32 * panning_left / 100;
                                right += sid_sample_buffer[i] as i32 * panning_right / 100;
                            }

                            store_audio(&mut audio_buffer, i, left, right);
                        }
                    }

                    for sample in audio_buffer.iter().take(total_sample_length * 2) {
                        let _ = audio_output_stream.try_push(*sample);
                    }
                    cycles = total_cycles_left;
                }

                let sid_num = min(sid_write.reg >> 5, (config.sid_count - 1) as u8);
                sids[sid_num as usize].write((sid_write.reg & 0x1f) as u32,  (sid_write.data) as u32);
            }
        } else {
            break;
        }
    }

    if total_cycles > 0 {
        let cycles = cycles_in_buffer.load(Ordering::SeqCst);
        if cycles > total_cycles {
            cycles_in_buffer.fetch_sub(total_cycles, Ordering::SeqCst);
        } else {
            cycles_in_buffer.store(0, Ordering::SeqCst);
        }
    }
}

#[inline]
fn add_dithering_and_limit_output(sample: i32, dithering: i32) -> i16 {
    (sample + dithering).clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

fn run<T>(device: &Device, config: &StreamConfig, sound_buffer: Arc<AtomicRingBuffer<i16>>, should_stop: Arc<AtomicBool>, should_pause: Arc<AtomicBool>) -> Result<(), anyhow::Error> where T: Sample {
    let channels = config.channels as usize;

    let err_fn = |err| {
        AUDIO_ERROR.store(true, Ordering::SeqCst);
        println!("ERROR: {err}\r");
    };

    let mut next_value = move || {
        T::from::<i16>(&sound_buffer.try_pop().unwrap_or(0))
    };

    let output_stream = move |data: &mut [T], _: &OutputCallbackInfo| {
        write_data(data, channels, &mut next_value)
    };

    let stream = device.build_output_stream(config, output_stream, err_fn)?;
    stream.play()?;

    while !should_stop.load(Ordering::SeqCst) {
        if should_pause.load(Ordering::SeqCst) {
            stream.pause()?;
        } else {
            stream.play()?;
        }
        thread::sleep(Duration::from_millis(STOP_PAUSE_LATENCY_IN_MILLIS));
    }

    Ok(())
}

fn write_data<T>(output: &mut [T], channels: usize, next_value: &mut dyn FnMut() -> T) where T: Sample {
    for frame in output.chunks_mut(channels) {
        for sample in frame.iter_mut() {
            *sample = next_value();
        }
    }
}
