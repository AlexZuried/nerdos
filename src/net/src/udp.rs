//! # UDP (User Datagram Protocol)
//!
//! UDP provides connectionless, unreliable datagram delivery.
//! It's used by DNS, DHCP, NTP, and many application protocols.
//!
//! ## Header Format (8 bytes)
//!
//! ```
//!  0      7 8     15 16    23 24    31
//! +--------+--------+--------+--------+
//! |     Source Port     |   Dest Port   |
//! +--------+--------+--------+--------+
//! |     Length          |    Checksum   |
//! +--------+--------+--------+--------+
//! ```

use super::*;
use super::ipv4::*;

// ---------------------------------------------------------------------------
// UDP Header
// ---------------------------------------------------------------------------

/// UDP datagram header.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UdpHeader {
    /// Source port.
    pub src_port: u16,
    /// Destination port.
    pub dst_port: u16,
    /// Length of header + payload.
    pub length: u16,
    /// Checksum (optional in IPv4, required in IPv6).
    pub checksum: u16,
}

impl UdpHeader {
    pub const SIZE: usize = 8;

    /// Parse a UDP header.
    pub fn parse(data: &[u8]) -> Option<(Self, &[u8])> {
        if data.len() < Self::SIZE {
            return None;
        }

        let header = UdpHeader {
            src_port: u16::from_be_bytes([data[0], data[1]]),
            dst_port: u16::from_be_bytes([data[2], data[3]]),
            length: u16::from_be_bytes([data[4], data[5]]),
            checksum: u16::from_be_bytes([data[6], data[7]]),
        };

        Some((header, &data[Self::SIZE..]))
    }

    /// Serialize to buffer.
    pub fn serialize(&self, buf: &mut [u8]) -> usize {
        assert!(buf.len() >= Self::SIZE);
        buf[0..2].copy_from_slice(&self.src_port.to_be_bytes());
        buf[2..4].copy_from_slice(&self.dst_port.to_be_bytes());
        buf[4..6].copy_from_slice(&self.length.to_be_bytes());
        buf[6..8].copy_from_slice(&self.checksum.to_be_bytes());
        Self::SIZE
    }
}

// ---------------------------------------------------------------------------
// Socket Table
// ---------------------------------------------------------------------------

/// A UDP socket.
pub struct UdpSocket {
    /// Local port number.
    pub local_port: u16,
    /// Local IP address.
    pub local_addr: Ipv4Address,
    /// Remote port number (0 = not connected).
    pub remote_port: u16,
    /// Remote IP address (UNSPECIFIED = not connected).
    pub remote_addr: Ipv4Address,
    /// Receive buffer.
    pub rx_buf: [u8; 2048],
    /// Received data length.
    pub rx_len: usize,
    /// Whether data is available.
    pub data_available: bool,
}

impl UdpSocket {
    /// Create a new unbound UDP socket.
    pub fn new() -> Self {
        UdpSocket {
            local_port: 0,
            local_addr: Ipv4Address::UNSPECIFIED,
            remote_port: 0,
            remote_addr: Ipv4Address::UNSPECIFIED,
            rx_buf: [0; 2048],
            rx_len: 0,
            data_available: false,
        }
    }

    /// Bind to a local port.
    pub fn bind(&mut self, addr: Ipv4Address, port: u16) {
        self.local_addr = addr;
        self.local_port = port;
    }

    /// Connect to a remote address.
    pub fn connect(&mut self, addr: Ipv4Address, port: u16) {
        self.remote_addr = addr;
        self.remote_port = port;
    }

    /// Receive data (non-blocking).
    pub fn recv(&mut self, buf: &mut [u8]) -> Option<usize> {
        if !self.data_available {
            return None;
        }

        let len = self.rx_len.min(buf.len());
        buf[..len].copy_from_slice(&self.rx_buf[..len]);
        self.data_available = false;
        self.rx_len = 0;

        Some(len)
    }
}

// Simple fixed-size socket table.
const MAX_UDP_SOCKETS: usize = 16;
static mut UDP_SOCKETS: [Option<UdpSocket>; MAX_UDP_SOCKETS] =
    [None; MAX_UDP_SOCKETS];

// ---------------------------------------------------------------------------
// Packet Processing
// ---------------------------------------------------------------------------

/// Process an incoming UDP packet.
pub fn process_packet(ip_header: &Ipv4Header, payload: &[u8]) {
    let (header, data) = match UdpHeader::parse(payload) {
        Some(h) => h,
        None => return,
    };

    // Find a matching socket.
    unsafe {
        for socket_opt in UDP_SOCKETS.iter_mut() {
            if let Some(ref mut socket) = socket_opt {
                if socket.local_port == header.dst_port {
                    // Deliver data.
                    let len = data.len().min(socket.rx_buf.len());
                    socket.rx_buf[..len].copy_from_slice(&data[..len]);
                    socket.rx_len = len;
                    socket.data_available = true;
                    return;
                }
            }
        }
    }

    // No socket found - could send ICMP Port Unreachable.
}

/// Create a new UDP socket.
pub fn socket() -> Option<usize> {
    unsafe {
        for (i, slot) in UDP_SOCKETS.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(UdpSocket::new());
                return Some(i);
            }
        }
        None
    }
}

/// Close a UDP socket.
pub fn close(handle: usize) {
    unsafe {
        if handle < MAX_UDP_SOCKETS {
            UDP_SOCKETS[handle] = None;
        }
    }
}

/// Get a mutable reference to a socket.
pub fn get_socket(handle: usize) -> Option<&'static mut UdpSocket> {
    unsafe {
        if handle < MAX_UDP_SOCKETS {
            UDP_SOCKETS[handle].as_mut()
        } else {
            None
        }
    }
}

/// Bind a socket to a port.
pub fn bind(handle: usize, addr: Ipv4Address, port: u16) -> Result<(), &'static str> {
    let socket = get_socket(handle).ok_or("Invalid socket")?;
    socket.bind(addr, port);
    Ok(())
}

/// Send a UDP datagram.
pub fn sendto(
    handle: usize,
    buf: &[u8],
    dst_addr: Ipv4Address,
    dst_port: u16,
) -> Result<usize, &'static str> {
    let socket = get_socket(handle).ok_or("Invalid socket")?;

    if socket.local_port == 0 {
        return Err("Socket not bound");
    }

    // Build UDP header + payload.
    let mut packet = [0u8; 1500];
    let header = UdpHeader {
        src_port: socket.local_port,
        dst_port,
        length: (UdpHeader::SIZE + buf.len()) as u16,
        checksum: 0, // Optional in IPv4.
    };

    header.serialize(&mut packet);
    packet[UdpHeader::SIZE..UdpHeader::SIZE + buf.len()].copy_from_slice(buf);

    let total_len = UdpHeader::SIZE + buf.len();

    // Send via IPv4.
    super::ipv4::send_packet(dst_addr, IPPROTO_UDP, &packet[..total_len])?;

    Ok(buf.len())
}

/// Receive a UDP datagram.
pub fn recvfrom(handle: usize, buf: &mut [u8]) -> Option<usize> {
    let socket = get_socket(handle)?;
    socket.recv(buf)
}
