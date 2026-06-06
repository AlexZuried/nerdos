//! # IPv4 Packet Handling
//!
//! Implements IPv4 packet parsing, routing, and forwarding.

use super::*;

// ---------------------------------------------------------------------------
// IPv4 Header
// ---------------------------------------------------------------------------

/// IPv4 header (minimum 20 bytes, up to 60 bytes with options).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Ipv4Header {
    /// Version (4) and IHL (header length in 32-bit words, minimum 5).
    pub ver_ihl: u8,
    /// Differentiated Services Code Point (DSCP) and Explicit Congestion Notification (ECN).
    pub dscp_ecn: u8,
    /// Total length (header + payload) in bytes.
    pub total_len: u16,
    /// Identification (for fragmentation).
    pub ident: u16,
    /// Flags and fragment offset.
    pub flags_frag: u16,
    /// Time to Live.
    pub ttl: u8,
    /// Protocol (ICMP=1, TCP=6, UDP=17).
    pub protocol: u8,
    /// Header checksum.
    pub checksum: u16,
    /// Source IP address.
    pub src: Ipv4Address,
    /// Destination IP address.
    pub dst: Ipv4Address,
}

/// IP Protocol numbers.
pub const IPPROTO_ICMP: u8 = 1;
pub const IPPROTO_IGMP: u8 = 2;
pub const IPPROTO_TCP: u8 = 6;
pub const IPPROTO_UDP: u8 = 17;
pub const IPPROTO_SCTP: u8 = 132;

/// IPv4 flags.
pub const IP_FLAG_RESERVED: u16 = 0x8000;
pub const IP_FLAG_DF: u16 = 0x4000;  // Don't Fragment
pub const IP_FLAG_MF: u16 = 0x2000;  // More Fragments
pub const IP_FRAG_OFFSET_MASK: u16 = 0x1FFF;

impl Ipv4Header {
    /// Header length in bytes.
    pub fn header_len(&self) -> usize {
        ((self.ver_ihl & 0x0F) as usize) * 4
    }

    /// Version (should always be 4).
    pub fn version(&self) -> u8 {
        self.ver_ihl >> 4
    }

    /// Parse an IPv4 header from a byte slice.
    pub fn parse(data: &[u8]) -> Option<(Self, &[u8])> {
        if data.len() < 20 {
            return None;
        }

        let ver_ihl = data[0];
        let ihl = (ver_ihl & 0x0F) as usize;

        if (ver_ihl >> 4) != 4 || ihl < 5 || data.len() < ihl * 4 {
            return None; // Not IPv4 or header too short.
        }

        let header = Ipv4Header {
            ver_ihl,
            dscp_ecn: data[1],
            total_len: u16::from_be_bytes([data[2], data[3]]),
            ident: u16::from_be_bytes([data[4], data[5]]),
            flags_frag: u16::from_be_bytes([data[6], data[7]]),
            ttl: data[8],
            protocol: data[9],
            checksum: u16::from_be_bytes([data[10], data[11]]),
            src: Ipv4Address([data[12], data[13], data[14], data[15]]),
            dst: Ipv4Address([data[16], data[17], data[18], data[19]]),
        };

        let payload = &data[header.header_len()..];
        Some((header, payload))
    }

    /// Serialize the header to a byte buffer.
    pub fn serialize(&self, buf: &mut [u8]) -> usize {
        let ihl = self.ver_ihl & 0x0F;
        let len = (ihl as usize) * 4;

        assert!(buf.len() >= len);

        buf[0] = self.ver_ihl;
        buf[1] = self.dscp_ecn;
        buf[2..4].copy_from_slice(&self.total_len.to_be_bytes());
        buf[4..6].copy_from_slice(&self.ident.to_be_bytes());
        buf[6..8].copy_from_slice(&self.flags_frag.to_be_bytes());
        buf[8] = self.ttl;
        buf[9] = self.protocol;
        // Checksum is calculated separately.
        buf[10..12].copy_from_slice(&[0, 0]);
        buf[12..16].copy_from_slice(&self.src.0);
        buf[16..20].copy_from_slice(&self.dst.0);

        // Calculate and insert checksum.
        let checksum = Self::calculate_checksum(&buf[..len]);
        buf[10..12].copy_from_slice(&checksum.to_be_bytes());

        len
    }

