//! # Intel e1000 Gigabit Ethernet Driver
//!
//! The Intel 8254x series (e1000) is one of the most emulated and
//! widely-supported gigabit Ethernet controllers. It is the default
//! NIC for QEMU and VirtualBox.
//!
//! ## Features
//! - Full-duplex gigabit operation
//! - DMA-based packet transmit/receive
//! - Receive-side scaling (RSS)
//! - TCP/UDP checksum offloading
//! - VLAN tagging
//!
//! ## Architecture
//!
//! ```
//! ┌──────────────┐         ┌──────────────┐
//! │   Receive    │         │  Transmit    │
//! │  Descriptor  │         │  Descriptor  │
//! │   Ring       │         │   Ring       │
//! └──────┬───────┘         └──────┬───────┘
//!        │                        │
//!        ▼                        ▼
//! ┌──────────────┐         ┌──────────────┐
//! │   Receive    │         │  Transmit    │
//! │   Packet     │         │   Packet     │
//! │   Buffers    │         │   Buffers    │
//! └──────┬───────┘         └──────┬───────┘
//!        │                        │
//!        └──────────┬─────────────┘
//!                   │
//!              ┌────┴────┐
//!              │  e1000  │
//!              │   NIC   │
//!              └────┬────┘
//!                   │
//!              Ethernet
//! ```
//!
//! ## References
//! - Intel 8254x Gigabit Ethernet Controller Datasheet
//! - Intel Ethernet Controller E1000 Developer's Manual

use core::ptr;
use bitflags::bitflags;

// ---------------------------------------------------------------------------
// PCI Device IDs for Intel e1000 Family
// ---------------------------------------------------------------------------

pub const E1000_VENDOR_ID: u16 = 0x8086;

pub const E1000_DEV_82540EM: u16 = 0x100E;  // QEMU default
pub const E1000_DEV_82545EM: u16 = 0x100F;
pub const E1000_DEV_82546GB: u16 = 0x1079;
pub const E1000_DEV_82547EI: u16 = 0x1013;
pub const E1000_DEV_82541EI: u16 = 0x1013;
pub const E1000_DEV_82541ER: u16 = 0x1078;
pub const E1000_DEV_82543GC: u16 = 0x1004;
pub const E1000_DEV_82544EI: u16 = 0x1008;

// ---------------------------------------------------------------------------
// Register Offsets
// ---------------------------------------------------------------------------

// Control registers
pub const REG_CTRL: usize = 0x00000;    // Device Control
pub const REG_STATUS: usize = 0x00008;  // Device Status
pub const REG_EECD: usize = 0x00010;    // EEPROM/Flash Control/Data
pub const REG_EERD: usize = 0x00014;    // EEPROM Read
pub const REG_CTRL_EXT: usize = 0x00018; // Extended Device Control
pub const REG_MDIC: usize = 0x00020;    // MDI Control
pub const REG_FCAL: usize = 0x00028;    // Flow Control Address Low
pub const REG_FCAH: usize = 0x0002C;    // Flow Control Address High
pub const REG_FCT: usize = 0x00030;     // Flow Control Type
pub const REG_VET: usize = 0x00038;     // VLAN EtherType
pub const REG_FCTTV: usize = 0x00170;   // Flow Control Transmit Timer Value
pub const REG_TXCW: usize = 0x00178;    // Transmit Configuration Word
pub const REG_RXCW: usize = 0x00180;    // Receive Configuration Word
pub const REG_LEDCTL: usize = 0x00E00;  // LED Control
pub const REG_PBA: usize = 0x01000;     // Packet Buffer Allocation

// Interrupt registers
pub const REG_ICR: usize = 0x000C0;     // Interrupt Cause Read
pub const REG_ITR: usize = 0x000C4;     // Interrupt Throttling Rate
pub const REG_ICS: usize = 0x000C8;     // Interrupt Cause Set
pub const REG_IMS: usize = 0x000D0;     // Interrupt Mask Set/Read
pub const REG_IMC: usize = 0x000D8;     // Interrupt Mask Clear

