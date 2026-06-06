//! # DHCP Client
//!
//! DHCP (Dynamic Host Configuration Protocol) automatically configures
//! network parameters (IP address, netmask, gateway, DNS).
//!
//! ## DHCP Message Format
//!
//! ```
//! 0                   8                   16                  24                  32
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |     op (1=req)  |   htype (1)   |   hlen (6)    |   hops (0)    |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                          xid (transaction ID)                                 |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |         secs (0)                |           flags (0x8000 = broadcast)        |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                       ciaddr (client IP - 0)                                  |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                       yiaddr (your IP - offered)                              |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                       siaddr (server IP)                                      |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                       giaddr (gateway IP - 0)                                 |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                                                                       |
//! |                       chaddr (client hardware addr - 16 bytes)        |
//! |                                                                       |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                                                                       |
//! |                       sname (server name - 64 bytes, null)            |
//! |                                                                       |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                                                                       |
//! |                       file (boot file - 128 bytes, null)              |
//! |                                                                       |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                       options (variable, magic cookie + options)      |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! ```

use super::*;

// ---------------------------------------------------------------------------
// DHCP Constants
// ---------------------------------------------------------------------------

/// DHCP magic cookie (first 4 bytes of options).
pub const DHCP_MAGIC_COOKIE: [u8; 4] = [0x63, 0x82, 0x53, 0x63];

/// BOOTP operation codes.
pub const BOOTP_REQUEST: u8 = 1;
pub const BOOTP_REPLY: u8 = 2;

/// Hardware type: Ethernet.
pub const HTYPE_ETHERNET: u8 = 1;

/// DHCP message types (option 53).
pub const DHCPDISCOVER: u8 = 1;
pub const DHCPOFFER: u8 = 2;
pub const DHCPREQUEST: u8 = 3;
pub const DHCPDECLINE: u8 = 4;
pub const DHCPACK: u8 = 5;
pub const DHCPNAK: u8 = 6;
pub const DHCPRELEASE: u8 = 7;
pub const DHCPINFORM: u8 = 8;

/// DHCP option codes.
pub const OPT_PAD: u8 = 0;
pub const OPT_SUBNET_MASK: u8 = 1;
pub const OPT_ROUTER: u8 = 3;
pub const OPT_DNS_SERVER: u8 = 6;
pub const OPT_HOSTNAME: u8 = 12;
pub const OPT_DOMAIN_NAME: u8 = 15;
pub const OPT_REQUESTED_IP: u8 = 50;
pub const OPT_LEASE_TIME: u8 = 51;
pub const OPT_MESSAGE_TYPE: u8 = 53;
pub const OPT_SERVER_ID: u8 = 54;
pub const OPT_PARAM_REQUEST: u8 = 55;
pub const OPT_RENEWAL_TIME: u8 = 58;
pub const OPT_REBINDING_TIME: u8 = 59;
pub const OPT_CLIENT_ID: u8 = 61;
pub const OPT_END: u8 = 255;

// ---------------------------------------------------------------------------
// DHCP Packet
// ---------------------------------------------------------------------------

/// DHCP/BOOTP packet (fixed-size portion).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhcpPacket {
    /// Operation code (1 = request, 2 = reply).
    pub op: u8,
    /// Hardware address type (1 = Ethernet).
    pub htype: u8,
    /// Hardware address length (6 for Ethernet).
    pub hlen: u8,
    /// Hops (used by relay agents).
    pub hops: u8,
    /// Transaction ID.
    pub xid: u32,
    /// Seconds elapsed.
    pub secs: u16,
    /// Flags.
    pub flags: u16,
    /// Client IP address.
    pub ciaddr: Ipv4Address,
    /// Your IP address (offered by server).
    pub yiaddr: Ipv4Address,
    /// Server IP address.
    pub siaddr: Ipv4Address,
    /// Gateway IP address.
    pub giaddr: Ipv4Address,
    /// Client hardware address (16 bytes).
    pub chaddr: [u8; 16],
    /// Server name (64 bytes).
    pub sname: [u8; 64],
    /// Boot file name (128 bytes).
    pub file: [u8; 128],
    // Options follow (must start with magic cookie).
}

impl DhcpPacket {
    pub const FIXED_SIZE: usize = 236;
    pub const MIN_SIZE: usize = 240; // Fixed + magic cookie.

    /// Create a DHCP Discover packet.
    pub fn discover(xid: u32, mac: MacAddress) -> Self {
        let mut pkt = Self::new(BOOTP_REQUEST, xid, mac);
        pkt
    }

    /// Create a DHCP Request packet.
    pub fn request(xid: u32, mac: MacAddress, requested_ip: Ipv4Address, server_ip: Ipv4Address) -> Self {
        let mut pkt = Self::new(BOOTP_REQUEST, xid, mac);
        // ciaddr would be set in a real implementation.
        pkt
    }

