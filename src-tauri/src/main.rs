// Copyright (C) 2022 - 2024 Wilfred Bos
// Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod device_state;
mod settings;
mod sid_device_listener;
mod sid_device_server;
mod utils;

use std::process::exit;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::{thread, time::Duration};
use async_broadcast::{broadcast, Receiver, Sender};
use parking_lot::Mutex;
use single_instance::SingleInstance;
use tauri::{tray::{MouseButton, TrayIconBuilder, TrayIconEvent}, App, AppHandle, Emitter, Listener, Manager, RunEvent, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent, Wry};
#[cfg(target_os = "macos")]
use tauri::ActivationPolicy;

use commands::{
    allow_external_ip_cmd,
    change_audio_device_cmd,
    change_filter_bias_6581_cmd,
    enable_digiboost_cmd,
    get_config_cmd,
    get_devices_cmd,
    reset_to_default_cmd,
    toggle_launch_at_start_cmd
};
use settings::Settings;
use sid_device_server::SidDeviceServer;
use tauri::menu::{CheckMenuItem, MenuBuilder};

use crate::device_state::DeviceState;
use crate::settings::Config;
use crate::sid_device_listener::SidDeviceListener;

type SidDeviceChannel = (Sender<(SettingsCommand, Option<i32>)>, Receiver<(SettingsCommand, Option<i32>)>);

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum SettingsCommand {
    SetAudioDevice,
    EnableDigiboost,
    DisableDigiboost,
    FilterBias6581
}

fn main() {
    let instance = SingleInstance::new("sid-device").unwrap();
    if !instance.is_single() {
        println!("ERROR: SID Device is already running\r");
        exit(1);
    }

    let (mut device_sender, device_receiver): SidDeviceChannel = broadcast(1);
    device_sender.set_overflow(true);

    let settings = Arc::new(Mutex::new(Settings::new()));

    let device_state = start_sid_device_thread(device_receiver, &settings);
    start_sid_device_detect_thread(&device_state, &settings);

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(device_state)
        .manage(settings.clone())
        .manage(device_sender.clone())
        .invoke_handler(tauri::generate_handler![
            get_devices_cmd,
            change_filter_bias_6581_cmd,
            toggle_launch_at_start_cmd,
            reset_to_default_cmd,
            change_audio_device_cmd,
            enable_digiboost_cmd,
            allow_external_ip_cmd,
            get_config_cmd
        ])
        .setup(move |app| {
            create_dialogs(app)?;
            setup_listeners(app, &settings);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building application");

    #[cfg(target_os = "macos")]
    let mut app = app;

    #[cfg(target_os = "macos")]
    app.set_activation_policy(ActivationPolicy::Accessory);

    app.run({
        move |app_handle, e| match e {
            RunEvent::ExitRequested { api: _, .. } => {
                let device_state = app_handle.state::<DeviceState>();
                device_state.quit.store(true, Ordering::SeqCst);
            }
            RunEvent::WindowEvent { label, event: WindowEvent::CloseRequested { api, .. }, .. } => {
                let app_handle = app_handle.clone();
                hide_window(&app_handle, &label);

                let device_state = app_handle.state::<DeviceState>();
                let quiting = device_state.quit.load(Ordering::SeqCst);
                if !quiting {
                    api.prevent_close();
                }
            }
            _ => {}
        }
    });
}

fn start_sid_device_thread(receiver: Receiver<(SettingsCommand, Option<i32>)>, settings: &Arc<Mutex<Settings>>) -> DeviceState {
    let device_state = DeviceState::new();

    let _sid_device_thread = thread::spawn({
        let settings_clone = settings.clone();
        let device_state = device_state.clone();

        move || {
            sid_device_loop(receiver, &settings_clone, device_state);
        }
    });

    device_state
}

fn sid_device_loop(receiver: Receiver<(SettingsCommand, Option<i32>)>, settings: &Arc<Mutex<Settings>>, device_state: DeviceState) {
    while device_state.restart.load(Ordering::SeqCst) {
        while device_state.error.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(500));
        }

        let mut sid_device_server = SidDeviceServer::new(settings.lock().get_config());

        device_state.init();

        let allow_external_connections = settings.lock().get_config().lock().allow_external_connections;

        let server_result = sid_device_server.start(allow_external_connections, receiver.clone(), device_state.device_ready.clone(), device_state.quit.clone());

        if let Err(server_result) = server_result {
            println!("ERROR: {server_result}\r");
            device_state.set_error(server_result);
        }
    }

    device_state.stopped.store(true, Ordering::SeqCst);
}