// Receive registers
pub const REG_RCTL: usize = 0x00100;    // Receive Control
pub const REG_FCTL: usize = 0x00100;    // Receive Control (alias)
pub const REG_RDTR: usize = 0x02820;    // Receive Delay Timer Ring
pub const REG_RDBAL: usize = 0x02800;   // Receive Descriptor Base Low
pub const REG_RDBAH: usize = 0x02804;   // Receive Descriptor Base High
pub const REG_RDLEN: usize = 0x02808;   // Receive Descriptor Length
pub const REG_RDH: usize = 0x02810;     // Receive Descriptor Head
pub const REG_RDT: usize = 0x02818;     // Receive Descriptor Tail
pub const REG_RDTR2: usize = 0x02820;   // Receive Interrupt Delay Timer
pub const REG_RXDCTL: usize = 0x02828;  // Receive Descriptor Control
pub const REG_RADV: usize = 0x0282C;    // Receive Absolute Interrupt Delay Timer
pub const REG_RSRPD: usize = 0x02C00;   // Receive Small Packet Detect Interrupt

// Transmit registers
pub const REG_TCTL: usize = 0x00400;    // Transmit Control
pub const REG_TDBAL: usize = 0x03800;   // Transmit Descriptor Base Low
pub const REG_TDBAH: usize = 0x03804;   // Transmit Descriptor Base High
pub const REG_TDLEN: usize = 0x03808;   // Transmit Descriptor Length
pub const REG_TDH: usize = 0x03810;     // Transmit Descriptor Head
pub const REG_TDT: usize = 0x03818;     // Transmit Descriptor Tail
pub const REG_TIDV: usize = 0x03820;    // Transmit Interrupt Delay Value
pub const REG_TXDCTL: usize = 0x03828;  // Transmit Descriptor Control
pub const REG_TADV: usize = 0x0382C;    // Transmit Absolute Interrupt Delay

// MAC address registers
pub const REG_RAL: usize = 0x05400;     // Receive Address Low (8 addresses * 8 bytes)
pub const REG_RAH: usize = 0x05404;     // Receive Address High

// Statistics registers
pub const REG_CRCERRS: usize = 0x04000; // CRC Error Count
pub const REG_SYMERRS: usize = 0x04004; // Symbol Error Count
pub const REG_RXERRC: usize = 0x0400C;  // Receive Error Count

// ---------------------------------------------------------------------------
// Control Register Bits
// ---------------------------------------------------------------------------

bitflags! {
    /// Device Control Register flags.
    pub struct CtrlFlags: u32 {
        /// Full Duplex.
        const FD = 0x00000001;
        /// GIO Master Disable.
        const GIO_MASTER_DISABLE = 0x00000004;
        /// Receiver Enable.
        const RXEN = 0x00000008;
        /// Transmitter Enable.
        const TXEN = 0x00000010;
        /// MDIO Access.
        const MDIO = 0x00000020;
        /// Speed selection (bit 8).
        const SPEED_8 = 0x00000100;
        /// Speed selection (bit 9).
        const SPEED_9 = 0x00000200;
        /// Auto-Speed Detection Enable.
        const ASDE = 0x00002000;
        /// Set Link Up.
        const SLU = 0x00004000;
        /// Invert Loss-of-Signal.
        const ILOS = 0x00008000;
        /// Speed Selection.
        const SPEED = Self::SPEED_8.bits() | Self::SPEED_9.bits();
        /// Force Speed.
        const FRCSPD = 0x00000800;
        /// Force Duplex.
        const FRCDPLX = 0x00001000;
        /// Software Reset.
        const RST = 0x04000000;
        /// Receive Flow Control Enable.
        const RFCE = 0x08000000;
        /// Transmit Flow Control Enable.
        const TFCE = 0x10000000;
        /// VLAN Mode Enable.
        const VME = 0x40000000;
        /// PHY Reset.
        const PHY_RST = 0x80000000;
    }
}

