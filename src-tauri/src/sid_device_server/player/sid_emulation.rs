// Copyright (C) 2026 Wilfred Bos
// Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.

use resid::{Sid, chip_model};
use thread_priority::{ThreadPriority, set_current_thread_priority};

use super::audio_renderer::{Config, SidWrite};

const SCRATCH_SAMPLES: usize = 8_192;
const MAX_SIDS_PER_THREAD: usize = 2;
const PARALLEL_MIN_CYCLES: u32 = 2_000;

const VOICE_MASK_DEFAULT: u32 = 0x07;
const VOICE_MASK_DIGIBOOST: u32 = 0x0f;

struct Chip {
    sid: Sid,
    index: usize,
    is_last: bool,
    scratch: Vec<i16>,
    output: Vec<i16>,
    event_sample_lengths: Vec<usize>,
}

struct ParallelRenderer {
    pool: rayon::ThreadPool,
    group_size: usize,
}

pub(super) struct SidEmulation {
    chips: Vec<Chip>,
    parallel: Option<ParallelRenderer>,
    max_threads: usize,
}

impl SidEmulation {
    pub(super) fn new(config: &Config) -> Self {
        let mut emulation = Self {
            chips: Vec::new(),
            parallel: None,
            max_threads: std::thread::available_parallelism().map_or(1, usize::from),
        };
        emulation.configure(config);
        emulation
    }

    fn configure_parallel(&mut self, sid_count: usize) {
        let groups = sid_count.div_ceil(MAX_SIDS_PER_THREAD).min(self.max_threads);

        if groups <= 1 {
            self.parallel = None;
            return;
        }

        let group_size = sid_count.div_ceil(groups);

        // The calling thread renders one group itself, so the pool needs one worker less.
        let workers = groups - 1;

        if let Some(parallel) = &mut self.parallel
            && parallel.pool.current_num_threads() == workers
        {
            parallel.group_size = group_size;
            return;
        }

        self.parallel = match rayon::ThreadPoolBuilder::new()
            .num_threads(workers)
            .thread_name(|i| format!("sid-render-{i}"))
            .start_handler(|_| {
                let _ = set_current_thread_priority(ThreadPriority::Max);
            })
            .build()
        {
            Ok(pool) => Some(ParallelRenderer { pool, group_size }),
            Err(error) => {
                println!("Cannot create SID render pool: {error}; using sequential rendering");
                None
            }
        };
    }

    pub(super) fn configure(&mut self, config: &Config) {
        let count = config.sid_count as usize;
        self.configure_parallel(count);

        let models = config.chip_model.iter().take(count);
        let total = models.len();
        self.chips = models
            .enumerate()
            .map(|(index, model)| Chip::new(index, index + 1 == total, model, config))
            .collect();
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

    pub(super) fn render(&mut self, events: &[SidWrite], total_cycles: u32) {
        match &self.parallel {
            Some(parallel) if total_cycles >= PARALLEL_MIN_CYCLES => {
                let mut groups = self.chips.chunks_mut(parallel.group_size);
                let first = groups.next();
                parallel.pool.in_place_scope(|scope| {
                    for chips in groups {
                        scope.spawn(move |_| render_group(chips, events));
                    }
                    // The calling thread does its share instead of idling while it waits.
                    if let Some(chips) = first {
                        render_group(chips, events);
                    }
                });
            }
            _ => render_group(&mut self.chips, events),
        }
    }

    pub(super) fn is_parallel(&self) -> bool {
        self.parallel.is_some()
    }

    pub(super) fn sample_count(&self) -> usize {
        self.chips.first().map_or(0, |chip| chip.output.len())
    }

    pub(super) fn samples(&self, index: usize) -> &[i16] {
        &self.chips[index].output
    }

    pub(super) fn event_sample_lengths(&self) -> &[usize] {
        self.chips.first().map_or(&[], |chip| &chip.event_sample_lengths)
    }
}

fn render_group(chips: &mut [Chip], events: &[SidWrite]) {
    for chip in chips {
        chip.render(events);
    }
}

impl Chip {
    fn new(index: usize, is_last: bool, model: &chip_model, config: &Config) -> Self {
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
        sid.set_voice_mask(if boost { VOICE_MASK_DIGIBOOST } else { VOICE_MASK_DEFAULT });
        sid.input(if boost { i16::MIN } else { 0 });
        sid.clock_delta(0xffff);

        Self {
            sid,
            index,
            is_last,
            scratch: vec![0; SCRATCH_SAMPLES],
            output: Vec::new(),
            event_sample_lengths: Vec::new(),
        }
    }

    fn is_event_for_this_sid(&self, reg: u16) -> bool {
        if (0x200..0x300).contains(&reg) {
            return false; // FM-OPL register space
        }
        let target = usize::from(reg >> 5);
        if self.is_last { target >= self.index } else { target == self.index }
    }

    fn render(&mut self, events: &[SidWrite]) {
        self.output.clear();
        self.event_sample_lengths.clear();
        for event in events {
            let start = self.output.len();
            let mut cycles = u32::from(event.cycles);
            if cycles != 0 {
                while cycles != 0 {
                    let (length, remaining) = self.sid.sample(cycles, &mut self.scratch, 1);
                    self.output.extend_from_slice(&self.scratch[..length]);
                    cycles = remaining;
                }
                if self.is_event_for_this_sid(event.reg) {
                    self.sid.write(u32::from(event.reg & 0x1f), u32::from(event.data));
                }
            }
            self.event_sample_lengths.push(self.output.len() - start);
        }
    }
}
