// Copyright (C) 2026 Wilfred Bos
// Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.

use std::net::IpAddr;

pub fn is_local_ip_address(ip_address: &str) -> bool {
    let Ok(ip) = ip_address.parse::<IpAddr>() else {
        return false;
    };

    if ip.is_loopback() {
        return true;
    }

    if_addrs::get_if_addrs()
        .is_ok_and(|interfaces| interfaces.iter().any(|interface| interface.ip() == ip))
}
