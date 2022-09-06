// Copyright (C) 2022 Wilfred Bos
// Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.

#![allow(clippy::upper_case_acronyms)]
use autocxx::prelude::*;

use ffi::reSID::*;
use std::pin::Pin;

include_cpp! {
    #include "resid10/sid.h"
    safety!(unsafe)
    generate!("reSID::SID")
}

const FILTER_SCALE: f64 = 0.97;

pub struct Sid {
    sid: cxx::UniquePtr<SID>
}

impl Default for Sid {
    fn default() -> Self {
        Self::new()
    }
}

impl Sid {
    pub fn new() -> Self {
        let mut sid = Sid {
            sid: SID::new().within_unique_ptr()
        };
        // always call adjust_filter_bias to ensure all buffers are initialized
        sid.adjust_filter_bias(0.0);
        sid
    }

    pub fn adjust_filter_bias(&mut self, dac_bias: f64) {
        SID::adjust_filter_bias(self.sid.pin_mut(), dac_bias);
    }

    pub fn set_chip_model(&mut self, model: chip_model) {
        SID::set_chip_model(self.sid.pin_mut(), model);
    }

    pub fn set_sampling_parameters(&mut self, clock_freq: f64, method: sampling_method, sample_freq: f64) -> bool {
        let pass_freq = sample_freq * 0.9 / 2.0;
        SID::set_sampling_parameters(self.sid.pin_mut(), clock_freq, method, sample_freq, pass_freq, FILTER_SCALE)
    }

    pub fn adjust_sampling_frequency(&mut self, sample_freq: f64) {
        SID::adjust_sampling_frequency(self.sid.pin_mut(), sample_freq)
    }

    pub fn enable_filter(&mut self, enable: bool) {
        SID::enable_filter(self.sid.pin_mut(), enable);
    }

    pub fn enable_external_filter(&mut self, enable: bool) {
        SID::enable_external_filter(self.sid.pin_mut(), enable);
    }

    pub fn set_voice_mask(&mut self, mask: u32) {
        SID::set_voice_mask(self.sid.pin_mut(), c_uint::from(mask));
    }

    pub fn input(&mut self, sample: i16) {
        SID::input(self.sid.pin_mut(), c_short::from(sample));
    }

    pub fn reset(&mut self) {
        SID::reset(self.sid.pin_mut());
    }

    pub fn read(&mut self, reg: u32) -> u32 {
        u32::from(SID::read(self.sid.pin_mut(), c_uint::from(reg)))
    }

    pub fn write(&mut self, reg: u32, data: u32) {
        SID::write(self.sid.pin_mut(), c_uint::from(reg), c_uint::from(data));
    }

    pub fn clock(&mut self) {
        SID::clock(self.sid.pin_mut());
    }

    pub fn clock_delta(&mut self, cycles: u32) {
        SID::clock1(self.sid.pin_mut(), c_int::from(cycles as i32));
    }

    pub fn sample(&mut self, cycles: u32, buffer: &mut [i16], interleave: i32) -> (usize, u32) {
        let mut delta = c_int::from(cycles as i32);
        let offset = unsafe {
            SID::clock2(
                self.sid.pin_mut(),
                Pin::new(&mut delta),
                buffer.as_mut_ptr() as *mut c_short,
                c_int::from(buffer.len() as i32),
                c_int::from(interleave)
            )
        };
        (i32::from(offset) as usize, i32::from(delta) as u32)
    }
}

pub use ffi::reSID::{chip_model, sampling_method};
