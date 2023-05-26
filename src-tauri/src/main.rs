// Copyright (C) 2022 - 2023 Wilfred Bos
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

use std::{thread, time::Duration};
use std::process::exit;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use async_broadcast::{broadcast, Receiver, Sender};
use parking_lot::Mutex;
use single_instance::SingleInstance;
use tauri::api::dialog::ask;
use tauri::window::WindowBuilder;
use tauri::{
    App,
    AppHandle,
    CustomMenuItem,
    Manager,
    RunEvent,
    SystemTray,
    SystemTrayEvent,
    SystemTrayHandle,
    SystemTrayMenu,
    SystemTrayMenuItem,
    WindowEvent,
    Window,
    Wry
};

use commands::{
    get_devices_cmd,
    change_filter_bias_6581_cmd,
    toggle_launch_at_start_cmd,
    reset_to_default_cmd,
    change_audio_device_cmd,
    enable_digiboost_cmd,
    allow_external_ip_cmd,
    get_config_cmd
};
use settings::Settings;
use sid_device_server::SidDeviceServer;

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

    let (mut device_sender, device_receiver):SidDeviceChannel = broadcast(1);
    device_sender.set_overflow(true);

    let settings = Arc::new(Mutex::new(Settings::new()));
    let system_tray = create_system_tray(settings.lock().get_config().lock().launch_at_start_enabled);

    let device_state = start_sid_device_thread(device_receiver, &settings);
    start_sid_device_detect_thread(&device_state, &settings);

    let app = tauri::Builder::default()
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
        .system_tray(system_tray)
        .on_page_load(move |window, _| {
            window.hide().unwrap();
        })
        .setup(move |app| {
            create_dialogs(app)?;
            setup_listeners(app);
            Ok(())
        })
        .on_system_tray_event(
            move |app_handle, event| match event {
                SystemTrayEvent::DoubleClick { position: _, size: _, .. } => {
                    hide_window(app_handle, "about");
                    show_settings_window(app_handle, "settings", &settings.lock().get_config().lock());
                }
                SystemTrayEvent::MenuItemClick { id, .. } => {
                    handle_menu_item_click(app_handle, &id, &settings);
                }
                _ => {}
            }
        )
        .build(tauri::generate_context!())
        .expect("error while building application");

    app.run({
        move |app_handle, e| match e {
            RunEvent::ExitRequested { api: _, .. } => {
                let device_state = app_handle.state::<DeviceState>();
                device_state.quit.store(true, Ordering::SeqCst);
            }
            RunEvent::WindowEvent { label,  event: WindowEvent::CloseRequested { api, .. }, .. } => {
                let app_handle = app_handle.clone();
                hide_window(&app_handle, &label);
                api.prevent_close();
            }
            RunEvent::WindowEvent { label,  event: WindowEvent::Moved { .. }, .. } => {
                let app_handle = app_handle.clone();
                let window = app_handle.get_window(&label).unwrap();
                // emit a blur event to fix an issue in Windows that a select box stays opened when moving the window
                window.emit("blur", None::<String>).unwrap();
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

        let server_result = sid_device_server.start(allow_external_connections,receiver.clone(), device_state.device_ready.clone(), device_state.quit.clone());

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
        move || {
            match SidDeviceListener::new() {
                Ok(listener) => sid_device_detect_loop(listener, &settings_clone, &device_state_clone),
                Err(err) => println!("ERROR: {err}\r")
            }
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
            },
            Err(err) => println!("ERROR: {err}\r")
        }
    }
}

fn handle_menu_item_click(app_handle: &AppHandle<Wry>, id: &str, settings: &Arc<Mutex<Settings>>) {
    match id {
        "exit" => {
            close_window(app_handle, "about");
            close_window(app_handle, "settings");
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
        "launch at startup" => {
            toggle_launch_at_start(&app_handle.tray_handle(), settings, id);

            let settings_window = app_handle.get_window("settings");
            settings_window.unwrap().emit("update-settings", &*settings.lock().get_config().lock()).unwrap();
        }
        _ => {}
    }
}

fn hide_window(app_handle: &AppHandle<Wry>, label_window: &str) {
    let window = app_handle.get_window(label_window).unwrap();
    window.hide().unwrap();
    window.emit("hide", None::<String>).unwrap();
}

fn close_window(app_handle: &AppHandle<Wry>, label_window: &str) {
    let window = app_handle.get_window(label_window);

    if let Some(window) = window {
        window.close().unwrap();
    } else {
        app_handle.exit(0);
    }
}

fn create_dialogs(app: &mut App<Wry>) -> Result<(), Box<dyn std::error::Error>> {
    let height_correction = if cfg!(target_os = "macos") { 44.0 } else { 0.0 };

    WindowBuilder::new(
        app,
        "about".to_string(),
        tauri::WindowUrl::App("/pages/about/index.html".into()))
        .title("SID Device - About")
        .inner_size(600.0, 512.0)
        .min_inner_size(600.0, 512.0 + height_correction)
        .center()
        .resizable(false)
        .fullscreen(false)
        .visible(false)
        .skip_taskbar(true)
        .build()?;

    WindowBuilder::new(
        app,
        "settings".to_string(),
        tauri::WindowUrl::App("/pages/settings/index.html".into()))
        .title("SID Device - Settings")
        .inner_size(600.0, 450.0)
        .min_inner_size(600.0, 450.0 + height_correction)
        .center()
        .resizable(false)
        .fullscreen(false)
        .visible(false)
        .skip_taskbar(true)
        .build()?;

    Ok(())
}

fn setup_listeners(app: &mut App<Wry>) {
    let about_window = app.get_window("about").unwrap();
    let settings_window = app.get_window("settings").unwrap();

    let _id = settings_window.listen("device-ready", {
        let app_handle = app.app_handle();
        let settings_window_clone = settings_window.clone();

        move |_event| {
            let device_state = app_handle.state::<DeviceState>();

            if device_state.device_ready.load(Ordering::SeqCst) {
                device_state.device_ready.store(false, Ordering::SeqCst);

                if device_state.error.load(Ordering::SeqCst) {
                    let error_clone = device_state.error.clone();
                    let about_window_clone = about_window.clone();
                    let settings_window_clone = settings_window_clone.clone();

                    let msg = device_state.error_msg.lock().to_owned() + "\r\rTry again?";

                    ask(None::<&Window<Wry>>, "SID-Device Error", msg, move |answer| {
                        if answer {
                            error_clone.store(false, Ordering::SeqCst);
                        } else {
                            about_window_clone.close().unwrap();
                            settings_window_clone.close().unwrap();
                        }
                    });
                } else {
                    settings_window_clone.emit("ready", None::<String>).unwrap();
                }
            }
        }
    });
}

fn toggle_launch_at_start(system_tray_handle: &SystemTrayHandle<Wry>, settings: &Arc<Mutex<Settings>>, menu_id: &str) {
    let launch_at_start = settings.lock().toggle_launch_at_start();

    let item_handle = system_tray_handle.get_item(menu_id);
    item_handle.set_selected(launch_at_start).unwrap();
}

fn create_system_tray(auto_launch_enabled: bool) -> SystemTray {
    let menu_item_about = CustomMenuItem::new("about".to_string(), "About");
    let menu_item_settings = CustomMenuItem::new("settings".to_string(), "Settings...");
    let mut menu_item_launch_startup = CustomMenuItem::new("launch at startup".to_string(), "Launch at startup");
    menu_item_launch_startup.selected = auto_launch_enabled;

    let menu_item_reset_connections = CustomMenuItem::new("reset".to_string(), "Reset connections");
    let menu_item_exit = CustomMenuItem::new("exit".to_string(), "Exit");

    let tray_menu = SystemTrayMenu::new()
        .add_item(menu_item_about)
        .add_item(menu_item_settings)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(menu_item_launch_startup)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(menu_item_reset_connections)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(menu_item_exit);

    SystemTray::new().with_tooltip("SID Device").with_menu(tray_menu)
}

fn show_about_window(app: &AppHandle<Wry>, title: &str) {
    let popup_window = app.get_window(title);

    if let Some(popup_window) = popup_window {
        popup_window.emit_to(title, "show", None::<String>).unwrap();

        show_window(&popup_window, "SID Device - About");
    }
}

fn show_settings_window(app: &AppHandle<Wry>, title: &str, config: &Config) {
    let popup_window = app.get_window(title);

    if let Some(popup_window) = popup_window {
        popup_window.emit_to(title, "show", None::<String>).unwrap();

        show_window(&popup_window, "SID Device - Settings");

        popup_window.emit_to(title, "update-settings", config).unwrap();
    }
}

fn show_window(popup_window: &Window<Wry>, title: &str) {
    let visible = popup_window.is_visible().unwrap();

    popup_window.set_title(title).unwrap();
    popup_window.show().unwrap();
    popup_window.unminimize().unwrap();

    if !visible {
        popup_window.center().unwrap();
    }

    popup_window.set_focus().unwrap();
}
