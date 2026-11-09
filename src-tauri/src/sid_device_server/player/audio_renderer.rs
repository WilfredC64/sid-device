// Copyright (C) 2022 - 2026 Wilfred Bos
// Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.

use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::{thread, time::{Duration, Instant}};

use atomicring::AtomicRingBuffer;
use cpal::{Device, FromSample, OutputCallbackInfo, SampleFormat, SizedSample, StreamConfig};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{Sender, Receiver, bounded};
use typed_builder::TypedBuilder;

use resid::{chip_model, sampling_method};
use thread_priority::{set_current_thread_priority, ThreadPriority};
use crate::utils::audio::get_device_display_name;
use super::playback_engine::{EmulationState, PAL_CLOCK, generate_sample, process_player_command, try_generate_sample};

pub static AUDIO_ERROR: AtomicBool = AtomicBool::new(false);

const AUDIO_BUFFER_SIZE: usize = 65_536;

const DEFAULT_FILTER_BIAS_6581: f64 = 0.24;
const DEFAULT_SAMPLE_RATE: u32 = 48_000;

const CYCLES_IN_BUFFER_THRESHOLD: u32 = 10_000;
const SOUND_BUFFER_SIZE_THRESHOLD: usize = 5_000;

const PAUSE_AUDIO_IDLE_TIME_IN_SEC: u64 = 2;
const STOP_PAUSE_LATENCY_IN_MILLIS: u64 = 10;

#[derive(Copy, Clone)]
pub struct SidWrite {
    pub reg: u16,
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
    Read,
    EnableFmOpl,
    DisableFmOpl,
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
    pub fm_opl_enabled: bool,

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

        if let Some(handle) = self.emulation_thread.take() {
            let _ = handle.join();
        }

