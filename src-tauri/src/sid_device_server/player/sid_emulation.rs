// Copyright (C) 2026 Wilfred Bos
// Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.

use rayon::prelude::*;
use resid::{Sid, chip_model};

use super::audio_renderer::{Config, SidWrite};

const SCRATCH_SAMPLES: usize = 8_192;

const VOICE_MASK_DEFAULT: u32 = 0x07;
const VOICE_MASK_DIGIBOOST: u32 = 0x0f;

pub(super) struct RenderedSids {
    pub samples: Vec<Vec<i16>>,
    pub event_sample_lengths: Vec<usize>,
}

struct Chip {
    sid: Sid,
    scratch: Vec<i16>,
    event_sample_lengths: Vec<usize>,
}

pub(super) struct SidEmulation {
    chips: Vec<Chip>,
    pool: Option<rayon::ThreadPool>,
    max_threads: usize,
    sids_per_task: usize,
    rendered: RenderedSids,
}

impl SidEmulation {
    pub(super) fn new(config: &Config) -> Self {
        let mut emulation = Self {
            chips: Vec::new(),
            pool: None,
            max_threads: std::thread::available_parallelism().map_or(1, usize::from),
            sids_per_task: config.sids_per_task.max(1),
            rendered: RenderedSids {
                samples: Vec::new(),
                event_sample_lengths: Vec::new(),
            },
        };
        emulation.configure(config);
        emulation
    }

    fn configure_pool(&mut self, sid_count: usize) {
        let threads = sid_count.div_ceil(self.sids_per_task).min(self.max_threads);

        if threads <= 1 {
            self.pool = None;
            return;
        }

        if self.pool.as_ref().is_some_and(|pool| pool.current_num_threads() == threads) {
            return;
        }

        self.pool = None;
        match rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|i| format!("sid-render-{i}"))
            .build()
        {
            Ok(pool) => self.pool = Some(pool),
            Err(error) => {
                println!("Cannot create SID render pool: {error}; using sequential rendering")
            }
        }
    }

    pub(super) fn configure(&mut self, config: &Config) {
        let count = config.sid_count as usize;
        self.configure_pool(count);
        self.chips.clear();
        self.rendered.samples.resize_with(count, Vec::new);
        self.rendered.samples.iter_mut().for_each(Vec::clear);
        self.rendered.event_sample_lengths.clear();

        for model in config.chip_model.iter().take(count) {
            let mut sid = Sid::new();
            sid.set_chip_model(model.clone());
            let _ = sid.set_sampling_parameters(
                config.clock as f64,
                config.sampling_method.clone(),
                config.sample_rate as f64,
            );
            sid.enable_filter(true);

            if *model == chip_model::MOS6581 {
                sid.adjust_filter_bias(config.filter_bias_6581);
            }

            let boost = *model == chip_model::MOS8580 && config.digiboost;
            sid.set_voice_mask(if boost { VOICE_MASK_DIGIBOOST } else { VOICE_MASK_DEFAULT  });
            sid.input(if boost { i16::MIN } else { 0 });
            sid.clock_delta(0xffff);

            self.chips.push(Chip {
                sid,
                scratch: vec![0; SCRATCH_SAMPLES],
                event_sample_lengths: Vec::new(),
            });
        }
    }

    pub(super) fn read(&mut self, index: usize, reg: u32) -> u8 {
        self.chips[index].sid.read(reg) as u8
    }

    pub(super) fn set_digiboost(&mut self, config: &Config) {
        for (chip, model) in self.chips.iter_mut().zip(&config.chip_model) {
            if *model == chip_model::MOS8580 {
                chip.sid.set_voice_mask(if config.digiboost { VOICE_MASK_DIGIBOOST } else { VOICE_MASK_DEFAULT });
                chip.sid.input(if config.digiboost { i16::MIN } else { 0 });
            }
        }
    }

    pub(super) fn set_filter_bias(&mut self, config: &Config) {
        for (chip, model) in self.chips.iter_mut().zip(&config.chip_model) {
            if *model == chip_model::MOS6581 {
                chip.sid.adjust_filter_bias(config.filter_bias_6581);
            }
        }
    }

    pub(super) fn set_sampling_frequency(&mut self, frequency: u32) {
        for chip in &mut self.chips {
            chip.sid.adjust_sampling_frequency(frequency as f64);
        }
    }

    pub(super) fn render(&mut self, events: &[SidWrite]) -> &RenderedSids {
        let count = self.chips.len();
        let group_size = self.sids_per_task;
        let samples = &mut self.rendered.samples;
        match &self.pool {
            Some(pool) => {
                pool.install(|| {
                    self.chips
                        .par_chunks_mut(group_size)
                        .zip(samples.par_chunks_mut(group_size))
                        .enumerate()
                        .for_each(|(group, (chips, samples))| {
                            for (offset, (chip, output)) in chips.iter_mut().zip(samples).enumerate() {
                                chip.render(group * group_size + offset, count, events, output);
                            }
                        });
                });
            }
            None => {
                for (index, (chip, output)) in self.chips.iter_mut().zip(samples.iter_mut()).enumerate() {
                    chip.render(index, count, events, output);
                }
            }
        }

        let lengths = &self.chips[0].event_sample_lengths;
        self.rendered.event_sample_lengths.clear();
        self.rendered.event_sample_lengths.extend_from_slice(lengths);
        &self.rendered
    }
}

impl Chip {
    fn render(&mut self, index: usize, count: usize, events: &[SidWrite], output: &mut Vec<i16>) {
        output.clear();
        self.event_sample_lengths.clear();
        for event in events {
            let start = output.len();
            let mut cycles = u32::from(event.cycles);
            if cycles != 0 {
                while cycles != 0 {
                    let (length, remaining) = self.sid.sample(cycles, &mut self.scratch, 1);
                    output.extend_from_slice(&self.scratch[..length]);
                    cycles = remaining;
                }
                if !(0x200..0x300).contains(&event.reg) && usize::from(event.reg >> 5).min(count - 1) == index {
                    self.sid.write(u32::from(event.reg & 0x1f), u32::from(event.data));
                }
            }
            self.event_sample_lengths.push(output.len() - start);
        }
    }
}
