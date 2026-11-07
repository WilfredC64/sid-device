// Copyright (C) 2026 Wilfred Bos
// Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.

use resid::{Sid, chip_model};
use thread_priority::{ThreadPriority, set_current_thread_priority};

use super::audio_renderer::{Config, SidWrite};

const SCRATCH_SAMPLES: usize = 8_192;

const PARALLEL_MIN_CYCLES: u32 = 2_000;

const VOICE_MASK_DEFAULT: u32 = 0x07;
const VOICE_MASK_DIGIBOOST: u32 = 0x0f;

struct Chip {
    sid: Sid,
    scratch: Vec<i16>,
    output: Vec<i16>,
    event_sample_lengths: Vec<usize>,
}

pub(super) struct SidEmulation {
    chips: Vec<Chip>,
    pool: Option<rayon::ThreadPool>,
    max_threads: usize,
    sids_per_task: usize,
    group_size: usize,
}

impl SidEmulation {
    pub(super) fn new(config: &Config) -> Self {
        let mut emulation = Self {
            chips: Vec::new(),
            pool: None,
            max_threads: std::thread::available_parallelism().map_or(1, usize::from),
            sids_per_task: config.sids_per_task.max(1),
            group_size: usize::MAX,
        };
        emulation.configure(config);
        emulation
    }

    fn configure_pool(&mut self, sid_count: usize) {
        let groups = sid_count.div_ceil(self.sids_per_task).min(self.max_threads);

        if groups <= 1 {
            self.pool = None;
            self.group_size = usize::MAX;
            return;
        }

        // Spread the chips evenly when there are fewer cores than groups.
        self.group_size = sid_count.div_ceil(groups);

        let workers = groups - 1;
        if self.pool.as_ref().is_some_and(|pool| pool.current_num_threads() == workers) {
            return;
        }

        self.pool = None;
        match rayon::ThreadPoolBuilder::new()
            .num_threads(workers)
            .thread_name(|i| format!("sid-render-{i}"))
            .start_handler(|_| {
                let _ = set_current_thread_priority(ThreadPriority::Max);
            })
            .build()
        {
            Ok(pool) => self.pool = Some(pool),
            Err(error) => {
                self.group_size = usize::MAX;
                println!("Cannot create SID render pool: {error}; using sequential rendering")
            }
        }
    }

    pub(super) fn configure(&mut self, config: &Config) {
        let count = config.sid_count as usize;
        self.configure_pool(count);
        self.chips.clear();

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
                output: Vec::new(),
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

    /// Renders all events on every chip. `total_cycles` is the sum of the event cycles and is
    /// only used to decide whether the batch is large enough to be worth rendering in parallel.
    pub(super) fn render(&mut self, events: &[SidWrite], total_cycles: u32) {
        let count = self.chips.len();
        let group_size = self.group_size;

        match &self.pool {
            Some(pool) if total_cycles >= PARALLEL_MIN_CYCLES => {
                let mut groups = self.chips.chunks_mut(group_size).enumerate();
                let first = groups.next();
                pool.in_place_scope(|scope| {
                    for (group, chips) in groups {
                        scope.spawn(move |_| render_group(chips, group * group_size, count, events));
                    }
                    // The calling thread does its share instead of idling while it waits.
                    if let Some((_, chips)) = first {
                        render_group(chips, 0, count, events);
                    }
                });
            }
            _ => render_group(&mut self.chips, 0, count, events),
        }
    }

    pub(super) fn is_parallel(&self) -> bool {
        self.pool.is_some()
    }

    pub(super) fn sample_count(&self) -> usize {
        self.chips.first().map_or(0, |chip| chip.output.len())
    }

    pub(super) fn samples(&self, index: usize) -> &[i16] {
        &self.chips[index].output
    }

    /// Number of samples produced for each event of the last rendered batch.
    pub(super) fn event_sample_lengths(&self) -> &[usize] {
        self.chips.first().map_or(&[], |chip| &chip.event_sample_lengths)
    }
}

fn render_group(chips: &mut [Chip], first_index: usize, count: usize, events: &[SidWrite]) {
    for (offset, chip) in chips.iter_mut().enumerate() {
        chip.render(first_index + offset, count, events);
    }
}

impl Chip {
    fn render(&mut self, index: usize, count: usize, events: &[SidWrite]) {
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
                if !(0x200..0x300).contains(&event.reg) && usize::from(event.reg >> 5).min(count - 1) == index {
                    self.sid.write(u32::from(event.reg & 0x1f), u32::from(event.data));
                }
            }
            self.event_sample_lengths.push(self.output.len() - start);
        }
    }
}