fn start_sid_device_detect_thread(device_state: &DeviceState, settings: &Arc<Mutex<Settings>>) {
    let _sid_device_detect_thread = thread::spawn({
        let device_state_clone = device_state.clone();
        let settings_clone = settings.clone();
        move || match SidDeviceListener::new() {
            Ok(listener) => sid_device_detect_loop(listener, &settings_clone, &device_state_clone),
            Err(err) => println!("ERROR: {err}\r")
        }
    });
}

fn sid_device_detect_loop(listener: SidDeviceListener, settings: &Arc<Mutex<Settings>>, device_state: &DeviceState) {
    loop {
        if device_state.stopped.load(Ordering::SeqCst) {
            break;
        }

        match listener.detect_client() {
            Ok(client) => {
                if let Some(client) = client {
                    let allow_external_connections = settings.lock().get_config().lock().allow_external_connections;

                    if allow_external_connections {
                        println!("Client detected with address: {}:{}", client.ip_address, client.port);

                        if let Err(err) = listener.respond(&client) {
                            println!("ERROR: Response could not be send: {err}\r");
                        }
                    }
                }
            }
            Err(err) => println!("ERROR: {err}\r"),
        }
    }
}

fn handle_menu_item_click(app_handle: &AppHandle<Wry>, id: &str, settings: &Arc<Mutex<Settings>>) {
    match id {
        "exit" => {
            exit_sid_device(app_handle);
        }
        "reset" => {
            let device_state = app_handle.state::<DeviceState>();
            device_state.reset();
        }
        "about" => {
            hide_window(app_handle, "settings");
            show_about_window(app_handle, "about");
        }
        "settings" => {
            hide_window(app_handle, "about");
            show_settings_window(app_handle, "settings", &settings.lock().get_config().lock());
        }
        "launch_at_startup" => {
            settings.lock().toggle_launch_at_start();

            let settings_window = app_handle.get_webview_window("settings");
            settings_window.unwrap().emit("update-settings", &*settings.lock().get_config().lock()).unwrap();
        }
        _ => {}
    }
}

fn exit_sid_device(app_handle: &AppHandle) {
    let windows = app_handle.webview_windows();
    for (_, window) in windows {
        window.close().unwrap();
    }

    let device_state = app_handle.state::<DeviceState>();
    device_state.quit.store(true, Ordering::SeqCst);
}

fn hide_window(app_handle: &AppHandle<Wry>, label_window: &str) {
    let window = app_handle.get_webview_window(label_window).unwrap();
    window.hide().unwrap();
    window.emit("hide", None::<String>).unwrap();
}

fn create_dialogs(app: &mut App<Wry>) -> Result<(), Box<dyn std::error::Error>> {
    let height_correction = if cfg!(target_os = "macos") { 44.0 } else { 0.0 };

    WebviewWindowBuilder::new(app, "about", WebviewUrl::App("/pages/about/index.html".into()))
        .title("SID Device - About")
        .inner_size(600.0, 512.0)
        .min_inner_size(600.0, 512.0 + height_correction)
        .center()
        .maximizable(false)
        .resizable(false)
        .fullscreen(false)
        .visible(false)
        .skip_taskbar(true)
        .build()?;

    WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("/pages/settings/index.html".into()))
        .title("SID Device - Settings")
        .inner_size(600.0, 450.0)
        .min_inner_size(600.0, 450.0 + height_correction)
        .center()
        .maximizable(false)
        .resizable(false)
        .fullscreen(false)
        .visible(false)
        .skip_taskbar(true)
        .build()?;

    Ok(())
}