    /// Create a base DHCP packet.
    fn new(op: u8, xid: u32, mac: MacAddress) -> Self {
        let mut chaddr = [0u8; 16];
        chaddr[..6].copy_from_slice(&mac.0);

        DhcpPacket {
            op,
            htype: HTYPE_ETHERNET,
            hlen: 6,
            hops: 0,
            xid,
            secs: 0,
            flags: 0x8000, // Broadcast flag.
            ciaddr: Ipv4Address::UNSPECIFIED,
            yiaddr: Ipv4Address::UNSPECIFIED,
            siaddr: Ipv4Address::UNSPECIFIED,
            giaddr: Ipv4Address::UNSPECIFIED,
            chaddr,
            sname: [0; 64],
            file: [0; 128],
        }
    }

    /// Serialize the fixed portion to a buffer.
    pub fn serialize_fixed(&self, buf: &mut [u8]) -> usize {
        assert!(buf.len() >= Self::FIXED_SIZE);

        buf[0] = self.op;
        buf[1] = self.htype;
        buf[2] = self.hlen;
        buf[3] = self.hops;
        buf[4..8].copy_from_slice(&self.xid.to_be_bytes());
        buf[8..10].copy_from_slice(&self.secs.to_be_bytes());
        buf[10..12].copy_from_slice(&self.flags.to_be_bytes());
        buf[12..16].copy_from_slice(&self.ciaddr.0);
        buf[16..20].copy_from_slice(&self.yiaddr.0);
        buf[20..24].copy_from_slice(&self.siaddr.0);
        buf[24..28].copy_from_slice(&self.giaddr.0);
        buf[28..44].copy_from_slice(&self.chaddr);
        buf[44..108].copy_from_slice(&self.sname);
        buf[108..236].copy_from_slice(&self.file);

        Self::FIXED_SIZE
    }

    /// Parse options from a buffer.
    pub fn parse_options(data: &[u8]) -> DhcpOptions {
        let mut opts = DhcpOptions::default();

        if data.len() < 4 || data[0..4] != DHCP_MAGIC_COOKIE {
            return opts; // Invalid magic cookie.
        }

        let mut i = 4;
        while i < data.len() {
            let code = data[i];
            if code == OPT_PAD {
                i += 1;
                continue;
            }
            if code == OPT_END {
                break;
            }
            if i + 1 >= data.len() {
                break;
            }
            let len = data[i + 1] as usize;
            if i + 2 + len > data.len() {
                break;
            }

            let value = &data[i + 2..i + 2 + len];

            match code {
                OPT_MESSAGE_TYPE => {
                    if len > 0 {
                        opts.message_type = Some(value[0]);
                    }
                }
                OPT_SUBNET_MASK => {
                    if len == 4 {
                        opts.subnet_mask = Some(Ipv4Address([
                            value[0], value[1], value[2], value[3],
                        ]));
                    }
                }
                OPT_ROUTER => {
                    if len >= 4 {
                        opts.router = Some(Ipv4Address([
                            value[0], value[1], value[2], value[3],
                        ]));
                    }
                }
                OPT_DNS_SERVER => {
                    if len >= 4 {
                        opts.dns_server = Some(Ipv4Address([
                            value[0], value[1], value[2], value[3],
                        ]));
                    }
                }
                OPT_LEASE_TIME => {
                    if len == 4 {
                        opts.lease_time = Some(u32::from_be_bytes([
                            value[0], value[1], value[2], value[3],
                        ]));
                    }
                }
                OPT_SERVER_ID => {
                    if len == 4 {
                        opts.server_id = Some(Ipv4Address([
                            value[0], value[1], value[2], value[3],
                        ]));
                    }
                }
                OPT_REQUESTED_IP => {
                    if len == 4 {
                        opts.requested_ip = Some(Ipv4Address([
                            value[0], value[1], value[2], value[3],
                        ]));
                    }
                }
                _ => {}
            }

            i += 2 + len;
        }

        opts
    }
}

// ---------------------------------------------------------------------------
// DHCP Options
// ---------------------------------------------------------------------------

/// Parsed DHCP options.
#[derive(Debug, Clone, Default)]
pub struct DhcpOptions {
    /// DHCP message type (53).
    pub message_type: Option<u8>,
    /// Subnet mask (1).
    pub subnet_mask: Option<Ipv4Address>,
    /// Router/gateway (3).
    pub router: Option<Ipv4Address>,
    /// DNS server (6).
    pub dns_server: Option<Ipv4Address>,
    /// Lease time in seconds (51).
    pub lease_time: Option<u32>,
    /// DHCP server identifier (54).
    pub server_id: Option<Ipv4Address>,
    /// Requested IP (50).
    pub requested_ip: Option<Ipv4Address>,
    /// Hostname (12).
    pub hostname: Option<()>,
}

// ---------------------------------------------------------------------------
// DHCP Client State Machine
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DhcpState {
    /// Not started.
    Idle,
    /// Sent DISCOVER, waiting for OFFER.
    Selecting,
    /// Sent REQUEST, waiting for ACK.
    Requesting,
    /// Bound with a lease.
    Bound,
    /// Renewing lease (unicast to server).
    Renewing,
    /// Rebinding lease (broadcast).
    Rebinding,
}

// ---------------------------------------------------------------------------
// DHCP Client
// ---------------------------------------------------------------------------

