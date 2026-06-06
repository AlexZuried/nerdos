//! # TCP (Transmission Control Protocol)
//!
//! TCP provides reliable, ordered, connection-oriented byte stream delivery.
//!
//! ## TCP State Machine
//!
//! ```
//! CLOSED --syn--> SYN_SENT --syn,ack--> ESTABLISHED
//! CLOSED <--syn-- LISTEN --syn--> SYN_RCVD --ack--> ESTABLISHED
//!
//! ESTABLISHED --fin--> FIN_WAIT_1 --ack--> FIN_WAIT_2 --fin--> TIME_WAIT
//! ESTABLISHED <--fin-- CLOSE_WAIT --close--> LAST_ACK --ack--> CLOSED
//! ESTABLISHED --fin+ack--> CLOSING --ack--> TIME_WAIT
//! ```

use super::*;
use super::ipv4::*;

// ---------------------------------------------------------------------------
// TCP Header
// ---------------------------------------------------------------------------

/// TCP segment header (minimum 20 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TcpHeader {
    /// Source port.
    pub src_port: u16,
    /// Destination port.
    pub dst_port: u16,
    /// Sequence number.
    pub seq: u32,
    /// Acknowledgment number.
    pub ack: u32,
    /// Data offset (header length in 32-bit words) and reserved.
    pub data_offset: u8,
    /// Flags.
    pub flags: u8,
    /// Window size.
    pub window: u16,
    /// Checksum.
    pub checksum: u16,
    /// Urgent pointer.
    pub urgent: u16,
}

// TCP flags
pub const TCP_FLAG_FIN: u8 = 0x01;
pub const TCP_FLAG_SYN: u8 = 0x02;
pub const TCP_FLAG_RST: u8 = 0x04;
pub const TCP_FLAG_PSH: u8 = 0x08;
pub const TCP_FLAG_ACK: u8 = 0x10;
pub const TCP_FLAG_URG: u8 = 0x20;

impl TcpHeader {
    pub const MIN_SIZE: usize = 20;

    /// Header length in bytes.
    pub fn header_len(&self) -> usize {
        ((self.data_offset >> 4) as usize) * 4
    }

    /// Parse a TCP header.
    pub fn parse(data: &[u8]) -> Option<(Self, &[u8])> {
        if data.len() < Self::MIN_SIZE {
            return None;
        }

        let header = TcpHeader {
            src_port: u16::from_be_bytes([data[0], data[1]]),
            dst_port: u16::from_be_bytes([data[2], data[3]]),
            seq: u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
            ack: u32::from_be_bytes([data[8], data[9], data[10], data[11]]),
            data_offset: data[12],
            flags: data[13],
            window: u16::from_be_bytes([data[14], data[15]]),
            checksum: u16::from_be_bytes([data[16], data[17]]),
            urgent: u16::from_be_bytes([data[18], data[19]]),
        };

        let len = header.header_len();
        if data.len() < len {
            return None;
        }

        Some((header, &data[len..]))
    }
}

// ---------------------------------------------------------------------------
// TCP States
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

// ---------------------------------------------------------------------------
// TCP Socket
// ---------------------------------------------------------------------------

/// A TCP socket.
pub struct TcpSocket {
    /// Socket state.
    pub state: TcpState,
    /// Local address.
    pub local_addr: Ipv4Address,
    /// Local port.
    pub local_port: u16,
    /// Remote address.
    pub remote_addr: Ipv4Address,
    /// Remote port.
    pub remote_port: u16,
    /// Send sequence number.
    pub snd_nxt: u32,
    /// Send unacknowledged.
    pub snd_una: u32,
    /// Receive sequence number.
    pub rcv_nxt: u32,
    /// Receive window.
    pub rcv_wnd: u16,
    /// Send window.
    pub snd_wnd: u16,
    /// Maximum segment size.
    pub mss: u16,
    /// Receive buffer.
    pub rx_buf: [u8; 8192],
    /// Receive buffer read position.
    pub rx_read: usize,
    /// Receive buffer write position.
    pub rx_write: usize,
    /// Send buffer.
    pub tx_buf: [u8; 8192],
    /// Send buffer read position.
    pub tx_read: usize,
    /// Send buffer write position.
    pub tx_write: usize,
}

impl TcpSocket {
    pub fn new() -> Self {
        TcpSocket {
            state: TcpState::Closed,
            local_addr: Ipv4Address::UNSPECIFIED,
            local_port: 0,
            remote_addr: Ipv4Address::UNSPECIFIED,
            remote_port: 0,
            snd_nxt: 0,
            snd_una: 0,
            rcv_nxt: 0,
            rcv_wnd: 8192,
            snd_wnd: 0,
            mss: 1460,
            rx_buf: [0; 8192],
            rx_read: 0,
            rx_write: 0,
            tx_buf: [0; 8192],
            tx_read: 0,
            tx_write: 0,
        }
    }

