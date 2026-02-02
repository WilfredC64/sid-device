// Copyright (C) 2022 - 2026 Wilfred Bos
// Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.

use cpal::traits::{DeviceTrait, HostTrait};
use cpal::Device;

pub fn get_available_audio_output_device_names() -> (Vec<String>, i32) {
    let host = cpal::default_host();
    let default_device_name = host
        .default_output_device()
        .map(|d| get_device_display_name(&d)).unwrap_or_default();

    let mut device_names = Vec::new();
    let mut default_index = 0_i32;

    if let Ok(devices) = host.output_devices() {
        for (i, device) in devices.enumerate() {
            let name = get_device_display_name(&device);
            if name == default_device_name {
                default_index = i as i32;
            }
            device_names.push(name);
        }
    }

    (device_names, default_index)
}

pub fn get_device_display_name(device: &Device) -> String {
    device.description().ok().map(|desc| {
        match desc.driver() {
            Some(driver) => format!("{} ({})", desc.name(), driver),
            None => desc.name().to_string(),
        }
    }).unwrap_or("Default".to_string())
}
