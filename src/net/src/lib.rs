//! # Network Stack for NerdOS
//!
//! A minimal but functional TCP/IP stack supporting:
//! - Ethernet II framing
//! - ARP (Address Resolution Protocol)
//! - IPv4 with basic routing
//! - ICMP (ping)
//! - UDP sockets
//! - TCP sockets (3-way handshake, basic data transfer)
//! - DHCP client
//!
//! ## Architecture
//!
//! ```
//! Application Layer (sockets)
//!       |
//!   Transport Layer (TCP/UDP)
//!       |
//!   Network Layer (IPv4/ICMP)
//!       |
//!   Link Layer (Ethernet/ARP)
//!       |
//!   NIC Driver (e1000)
//! ```

#![no_std]

pub mod ethernet;
pub mod arp;
pub mod ipv4;
pub mod icmp;
pub mod udp;
pub mod tcp;
pub mod dhcp;
pub mod socket;

// ---------------------------------------------------------------------------
// MAC Address
// ---------------------------------------------------------------------------

/// A 48-bit Ethernet MAC address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacAddress(pub [u8; 6]);

impl MacAddress {
    /// Broadcast MAC address (FF:FF:FF:FF:FF:FF).
    pub const BROADCAST: Self = MacAddress([0xFF; 6]);

    /// Check if this is the broadcast address.
    pub fn is_broadcast(&self) -> bool {
        self.0 == [0xFF; 6]
    }

    /// Check if this is a multicast address.
    pub fn is_multicast(&self) -> bool {
        self.0[0] & 0x01 != 0
    }

    /// Format as a human-readable string.
    pub fn to_string(&self) -> [u8; 17] {
        let mut buf = [0u8; 17];
        const HEX: &[u8] = b"0123456789ABCDEF";

        for (i, byte) in self.0.iter().enumerate() {
            buf[i * 3] = HEX[(byte >> 4) as usize];
            buf[i * 3 + 1] = HEX[(byte & 0x0F) as usize];
            if i < 5 {
                buf[i * 3 + 2] = b':';
            }
        }

        buf
    }
}

// ---------------------------------------------------------------------------
// IPv4 Address
// ---------------------------------------------------------------------------

/// A 32-bit IPv4 address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ipv4Address(pub [u8; 4]);

impl Ipv4Address {
    /// The unspecified address (0.0.0.0).
    pub const UNSPECIFIED: Self = Ipv4Address([0, 0, 0, 0]);
    /// The loopback address (127.0.0.1).
    pub const LOCALHOST: Self = Ipv4Address([127, 0, 0, 1]);
    /// The broadcast address (255.255.255.255).
    pub const BROADCAST: Self = Ipv4Address([255, 255, 255, 255]);

    /// Create an IPv4 address from four octets.
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Ipv4Address([a, b, c, d])
    }

    /// Create from a u32 in network byte order.
    pub const fn from_u32(addr: u32) -> Self {
        Ipv4Address([
            (addr >> 24) as u8,
            (addr >> 16) as u8,
            (addr >> 8) as u8,
            addr as u8,
        ])
    }

    /// Convert to a u32 in network byte order.
    pub const fn to_u32(&self) -> u32 {
        ((self.0[0] as u32) << 24)
            | ((self.0[1] as u32) << 16)
            | ((self.0[2] as u32) << 8)
            | (self.0[3] as u32)
    }

    /// Check if this is the unspecified address.
    pub fn is_unspecified(&self) -> bool {
        self.0 == [0, 0, 0, 0]
    }

    /// Check if this is a loopback address (127.0.0.0/8).
    pub fn is_loopback(&self) -> bool {
        self.0[0] == 127
    }

    /// Check if this is a private address.
    pub fn is_private(&self) -> bool {
        // 10.0.0.0/8
        if self.0[0] == 10 {
            return true;
        }
        // 172.16.0.0/12
        if self.0[0] == 172 && (self.0[1] & 0xF0) == 16 {
            return true;
        }
        // 192.168.0.0/16
        if self.0[0] == 192 && self.0[1] == 168 {
            return true;
        }
        false
    }

    /// Check if this is a multicast address (224.0.0.0/4).
    pub fn is_multicast(&self) -> bool {
        self.0[0] & 0xF0 == 224
    }

    /// Format as a dotted decimal string.
    pub fn to_string(&self) -> [u8; 15] {
        // Simple formatter: "xxx.xxx.xxx.xxx\0"
        let mut buf = [0u8; 15];
        let mut pos = 0;

        for (i, byte) in self.0.iter().enumerate() {
            // Convert byte to decimal string.
            let mut n = *byte;
            let mut digits = [0u8; 3];
            let mut num_digits = 0;

            if n == 0 {
                digits[0] = b'0';
                num_digits = 1;
            } else {
                while n > 0 {
                    digits[num_digits] = b'0' + (n % 10);
                    n /= 10;
                    num_digits += 1;
                }
            }

            // Write digits in reverse order.
            for j in (0..num_digits).rev() {
                buf[pos] = digits[j];
                pos += 1;
            }

            // Write dot (except after last octet).
            if i < 3 {
                buf[pos] = b'.';
                pos += 1;
            }
        }

        buf
    }
}