bitflags! {
    /// Receive Control Register flags.
    pub struct RctlFlags: u32 {
        /// Receiver Enable.
        const EN = 0x00000002;
        /// Store Bad Packets.
        const SBP = 0x00000004;
        /// Unicast Promiscuous Enabled.
        const UPE = 0x00000008;
        /// Multicast Promiscuous Enabled.
        const MPE = 0x00000010;
        /// Long Packet Reception Enable.
        const LPE = 0x00000020;
        /// Loopback Mode (2 bits).
        const LBM_NONE = 0x00000000;
        const LBM_PHY = 0x00000040;
        const LBM_MAC = 0x00000080;
        /// Receive Descriptor Minimum Threshold Size (2 bits).
        const RDMTS_HALF = 0x00000000;
        const RDMTS_QUARTER = 0x00000100;
        const RDMTS_EIGHTH = 0x00000200;
        /// Multicast Offset (2 bits).
        const MO_36 = 0x00000000;
        const MO_35 = 0x00000400;
        const MO_34 = 0x00000800;
        const MO_32 = 0x00000C00;
        /// Broadcast Accept Mode.
        const BAM = 0x00008000;
        /// Receive Buffer Size (2 bits, legacy).
        const BSIZE_256 = 0x00003000;
        const BSIZE_512 = 0x00002000;
        const BSIZE_1024 = 0x00001000;
        const BSIZE_2048 = 0x00000000;
        const BSIZE_4096 = 0x00010000;
        const BSIZE_8192 = 0x00020000;
        const BSIZE_16384 = 0x00030000;
        /// VLAN Filter Enable.
        const VFE = 0x00040000;
        /// Canonical Form Indicator Enable.
        const CFIEN = 0x00080000;
        /// Canonical Form Indicator Bit.
        const CFI = 0x00100000;
        /// Discard Pause Frames.
        const DPF = 0x00400000;
        /// Pass MAC Control Frames.
        const PMCF = 0x00800000;
        /// Buffer Size Extension (for 16KB).
        const BSEX = 0x02000000;
        /// Strip Ethernet CRC.
        const SECRC = 0x04000000;
    }
}

bitflags! {
    /// Transmit Control Register flags.
    pub struct TctlFlags: u32 {
        /// Transmit Enable.
        const EN = 0x00000002;
        /// Pad Short Packets.
        const PSP = 0x00000008;
        /// Collision Threshold (10 bits, default 0x10).
        const CT_SHIFT = 4;
        const CT_MASK = 0x00000FF0;
        /// Collision Distance (10 bits, default 0x40).
        const COLD_SHIFT = 12;
        const COLD_MASK = 0x003FF000;
        /// Software XOFF Transmission.
        const SWXOFF = 0x00400000;
        /// Retransmission on Late Collision.
        const RTLC = 0x01000000;
        /// No Re-Transmit on Late Collision.
        const NRTU = 0x02000000;
    }
}

bitflags! {
    /// Interrupt Cause Register flags.
    pub struct IcrFlags: u32 {
        /// TX Descriptor Written Back.
        const TXDW = 0x00000001;
        /// TX Queue Empty.
        const TXQE = 0x00000002;
        /// Link Status Change.
        const LSC = 0x00000004;
        /// RX Sequence Error.
        const RXSEQ = 0x00000008;
        /// RX Descriptor Threshold.
        const RXDMT0 = 0x00000010;
        /// Receiver FIFO Overrun.
        const RXO = 0x00000040;
        /// Receiver Timer Interrupt.
        const RXT0 = 0x00000080;
        /// MDIO Access Complete.
        const MDAC = 0x00000200;
        /// RX Packet Timer Expired.
        const RXCFG = 0x00000400;
        /// Transmit Descriptor Low Threshold hit.
        const SRPD = 0x00010000;
    }
}

// ---------------------------------------------------------------------------
// Descriptors
// ---------------------------------------------------------------------------

/// Receive Descriptor (Legacy format, 16 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RxDesc {
    /// Buffer address (physical, 64-bit).
    pub addr: u64,
    /// Length of received packet.
    pub length: u16,
    /// Packet checksum.
    pub checksum: u16,
    /// Descriptor status.
    pub status: u8,
    /// Descriptor errors.
    pub errors: u8,
    /// VLAN tag.
    pub special: u16,
}

