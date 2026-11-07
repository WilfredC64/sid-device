// Copyright (C) 2026 Wilfred Bos
// Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.

use std::cmp::min;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use atomicring::AtomicRingBuffer;
use crossbeam_channel::Receiver;
use nuked_opl3::Opl3Chip;
use resid::{chip_model, sampling_method};

use super::audio_renderer::{Config, PlayerCommand, SidWrite};
use super::sid_emulation::SidEmulation;

pub(super) const PAL_CLOCK: u32 = 985_248;
const NTSC_CLOCK: u32 = 1_022_727;

const AUDIO_STREAM_LIMIT: usize = 10_000;
const AUDIO_STREAM_MAX_LIMIT: usize = 55_000;

const CYCLES_PER_BATCH: u32 = 5_000;
const MAX_EVENTS_PER_BATCH: usize = 1_024;

const FM_TEST_REGISTER: u8 = 0x01;
const FM_DIGI_ARM_VALUE: u8 = 0x04;
const FM_DIGI_ADDR_A: u8 = 0xa0;
const FM_DIGI_ADDR_B: u8 = 0xa1;
const FM_DIGI_SCALE: i32 = 64;

pub(super) struct EmulationState {
    sids: SidEmulation,
    opl3: Opl3Chip,
    opl3_address: u8,
    opl3_digi_armed: bool,
    opl3_digi_value: [i16; 2],
    events: Vec<SidWrite>,
    opl3_buffer: Vec<i16>,
}

impl EmulationState {
    pub(super) fn new(config: &mut Config) -> Self {
        config.config_changed = false;
        Self {
            sids: SidEmulation::new(config),
            opl3: Opl3Chip::new(config.sample_rate),
            opl3_address: 0,
            opl3_digi_armed: false,
            opl3_digi_value: [0, 0],
            events: Vec::with_capacity(MAX_EVENTS_PER_BATCH),
            opl3_buffer: Vec::new(),
        }
    }

    fn configure(&mut self, config: &mut Config) {
        self.sids.configure(config);
        if config.fm_opl_enabled {
            self.reset_opl(config.sample_rate);
        }
        config.config_changed = false;
    }

    fn reset_opl(&mut self, sample_rate: u32) {
        self.opl3.reset(sample_rate);
        self.opl3_address = 0;
        self.opl3_digi_armed = false;
        self.opl3_digi_value = [0, 0];
    }

    pub(super) fn read(&mut self, config: &mut Config, reg: i32) -> u8 {
        if config.config_changed {
            self.configure(config);
        }
        let sid_num = min(reg >> 5, config.sid_count - 1) as usize;
        self.sids.read(sid_num, reg as u32 & 0x1f)
    }
}

pub(super) fn process_player_command(
    receiver: &Receiver<(PlayerCommand, Option<i32>)>,
    config: &mut Config,
    state: &mut EmulationState,
) -> Option<(PlayerCommand, Option<i32>)> {
    let (command, param) = receiver.try_recv().ok()?;
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
            state.reset_opl(config.sample_rate);
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
    Some((command, param))
}

pub(super) fn try_generate_sample(
    output: &Arc<AtomicRingBuffer<i16>>,
    queue: &Arc<AtomicRingBuffer<SidWrite>>,
    cycles_in_buffer: &Arc<AtomicU32>,
    config: &mut Config,
    state: &mut EmulationState,
) {
    if !queue.is_empty() && output.len() < AUDIO_STREAM_LIMIT {
        generate_sample(output, queue, cycles_in_buffer, config, state);
    }
}

