// Copyright (C) 2022 Wilfred Bos
// Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use app_dirs2::*;
use auto_launch::{AutoLaunch, AutoLaunchBuilder};
use parking_lot::Mutex;

const APP_INFO: AppInfo = AppInfo{ name: "siddevice", author: "siddevice" };
const CONFIG_FILE_NAME: &str = "config.json";
const DEFAULT_FILTER_BIAS_6581: i32 = 24;
const WRITE_CONFIG_DELAY_IN_SEC: u64 = 2;

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub digiboost_enabled: bool,
    pub allow_external_connections: bool,
    pub audio_device_number: Option<i32>,
    pub filter_bias_6581: Option<i32>,
    pub default_filter_bias_6581: i32,
    pub launch_at_start_enabled: bool
}

impl Config {
    pub fn new(
        digiboost_enabled: bool,
        launch_at_start_enabled: bool,
        allow_external_connections: bool,
        audio_device_number: Option<i32>,
        filter_bias_6581: Option<i32>,
        default_filter_bias_6581: i32
    ) -> Config {
        Config {
            digiboost_enabled,
            launch_at_start_enabled,
            allow_external_connections,
            audio_device_number,
            filter_bias_6581,
            default_filter_bias_6581
        }
    }
}

pub struct Settings {
    config: Arc<Mutex<Config>>,
    auto_launch: AutoLaunch,
    save_in_progress: Arc<AtomicBool>,
    last_save: Arc<Mutex<Instant>>
}

impl Settings {
    pub fn new() -> Settings {
        let save_in_progress = Arc::new(AtomicBool::new(false));
        let last_save = Arc::new(Mutex::new(Instant::now()));

        let auto_launch = AutoLaunchBuilder::new()
            .set_app_name("sid-device")
            .set_app_path(std::env::current_exe().unwrap().to_str().unwrap())
            .set_use_launch_agent(true)
            .build()
            .unwrap();

        let config = Arc::new(Mutex::new(Self::load_config(auto_launch.is_enabled().unwrap())));

        Settings {
            auto_launch,
            config,
            save_in_progress,
            last_save
        }
    }

    pub fn save_config(&mut self) {
        *self.last_save.lock() = Instant::now();

        let save_in_progress = self.save_in_progress.load(Ordering::SeqCst);
        if !save_in_progress {
            let last_save_clone = self.last_save.clone();
            let save_in_progress_clone = self.save_in_progress.clone();

            let _saving_thread = thread::spawn({
                self.save_in_progress.store(true, Ordering::SeqCst);

                let config_clone = self.config.clone();
                move || loop {
                    thread::sleep(Duration::from_millis(100));

                    if last_save_clone.lock().elapsed().as_secs() < WRITE_CONFIG_DELAY_IN_SEC {
                        continue;
                    }

                    let config_filename = Self::get_config_filename();
                    let writer = BufWriter::new(File::create(&config_filename).unwrap());
                    serde_json::to_writer(writer, &*config_clone.lock()).unwrap();

                    save_in_progress_clone.store(false, Ordering::SeqCst);
                    break;
                }
            });
        }
    }

    pub fn get_config(&mut self) -> Arc<Mutex<Config>> {
        self.config.clone()
    }

    pub fn reset_config(&mut self) {
        self.config = Arc::new(Mutex::new(Self::get_default_config(self.auto_launch.is_enabled().unwrap())));
        self.save_config();
    }

    pub fn toggle_launch_at_start(&mut self) -> bool {
        let auto_launch_enabled = self.auto_launch.is_enabled().unwrap();
        if auto_launch_enabled {
            self.auto_launch.disable().unwrap();
        } else {
            self.auto_launch.enable().unwrap();
        }

        let mut config = self.config.lock();
        config.launch_at_start_enabled = !auto_launch_enabled;

        !auto_launch_enabled
    }

    fn get_config_filename() -> PathBuf {
        let app_root = app_root(AppDataType::UserConfig, &APP_INFO).unwrap();
        let path = Path::new(app_root.as_os_str());
        path.join(CONFIG_FILE_NAME)
    }

    fn load_config(auto_launch_enabled: bool) -> Config {
        let config_filename = Self::get_config_filename();
        if Path::new(config_filename.as_path()).exists() {
            let file = File::open(&config_filename).unwrap();
            let reader = BufReader::new(file);
            let config: Option<Config> = serde_json::from_reader(reader).ok();

            if let Some(mut config) = config {
                if config.filter_bias_6581.is_none() {
                    config.filter_bias_6581 = Some(DEFAULT_FILTER_BIAS_6581);
                }
                config.default_filter_bias_6581 = DEFAULT_FILTER_BIAS_6581;

                config.launch_at_start_enabled = auto_launch_enabled;
                return config;
            }
        }
        Self::get_default_config(auto_launch_enabled)
    }

    fn get_default_config(auto_launch_enabled: bool) -> Config {
        Config::new(
            false,
            auto_launch_enabled,
            false,
            None,
            Some(DEFAULT_FILTER_BIAS_6581),
            DEFAULT_FILTER_BIAS_6581
        )
    }
}
