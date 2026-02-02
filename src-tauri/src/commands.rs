// Copyright (C) 2022 - 2024 Wilfred Bos
// Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use async_broadcast::Sender;
use futures_lite::future::block_on;
use parking_lot::Mutex;
use tauri::{command, AppHandle, Emitter, State, WebviewWindow, Wry};

#[cfg(desktop)]
use tauri_plugin_autostart::ManagerExt;

use crate::device_state::DeviceState;
use crate::utils::audio;
use crate::{Config, Settings, SettingsCommand};

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
    #[cfg(desktop)]
    {
        let autolaunch_enabled = app_handle.autolaunch().is_enabled().unwrap();
        if autolaunch_enabled {
            app_handle.autolaunch().disable().unwrap();
        } else {
            app_handle.autolaunch().enable().unwrap();
        }
        settings.lock().set_launch_at_start(!autolaunch_enabled);
    }
}

#[command]
pub fn reset_to_default_cmd(window: WebviewWindow<Wry>, device_state: State<'_, DeviceState>, settings: State<'_, Arc<Mutex<Settings>>>) {
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

    device_state.device_ready.store(false, Ordering::Relaxed);
    device_state.reset();

    settings.lock().save_config();
}

#[command]
pub fn get_config_cmd(settings: State<'_, Arc<Mutex<Settings>>>) -> Config {
    *settings.lock().get_config().lock()
}