    /// Available data in receive buffer.
    pub fn rx_available(&self) -> usize {
        if self.rx_write >= self.rx_read {
            self.rx_write - self.rx_read
        } else {
            self.rx_buf.len() - self.rx_read + self.rx_write
        }
    }

    /// Space available in receive buffer.
    pub fn rx_space(&self) -> usize {
        self.rx_buf.len() - self.rx_available() - 1
    }

    /// Read data from receive buffer.
    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let available = self.rx_available();
        let to_read = available.min(buf.len());

        for i in 0..to_read {
            buf[i] = self.rx_buf[self.rx_read];
            self.rx_read = (self.rx_read + 1) % self.rx_buf.len();
        }

        to_read
    }

    /// Write data to send buffer.
    pub fn write(&mut self, buf: &[u8]) -> usize {
        let space = if self.tx_write >= self.tx_read {
            self.tx_buf.len() - self.tx_write + self.tx_read - 1
        } else {
            self.tx_read - self.tx_write - 1
        };

        let to_write = space.min(buf.len());

        for i in 0..to_write {
            self.tx_buf[self.tx_write] = buf[i];
            self.tx_write = (self.tx_write + 1) % self.tx_buf.len();
        }

        to_write
    }
}

// ---------------------------------------------------------------------------
// Socket Table
// ---------------------------------------------------------------------------

const MAX_TCP_SOCKETS: usize = 16;
static mut TCP_SOCKETS: [Option<TcpSocket>; MAX_TCP_SOCKETS] =
    [None; MAX_TCP_SOCKETS];

/// Create a new TCP socket.
pub fn socket() -> Option<usize> {
    unsafe {
        for (i, slot) in TCP_SOCKETS.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(TcpSocket::new());
                return Some(i);
            }
        }
        None
    }
}

/// Close a TCP socket.
pub fn close(handle: usize) {
    unsafe {
        if handle < MAX_TCP_SOCKETS {
            TCP_SOCKETS[handle] = None;
        }
    }
}