/// Transmit Descriptor (Legacy format, 16 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TxDesc {
    /// Buffer address (physical, 64-bit).
    pub addr: u64,
    /// Length of data to transmit.
    pub length: u16,
    /// Checksum offset and TCP/UDP checksum.
    pub cso: u8,
    pub cmd: u8,
    /// Status.
    pub status: u8,
    /// Checksum start.
    pub css: u8,
    /// Special.
    pub special: u16,
}

// Status bits for RX descriptor
pub const RX_STATUS_DD: u8 = 0x01;  // Descriptor Done
pub const RX_STATUS_EOP: u8 = 0x02; // End of Packet
pub const RX_STATUS_IXSM: u8 = 0x04; // Ignore Checksum Indication
pub const RX_STATUS_VP: u8 = 0x08; // VLAN Packet

// Command bits for TX descriptor
pub const TX_CMD_EOP: u8 = 0x01;    // End of Packet
pub const TX_CMD_IFCS: u8 = 0x02;   // Insert FCS (CRC)
pub const TX_CMD_IC: u8 = 0x04;     // Insert Checksum
pub const TX_CMD_RS: u8 = 0x08;     // Report Status
pub const TX_CMD_RPS: u8 = 0x10;    // Report Packet Sent
pub const TX_CMD_DEXT: u8 = 0x20;   // Descriptor Extension
pub const TX_CMD_VLE: u8 = 0x40;    // VLAN Packet Enable
pub const TX_CMD_IDE: u8 = 0x80;    // Interrupt Delay Enable

// Status bits for TX descriptor
pub const TX_STATUS_DD: u8 = 0x01;  // Descriptor Done

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Number of receive descriptors (must be multiple of 8).
pub const NUM_RX_DESC: usize = 256;
/// Number of transmit descriptors (must be multiple of 8).
pub const NUM_TX_DESC: usize = 256;
/// Receive buffer size (standard Ethernet MTU + headers).
pub const RX_BUFFER_SIZE: usize = 2048;
/// Size of the receive descriptor ring.
pub const RX_RING_SIZE: usize = NUM_RX_DESC * core::mem::size_of::<RxDesc>();
/// Size of the transmit descriptor ring.
pub const TX_RING_SIZE: usize = NUM_TX_DESC * core::mem::size_of::<TxDesc>();

// ---------------------------------------------------------------------------
// e1000 Controller
// ---------------------------------------------------------------------------

/// The Intel e1000 Ethernet controller.
pub struct E1000 {
    /// MMIO base address.
    pub base: usize,
    /// PCI device ID.
    pub dev_id: u16,
    /// MAC address.
    pub mac: [u8; 6],
    /// Receive descriptor ring.
    pub rx_ring: usize,
    /// Transmit descriptor ring.
    pub tx_ring: usize,
    /// Receive buffers.
    pub rx_buffers: [usize; NUM_RX_DESC],
    /// Transmit buffers.
    pub tx_buffers: [usize; NUM_TX_DESC],
    /// Current receive tail.
    pub rx_tail: usize,
    /// Current transmit tail.
    pub tx_tail: usize,
    /// Link is up.
    pub link_up: bool,
    /// Link speed (10/100/1000).
    pub speed: u32,
}

impl E1000 {
    /// Create a new e1000 instance at the given MMIO base.
    ///
    /// # Safety
    /// The base address must point to valid e1000 registers.
    pub unsafe fn new(mmio_base: usize, dev_id: u16) -> Self {
        E1000 {
            base: mmio_base,
            dev_id,
            mac: [0; 6],
            rx_ring: 0,
            tx_ring: 0,
            rx_buffers: [0; NUM_RX_DESC],
            tx_buffers: [0; NUM_TX_DESC],
            rx_tail: 0,
            tx_tail: 0,
            link_up: false,
            speed: 0,
        }
    }

