// Copyright (C) 2022 Wilfred Bos
// Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use async_broadcast::Sender;
use futures_lite::{future::block_on};
use parking_lot::Mutex;
use tauri::{AppHandle, command, State, Window, Wry};

use crate::device_state::DeviceState;
use crate::{Config, Settings, SettingsCommand};
use crate::toggle_launch_at_start;
use crate::utils::audio;

#[derive(serde::Serialize)]
pub struct DevicesResponse {
    devices: Vec<String>,
    default_device: i32
}

#[command]
pub fn get_devices_cmd() -> DevicesResponse {
    let (devices, default_device) = audio::get_available_audio_output_device_names();

    DevicesResponse {
        devices,
        default_device
    }
}

#[command]
pub fn change_filter_bias_6581_cmd(filter_bias_6581: i32, settings: State<'_, Arc<Mutex<Settings>>>, sender: State<'_, Sender<(SettingsCommand, Option<i32>)>>) {
    block_on(async {
        settings.lock().get_config().lock().filter_bias_6581 = Some(filter_bias_6581);

        let _ = sender.broadcast((SettingsCommand::FilterBias6581, Some(filter_bias_6581))).await.unwrap();
        settings.lock().save_config();
    });
}


#[command]
pub fn toggle_launch_at_start_cmd(app_handle: AppHandle<Wry>, settings: State<'_, Arc<Mutex<Settings>>>) {
    toggle_launch_at_start(&app_handle.tray_handle(), &settings, "launch at startup");
}

#[command]
pub fn reset_to_default_cmd(window: Window<Wry>, device_state: State<'_, DeviceState>, settings: State<'_, Arc<Mutex<Settings>>>) {
    settings.lock().reset_config();
    device_state.reset();

    window.emit("update-settings", &*settings.lock().get_config().lock()).unwrap();
}

#[command]
pub fn change_audio_device_cmd(device_index: i32, settings: State<'_, Arc<Mutex<Settings>>>, sender: State<'_, Sender<(SettingsCommand, Option<i32>)>>) {
    block_on(async {
        let audio_device_number = if device_index < 1 {
            None
        } else {
            Some(device_index - 1)
        };

        settings.lock().get_config().lock().audio_device_number = audio_device_number;
        let _ = sender.broadcast((SettingsCommand::SetAudioDevice, audio_device_number)).await.unwrap();
        settings.lock().save_config();
    });
}

#[command]
pub fn enable_digiboost_cmd(digi_boost_enabled: bool, settings: State<'_, Arc<Mutex<Settings>>>, sender: State<'_, Sender<(SettingsCommand, Option<i32>)>>) {
    block_on(async {
        settings.lock().get_config().lock().digiboost_enabled = digi_boost_enabled;

        let command = if digi_boost_enabled {
            SettingsCommand::EnableDigiboost
        } else {
            SettingsCommand::DisableDigiboost
        };

        let _ = sender.broadcast((command, None)).await.unwrap();
        settings.lock().save_config();
    });
}

#[command]
pub fn allow_external_ip_cmd(external_ip_allowed: bool, device_state: State<'_, DeviceState>, settings: State<'_, Arc<Mutex<Settings>>>) {
    settings.lock().get_config().lock().allow_external_connections = external_ip_allowed;

    device_state.device_ready.store(false, Ordering::SeqCst);
    device_state.reset();

    settings.lock().save_config();
}

#[command]
pub fn get_config_cmd(settings: State<'_, Arc<Mutex<Settings>>>) -> Config {
    *settings.lock().get_config().lock()
}
