// Copyright (C) 2023 Wilfred Bos
// Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.

use std::{io, thread};
use std::io::ErrorKind;
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

const LISTENING_ADDRESS : &str = "0.0.0.0";
const LISTENING_PORT : &str = "6581";

const MAX_DATA_SIZE: usize = 512;

const MAGIC_ID: &str = "SidDevice";

pub struct Client {
    pub ip_address: String,
    pub port: u16
}

pub struct SidDeviceListener {
    socket: UdpSocket,
    hostname: String,
    os_name: String
}

impl SidDeviceListener {
    pub fn new() -> io::Result<Self> {
        let socket = UdpSocket::bind([LISTENING_ADDRESS, LISTENING_PORT].join(":"))?;
        socket.set_nonblocking(true)?;

        let info = os_info::get();

        let os_name = if let Some(edition) = info.edition() {
            format!("{} {}", edition, info.bitness())
        } else {
            format!("{} {} {}", info.os_type(), info.version(), info.bitness())
        };

        Ok(Self {
            socket,
            hostname: hostname::get().unwrap().to_str().unwrap().to_string(),
            os_name
        })
    }

    pub fn detect_client(&self) -> io::Result<Option<Client>> {
        let mut buffer = [0; MAX_DATA_SIZE];

        loop {
            match self.socket.recv_from(&mut buffer) {
                Ok(response)  => {
                    let client = self.handle_response(&buffer, response);

                    if client.is_some() {
                        return Ok(client);
                    }
                }
                Err(e) if e.kind() == ErrorKind::TimedOut => {
                    return Ok(None);
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(100));
                    return Ok(None);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }

    pub fn respond(&self, client: &Client) -> io::Result<usize> {
        let client_address = format!("{}:{}", &client.ip_address, &client.port);
        let data = format!("{},{},{}", MAGIC_ID, &self.hostname, &self.os_name);
        self.socket.send_to(data.as_bytes(), client_address)
    }

    fn handle_response(&self, buffer: &[u8], (size, source_address): (usize, SocketAddr)) -> Option<Client> {
        if size >= MAGIC_ID.len() && self.is_valid_packet(buffer) {
            return Some(Client {
                ip_address: source_address.ip().to_string(),
                port: source_address.port()
            });
        }
        None
    }

    fn is_valid_packet(&self, buffer: &[u8]) -> bool {
        buffer[0..MAGIC_ID.len()].eq(MAGIC_ID.as_bytes())
    }
}