    /// Calculate the Internet Checksum (RFC 1071).
    fn calculate_checksum(data: &[u8]) -> u16 {
        let mut sum: u32 = 0;

        // Sum all 16-bit words.
        let mut i = 0;
        while i + 1 < data.len() {
            sum += u16::from_be_bytes([data[i], data[i + 1]]) as u32;
            i += 2;
        }

        // Add left-over byte, if any.
        if i < data.len() {
            sum += (data[i] as u32) << 8;
        }

        // Fold 32-bit sum to 16 bits.
        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        // One's complement.
        !(sum as u16)
    }

    /// Verify the header checksum.
    pub fn verify_checksum(&self, data: &[u8]) -> bool {
        Self::calculate_checksum(data) == 0
    }
}

// ---------------------------------------------------------------------------
// Packet Processing
// ---------------------------------------------------------------------------

/// Process an incoming IPv4 packet.
pub fn process_packet(data: &[u8]) {
    let (header, payload) = match Ipv4Header::parse(data) {
        Some(h) => h,
        None => return, // Invalid header.
    };

    let config = super::config();

    // Check if packet is for us.
    if header.dst != config.ip
        && header.dst != Ipv4Address::BROADCAST
        && !header.dst.is_multicast() {
        // Not for us (would route in a real implementation).
        return;
    }

    // Check TTL.
    if header.ttl == 0 {
        // TTL expired - send ICMP Time Exceeded.
        return;
    }

    // Verify checksum.
    if !header.verify_checksum(&data[..header.header_len()]) {
        return; // Corrupted packet.
    }

    // Dispatch based on protocol.
    match header.protocol {
        IPPROTO_ICMP => {
            super::icmp::process_packet(&header, payload);
        }
        IPPROTO_UDP => {
            super::udp::process_packet(&header, payload);
        }
        IPPROTO_TCP => {
            super::tcp::process_packet(&header, payload);
        }
        _ => {
            // Unknown protocol - could send ICMP Destination Unreachable.
        }
    }
}

/// Send an IPv4 packet.
///
/// # Arguments
/// * `dst` - Destination IP address
/// * `protocol` - IP protocol number
/// * `payload` - Data to send
///
/// # Returns
/// `Ok(())` on success, or an error string.
pub fn send_packet(dst: Ipv4Address, protocol: u8, payload: &[u8]) -> Result<(), &'static str> {
    let config = super::config();

    // Determine the destination MAC address.
    let dst_mac = if super::config().is_local(dst) {
        // On local network - use ARP to find MAC.
        match super::arp::lookup(dst) {
            Some(mac) => mac,
            None => {
                // Need to ARP first.
                super::arp::send_request(dst);
                return Err("ARP needed");
            }
        }
    } else {
        // Off local network - send to gateway.
        if config.gateway.is_unspecified() {
            return Err("No gateway configured");
        }
        match super::arp::lookup(config.gateway) {
            Some(mac) => mac,
            None => {
                super::arp::send_request(config.gateway);
                return Err("ARP needed for gateway");
            }
        }
    };

    // Build the IPv4 header.
    let mut header = Ipv4Header {
        ver_ihl: 0x45, // Version 4, IHL 5 (20 bytes).
        dscp_ecn: 0,
        total_len: (20 + payload.len()) as u16,
        ident: 0, // TODO: Use a counter.
        flags_frag: IP_FLAG_DF, // Don't fragment.
        ttl: 64,
        protocol,
        checksum: 0,
        src: config.ip,
        dst,
    };

    // Serialize header and payload.
    let total_len = 20 + payload.len();
    // In a real implementation, allocate a buffer and build the full packet,
    // then call the NIC driver to transmit.

    Ok(())
}