/// Get a mutable reference to a TCP socket.
pub fn get_socket(handle: usize) -> Option<&'static mut TcpSocket> {
    unsafe {
        if handle < MAX_TCP_SOCKETS {
            TCP_SOCKETS[handle].as_mut()
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Packet Processing
// ---------------------------------------------------------------------------

/// Process an incoming TCP segment.
pub fn process_packet(ip_header: &Ipv4Header, payload: &[u8]) {
    let (header, data) = match TcpHeader::parse(payload) {
        Some(h) => h,
        None => return,
    };

    // Find matching socket.
    unsafe {
        for socket_opt in TCP_SOCKETS.iter_mut() {
            if let Some(ref mut socket) = socket_opt {
                if socket.local_port == header.dst_port
                    && socket.remote_port == header.src_port
                    && socket.remote_addr == ip_header.src {
                    // Process segment for this socket.
                    process_segment(socket, &header, data, ip_header);
                    return;
                }
            }
        }

        // Check for listening sockets.
        for socket_opt in TCP_SOCKETS.iter_mut() {
            if let Some(ref mut socket) = socket_opt {
                if socket.state == TcpState::Listen
                    && socket.local_port == header.dst_port {
                    // Handle incoming connection.
                    if header.flags & TCP_FLAG_SYN != 0 {
                        // Accept connection.
                        socket.state = TcpState::SynReceived;
                        socket.remote_addr = ip_header.src;
                        socket.remote_port = header.src_port;
                        socket.rcv_nxt = header.seq.wrapping_add(1);
                        socket.snd_nxt = 100; // Initial sequence number.
                        socket.snd_una = socket.snd_nxt;

                        // Send SYN-ACK.
                        // In a real implementation, queue the segment for transmission.
                    }
                    return;
                }
            }
        }
    }

    // No matching socket - send RST.
}

/// Process a TCP segment for an established socket.
fn process_segment(socket: &mut TcpSocket, header: &TcpHeader, data: &[u8], _ip: &Ipv4Header) {
    // Check ACK.
    if header.flags & TCP_FLAG_ACK != 0 {
        socket.snd_una = header.ack;
    }

    match socket.state {
        TcpState::SynSent => {
            if header.flags & TCP_FLAG_SYN != 0 {
                socket.state = TcpState::Established;
                socket.rcv_nxt = header.seq.wrapping_add(1);
                socket.snd_wnd = header.window;
                // Send ACK.
            }
        }
        TcpState::Established => {
            // Process data.
            if !data.is_empty() && header.seq == socket.rcv_nxt {
                // In-order data - buffer it.
                let to_copy = data.len().min(socket.rx_space());
                for i in 0..to_copy {
                    socket.rx_buf[socket.rx_write] = data[i];
                    socket.rx_write = (socket.rx_write + 1) % socket.rx_buf.len();
                }
                socket.rcv_nxt = header.seq.wrapping_add(to_copy as u32);

                // Send ACK.
                // In a real implementation, queue an ACK segment.
            }

            // Check for FIN.
            if header.flags & TCP_FLAG_FIN != 0 {
                socket.rcv_nxt = header.seq.wrapping_add(1);
                socket.state = TcpState::CloseWait;
                // Send ACK for FIN.
            }
        }
        TcpState::FinWait1 => {
            if header.flags & TCP_FLAG_FIN != 0 {
                if header.flags & TCP_FLAG_ACK != 0 {
                    socket.state = TcpState::TimeWait;
                } else {
                    socket.state = TcpState::Closing;
                }
                socket.rcv_nxt = header.seq.wrapping_add(1);
            } else if header.ack == socket.snd_nxt.wrapping_add(1) {
                socket.state = TcpState::FinWait2;
            }
        }
        TcpState::FinWait2 => {
            if header.flags & TCP_FLAG_FIN != 0 {
                socket.rcv_nxt = header.seq.wrapping_add(1);
                socket.state = TcpState::TimeWait;
                // Send ACK, start TIME_WAIT timer.
            }
        }
        TcpState::LastAck => {
            if header.flags & TCP_FLAG_ACK != 0 {
                socket.state = TcpState::Closed;
            }
        }
        TcpState::Closing => {
            if header.flags & TCP_FLAG_ACK != 0 {
                socket.state = TcpState::TimeWait;
            }
        }
        _ => {}
    }
}

/// Connect to a remote host (3-way handshake).
pub fn connect(handle: usize, addr: Ipv4Address, port: u16) -> Result<(), &'static str> {
    let socket = get_socket(handle).ok_or("Invalid socket")?;

    if socket.state != TcpState::Closed {
        return Err("Socket not closed");
    }

    socket.remote_addr = addr;
    socket.remote_port = port;
    socket.local_addr = super::config().ip;
    socket.local_port = allocate_local_port();
    socket.snd_nxt = 100; // Initial sequence number (should be random).
    socket.snd_una = socket.snd_nxt;
    socket.state = TcpState::SynSent;

    // Send SYN.
    // In a real implementation, build and transmit a SYN segment.

    Ok(())
}

/// Listen for incoming connections.
pub fn listen(handle: usize, port: u16) -> Result<(), &'static str> {
    let socket = get_socket(handle).ok_or("Invalid socket")?;

    socket.local_addr = super::config().ip;
    socket.local_port = port;
    socket.state = TcpState::Listen;

    Ok(())
}

/// Accept a connection (blocks until available).
pub fn accept(listen_handle: usize) -> Option<usize> {
    // In a real implementation, this would:
    // 1. Block until a new connection is established.
    // 2. Allocate a new socket for the connection.
    // 3. Return the new socket handle.
    None
}

/// Send data on a TCP socket.
pub fn send(handle: usize, buf: &[u8]) -> Result<usize, &'static str> {
    let socket = get_socket(handle).ok_or("Invalid socket")?;

    if socket.state != TcpState::Established {
        return Err("Not connected");
    }

    let written = socket.write(buf);

    // In a real implementation, this would also:
    // 1. Segment the data into MSS-sized chunks.
    // 2. Build TCP segments with proper sequence numbers.
    // 3. Queue them for transmission.
    // 4. Start retransmission timer.

    Ok(written)
}

/// Receive data from a TCP socket.
pub fn recv(handle: usize, buf: &mut [u8]) -> Result<usize, &'static str> {
    let socket = get_socket(handle).ok_or("Invalid socket")?;

    if socket.state != TcpState::Established && socket.state != TcpState::CloseWait {
        return Err("Not connected");
    }

    if socket.rx_available() == 0 {
        return Ok(0); // No data available (non-blocking).
    }

    Ok(socket.read(buf))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Allocate a local port number.
fn allocate_local_port() -> u16 {
    // Simple allocation starting from 49152 (ephemeral range).
    static mut NEXT_PORT: u16 = 49152;
    unsafe {
        let port = NEXT_PORT;
        NEXT_PORT += 1;
        if NEXT_PORT < 49152 {
            NEXT_PORT = 49152;
        }
        port
    }
}