        self.should_stop_audio_generator.store(false, Ordering::SeqCst);
    }

    fn stop_audio_producer_thread(&mut self) {
        self.should_stop_audio_producer.store(true, Ordering::SeqCst);

        if let Some(handle) = self.audio_thread.take() {
            let _ = handle.join();
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

        let config = self.config.clone();
        let sound_buffer = self.sound_buffer.clone();
        let queue = self.queue.clone();
        let in_cmd_receiver = self.in_cmd_receiver.clone();
        let out_sid_read_sender = self.out_sid_read_sender.clone();

        let device_state = DeviceState {
            should_stop: self.should_stop_audio_generator.clone(),
            should_pause: self.should_pause.clone(),
            queue_started: self.queue_started.clone(),
            aborted: self.aborted.clone(),
            cycles_in_buffer: self.cycles_in_buffer.clone()
        };

        self.emulation_thread = Some(thread::spawn(move || {
            Self::sid_emulation_thread(
                &queue,
                &in_cmd_receiver,
                &out_sid_read_sender,
                &config,
                &sound_buffer,
                device_state
            )
        }));
    }

    fn start_audio_thread(&mut self, audio_device_number: Option<i32>, log_device_name: bool) {
        let device = Self::get_audio_device(audio_device_number);
        let device_config = match device.default_output_config() {
            Ok(cfg) => cfg,
            Err(e) => {
                println!("ERROR: Failed to get audio device config: {e}\r");
                AUDIO_ERROR.store(true, Ordering::SeqCst);
                return;
            }
        };

        let sample_rate = device_config.sample_rate();

        let mut config = self.config.lock();
        config.sample_rate = sample_rate;

        let should_stop = self.should_stop_audio_producer.clone();
        let should_pause = self.should_pause.clone();
        let sound_buffer = self.sound_buffer.clone();

        if log_device_name {
            println!("Using audio device: \"{}\" (sample rate: {})\r", get_device_display_name(&device), sample_rate);
        }

        self.audio_thread = Some(thread::spawn(move || {
            macro_rules! run_as {
                ($sample:ty) => {
                    run::<$sample>(&device, device_config.into(), sound_buffer, should_stop, should_pause)
                };
            }

            let _ = match device_config.sample_format() {
                SampleFormat::I8 => run_as!(i8),
                SampleFormat::U8 => run_as!(u8),
                SampleFormat::I16 => run_as!(i16),
                SampleFormat::U16 => run_as!(u16),
                SampleFormat::I32 => run_as!(i32),
                SampleFormat::U32 => run_as!(u32),
                SampleFormat::F32 => run_as!(f32),
                SampleFormat::I64 => run_as!(i64),
                SampleFormat::U64 => run_as!(u64),
                SampleFormat::F64 => run_as!(f64),
                _ => Ok(()),
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
        queue: &Arc<AtomicRingBuffer<SidWrite>>,
        in_cmd_receiver_clone: &Receiver<(PlayerCommand, Option<i32>)>,
        out_sid_read_sender: &Sender<u8>,
        config: &Arc<Mutex<Config>>,
        sound_buffer: &Arc<AtomicRingBuffer<i16>>,
        device_state: DeviceState
    ) {
        let _ = set_current_thread_priority(ThreadPriority::Max);
        let mut state = EmulationState::new(&mut config.lock());
        let mut last_activity = Instant::now();

        loop {
            let mut config = config.lock();

            if device_state.should_stop.load(Ordering::Relaxed) {
                break;
            }
            if device_state.aborted.load(Ordering::SeqCst) {
                sound_buffer.clear();
                device_state.aborted.store(false, Ordering::SeqCst);
            }

            if !queue.is_empty() && device_state.queue_started.load(Ordering::Relaxed) {
                last_activity = Instant::now();
                if device_state.should_pause.load(Ordering::Relaxed) {
                    device_state.should_pause.store(false, Ordering::Relaxed);
                }
            } else if !device_state.should_pause.load(Ordering::Relaxed) && last_activity.elapsed() >= Duration::from_secs(PAUSE_AUDIO_IDLE_TIME_IN_SEC) {
                device_state.should_pause.store(true, Ordering::Relaxed);
            }

            let cmd = process_player_command(in_cmd_receiver_clone, &mut config, &mut state);

            if let Some((command, param1)) = cmd {
                if command == PlayerCommand::Read {
                    while !queue.is_empty() {
                        generate_sample(sound_buffer, queue, &device_state.cycles_in_buffer, &mut config, &mut state);
                    }

                    let sid_env_out = state.read(&mut config, param1.unwrap_or(0));
                    let _ = out_sid_read_sender.send(sid_env_out);
                }
            } else {
                if device_state.should_pause.load(Ordering::Relaxed) || !device_state.queue_started.load(Ordering::Relaxed) {
                    drop(config);
                    thread::sleep(Duration::from_millis(5));
                    continue;
                }

                try_generate_sample(sound_buffer, queue, &device_state.cycles_in_buffer, &mut config, &mut state);
                if Self::has_enough_data(sound_buffer, &device_state) {
                    drop(config);
                    thread::sleep(Duration::from_millis(1));
                }
            }
        }
    }

    #[inline]
    fn has_enough_data(sound_buffer: &Arc<AtomicRingBuffer<i16>>, device_state: &DeviceState) -> bool {
        device_state.cycles_in_buffer.load(Ordering::Relaxed) > CYCLES_IN_BUFFER_THRESHOLD && sound_buffer.len() > SOUND_BUFFER_SIZE_THRESHOLD
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
            .fm_opl_enabled(false)
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

fn run<T>(device: &Device, config: StreamConfig, sound_buffer: Arc<AtomicRingBuffer<i16>>, should_stop: Arc<AtomicBool>, should_pause: Arc<AtomicBool>) -> Result<(), anyhow::Error>
where
    T: SizedSample + FromSample<i16>
{
    let err_fn = |err| {
        AUDIO_ERROR.store(true, Ordering::SeqCst);
        println!("ERROR: {err}\r");
    };

    let output_stream = move |data: &mut [T], _: &OutputCallbackInfo| {
        for sample_slot in data.iter_mut() {
            *sample_slot = T::from_sample(sound_buffer.try_pop().unwrap_or(0));
        }
    };

    let stream = device.build_output_stream(&config, output_stream, err_fn, None)?;
    stream.pause()?;
    let mut is_paused = true;

    while !should_stop.load(Ordering::Relaxed) {
        let should_pause_now = should_pause.load(Ordering::Relaxed);
        if should_pause_now != is_paused {
            if should_pause_now {
                stream.pause()?;
            } else {
                stream.play()?;
            }
            is_paused = should_pause_now;
        }

        thread::sleep(Duration::from_millis(STOP_PAUSE_LATENCY_IN_MILLIS));
    }

    Ok(())
}