// ---------------------------------------------------------------------------
// Ethernet Frame
// ---------------------------------------------------------------------------

/// Ethernet II frame header (14 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct EthernetHeader {
    /// Destination MAC address.
    pub dst: MacAddress,
    /// Source MAC address.
    pub src: MacAddress,
    /// EtherType (big-endian).
    pub ethertype: u16,
}

/// Common EtherType values.
pub const ETHERTYPE_ARP: u16 = 0x0806;
pub const ETHERTYPE_IPV4: u16 = 0x0800;
pub const ETHERTYPE_IPV6: u16 = 0x86DD;
pub const ETHERTYPE_VLAN: u16 = 0x8100;

// ---------------------------------------------------------------------------
// Network Configuration
// ---------------------------------------------------------------------------

/// Global network configuration.
pub struct NetworkConfig {
    /// Local MAC address.
    pub mac: MacAddress,
    /// Local IPv4 address.
    pub ip: Ipv4Address,
    /// Subnet mask.
    pub netmask: Ipv4Address,
    /// Default gateway.
    pub gateway: Ipv4Address,
    /// DNS server.
    pub dns: Ipv4Address,
    /// MTU.
    pub mtu: u16,
}

impl NetworkConfig {
    /// Create a default configuration (unconfigured).
    pub const fn default() -> Self {
        NetworkConfig {
            mac: MacAddress([0, 0, 0, 0, 0, 0]),
            ip: Ipv4Address::UNSPECIFIED,
            netmask: Ipv4Address::new(255, 255, 255, 0),
            gateway: Ipv4Address::UNSPECIFIED,
            dns: Ipv4Address::UNSPECIFIED,
            mtu: 1500,
        }
    }

    /// Check if the network is configured.
    pub fn is_configured(&self) -> bool {
        !self.ip.is_unspecified()
    }

    /// Check if an IP is on the local network.
    pub fn is_local(&self, ip: Ipv4Address) -> bool {
        let netmask_u32 = self.netmask.to_u32();
        let ip_u32 = ip.to_u32();
        let local_u32 = self.ip.to_u32();
        (ip_u32 & netmask_u32) == (local_u32 & netmask_u32)
    }
}

// ---------------------------------------------------------------------------
// Global State
// ---------------------------------------------------------------------------

static mut NET_CONFIG: NetworkConfig = NetworkConfig::default();

/// Get the network configuration.
pub fn config() -> &'static NetworkConfig {
    unsafe { &NET_CONFIG }
}

/// Get a mutable reference to the network configuration.
///
/// # Safety
/// Must not be called concurrently.
pub unsafe fn config_mut() -> &'static mut NetworkConfig {
    &mut NET_CONFIG
}

/// Initialize the network subsystem.
pub fn init() {
    // Initialize ARP table, socket table, etc.
}

/// Process an incoming Ethernet frame.
///
/// This is called by the NIC driver interrupt handler.
pub fn process_frame(frame: &[u8]) {
    if frame.len() < 14 {
        return; // Frame too short.
    }

    let ethertype = u16::from_be_bytes([frame[12], frame[13]]);

    match ethertype {
        ETHERTYPE_ARP => {
            // arp::process_packet(&frame[14..]);
        }
        ETHERTYPE_IPV4 => {
            ipv4::process_packet(&frame[14..]);
        }
        _ => {
            // Unknown EtherType, drop.
        }
    }
}