/// DHCP client state.
pub struct DhcpClient {
    pub state: DhcpState,
    pub xid: u32,
    pub mac: MacAddress,
    pub offered_ip: Ipv4Address,
    pub server_ip: Ipv4Address,
    pub lease_time: u32,
    pub lease_start: u64,
    pub retries: u32,
}

impl DhcpClient {
    /// Create a new DHCP client.
    pub fn new(mac: MacAddress) -> Self {
        DhcpClient {
            state: DhcpState::Idle,
            xid: 0x12345678, // Should be random.
            mac,
            offered_ip: Ipv4Address::UNSPECIFIED,
            server_ip: Ipv4Address::UNSPECIFIED,
            lease_time: 0,
            lease_start: 0,
            retries: 0,
        }
    }

    /// Start DHCP (send DISCOVER).
    pub fn start(&mut self) {
        self.state = DhcpState::Selecting;
        self.retries = 0;

        // Build and send DHCP Discover.
        let discover = DhcpPacket::discover(self.xid, self.mac);

        // Build options.
        let mut options = [0u8; 64];
        let mut opt_len = 0;

        // Magic cookie.
        options[0..4].copy_from_slice(&DHCP_MAGIC_COOKIE);
        opt_len += 4;

        // Message type: DHCP Discover.
        options[opt_len] = OPT_MESSAGE_TYPE;
        options[opt_len + 1] = 1;
        options[opt_len + 2] = DHCPDISCOVER;
        opt_len += 3;

        // Parameter request list.
        options[opt_len] = OPT_PARAM_REQUEST;
        options[opt_len + 1] = 4;
        options[opt_len + 2] = OPT_SUBNET_MASK as u8;
        options[opt_len + 3] = OPT_ROUTER as u8;
        options[opt_len + 4] = OPT_DNS_SERVER as u8;
        options[opt_len + 5] = OPT_DOMAIN_NAME as u8;
        opt_len += 6;

        // Client identifier.
        options[opt_len] = OPT_CLIENT_ID;
        options[opt_len + 1] = 7;
        options[opt_len + 2] = HTYPE_ETHERNET;
        options[opt_len + 3..opt_len + 9].copy_from_slice(&self.mac.0);
        opt_len += 9;

        // End.
        options[opt_len] = OPT_END;
        opt_len += 1;

        // Build full packet (fixed + options).
        // In a real implementation, this would be sent as a UDP datagram
        // to 255.255.255.255:67 from 0.0.0.0:68.
    }

    /// Process a received DHCP packet.
    pub fn process_packet(&mut self, pkt: &DhcpPacket, opts: &DhcpOptions) {
        // Verify xid matches.
        if pkt.xid != self.xid {
            return;
        }

        match self.state {
            DhcpState::Selecting => {
                if opts.message_type == Some(DHCPOFFER) {
                    self.offered_ip = pkt.yiaddr;
                    self.server_ip = opts.server_id.unwrap_or(pkt.siaddr);

                    // Send DHCP Request.
                    self.state = DhcpState::Requesting;
                    // Build and send DHCP Request packet.
                }
            }
            DhcpState::Requesting => {
                if opts.message_type == Some(DHCPACK) {
                    // Lease acquired!
                    self.lease_time = opts.lease_time.unwrap_or(3600);
                    self.lease_start = 0; // Should be current time.

                    // Apply configuration.
                    unsafe {
                        let config = super::config_mut();
                        config.ip = self.offered_ip;
                        config.netmask = opts.subnet_mask.unwrap_or(Ipv4Address::new(255, 255, 255, 0));
                        config.gateway = opts.router.unwrap_or(Ipv4Address::UNSPECIFIED);
                        config.dns = opts.dns_server.unwrap_or(Ipv4Address::UNSPECIFIED);
                    }

                    self.state = DhcpState::Bound;
                } else if opts.message_type == Some(DHCPNAK) {
                    // NAK received - start over.
                    self.state = DhcpState::Idle;
                }
            }
            DhcpState::Renewing | DhcpState::Rebinding => {
                if opts.message_type == Some(DHCPACK) {
                    self.lease_time = opts.lease_time.unwrap_or(self.lease_time);
                    self.lease_start = 0;
                    self.state = DhcpState::Bound;
                }
            }
            _ => {}
        }
    }

    /// Check if the lease needs renewal.
    pub fn poll(&mut self, _current_time: u64) {
        if self.state != DhcpState::Bound {
            return;
        }

        // In a real implementation:
        // if elapsed > lease_time / 2: state = Renewing, send unicast request.
        // if elapsed > lease_time * 0.875: state = Rebinding, send broadcast request.
        // if elapsed > lease_time: state = Idle, config lost.
    }

    /// Release the lease.
    pub fn release(&mut self) {
        if self.state == DhcpState::Bound {
            // Send DHCP Release.
            self.state = DhcpState::Idle;

            // Clear configuration.
            unsafe {
                let config = super::config_mut();
                config.ip = Ipv4Address::UNSPECIFIED;
                config.gateway = Ipv4Address::UNSPECIFIED;
                config.dns = Ipv4Address::UNSPECIFIED;
            }
        }
    }
}
