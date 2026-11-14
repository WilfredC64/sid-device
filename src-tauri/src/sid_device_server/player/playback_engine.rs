// Copyright (C) 2026 Wilfred Bos
// Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.

use std::cmp::min;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use atomicring::AtomicRingBuffer;
use nuked_opl3::Opl3Chip;
use resid::{chip_model, sampling_method};

use super::audio_renderer::{Config, PlayerCommand, SidWrite};
use super::sid_emulation::SidEmulation;

pub(super) const PAL_CLOCK: u32 = 985_248;
const NTSC_CLOCK: u32 = 1_022_727;

const OUTPUT_CHANNELS: usize = 2;

const AUDIO_STREAM_LIMIT_IN_MILLIS: usize = 100;
const AUDIO_STREAM_MAX_LIMIT_IN_MILLIS: usize = 500;

const CYCLES_PER_BATCH: u32 = 5_000;
const MAX_EVENTS_PER_BATCH: usize = 1_024;

const PARALLEL_CYCLES_PER_BATCH: u32 = 20_000;
const PARALLEL_MAX_EVENTS_PER_BATCH: usize = 4_096;

const MAX_EVENT_CYCLES: u32 = u16::MAX as u32;

const FM_INDEX_PORT: u16 = 0x240;
const FM_DATA_PORT: u16 = 0x250;
const FM_TEST_REGISTER: u8 = 0x01;
const FM_DIGI_ARM_VALUE: u8 = 0x04;
const FM_DIGI_ADDR_A: u8 = 0xa0;
const FM_DIGI_ADDR_B: u8 = 0xa1;
const FM_DIGI_SCALE: i32 = 64;

struct FmOpl {
    chip: Opl3Chip,
    address: u8,
    digi_armed: bool,
    digi_value: [i16; 2],
    buffer: Vec<i16>,
}

impl FmOpl {
    fn new(sample_rate: u32) -> Self {
        Self {
            chip: Opl3Chip::new(sample_rate),
            address: 0,
            digi_armed: false,
            digi_value: [0, 0],
            buffer: Vec::new(),
        }
    }

    fn reset(&mut self, sample_rate: u32) {
        self.chip.reset(sample_rate);
        self.address = 0;
        self.digi_armed = false;
        self.digi_value = [0, 0];
    }

    fn render(&mut self, events: &[SidWrite], event_sample_lengths: &[usize], sample_count: usize) {
        self.buffer.resize(sample_count * 2, 0);
        let mut offset = 0;
        for (event, &length) in events.iter().zip(event_sample_lengths) {
            let end = offset + length * 2;
            if length != 0 {
                let buffer = &mut self.buffer[offset..end];
                if self.digi_armed {
                    let sample = (self.digi_value[0] as i32 + self.digi_value[1] as i32)
                        .clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                    buffer.fill(sample);
                } else {
                    let _ = self.chip.generate_stream(buffer);
                }
            }
            offset = end;

            if event.cycles != 0 {
                self.write(event.reg, event.data);
            }
        }
    }

    fn write(&mut self, reg: u16, data: u8) {
        match reg {
            FM_INDEX_PORT => self.address = data,
            FM_DATA_PORT => {
                if self.address == FM_TEST_REGISTER {
                    self.digi_armed = data == FM_DIGI_ARM_VALUE;
                }
                if self.digi_armed && (self.address == FM_DIGI_ADDR_A || self.address == FM_DIGI_ADDR_B) {
                    // Digi mode holds raw unsigned PCM instead of synthesizing OPL audio.
                    self.digi_value[(self.address - FM_DIGI_ADDR_A) as usize] = ((data as i32 - 128) * FM_DIGI_SCALE) as i16;
                } else {
                    self.chip.write_register(self.address as u16, data);
                }
            }
            _ => {}
        }
    }
}

pub(super) struct EmulationState {
    sids: SidEmulation,
    opl: FmOpl,
    events: Vec<SidWrite>,
    mix_left: Vec<i32>,
    mix_right: Vec<i32>,
}