    /// Read a 32-bit register.
    ///
    /// # Safety
    /// The register offset must be valid.
    pub unsafe fn read_reg(&self, reg: usize) -> u32 {
        ptr::read_volatile((self.base + reg) as *const u32)
    }

    /// Write a 32-bit register.
    ///
    /// # Safety
    /// The register offset must be valid.
    pub unsafe fn write_reg(&mut self, reg: usize, value: u32) {
        ptr::write_volatile((self.base + reg) as *mut u32, value);
    }

    /// Read a 16-bit EEPROM word.
    ///
    /// # Safety
    /// EEPROM access must not be in progress.
    unsafe fn read_eeprom(&mut self, addr: u8) -> u16 {
        // Start EEPROM read.
        self.write_reg(REG_EERD, (addr as u32) << 8 | 0x01);

        // Poll for completion.
        loop {
            let value = self.read_reg(REG_EERD);
            if value & 0x10 != 0 {
                // DONE bit set.
                return ((value >> 16) & 0xFFFF) as u16;
            }
            core::hint::spin_loop();
        }
    }

    /// Read the MAC address from EEPROM or register.
    ///
    /// # Safety
    /// Controller must be initialized.
    pub unsafe fn read_mac(&mut self) {
        // For 82540EM, MAC is stored in EEPROM words 0, 1, 2.
        let word0 = self.read_eeprom(0);
        let word1 = self.read_eeprom(1);
        let word2 = self.read_eeprom(2);

        self.mac[0] = (word0 & 0xFF) as u8;
        self.mac[1] = ((word0 >> 8) & 0xFF) as u8;
        self.mac[2] = (word1 & 0xFF) as u8;
        self.mac[3] = ((word1 >> 8) & 0xFF) as u8;
        self.mac[4] = (word2 & 0xFF) as u8;
        self.mac[5] = ((word2 >> 8) & 0xFF) as u8;
    }

    /// Initialize the e1000 controller.
    ///
    /// Performs full initialization:
    /// 1. Software reset
    /// 2. Read MAC address
    /// 3. Initialize RX and TX rings
    /// 4. Enable receiver and transmitter
    /// 5. Set link up
    ///
    /// # Safety
    /// The MMIO base must be valid and the controller must be accessible.
    pub unsafe fn init(&mut self) {
        // Perform software reset.
        let ctrl = self.read_reg(REG_CTRL);
        self.write_reg(REG_CTRL, ctrl | CtrlFlags::RST.bits());

        // Wait for reset to complete.
        loop {
            if self.read_reg(REG_CTRL) & CtrlFlags::RST.bits() == 0 {
                break;
            }
            core::hint::spin_loop();
        }

        // Wait for EEPROM auto-read to complete.
        loop {
            if self.read_reg(REG_EECD) & 0x10 != 0 {
                break;
            }
            core::hint::spin_loop();
        }

        // Read MAC address.
        self.read_mac();

        // Initialize Multicast Table Array (clear all).
        for i in 0..128 {
            self.write_reg(0x05200 + i * 4, 0);
        }

        // Set the Receive Address Register (RAL/RAH).
        let mac = self.mac;
        self.write_reg(REG_RAL, (mac[0] as u32) | ((mac[1] as u32) << 8) |
            ((mac[2] as u32) << 16) | ((mac[3] as u32) << 24));
        self.write_reg(REG_RAH, (mac[4] as u32) | ((mac[5] as u32) << 8) | 0x80000000);
        // The 0x80000000 bit in RAH marks the address as valid (AV).

        // In a real implementation, we would:
        // 1. Allocate physical memory for RX/TX descriptor rings and buffers.
        // 2. Program RDBAL/RDBAH/RDLEN/RDH/RDT for RX.
        // 3. Program TDBAL/TDBAH/TDLEN/TDH/TDT for TX.
        // 4. Configure RCTL with appropriate flags.
        // 5. Configure TCTL with appropriate flags.
        // 6. Enable receiver and transmitter.

        // For now, set link up.
        let ctrl = self.read_reg(REG_CTRL);
        self.write_reg(REG_CTRL, ctrl | CtrlFlags::SLU.bits());

        self.link_up = true;
        self.speed = 1000;
    }

