//! # Socket API
//!
//! Provides a unified socket interface for user applications.
//! This abstracts over UDP and TCP sockets.

use super::*;

// ---------------------------------------------------------------------------
// Socket Types
// ---------------------------------------------------------------------------

/// Supported socket types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    /// UDP datagram socket.
    Datagram,
    /// TCP stream socket.
    Stream,
}

/// Socket address (IP + port).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketAddr {
    /// IP address.
    pub ip: Ipv4Address,
    /// Port number.
    pub port: u16,
}

impl SocketAddr {
    /// Create a new socket address.
    pub const fn new(ip: Ipv4Address, port: u16) -> Self {
        SocketAddr { ip, port }
    }

    /// Parse from dotted decimal and port string (e.g., "192.168.1.1:80").
    pub fn parse(s: &str) -> Option<Self> {
        // Find the colon separator.
        let colon_pos = s.find(':')?;
        let ip_str = &s[..colon_pos];
        let port_str = &s[colon_pos + 1..];

        // Parse IP address.
        let ip = parse_ipv4(ip_str)?;

        // Parse port.
        let port = parse_u16(port_str)?;

        Some(SocketAddr::new(ip, port))
    }
}

/// Socket handle (index into socket table).
pub type SocketHandle = usize;

// ---------------------------------------------------------------------------
// Socket Table
// ---------------------------------------------------------------------------

/// A generic socket.
pub enum Socket {
    /// UDP socket.
    Udp(SocketHandle),
    /// TCP socket.
    Tcp(SocketHandle),
}

/// Create a new socket.
pub fn socket(ty: SocketType) -> Result<Socket, &'static str> {
    match ty {
        SocketType::Datagram => {
            let handle = super::udp::socket().ok_or("No free UDP sockets")?;
            Ok(Socket::Udp(handle))
        }
        SocketType::Stream => {
            let handle = super::tcp::socket().ok_or("No free TCP sockets")?;
            Ok(Socket::Tcp(handle))
        }
    }
}

/// Close a socket.
pub fn close(sock: Socket) {
    match sock {
        Socket::Udp(h) => super::udp::close(h),
        Socket::Tcp(h) => super::tcp::close(h),
    }
}

/// Bind a socket to a local address.
pub fn bind(sock: &Socket, addr: SocketAddr) -> Result<(), &'static str> {
    match sock {
        Socket::Udp(h) => super::udp::bind(*h, addr.ip, addr.port),
        Socket::Tcp(h) => super::tcp::listen(*h, addr.port),
    }
}

/// Connect a socket to a remote address.
pub fn connect(sock: &Socket, addr: SocketAddr) -> Result<(), &'static str> {
    match sock {
        Socket::Udp(h) => {
            let s = super::udp::get_socket(*h).ok_or("Invalid socket")?;
            s.connect(addr.ip, addr.port);
            Ok(())
        }
        Socket::Tcp(h) => super::tcp::connect(*h, addr.ip, addr.port),
    }
}

/// Send data on a socket.
pub fn send(sock: &Socket, buf: &[u8]) -> Result<usize, &'static str> {
    match sock {
        Socket::Udp(h) => {
            let s = super::udp::get_socket(*h).ok_or("Invalid socket")?;
            super::udp::sendto(*h, buf, s.remote_addr, s.remote_port)
        }
        Socket::Tcp(h) => super::tcp::send(*h, buf),
    }
}

/// Send data to a specific address (UDP only).
pub fn sendto(sock: &Socket, buf: &[u8], addr: SocketAddr) -> Result<usize, &'static str> {
    match sock {
        Socket::Udp(h) => super::udp::sendto(*h, buf, addr.ip, addr.port),
        Socket::Tcp(_) => Err("Cannot use sendto with TCP"),
    }
}

/// Receive data from a socket.
pub fn recv(sock: &Socket, buf: &mut [u8]) -> Result<usize, &'static str> {
    match sock {
        Socket::Udp(h) => super::udp::recvfrom(*h, buf).ok_or("No data available"),
        Socket::Tcp(h) => super::tcp::recv(*h, buf),
    }
}

// ---------------------------------------------------------------------------
// Helper Functions
// ---------------------------------------------------------------------------

/// Parse an IPv4 address from a string (e.g., "192.168.1.1").
fn parse_ipv4(s: &str) -> Option<Ipv4Address> {
    let mut octets = [0u8; 4];
    let mut octet_idx = 0;
    let mut current = 0u8;

    for ch in s.bytes() {
        if ch == b'.' {
            if octet_idx >= 3 {
                return None;
            }
            octets[octet_idx] = current;
            octet_idx += 1;
            current = 0;
        } else if ch >= b'0' && ch <= b'9' {
            current = current * 10 + (ch - b'0');
            if current > 255 {
                return None;
            }
        } else {
            return None;
        }
    }

    if octet_idx != 3 {
        return None;
    }
    octets[3] = current;

    Some(Ipv4Address(octets))
}

/// Parse a u16 from a string.
fn parse_u16(s: &str) -> Option<u16> {
    let mut value: u16 = 0;
    for ch in s.bytes() {
        if ch < b'0' || ch > b'9' {
            return None;
        }
        value = value * 10 + (ch - b'0') as u16;
    }
    Some(value)
}