impl EmulationState {
    pub(super) fn new(config: &mut Config) -> Self {
        config.config_changed = false;
        Self {
            sids: SidEmulation::new(config),
            opl: FmOpl::new(config.sample_rate),
            events: Vec::with_capacity(MAX_EVENTS_PER_BATCH),
            mix_left: Vec::new(),
            mix_right: Vec::new(),
        }
    }

    fn configure(&mut self, config: &mut Config) {
        self.sids.configure(config);
        if config.fm_opl_enabled {
            self.opl.reset(config.sample_rate);
        }
        config.config_changed = false;
    }

    pub(super) fn read(&mut self, config: &mut Config, reg: i32) -> u8 {
        if config.config_changed {
            self.configure(config);
        }
        let sid_num = min(reg >> 5, config.sid_count - 1) as usize;
        self.sids.read(sid_num, reg as u32 & 0x1f)
    }

    fn mix(&mut self, config: &Config) {
        let sample_count = self.sids.sample_count();

        self.mix_left.clear();
        self.mix_left.resize(sample_count, 0);
        self.mix_right.clear();
        self.mix_right.resize(sample_count, 0);

        let sid_count = config.sid_count as usize;
        for index in 0..sid_count {
            let (position_left, position_right) = if sid_count == 1 {
                (100, 100)
            } else {
                (config.position_left[index], config.position_right[index])
            };
            let samples = self.sids.samples(index);
            for ((left, right), &sample) in self.mix_left.iter_mut().zip(self.mix_right.iter_mut()).zip(samples) {
                *left += sample as i32 * position_left / 100;
                *right += sample as i32 * position_right / 100;
            }
        }

        if config.fm_opl_enabled {
            self.opl.render(&self.events, self.sids.event_sample_lengths(), sample_count);
            let (frames, _) = self.opl.buffer.as_chunks::<2>();
            for ((left, right), frame) in self.mix_left.iter_mut().zip(self.mix_right.iter_mut()).zip(frames) {
                *left += frame[0] as i32;
                *right += frame[1] as i32;
            }
        }
    }
}

pub(super) fn process_player_command(
    command: PlayerCommand,
    param: Option<i32>,
    config: &mut Config,
    state: &mut EmulationState,
) {
    match command {
        PlayerCommand::SetModel => {
            if let Some(param) = param {
                let sid_number = param >> 8;
                if sid_number >= 0 && sid_number < config.sid_count {
                    config.chip_model[sid_number as usize] = if param & 0xff == 0 {
                        chip_model::MOS6581
                    } else {
                        chip_model::MOS8580
                    };
                }
                config.config_changed = true;
            }
        }
        PlayerCommand::SetClock => {
            config.clock = if param.unwrap() == 0 {
                PAL_CLOCK
            } else {
                NTSC_CLOCK
            };
            config.config_changed = true;
        }
        PlayerCommand::SetSidCount => {
            let count = param.unwrap() as usize;
            config.sid_count = count as i32;
            config.chip_model = vec![config.chip_model[0].clone(); count];
            config.position_left = vec![0; count];
            config.position_right = vec![0; count];
            config.config_changed = true;
        }
        PlayerCommand::SetPosition => {
            if let Some(param) = param {
                let position = ((param & 0xff) as i8) as i32;
                let sid_number = param >> 8;
                if sid_number >= 0 && sid_number < config.sid_count {
                    config.position_left[sid_number as usize] = if position >= 0 { 100 } else { 100 + position };
                    config.position_right[sid_number as usize] = if position <= 0 { 100 } else { 100 - position };
                }
            }
        }
        PlayerCommand::SetSamplingMethod => {
            config.sampling_method = if param.unwrap() == 1 {
                sampling_method::SAMPLE_RESAMPLE
            } else {
                sampling_method::SAMPLE_INTERPOLATE
            };
            config.config_changed = true;
        }
        PlayerCommand::EnableDigiboost | PlayerCommand::DisableDigiboost => {
            config.digiboost = command == PlayerCommand::EnableDigiboost;
            state.sids.set_digiboost(config);
        }
        PlayerCommand::EnableFmOpl => {
            config.fm_opl_enabled = true;
            state.opl.reset(config.sample_rate);
        }
        PlayerCommand::DisableFmOpl => config.fm_opl_enabled = false,
        PlayerCommand::SetFilterBias6581 => {
            if let Some(param) = param {
                config.filter_bias_6581 = param as f64 / 100.0;
                state.sids.set_filter_bias(config);
            }
        }
        PlayerCommand::SetSamplingFrequency => {
            if let Some(param) = param {
                state.sids.set_sampling_frequency(param as u32);
            }
        }
        PlayerCommand::Reset => config.config_changed = true,
        PlayerCommand::Read => {}
    }
}

