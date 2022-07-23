// Copyright (C) 2022 Wilfred Bos
// Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;

pub struct DeviceState {
    pub device_ready: Arc<AtomicBool>,
    pub restart: Arc<AtomicBool>,
    pub quit: Arc<AtomicBool>,
    pub error: Arc<AtomicBool>,
    pub error_msg: Arc<Mutex<String>>
}

impl DeviceState {
    pub fn new() -> DeviceState {
        DeviceState {
            device_ready: Arc::new(AtomicBool::new(false)),
            restart: Arc::new(AtomicBool::new(true)),
            quit: Arc::new(AtomicBool::new(false)),
            error: Arc::new(AtomicBool::new(false)),
            error_msg: Arc::new(Mutex::new(String::new()))
        }
    }

    pub fn init(&self) {
        self.restart.store(false, Ordering::SeqCst);
        self.quit.store(false, Ordering::SeqCst);
    }

    pub fn reset(&self) {
        self.restart.store(true, Ordering::SeqCst);
        self.quit.store(true, Ordering::SeqCst);
        self.error.store(false, Ordering::SeqCst);
    }

    pub fn set_error(&self, error_msg: String) {
        *self.error_msg.lock() = error_msg;
        self.error.store(true, Ordering::SeqCst);
        self.restart.store(true, Ordering::SeqCst);
        self.device_ready.store(true, Ordering::SeqCst);
    }

    pub fn clone(&self) -> DeviceState {
        DeviceState {
            device_ready: self.device_ready.clone(),
            restart: self.restart.clone(),
            quit: self.quit.clone(),
            error: self.error.clone(),
            error_msg: self.error_msg.clone()
        }
    }
}