fn setup_listeners(app: &mut App<Wry>, settings: &Arc<Mutex<Settings>>) {
    let app_handle = app.app_handle();

    let settings_window = app.get_webview_window("settings").unwrap();
    settings_window.listen("device-ready", {
        let app_handle = app_handle.clone();
        let settings_window_clone = settings_window.clone();

        move |_event| {
            let device_state = app_handle.state::<DeviceState>();

            if device_state.device_ready.load(Ordering::SeqCst) {
                device_state.device_ready.store(false, Ordering::SeqCst);

                if device_state.error.load(Ordering::SeqCst) {
                    let msg = device_state.error_msg.lock().to_owned();
                    settings_window_clone.emit("error", Some(msg)).unwrap();
                } else {
                    app_handle.tray_by_id("sid_device_tray_icon").unwrap().set_visible(true).unwrap();
                    settings_window_clone.emit("ready", None::<String>).unwrap();
                }
            }
        }
    });

    create_system_tray(app_handle, settings);

    let _id = settings_window.listen("retry", {
        let app_handle = app_handle.clone();

        move |_event| {
            let device_state = app_handle.state::<DeviceState>();
            device_state.error.store(false, Ordering::SeqCst);
        }
    });

    let _id = settings_window.listen("exit", {
        let app_handle = app_handle.clone();

        move |_event| {
            exit_sid_device(&app_handle);
        }
    });
}

fn create_system_tray(app: &AppHandle<Wry>, settings: &Arc<Mutex<Settings>>) {
    if app.tray_by_id("sid_device_tray_icon").is_some() {
        return;
    }

    let auto_launch_enabled = settings.lock().get_config().lock().launch_at_start_enabled;
    let launch_at_startup_menu_item = CheckMenuItem::with_id(app, "launch_at_startup", "Launch at startup", true, auto_launch_enabled, None::<&str>).unwrap();

    let menu = MenuBuilder::with_id(app, "sid_device_tray_menu")
        .text("about", "About")
        .text("settings", "Settings...")
        .separator()
        .item(&launch_at_startup_menu_item)
        .separator()
        .text("reset", "Reset connections")
        .separator()
        .text("exit", "Exit")
        .build()
        .unwrap();

    TrayIconBuilder::with_id("sid_device_tray_icon")
        .tooltip("SID Device")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .menu_on_left_click(false)
        .on_tray_icon_event({
            let settings = settings.clone();
            move |tray, event| {
                // refresh state CheckMenuItem for "Launch at startup"
                let launch_at_start = settings.lock().get_config().lock().launch_at_start_enabled;
                launch_at_startup_menu_item.set_checked(launch_at_start).unwrap();

                if let TrayIconEvent::DoubleClick { button: MouseButton::Left, .. } = event {
                    let app = tray.app_handle();

                    hide_window(app, "about");
                    show_settings_window(app, "settings", &settings.lock().get_config().lock());
                }
            }
        })
        .on_menu_event({
            let settings = settings.clone();
            move |app, event| {
                handle_menu_item_click(app, event.id.as_ref(), &settings);
            }
        })
        .build(app)
        .unwrap()
        .set_visible(false)
        .unwrap();
}

fn show_about_window(app: &AppHandle<Wry>, title: &str) {
    let popup_window = app.get_webview_window(title);

    if let Some(popup_window) = popup_window {
        popup_window.emit_to(title, "show", None::<String>).unwrap();

        show_window(&popup_window, "SID Device - About");
    }
}

fn show_settings_window(app: &AppHandle<Wry>, title: &str, config: &Config) {
    let popup_window = app.get_webview_window(title);

    if let Some(popup_window) = popup_window {
        popup_window.emit_to(title, "show", None::<String>).unwrap();

        show_window(&popup_window, "SID Device - Settings");

        popup_window
            .emit_to(title, "update-settings", config)
            .unwrap();
    }
}

fn show_window(popup_window: &WebviewWindow<Wry>, title: &str) {
    let visible = popup_window.is_visible().unwrap();

    popup_window.set_title(title).unwrap();
    popup_window.show().unwrap();
    popup_window.unminimize().unwrap();

    if !visible {
        popup_window.center().unwrap();
    }

    popup_window.set_focus().unwrap();
}
