// Copyright (C) 2021 - 2022 Wilfred Bos
// Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.

mod player;

use std::io::{self, ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream, Shutdown};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::{thread, time::Duration};

use async_broadcast::Receiver;
use parking_lot::Mutex;

use player::Player;
use crate::{Config, SettingsCommand};

const LOCAL_HOST: &str = "127.0.0.1";
const ALLOW_ALL_HOST: &str = "0.0.0.0";
const DEFAULT_PORT_NUMBER: &str = "6581";

const PROTOCOL_VERSION: u8 = 4;
const NUMBER_OF_DEVICES: u8 = 2;
const SID_WRITE_SIZE: usize = 4;

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub enum SidClock {
    Pal = 0,
    Ntsc = 1
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub enum SamplingMethod {
    Best = 0,
    Fast = 1
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
enum CommandResponse {
    Ok = 0,
    Busy,
    Error,
    Read,
    Version,
    Count,
    Info
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
enum Command {
    Flush = 0,
    TrySetSidCount,
    Mute,
    TryReset,
    TryDelay,
    TryWrite,
    TryRead,
    GetVersion,
    TrySetSampling,
    TrySetClock,
    GetConfigCount,
    GetConfigInfo,
    SetSidPosition,
    SetSidLevel,
    TrySetSidModel,
    SetDelay,
    SetFadeIn,
    SetFadeOut,
    SetPsidHeader,
    Unknown
}

impl Command {
    pub fn from_u8(value: u8) -> Command {
        match value {
            0 => Command::Flush,
            1 => Command::TrySetSidCount,
            2 => Command::Mute,
            3 => Command::TryReset,
            4 => Command::TryDelay,
            5 => Command::TryWrite,
            6 => Command::TryRead,
            7 => Command::GetVersion,
            8 => Command::TrySetSampling,
            9 => Command::TrySetClock,
            10 => Command::GetConfigCount,
            11 => Command::GetConfigInfo,
            12 => Command::SetSidPosition,
            13 => Command::SetSidLevel,
            14 => Command::TrySetSidModel,
            15 => Command::SetDelay,
            16 => Command::SetFadeIn,
            17 => Command::SetFadeOut,
            18 => Command::SetPsidHeader,
            _ => Command::Unknown,
        }
    }
}

pub struct SidDeviceServer {
    config: Arc<Mutex<Config>>,
    connection_count: Arc<AtomicI32>
}

impl SidDeviceServer {
    pub fn new(config: Arc<Mutex<Config>>) -> SidDeviceServer {
        let connection_count = Arc::new(AtomicI32::new(0));
        SidDeviceServer {
            config,
            connection_count
        }
    }

    pub fn start(
            &mut self,
            allow_external_connections: bool,
            receiver: Receiver<(SettingsCommand, Option<i32>)>,
            device_ready: Arc<AtomicBool>,
            quit: Arc<AtomicBool>) -> Result<(), String> {
        let host = if allow_external_connections {
            ALLOW_ALL_HOST
        } else {
            LOCAL_HOST
        };

        let listener = TcpListener::bind([host, DEFAULT_PORT_NUMBER].join(":"));
        if let Err(error) = listener {
            return Err(
                if error.kind() == ErrorKind::AddrInUse || error.kind() == ErrorKind::PermissionDenied {
                    "Another SID device seems to be already running on port 6581. Please close it and try again.".to_string()
                } else {
                    error.to_string()
                }
            );
        }

        let listener = listener.unwrap();
        listener.set_nonblocking(true).expect("Cannot set non-blocking");

        println!("Listening on: {}\r", listener.local_addr().unwrap());

        device_ready.store(true, Ordering::SeqCst);

        loop {
            match listener.accept() {
                Ok((stream, address)) => {
                    println!("New client connected: {address}\r");

                    let local_quit = quit.clone();
                    let receiver_clone: Receiver<(SettingsCommand, Option<i32>)> = receiver.clone();
                    let local_connection_count = self.connection_count.clone();
                    let config = self.config.clone();

                    let _ = thread::spawn(move || {
                        local_connection_count.fetch_add(1, Ordering::SeqCst);
                        let mut sid_device_thread = SidDeviceServerThread::new(config);
                        sid_device_thread.handle_client(stream, receiver_clone, local_quit);
                        local_connection_count.fetch_sub(1, Ordering::SeqCst);
                    });
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    if quit.load(Ordering::SeqCst) {
                        println!("User interruption. Quitting...\r");
                        break;
                    }
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Err(e) => {
                    println!("ERROR: {e}\r");
                    break;
                }
            }
        }

        // wait for connections to close
        while self.connection_count.load(Ordering::SeqCst) > 0 {
            thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    }
}

pub struct SidDeviceServerThread {
    player: Player
}

impl SidDeviceServerThread {
    pub fn new(config: Arc<Mutex<Config>>) -> SidDeviceServerThread {
        let config = config.lock();
        let device_numer = config.audio_device_number;

        let mut player = Player::new(device_numer);
        player.enable_digiboost(config.digiboost_enabled);
        player.set_filter_bias_6581(config.filter_bias_6581);

        SidDeviceServerThread {
            player
        }
    }

    fn handle_client(&mut self, mut stream: TcpStream, mut receiver: Receiver<(SettingsCommand, Option<i32>)>, quit: Arc<AtomicBool>) {
        let mut data = [0u8; 4096];
        stream.set_read_timeout(Some(Duration::from_millis(100))).unwrap();
        stream.set_write_timeout(Some(Duration::from_millis(100))).unwrap();
        stream.set_nonblocking(false).unwrap();

        loop {
            if quit.load(Ordering::SeqCst) {
                stream.shutdown(Shutdown::Both).unwrap();
                self.player.flush();
                break;
            }

            if let Ok((command, param1)) = receiver.try_recv() {
                match command {
                    SettingsCommand::SetAudioDevice => {
                        self.player.set_audio_device(param1);
                    }
                    SettingsCommand::EnableDigiboost => {
                        self.player.enable_digiboost(true);
                    }
                    SettingsCommand::DisableDigiboost => {
                        self.player.enable_digiboost(false);
                    }
                    SettingsCommand::FilterBias6581 => {
                        self.player.set_filter_bias_6581(param1);
                    }
                }
            }

            match stream.read(&mut data) {
                Ok(size) => {
                    if size >= 4 {
                        self.process_command(&mut stream, &data[0..size]).unwrap();
                    } else if size == 0 {
                        println!("Client disconnected: {}\r", stream.peer_addr().unwrap());
                        stream.shutdown(Shutdown::Both).unwrap();
                        break;
                    }
                }
                Err(e) if e.kind() == ErrorKind::TimedOut || e.kind() == ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    println!("ERROR: {}, {:?}\r", e, e.kind());
                    println!("Terminating connection for client: {}\r", stream.peer_addr().unwrap());
                    stream.shutdown(Shutdown::Both).unwrap();
                    break;
                }
            }
        }
    }

    fn process_command(&mut self, stream: &mut TcpStream, data: &[u8]) -> io::Result<()> {
        let command: Command = Command::from_u8(data[0]);

        if matches!(command, Command::Unknown) {
            println!("ERROR: Unknown command.\r");
            stream.write_all(&[CommandResponse::Error as u8])?;
            stream.flush()?;
            return Ok(());
        }

        let sid_number: u8 = data[1];
        let data_length: usize = ((data[2] as usize) << 8) + (data[3] as usize);

        if data_length > data.len() - 4 && !matches!(command, Command::Flush) {
            println!("ERROR: Not all data is retrieved. {} {} {}\r", command as u8, data_length, data.len() - 4);
            stream.write_all(&[CommandResponse::Error as u8])?;
            stream.flush()?;
            return Ok(());
        }

        match command {
            Command::TryWrite => {
                if self.player.has_error() {
                    println!("ERROR: Audio error occurred.\r");
                    stream.shutdown(Shutdown::Both)?;
                } else if data_length % 4 != 0 {
                    println!("ERROR: TryWrite write data size for write data.\r");
                    stream.write_all(&[CommandResponse::Error as u8])?;
                } else if !self.player.has_max_data_in_buffer() {
                    if data_length >= 4 {
                        let _ = self.process_writes(&data[4..]);
                    }
                    stream.write_all(&[CommandResponse::Ok as u8])?;
                } else {
                    stream.write_all(&[CommandResponse::Busy as u8])?;
                }
            }
            Command::TryRead => {
                if self.player.has_error() {
                    println!("ERROR: Audio error occurred.\r");
                    stream.shutdown(Shutdown::Both)?;
                } else if data_length < 3 || (data_length - 3) % 4 != 0 {
                    println!("ERROR: TryRead missing read data.\r");
                    stream.write_all(&[CommandResponse::Error as u8])?;
                } else if !self.player.has_max_data_in_buffer() {
                    let read_value = self.process_writes(&data[4..]);
                    stream.write_all(&[CommandResponse::Read as u8, read_value])?;
                } else {
                    stream.write_all(&[CommandResponse::Busy as u8])?;
                }
            }
            Command::TryDelay => {
                if self.player.has_error() {
                    println!("ERROR: Audio error occurred.\r");
                    stream.shutdown(Shutdown::Both)?;
                } else if data_length < 2 {
                    println!("ERROR: TryDelay missing cycle data.\r");
                    stream.write_all(&[CommandResponse::Error as u8])?;
                } else if !self.player.has_max_data_in_buffer() {
                    let cycles = ((data[4] as u16) << 8) + data[5] as u16;
                    self.player.write_to_sid(0x1e + sid_number * 0x20, 0, cycles);
                    if self.player.has_min_data_in_buffer() {
                        self.player.start_draining();
                    }
                    stream.write_all(&[CommandResponse::Ok as u8])?;
                } else {
                    stream.write_all(&[CommandResponse::Busy as u8])?;
                }
            }
            Command::TryReset => {
                if data_length == 1 {
                    if !self.player.has_max_data_in_buffer() {
                        self.player.reset();
                        stream.write_all(&[CommandResponse::Ok as u8])?;
                    } else {
                        stream.write_all(&[CommandResponse::Busy as u8])?;
                    }
                } else {
                    println!("ERROR: TryReset missing data for volume.\r");
                    stream.write_all(&[CommandResponse::Error as u8])?;
                }
            }
            Command::GetVersion => {
                stream.write_all(&[CommandResponse::Version as u8, PROTOCOL_VERSION])?;
            }
            Command::GetConfigCount => {
                stream.write_all(&[CommandResponse::Count as u8, NUMBER_OF_DEVICES])?;
            }
            Command::GetConfigInfo => {
                let mut response = vec![CommandResponse::Info as u8, sid_number & 0x01];
                if sid_number == 0 {
                    response.append(&mut b"reSID Device (6581)\0".to_vec());
                } else {
                    response.append(&mut b"reSID Device (8580)\0".to_vec());
                }
                stream.write_all(response.as_slice())?;
            }
            Command::Flush => {
                self.player.flush();
                stream.write_all(&[CommandResponse::Ok as u8])?;
            }
            Command::TrySetSidCount => {
                if sid_number > 0 && sid_number <= 8 {
                    self.player.set_sid_count(sid_number as i32);
                    stream.write_all(&[CommandResponse::Ok as u8])?;
                } else {
                    println!("ERROR: TrySetSidCount sid count should be in range 1..8.\r");
                    stream.write_all(&[CommandResponse::Error as u8])?;
                }
            }
            Command::TrySetSidModel => {
                if data_length == 1 {
                    let sid_model = data[4];
                    self.player.set_model(((sid_number as i32) << 8) | sid_model as i32);
                    stream.write_all(&[CommandResponse::Ok as u8])?;
                } else {
                    println!("ERROR: TrySetSidModel missing data for SID model.\r");
                    stream.write_all(&[CommandResponse::Error as u8])?;
                }
            }
            Command::TrySetClock => {
                if data_length == 1 {
                    let sid_clock = data[4];
                    self.player.set_clock(sid_clock as i32);
                    stream.write_all(&[CommandResponse::Ok as u8])?;
                } else {
                    println!("ERROR: TrySetClock missing data for clock.\r");
                    stream.write_all(&[CommandResponse::Error as u8])?;
                }
            }
            Command::SetSidPosition => {
                if data_length == 1 {
                    let position = data[4];
                    self.player.set_position(((sid_number as i32) << 8) | position as i32);
                    stream.write_all(&[CommandResponse::Ok as u8])?;
                } else {
                    println!("ERROR: SetSidPosition missing data for SID position.\r");
                    stream.write_all(&[CommandResponse::Error as u8])?;
                }
            }
            Command::TrySetSampling => {
                if data_length == 1 {
                    let sampling_method = data[4];
                    self.player.set_sampling_method(sampling_method as i32);
                    stream.write_all(&[CommandResponse::Ok as u8])?;
                } else {
                    println!("ERROR: TrySetSampling missing data for sampling method.\r");
                    stream.write_all(&[CommandResponse::Error as u8])?;
                }
            }
            _ => {
                // return Ok for not implemented methods
                stream.write_all(&[CommandResponse::Ok as u8])?;
            }
        }
        stream.flush()?;
        Ok(())
    }

    fn process_writes(&mut self, data: &[u8]) -> u8 {
        let number_of_sid_writes = data.len() / SID_WRITE_SIZE;
        let write_data_length = number_of_sid_writes * SID_WRITE_SIZE;

        for n in (0..write_data_length).step_by(SID_WRITE_SIZE) {
            let cycles = ((data[n] as u16) << 8) + data[n + 1] as u16;
            let reg = data[n + 2];
            let val = data[n + 3];
            self.player.write_to_sid(reg, val, cycles);
        }

        if self.player.has_min_data_in_buffer() {
            self.player.start_draining();
        }

        if data.len() == write_data_length + 3 {
            let cycles = ((data[write_data_length] as u16) << 8) + data[write_data_length + 1] as u16;
            let reg = data[write_data_length + 2];
            self.player.read_from_sid(reg, cycles)
        } else {
            0
        }
    }
}