pub(super) fn generate_sample(
    output: &Arc<AtomicRingBuffer<i16>>,
    queue: &Arc<AtomicRingBuffer<SidWrite>>,
    cycles_in_buffer: &Arc<AtomicU32>,
    config: &mut Config,
    state: &mut EmulationState,
) {
    if output.len() > AUDIO_STREAM_MAX_LIMIT {
        return;
    }
    if config.config_changed {
        state.configure(config);
    }

    let total_cycles = collect_batch(queue, &mut state.events);
    let rendered = state.sids.render(&state.events);
    let sample_count = rendered.samples[0].len();

    if config.fm_opl_enabled {
        state.opl3_buffer.resize(sample_count * 2, 0);
        let mut offset = 0;
        for (event, &length) in state.events.iter().zip(&rendered.event_sample_lengths) {
            let end = offset + length * 2;
            if length != 0 {
                let buffer = &mut state.opl3_buffer[offset..end];
                if state.opl3_digi_armed {
                    let sample = (state.opl3_digi_value[0] as i32 + state.opl3_digi_value[1] as i32)
                        .clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                    buffer.fill(sample);
                } else {
                    let _ = state.opl3.generate_stream(buffer);
                }
            }
            offset = end;
            // Writes happen after their cycle interval, even if it generated no samples.
            if event.cycles != 0 && (0x200..0x300).contains(&event.reg) {
                apply_opl_write(
                    &mut state.opl3,
                    &mut state.opl3_address,
                    &mut state.opl3_digi_armed,
                    &mut state.opl3_digi_value,
                    event,
                );
            }
        }
    }

    for i in 0..sample_count {
        let (mut left, mut right) = if config.sid_count == 1 {
            let sample = rendered.samples[0][i] as i32;
            (sample, sample)
        } else {
            let mut left = 0;
            let mut right = 0;
            for (j, samples) in rendered.samples.iter().enumerate() {
                left += samples[i] as i32 * config.position_left[j] / 100;
                right += samples[i] as i32 * config.position_right[j] / 100;
            }
            (left, right)
        };
        if config.fm_opl_enabled {
            left += state.opl3_buffer[i * 2] as i32;
            right += state.opl3_buffer[i * 2 + 1] as i32;
        }
        let rand = fastrand::u64(..);
        let dithering = (rand & 1) as i32 - ((rand >> 1) & 1) as i32;
        let _ = output.try_push(limit_output(left, dithering));
        let _ = output.try_push(limit_output(right, dithering));
    }

    if total_cycles > 0 {
        let cycles = cycles_in_buffer.load(Ordering::Relaxed);
        if cycles > total_cycles {
            cycles_in_buffer.fetch_sub(total_cycles, Ordering::Relaxed);
        } else {
            cycles_in_buffer.store(0, Ordering::Relaxed);
        }
    }
}

fn collect_batch(queue: &AtomicRingBuffer<SidWrite>, events: &mut Vec<SidWrite>) -> u32 {
    events.clear();
    let mut total_cycles = 0;
    while total_cycles < CYCLES_PER_BATCH && events.len() < MAX_EVENTS_PER_BATCH {
        let Some(event) = queue.try_pop() else { break };
        total_cycles += u32::from(event.cycles);
        events.push(event);
    }
    total_cycles
}

fn apply_opl_write(
    opl3: &mut Opl3Chip,
    address: &mut u8,
    digi_armed: &mut bool,
    digi_value: &mut [i16; 2],
    event: &SidWrite,
) {
    // The FM cartridge uses an index/data port pair ($DF40/$DF50).
    match event.reg - 0x200 {
        0x40 => *address = event.data,
        0x50 => {
            if *address == FM_TEST_REGISTER {
                *digi_armed = event.data == FM_DIGI_ARM_VALUE;
            }
            if *digi_armed && (*address == FM_DIGI_ADDR_A || *address == FM_DIGI_ADDR_B) {
                // Digi mode holds raw unsigned PCM instead of synthesizing OPL audio.
                digi_value[(*address - FM_DIGI_ADDR_A) as usize] = ((event.data as i32 - 128) * FM_DIGI_SCALE) as i16;
            } else {
                opl3.write_register(*address as u16, event.data);
            }
        }
        _ => {}
    }
}

#[inline]
fn limit_output(sample: i32, dithering: i32) -> i16 {
    sample.saturating_add(dithering).clamp(i16::MIN as i32, i16::MAX as i32) as i16
}