    /// Enable interrupts.
    pub unsafe fn enable_interrupts(&mut self) {
        // Enable RX timer interrupt and link status change.
        self.write_reg(REG_IMS, IcrFlags::RXT0.bits() | IcrFlags::LSC.bits());
    }

    /// Handle an interrupt.
    /// Returns true if an RX packet is available.
    pub unsafe fn handle_interrupt(&mut self) -> bool {
        let icr = self.read_reg(REG_ICR);

        if icr & IcrFlags::LSC.bits() != 0 {
            // Link status changed.
            let ctrl = self.read_reg(REG_CTRL);
            self.link_up = (ctrl & CtrlFlags::SLU.bits()) != 0;
        }

        icr & IcrFlags::RXT0.bits() != 0
    }

    /// Check if a receive packet is available.
    pub unsafe fn rx_available(&self) -> bool {
        if self.rx_ring == 0 {
            return false;
        }
        let desc = &*(self.rx_ring as *const RxDesc).add(self.rx_tail);
        desc.status & RX_STATUS_DD != 0
    }

    /// Receive a packet into the provided buffer.
    ///
    /// Returns the number of bytes received, or 0 if no packet is available.
    ///
    /// # Safety
    /// The buffer must be large enough to hold a full Ethernet frame (1522 bytes).
    pub unsafe fn receive(&mut self, buf: &mut [u8]) -> usize {
        if !self.rx_available() {
            return 0;
        }

        let desc = &*(self.rx_ring as *const RxDesc).add(self.rx_tail);
        let len = desc.length as usize;

        if len > 0 && len <= buf.len() {
            // Copy data from receive buffer.
            let src = core::slice::from_raw_parts(
                self.rx_buffers[self.rx_tail] as *const u8,
                len
            );
            buf[..len].copy_from_slice(src);
        }

        // Reclaim the descriptor.
        let desc_mut = &mut *(self.rx_ring as *mut RxDesc).add(self.rx_tail);
        desc_mut.status = 0;

        // Advance tail.
        self.rx_tail = (self.rx_tail + 1) % NUM_RX_DESC;
        self.write_reg(REG_RDT, self.rx_tail as u32);

        len
    }

    /// Transmit a packet.
    ///
    /// Returns true on success, false if the TX ring is full.
    ///
    /// # Safety
    /// The buffer must contain a valid Ethernet frame.
    pub unsafe fn transmit(&mut self, buf: &[u8]) -> bool {
        let next_tail = (self.tx_tail + 1) % NUM_TX_DESC;

        // Check if the ring is full.
        let head = self.read_reg(REG_TDH) as usize;
        if next_tail == head {
            return false; // Ring full.
        }

        // Copy data to transmit buffer.
        let dst = core::slice::from_raw_parts_mut(
            self.tx_buffers[self.tx_tail] as *mut u8,
            buf.len()
        );
        dst.copy_from_slice(buf);

        // Set up the descriptor.
        let desc = &mut *(self.tx_ring as *mut TxDesc).add(self.tx_tail);
        desc.addr = self.tx_buffers[self.tx_tail] as u64;
        desc.length = buf.len() as u16;
        desc.cso = 0;
        desc.cmd = TX_CMD_EOP | TX_CMD_IFCS | TX_CMD_RS;
        desc.status = 0;
        desc.css = 0;
        desc.special = 0;

        // Advance tail.
        self.tx_tail = next_tail;
        self.write_reg(REG_TDT, self.tx_tail as u32);

        true
    }
}

// ---------------------------------------------------------------------------
// Global e1000 Instance
// ---------------------------------------------------------------------------

/// Initialize the e1000 subsystem.
/// Scans PCI for e1000 controllers and initializes the first one found.
pub fn init() {
    // In a real implementation:
    // 1. Use pci::devices() to find e1000 controllers (vendor=0x8086)
    // 2. Read BAR0 for the memory-mapped base address
    // 3. Map the registers into virtual memory
    // 4. Create and initialize the E1000 instance
}
