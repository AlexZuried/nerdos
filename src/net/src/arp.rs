//! # ARP (Address Resolution Protocol)
//!
//! ARP maps IP addresses to MAC addresses on the local network.
//! It operates entirely at the link layer.
//!
//! ## Packet Format
//!
//! ```
//! 0                   8                   16                  24                  32
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |        HTYPE (Ethernet=1)     |        PTYPE (IPv4=0x0800)                  |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! | HLEN (6)      | PLEN (4)      |        OPER (1=request, 2=reply)            |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                       SHA (Sender Hardware Address - 6 bytes)               |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                       SPA (Sender Protocol Address - 4 bytes)               |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                       THA (Target Hardware Address - 6 bytes)               |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                       TPA (Target Protocol Address - 4 bytes)               |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! ```

use super::*;
use core::mem;

// ---------------------------------------------------------------------------
// ARP Packet
// ---------------------------------------------------------------------------

/// ARP packet header and payload.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ArpPacket {
    /// Hardware type (1 = Ethernet).
    pub htype: u16,
    /// Protocol type (0x0800 = IPv4).
    pub ptype: u16,
    /// Hardware address length (6 for Ethernet).
    pub hlen: u8,
    /// Protocol address length (4 for IPv4).
    pub plen: u8,
    /// Operation (1 = request, 2 = reply).
    pub oper: u16,
    /// Sender hardware address (MAC).
    pub sha: MacAddress,
    /// Sender protocol address (IP).
    pub spa: Ipv4Address,
    /// Target hardware address (MAC).
    pub tha: MacAddress,
    /// Target protocol address (IP).
    pub tpa: Ipv4Address,
}

impl ArpPacket {
    /// Size of an ARP packet (28 bytes for Ethernet+IPv4).
    pub const SIZE: usize = 28;

    /// Create an ARP request packet.
    pub fn request(src_mac: MacAddress, src_ip: Ipv4Address, target_ip: Ipv4Address) -> Self {
        ArpPacket {
            htype: 1,       // Ethernet
            ptype: 0x0800,  // IPv4
            hlen: 6,        // MAC address length
            plen: 4,        // IPv4 address length
            oper: 1,        // Request
            sha: src_mac,
            spa: src_ip,
            tha: MacAddress::BROADCAST,
            tpa: target_ip,
        }
    }

    /// Create an ARP reply packet.
    pub fn reply(
        src_mac: MacAddress,
        src_ip: Ipv4Address,
        dst_mac: MacAddress,
        dst_ip: Ipv4Address,
    ) -> Self {
        ArpPacket {
            htype: 1,
            ptype: 0x0800,
            hlen: 6,
            plen: 4,
            oper: 2,        // Reply
            sha: src_mac,
            spa: src_ip,
            tha: dst_mac,
            tpa: dst_ip,
        }
    }

    /// Parse an ARP packet from a byte slice.
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < Self::SIZE {
            return None;
        }

        let htype = u16::from_be_bytes([data[0], data[1]]);
        let ptype = u16::from_be_bytes([data[2], data[3]]);

        // Validate this is Ethernet+IPv4 ARP.
        if htype != 1 || ptype != 0x0800 {
            return None;
        }

        let hlen = data[4];
        let plen = data[5];
        let oper = u16::from_be_bytes([data[6], data[7]]);

        if hlen != 6 || plen != 4 {
            return None;
        }

        Some(ArpPacket {
            htype,
            ptype,
            hlen,
            plen,
            oper,
            sha: MacAddress([data[8], data[9], data[10], data[11], data[12], data[13]]),
            spa: Ipv4Address([data[14], data[15], data[16], data[17]]),
            tha: MacAddress([data[18], data[19], data[20], data[21], data[22], data[23]]),
            tpa: Ipv4Address([data[24], data[25], data[26], data[27]]),
        })
    }

    /// Serialize the ARP packet to a byte buffer.
    pub fn serialize(&self, buf: &mut [u8]) -> usize {
        assert!(buf.len() >= Self::SIZE);

        buf[0..2].copy_from_slice(&self.htype.to_be_bytes());
        buf[2..4].copy_from_slice(&self.ptype.to_be_bytes());
        buf[4] = self.hlen;
        buf[5] = self.plen;
        buf[6..8].copy_from_slice(&self.oper.to_be_bytes());
        buf[8..14].copy_from_slice(&self.sha.0);
        buf[14..18].copy_from_slice(&self.spa.0);
        buf[18..24].copy_from_slice(&self.tha.0);
        buf[24..28].copy_from_slice(&self.tpa.0);

        Self::SIZE
    }
}

// ---------------------------------------------------------------------------
// ARP Table (Cache)
// ---------------------------------------------------------------------------

/// A cached ARP entry.
#[derive(Debug, Clone, Copy)]
pub struct ArpEntry {
    /// IP address.
    pub ip: Ipv4Address,
    /// MAC address.
    pub mac: MacAddress,
    /// Age in ticks.
    pub age: u64,
    /// Is this entry valid?
    pub valid: bool,
}

/// The ARP table (a simple fixed-size cache).
const ARP_TABLE_SIZE: usize = 32;

static mut ARP_TABLE: [ArpEntry; ARP_TABLE_SIZE] =
    [ArpEntry {
        ip: Ipv4Address::UNSPECIFIED,
        mac: MacAddress([0; 6]),
        age: 0,
        valid: false,
    }; ARP_TABLE_SIZE];

/// Look up a MAC address in the ARP table.
pub fn lookup(ip: Ipv4Address) -> Option<MacAddress> {
    unsafe {
        for entry in ARP_TABLE.iter() {
            if entry.valid && entry.ip == ip {
                return Some(entry.mac);
            }
        }
    }
    None
}

/// Add or update an entry in the ARP table.
pub fn update(ip: Ipv4Address, mac: MacAddress) {
    unsafe {
        // Check if entry already exists.
        for entry in ARP_TABLE.iter_mut() {
            if entry.valid && entry.ip == ip {
                entry.mac = mac;
                entry.age = 0;
                return;
            }
        }

        // Find a free slot or the oldest entry.
        let mut oldest_idx = 0;
        let mut oldest_age = 0;

        for (i, entry) in ARP_TABLE.iter_mut().enumerate() {
            if !entry.valid {
                entry.ip = ip;
                entry.mac = mac;
                entry.age = 0;
                entry.valid = true;
                return;
            }
            if entry.age > oldest_age {
                oldest_age = entry.age;
                oldest_idx = i;
            }
        }

        // Replace oldest entry.
        ARP_TABLE[oldest_idx] = ArpEntry {
            ip,
            mac,
            age: 0,
            valid: true,
        };
    }
}

/// Process an incoming ARP packet.
pub fn process_packet(packet: &ArpPacket) {
    let config = super::config();

    match packet.oper {
        1 => {
            // ARP Request: is it asking for our IP?
            if packet.tpa == config.ip {
                // Send reply.
                let reply = ArpPacket::reply(
                    config.mac,
                    config.ip,
                    packet.sha,
                    packet.spa,
                );

                // Queue reply for transmission.
                // In a real implementation, this would call the NIC driver.

                // Also cache the sender's mapping.
                update(packet.spa, packet.sha);
            }
        }
        2 => {
            // ARP Reply: cache the mapping.
            update(packet.spa, packet.sha);
        }
        _ => {}
    }
}

/// Send an ARP request for an IP address.
pub fn send_request(target_ip: Ipv4Address) {
    let config = super::config();

    let request = ArpPacket::request(config.mac, config.ip, target_ip);

    // In a real implementation, this would build an Ethernet frame
    // and transmit it via the NIC driver.
}
