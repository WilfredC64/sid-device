// Copyright (C) 2022 - 2026 Wilfred Bos
// Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.

#![allow(clippy::upper_case_acronyms)]

use cxx::UniquePtr;

#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("resid_bridge.h");

        #[namespace = "reSID"]
        type SID;

        #[namespace = "resid_bridge"]
        fn new_sid() -> UniquePtr<SID>;

        #[namespace = "resid_bridge"]
        fn set_chip_model(sid: Pin<&mut SID>, model: u32);

        #[namespace = "resid_bridge"]
        fn set_voice_mask(sid: Pin<&mut SID>, mask: u32);

        #[namespace = "resid_bridge"]
        fn enable_filter(sid: Pin<&mut SID>, enable: bool);

        #[namespace = "resid_bridge"]
        fn adjust_filter_bias(sid: Pin<&mut SID>, dac_bias: f64);

        #[namespace = "resid_bridge"]
        fn enable_external_filter(sid: Pin<&mut SID>, enable: bool);

        #[namespace = "resid_bridge"]
        fn set_sampling_parameters(sid: Pin<&mut SID>, clock_freq: f64, method: u32, sample_freq: f64, pass_freq: f64, filter_scale: f64) -> bool;

        #[namespace = "resid_bridge"]
        fn adjust_sampling_frequency(sid: Pin<&mut SID>, sample_freq: f64);

        #[namespace = "resid_bridge"]
        fn clock(sid: Pin<&mut SID>);

        #[namespace = "resid_bridge"]
        fn clock_delta(sid: Pin<&mut SID>, delta_t: i32);

        #[namespace = "resid_bridge"]
        fn clock_buffer(sid: Pin<&mut SID>, delta_t: &mut i32, buf: &mut [i16], interleave: i32) -> i32;

        #[namespace = "resid_bridge"]
        fn reset(sid: Pin<&mut SID>);

        #[namespace = "resid_bridge"]
        fn read(sid: Pin<&mut SID>, reg: u32) -> u32;

        #[namespace = "resid_bridge"]
        fn write(sid: Pin<&mut SID>, reg: u32, value: u32);

        #[namespace = "resid_bridge"]
        fn input(sid: Pin<&mut SID>, sample: i16);
    }
}

const FILTER_SCALE: f64 = 0.97;

#[repr(u32)]
#[derive(Clone, PartialEq)]
#[allow(non_camel_case_types)]
pub enum chip_model {
    MOS6581 = 0,
    MOS8580 = 1,
}

#[repr(u32)]
#[derive(Clone, PartialEq)]
#[allow(non_camel_case_types)]
pub enum sampling_method {
    SAMPLE_FAST = 0,
    SAMPLE_INTERPOLATE = 1,
    SAMPLE_RESAMPLE = 2,
    SAMPLE_RESAMPLE_FASTMEM = 3,
}

pub struct Sid {
    sid: UniquePtr<ffi::SID>,
}

impl Default for Sid {
    fn default() -> Self {
        Self::new()
    }
}

impl Sid {
    pub fn new() -> Self {
        Sid { sid: ffi::new_sid() }
    }

    pub fn adjust_filter_bias(&mut self, dac_bias: f64) {
        ffi::adjust_filter_bias(self.sid.pin_mut(), dac_bias);
    }

    pub fn set_chip_model(&mut self, model: chip_model) {
        ffi::set_chip_model(self.sid.pin_mut(), model as u32);
    }

    pub fn set_sampling_parameters(&mut self, clock_freq: f64, method: sampling_method, sample_freq: f64) -> bool {
        let pass_freq = sample_freq * 0.9 / 2.0;
        ffi::set_sampling_parameters(self.sid.pin_mut(), clock_freq, method as u32, sample_freq, pass_freq, FILTER_SCALE)
    }

    pub fn adjust_sampling_frequency(&mut self, sample_freq: f64) {
        ffi::adjust_sampling_frequency(self.sid.pin_mut(), sample_freq);
    }

    pub fn enable_filter(&mut self, enable: bool) {
        ffi::enable_filter(self.sid.pin_mut(), enable);
    }

    pub fn enable_external_filter(&mut self, enable: bool) {
        ffi::enable_external_filter(self.sid.pin_mut(), enable);
    }

    pub fn set_voice_mask(&mut self, mask: u32) {
        ffi::set_voice_mask(self.sid.pin_mut(), mask);
    }

    pub fn input(&mut self, sample: i16) {
        ffi::input(self.sid.pin_mut(), sample);
    }

    pub fn reset(&mut self) {
        ffi::reset(self.sid.pin_mut());
    }

    pub fn read(&mut self, reg: u32) -> u32 {
        ffi::read(self.sid.pin_mut(), reg)
    }

    pub fn write(&mut self, reg: u32, data: u32) {
        ffi::write(self.sid.pin_mut(), reg, data);
    }

    pub fn clock(&mut self) {
        ffi::clock(self.sid.pin_mut());
    }

    pub fn clock_delta(&mut self, cycles: u32) {
        ffi::clock_delta(self.sid.pin_mut(), cycles as i32);
    }

    pub fn sample(&mut self, cycles: u32, buffer: &mut [i16], interleave: i32) -> (usize, u32) {
        let mut delta = cycles as i32;
        let offset = ffi::clock_buffer(self.sid.pin_mut(), &mut delta, buffer, interleave);
        (offset as usize, delta as u32)
    }
}