#[inline]
fn ms_to_samples(sample_rate: u32, millis: usize) -> usize {
    sample_rate as usize * OUTPUT_CHANNELS * millis / 1000
}

#[inline]
fn cycles_to_samples(cycles: u32, clock: u32, sample_rate: u32) -> usize {
    (u64::from(cycles) * u64::from(sample_rate)).div_ceil(u64::from(clock)) as usize * OUTPUT_CHANNELS
}

pub(super) fn try_fill_audio_buffer(
    output: &Arc<AtomicRingBuffer<i16>>,
    queue: &Arc<AtomicRingBuffer<SidWrite>>,
    cycles_in_buffer: &Arc<AtomicU32>,
    config: &mut Config,
    state: &mut EmulationState,
) -> bool {
    !queue.is_empty()
        && output.len() < ms_to_samples(config.sample_rate, AUDIO_STREAM_LIMIT_IN_MILLIS)
        && try_render_batch(output, queue, cycles_in_buffer, config, state)
}

pub(super) fn try_render_batch(
    output: &Arc<AtomicRingBuffer<i16>>,
    queue: &Arc<AtomicRingBuffer<SidWrite>>,
    cycles_in_buffer: &Arc<AtomicU32>,
    config: &mut Config,
    state: &mut EmulationState,
) -> bool {
    if config.config_changed {
        state.configure(config);
    }

    let (max_cycles, max_events) = if state.sids.is_parallel() {
        (PARALLEL_CYCLES_PER_BATCH, PARALLEL_MAX_EVENTS_PER_BATCH)
    } else {
        (CYCLES_PER_BATCH, MAX_EVENTS_PER_BATCH)
    };

    let max_batch_samples = cycles_to_samples(max_cycles + MAX_EVENT_CYCLES, config.clock, config.sample_rate);
    let max_limit = min(
        ms_to_samples(config.sample_rate, AUDIO_STREAM_MAX_LIMIT_IN_MILLIS),
        output.capacity().saturating_sub(max_batch_samples),
    );
    if output.len() > max_limit {
        return false;
    }

    let total_cycles = collect_batch(queue, &mut state.events, max_cycles, max_events);

    state.sids.render(&state.events, total_cycles);
    state.mix(config);
    push_output(output, &state.mix_left, &state.mix_right);

    if total_cycles > 0 {
        let cycles = cycles_in_buffer.load(Ordering::Relaxed);
        if cycles > total_cycles {
            cycles_in_buffer.fetch_sub(total_cycles, Ordering::Relaxed);
        } else {
            cycles_in_buffer.store(0, Ordering::Relaxed);
        }
    }
    !state.events.is_empty()
}

fn collect_batch(queue: &AtomicRingBuffer<SidWrite>, events: &mut Vec<SidWrite>, max_cycles: u32, max_events: usize) -> u32 {
    events.clear();
    let mut total_cycles = 0;
    for _ in 0..max_events {
        if total_cycles >= max_cycles {
            break;
        }
        let Some(event) = queue.try_pop() else { break };
        total_cycles += u32::from(event.cycles);
        events.push(event);
    }
    total_cycles
}

fn push_output(output: &AtomicRingBuffer<i16>, left: &[i32], right: &[i32]) {
    for (&left, &right) in left.iter().zip(right) {
        let rand = fastrand::u64(..);
        let dithering = (rand & 1) as i32 - ((rand >> 1) & 1) as i32;
        let _ = output.try_push(limit_output(left, dithering));
        let _ = output.try_push(limit_output(right, dithering));
    }
}

#[inline]
fn limit_output(sample: i32, dithering: i32) -> i16 {
    sample.saturating_add(dithering).clamp(i16::MIN as i32, i16::MAX as i32) as i16
}
