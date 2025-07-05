// Copyright (C) 2022 Wilfred Bos
// Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.

mod audio_renderer;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use atomicring::AtomicRingBuffer;
use audio_renderer::AudioRenderer;
use crossbeam_channel::{Receiver, Sender};

use crate::sid_device_server::player::audio_renderer::{AUDIO_ERROR, PlayerCommand, SidWrite};

const SID_WRITES_BUFFER_SIZE: usize = 65_536;
const MAX_CYCLES_IN_BUFFER: u32 = 63*312 * 50 * 3; // ~3 seconds
const MIN_CYCLES_TO_DRAIN_QUEUE: u32 = 500_000;
const MIN_WRITES_TO_DRAIN_QUEUE: usize = 300;

pub struct Player {
    cycles_in_buffer: Arc<AtomicU32>,
    queue: Arc<AtomicRingBuffer<SidWrite>>,
    queue_started: Arc<AtomicBool>,
    aborted: Arc<AtomicBool>,
    player_cmd_sender: Sender<(PlayerCommand, Option<i32>)>,
    sid_read_receiver: Receiver<u8>,
    audio_device: AudioRenderer
}

impl Player {
    pub fn new(audio_device_number: Option<i32>) -> Player {
        let cycles_in_buffer = Arc::new(AtomicU32::new(0));
        let buf = Arc::new(AtomicRingBuffer::<SidWrite>::with_capacity(SID_WRITES_BUFFER_SIZE));
        let aborted = Arc::new(AtomicBool::new(false));
        let queue_started = Arc::new(AtomicBool::new(false));

        let mut audio_device = AudioRenderer::new(
            buf.clone(),
            queue_started.clone(),
            aborted.clone(),
            cycles_in_buffer.clone()
        );

        audio_device.start(audio_device_number);

        let player_cmd_sender = audio_device.get_channel_sender();

        let sid_read_receiver = audio_device.get_sid_read_receiver();

        Player {
            cycles_in_buffer,
            queue: buf,
            queue_started,
            aborted,
            player_cmd_sender,
            sid_read_receiver,
            audio_device
        }
    }

    pub fn has_error(&self) -> bool {
        AUDIO_ERROR.load(Ordering::SeqCst)
    }

    pub fn has_max_data_in_buffer(&self) -> bool {
        let cycles = self.cycles_in_buffer.load(Ordering::SeqCst);
        let enough_data = self.queue.len() > SID_WRITES_BUFFER_SIZE / 2 || cycles > MAX_CYCLES_IN_BUFFER;
        if enough_data {
            self.start_draining();
        }
        enough_data
    }

    pub fn has_min_data_in_buffer(&self) -> bool {
        self.cycles_in_buffer.load(Ordering::SeqCst) > MIN_CYCLES_TO_DRAIN_QUEUE || self.queue.len() > MIN_WRITES_TO_DRAIN_QUEUE
    }

    pub fn start_draining(&self) {
        self.queue_started.store(true, Ordering::SeqCst);
    }

    pub fn write_to_sid(&mut self, reg: u8, data: u8, cycles: u16) {
        let sid_write = SidWrite {reg, data, cycles};
        let _ = self.queue.try_push(sid_write);
        self.cycles_in_buffer.fetch_add(cycles as u32, Ordering::SeqCst);
    }

    pub fn read_from_sid(&mut self, reg: u8, cycles: u16) -> u8 {
        self.queue_started.store(true, Ordering::SeqCst);
        self.dummy_write(reg, cycles);

        let _ = self.player_cmd_sender.send((PlayerCommand::Read, Some(reg as i32)));
        let sid_env_out = self.sid_read_receiver.recv();
        sid_env_out.unwrap_or(0)
    }

    pub fn flush(&mut self) {
        self.clear_queue();
        self.aborted.store(true, Ordering::SeqCst);
    }

    pub fn reset(&self) {
        let _ = self.player_cmd_sender.send((PlayerCommand::Reset, None));
    }

    pub fn enable_digiboost(&self, enabled: bool) {
        let command = if enabled {
            PlayerCommand::EnableDigiboost
        } else {
            PlayerCommand::DisableDigiboost
        };
        let _ = self.player_cmd_sender.send((command, None));
    }

    pub fn set_filter_bias_6581(&self, filter_bias: Option<i32>) {
        let _ = self.player_cmd_sender.send((PlayerCommand::SetFilterBias6581, filter_bias));
    }

    pub fn set_model(&self, model: i32) {
        let _ = self.player_cmd_sender.send((PlayerCommand::SetModel, Some(model)));
    }

    pub fn set_clock(&self, clock: i32) {
        let _ = self.player_cmd_sender.send((PlayerCommand::SetClock, Some(clock)));
    }

    pub fn set_sid_count(&mut self, count: i32) {
        self.clear_queue();  // clear queue so there are no writes for multiple SIDs anymore
        self.audio_device.restart(None);

        let _ = self.player_cmd_sender.send((PlayerCommand::SetSidCount, Some(count)));
    }

    pub fn set_position(&self, position: i32) {
        let _ = self.player_cmd_sender.send((PlayerCommand::SetPosition, Some(position)));
    }

    pub fn set_sampling_method(&self, sampling_method: i32) {
        let _ = self.player_cmd_sender.send((PlayerCommand::SetSamplingMethod, Some(sampling_method)));
    }

    pub fn set_audio_device(&mut self, audio_device_number: Option<i32>) {
        self.clear_queue();
        self.audio_device.set_audio_device(audio_device_number);
    }

    fn clear_queue(&self) {
        self.cycles_in_buffer.store(0, Ordering::SeqCst);
        self.queue.clear();
        self.queue_started.store(false, Ordering::SeqCst);
    }

    fn dummy_write(&mut self, reg: u8, cycles: u16) {
        self.write_to_sid((reg & 0xe0) + 0x1e, 0, cycles);
    }
}
