//! # ICMP (Internet Control Message Protocol)
//!
//! ICMP is used for error reporting and diagnostics, most notably
//! the "ping" command (Echo Request/Reply).
//!
//! ## ICMP Types
//!
//! | Type | Name                   |
//! |------|------------------------|
//! | 0    | Echo Reply             |
//! | 3    | Destination Unreachable|
//! | 5    | Redirect               |
//! | 8    | Echo Request           |
//! | 11   | Time Exceeded          |

use super::*;
use super::ipv4::*;

// ---------------------------------------------------------------------------
// ICMP Types and Codes
// ---------------------------------------------------------------------------

pub const ICMP_ECHO_REPLY: u8 = 0;
pub const ICMP_DEST_UNREACHABLE: u8 = 3;
pub const ICMP_REDIRECT: u8 = 5;
pub const ICMP_ECHO_REQUEST: u8 = 8;
pub const ICMP_TIME_EXCEEDED: u8 = 11;

// Destination Unreachable codes
pub const ICMP_NET_UNREACHABLE: u8 = 0;
pub const ICMP_HOST_UNREACHABLE: u8 = 1;
pub const ICMP_PROTOCOL_UNREACHABLE: u8 = 2;
pub const ICMP_PORT_UNREACHABLE: u8 = 3;
pub const ICMP_FRAGMENTATION_NEEDED: u8 = 4;

// Time Exceeded codes
pub const ICMP_TTL_EXCEEDED: u8 = 0;
pub const ICMP_FRAG_REASSEMBLY_TIME: u8 = 1;

// ---------------------------------------------------------------------------
// ICMP Echo Packet
// ---------------------------------------------------------------------------

/// ICMP Echo Request/Reply packet.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IcmpEcho {
    /// ICMP type (8 = request, 0 = reply).
    pub type_: u8,
    /// ICMP code (always 0 for echo).
    pub code: u8,
    /// Checksum.
    pub checksum: u16,
    /// Identifier (to match requests with replies).
    pub ident: u16,
    /// Sequence number.
    pub seq: u16,
    // Data follows.
}

impl IcmpEcho {
    pub const HEADER_SIZE: usize = 8;

    /// Parse an ICMP echo packet.
    pub fn parse(data: &[u8]) -> Option<(Self, &[u8])> {
        if data.len() < Self::HEADER_SIZE {
            return None;
        }

        let packet = IcmpEcho {
            type_: data[0],
            code: data[1],
            checksum: u16::from_be_bytes([data[2], data[3]]),
            ident: u16::from_be_bytes([data[4], data[5]]),
            seq: u16::from_be_bytes([data[6], data[7]]),
        };

        Some((packet, &data[Self::HEADER_SIZE..]))
    }

    /// Serialize to a buffer.
    pub fn serialize(&self, buf: &mut [u8], data: &[u8]) -> usize {
        assert!(buf.len() >= Self::HEADER_SIZE + data.len());

        buf[0] = self.type_;
        buf[1] = self.code;
        buf[2..4].copy_from_slice(&[0, 0]); // Checksum placeholder.
        buf[4..6].copy_from_slice(&self.ident.to_be_bytes());
        buf[6..8].copy_from_slice(&self.seq.to_be_bytes());
        buf[Self::HEADER_SIZE..Self::HEADER_SIZE + data.len()].copy_from_slice(data);

        let total_len = Self::HEADER_SIZE + data.len();
        let checksum = Self::calculate_checksum(&buf[..total_len]);
        buf[2..4].copy_from_slice(&checksum.to_be_bytes());

        total_len
    }

    /// Calculate ICMP checksum.
    fn calculate_checksum(data: &[u8]) -> u16 {
        let mut sum: u32 = 0;
        let mut i = 0;
        while i + 1 < data.len() {
            sum += u16::from_be_bytes([data[i], data[i + 1]]) as u32;
            i += 2;
        }
        if i < data.len() {
            sum += (data[i] as u32) << 8;
        }
        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        !(sum as u16)
    }

    /// Verify checksum.
    pub fn verify_checksum(data: &[u8]) -> bool {
        Self::calculate_checksum(data) == 0
    }
}

// ---------------------------------------------------------------------------
// Echo Reply Tracking
// ---------------------------------------------------------------------------

/// Ping statistics.
pub struct PingStats {
    /// Sequence number of last sent packet.
    pub seq: u16,
    /// Number of sent packets.
    pub sent: u32,
    /// Number of received replies.
    pub received: u32,
    /// Last round-trip time in ms.
    pub last_rtt: u32,
    /// Whether a reply is pending.
    pub pending: bool,
}

static mut PING_STATS: PingStats = PingStats {
    seq: 0,
    sent: 0,
    received: 0,
    last_rtt: 0,
    pending: false,
};

/// Get ping statistics.
pub fn stats() -> &'static PingStats {
    unsafe { &PING_STATS }
}

// ---------------------------------------------------------------------------
// Packet Processing
// ---------------------------------------------------------------------------

/// Process an incoming ICMP packet.
pub fn process_packet(ip_header: &Ipv4Header, payload: &[u8]) {
    if payload.len() < 4 {
        return;
    }

    let icmp_type = payload[0];

    match icmp_type {
        ICMP_ECHO_REQUEST => {
            // Reply to ping requests.
            let (echo, data) = match IcmpEcho::parse(payload) {
                Some(e) => e,
                None => return,
            };

            if !IcmpEcho::verify_checksum(payload) {
                return; // Corrupted.
            }

            // Build reply.
            let reply = IcmpEcho {
                type_: ICMP_ECHO_REPLY,
                code: 0,
                checksum: 0,
                ident: echo.ident,
                seq: echo.seq,
            };

            // Send reply back.
            // In a real implementation, build and send ICMP packet.
        }
        ICMP_ECHO_REPLY => {
            // Handle ping reply.
            let (echo, _) = match IcmpEcho::parse(payload) {
                Some(e) => e,
                None => return,
            };

            if !IcmpEcho::verify_checksum(payload) {
                return;
            }

            unsafe {
                PING_STATS.received += 1;
                PING_STATS.pending = false;
                // Calculate RTT from timestamp in data.
            }
        }
        ICMP_DEST_UNREACHABLE => {
            // Handle destination unreachable.
        }
        ICMP_TIME_EXCEEDED => {
            // Handle time exceeded.
        }
        _ => {
            // Unknown ICMP type.
        }
    }
}

/// Send a ping (echo request).
pub fn ping(dst: Ipv4Address) -> Result<(), &'static str> {
    unsafe {
        if PING_STATS.pending {
            return Err("Ping already in progress");
        }

        let seq = PING_STATS.seq;
        PING_STATS.seq += 1;
        PING_STATS.sent += 1;
        PING_STATS.pending = true;

        let echo = IcmpEcho {
            type_: ICMP_ECHO_REQUEST,
            code: 0,
            checksum: 0,
            ident: 0x1234,
            seq,
        };

        // Build payload with timestamp.
        let mut payload = [0u8; 56]; // Standard ping payload.
        // Put timestamp in first 8 bytes.
        let ticks = kernel_core::clock::get_ticks();
        payload[0..8].copy_from_slice(&ticks.to_be_bytes());

        // Serialize and send.
        let mut packet = [0u8; 64];
        let len = echo.serialize(&mut packet, &payload);

        super::ipv4::send_packet(dst, IPPROTO_ICMP, &packet[..len])
    }
}
