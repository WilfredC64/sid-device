// Copyright (C) 2022 Wilfred Bos
// Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.

use cpal::Device;
use cpal::traits::{DeviceTrait, HostTrait};

pub fn get_available_audio_output_device_names() -> (Vec<String>, i32) {
    let host = cpal::default_host();
    let default_device = host.default_output_device().unwrap();
    let default_device_name = default_device.name().unwrap();

    let mut default_device = 0_i32;
    let devices = get_available_audio_output_devices().iter().enumerate().map(|(index, device)| {
        let device_name = device.name().unwrap();
        if device_name.eq(&default_device_name) {
            default_device = index as i32;
        }
        device_name
    }).collect();
    (devices, default_device)
}

pub fn get_available_audio_output_devices() -> Vec<Device> {
    let host = cpal::default_host();

    if let Ok(devices) = host.output_devices() {
        devices.enumerate().map(|(_size, device)| device).collect()
    } else {
        vec![host.default_output_device().expect("Failed to find a default output device")]
    }
}

