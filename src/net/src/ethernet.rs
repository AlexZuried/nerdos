//! # Ethernet Frame Handling
//!
//! Provides functions for constructing and parsing Ethernet II frames.

use super::*;

/// Minimum Ethernet frame size (64 bytes including FCS).
pub const MIN_FRAME_SIZE: usize = 60;
/// Maximum Ethernet frame size (1518 bytes including FCS).
pub const MAX_FRAME_SIZE: usize = 1514;
/// Size of the Ethernet header.
pub const HEADER_SIZE: usize = 14;

/// Parse an Ethernet header from a byte slice.
///
/// Returns `Some(EthernetHeader)` if the slice contains a valid header.
pub fn parse_header(data: &[u8]) -> Option<(EthernetHeader, &[u8])> {
    if data.len() < HEADER_SIZE {
        return None;
    }

    let dst = MacAddress([data[0], data[1], data[2], data[3], data[4], data[5]]);
    let src = MacAddress([data[6], data[7], data[8], data[9], data[10], data[11]]);
    let ethertype = u16::from_be_bytes([data[12], data[13]]);

    Some((EthernetHeader { dst, src, ethertype }, &data[HEADER_SIZE..]))
}

/// Build an Ethernet frame header.
///
/// Writes the header into `buf` and returns the remaining buffer.
///
/// # Panics
/// Panics if `buf` is smaller than `HEADER_SIZE`.
pub fn build_header<'a>(
    buf: &'a mut [u8],
    dst: MacAddress,
    src: MacAddress,
    ethertype: u16,
) -> &'a mut [u8] {
    assert!(buf.len() >= HEADER_SIZE);

    buf[0..6].copy_from_slice(&dst.0);
    buf[6..12].copy_from_slice(&src.0);
    buf[12..14].copy_from_slice(&ethertype.to_be_bytes());

    &mut buf[HEADER_SIZE..]
}

/// Check if a frame is addressed to us or is broadcast.
pub fn is_for_us(header: &EthernetHeader, our_mac: MacAddress) -> bool {
    header.dst.is_broadcast() || header.dst == our_mac
}
