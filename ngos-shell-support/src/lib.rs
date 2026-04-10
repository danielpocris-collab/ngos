//! Canonical subsystem role:
//! - subsystem: native shell support utilities
//! - owner layer: Layer 3
//! - semantic owner: `userland-native`
//! - truth path role: shared support helpers for shell/bootstrap/path/network
//!   orchestration
//!
//! Canonical contract families handled here:
//! - bootstrap helper contracts
//! - shell network frame helper contracts
//!
//! This module may provide shared support helpers for native shell orchestration,
//! but it must not redefine kernel, ABI, or subsystem truth from lower layers.

#![no_std]
extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use ngos_user_abi::BootstrapArgs;

pub fn parse_ipv4(text: &str) -> Option<[u8; 4]> {
    let mut octets = [0u8; 4];
    let mut parts = text.split('.');
    for octet in &mut octets {
        *octet = parts.next()?.parse::<u8>().ok()?;
    }
    if parts.next().is_some() {
        return None;
    }
    Some(octets)
}

pub fn render_ipv4(addr: [u8; 4]) -> String {
    format!("{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3])
}

pub fn checksum16(bytes: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut chunks = bytes.chunks_exact(2);
    for chunk in &mut chunks {
        sum = sum.wrapping_add(u16::from_be_bytes([chunk[0], chunk[1]]) as u32);
    }
    if let Some(byte) = chunks.remainder().first() {
        sum = sum.wrapping_add((*byte as u32) << 8);
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

pub fn build_udp_ipv4_frame(
    src_mac: [u8; 6],
    dst_mac: [u8; 6],
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    src_port: u16,
    dst_port: u16,
    payload: &[u8],
) -> Vec<u8> {
    let udp_len = 8 + payload.len();
    let ip_len = 20 + udp_len;
    let mut frame = Vec::with_capacity(14 + ip_len);
    frame.extend_from_slice(&dst_mac);
    frame.extend_from_slice(&src_mac);
    frame.extend_from_slice(&0x0800u16.to_be_bytes());

    let mut ip_header = [0u8; 20];
    ip_header[0] = 0x45;
    ip_header[2..4].copy_from_slice(&(ip_len as u16).to_be_bytes());
    ip_header[8] = 64;
    ip_header[9] = 17;
    ip_header[12..16].copy_from_slice(&src_ip);
    ip_header[16..20].copy_from_slice(&dst_ip);
    let ip_checksum = checksum16(&ip_header);
    ip_header[10..12].copy_from_slice(&ip_checksum.to_be_bytes());
    frame.extend_from_slice(&ip_header);

    let mut udp = Vec::with_capacity(udp_len);
    udp.extend_from_slice(&src_port.to_be_bytes());
    udp.extend_from_slice(&dst_port.to_be_bytes());
    udp.extend_from_slice(&(udp_len as u16).to_be_bytes());
    udp.extend_from_slice(&0u16.to_be_bytes());
    udp.extend_from_slice(payload);

    let mut pseudo = Vec::with_capacity(12 + udp.len());
    pseudo.extend_from_slice(&src_ip);
    pseudo.extend_from_slice(&dst_ip);
    pseudo.push(0);
    pseudo.push(17);
    pseudo.extend_from_slice(&(udp_len as u16).to_be_bytes());
    pseudo.extend_from_slice(&udp);
    let udp_checksum = checksum16(&pseudo);
    udp[6..8].copy_from_slice(&udp_checksum.to_be_bytes());
    frame.extend_from_slice(&udp);
    frame
}

pub fn bootstrap_env_value<'a>(bootstrap: &'a BootstrapArgs<'_>, key: &str) -> Option<&'a str> {
    bootstrap.envp.iter().find_map(|entry| {
        let (entry_key, entry_value) = entry.split_once('=')?;
        (entry_key == key).then_some(entry_value)
    })
}

pub fn bootstrap_has_arg(bootstrap: &BootstrapArgs<'_>, needle: &str) -> bool {
    bootstrap.argv.iter().any(|arg| *arg == needle)
}
