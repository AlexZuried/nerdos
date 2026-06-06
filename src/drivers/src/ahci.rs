//! # AHCI SATA Controller Driver
//!
//! AHCI (Advanced Host Controller Interface) is the standard interface
//! for SATA controllers. It provides:
//! - NCQ (Native Command Queuing)
//! - Hot-plugging support
//! - Port multiplier support
//! - DMA transfers
//!
//! ## Architecture
//!
//! ```
//! Host (CPU)                           Device (HDD/SSD)
//! ┌─────────────┐                     ┌──────────────┐
//! │ Command List│─────┐               │              │
//! │  (1KB/port) │     │               │              │
//! └─────────────┘     │    ┌─────┐    │  Command     │
│ Received FIS │─────┼───▶│ Port│───▶│  Processing  │
//! │  (4KB/port) │     │    │     │    │              │
//! └─────────────┘     │    │ AHCI│    │  Data DMA    │
//!      │              │    │ HBA │    │              │
//!      │              │    └─────┘    └──────────────┘
//!      │              │
//!      ▼              │
//! ┌─────────────┐     │
//! │ Command     │─────┘
//! │  Tables     │
//! └─────────────┘
//! ```
//!
//! ## References
//! - Intel AHCI Specification v1.3.1
//! - Serial ATA Revision 3.2

use core::ptr;
use bitflags::bitflags;

// ---------------------------------------------------------------------------
// AHCI Memory-Mapped Registers
// ---------------------------------------------------------------------------

/// AHCI Base Memory Register (ABAR) offsets.
pub mod regs {
    // Host Capabilities (CAP) - RO
    pub const CAP: usize = 0x00;
    // Global Host Control (GHC) - RW
    pub const GHC: usize = 0x04;
    // Interrupt Status (IS) - RW1C
    pub const IS: usize = 0x08;
    // Ports Implemented (PI) - RO
    pub const PI: usize = 0x0C;
    // AHCI Version (VS) - RO
    pub const VS: usize = 0x10;
    // Command Completion Coalescing Control (CCC_CTL) - RW
    pub const CCC_CTL: usize = 0x14;
    // Command Completion Coalescing Ports (CCC_PORTS) - RW
    pub const CCC_PORTS: usize = 0x18;
    // Enclosure Management Location (EM_LOC) - RO
    pub const EM_LOC: usize = 0x1C;
    // Enclosure Management Control (EM_CTL) - RW
    pub const EM_CTL: usize = 0x20;
    // Host Capabilities Extended (CAP2) - RO
    pub const CAP2: usize = 0x24;
    // BIOS/OS Handoff Control and Status (BOHC) - RW
    pub const BOHC: usize = 0x28;

    // Port registers start at offset 0x100, each port is 0x80 bytes.
    pub const PORT_BASE: usize = 0x100;
    pub const PORT_SIZE: usize = 0x80;

    // Port register offsets (relative to port base).
    pub mod port {
        // Command List Base Address (CLB) - RW
        pub const CLB: usize = 0x00;
        // Command List Base Address Upper (CLBU) - RW
        pub const CLBU: usize = 0x04;
        // FIS Base Address (FB) - RW
        pub const FB: usize = 0x08;
        // FIS Base Address Upper (FBU) - RW
        pub const FBU: usize = 0x0C;
        // Interrupt Status (IS) - RW1C
        pub const IS: usize = 0x10;
        // Interrupt Enable (IE) - RW
        pub const IE: usize = 0x14;
        // Command and Status (CMD) - RW
        pub const CMD: usize = 0x18;
        // Task File Data (TFD) - RO
        pub const TFD: usize = 0x20;
        // Signature (SIG) - RO
        pub const SIG: usize = 0x24;
        // Serial ATA Status (SSTS) - RO
        pub const SSTS: usize = 0x28;
        // Serial ATA Control (SCTL) - RW
        pub const SCTL: usize = 0x2C;
        // Serial ATA Error (SERR) - RW1C
        pub const SERR: usize = 0x30;
        // Serial ATA Active (SACT) - RW
        pub const SACT: usize = 0x34;
        // Command Issue (CI) - RW
        pub const CI: usize = 0x38;
    }
}

// ---------------------------------------------------------------------------
// Capability Register Bits
// ---------------------------------------------------------------------------

bitflags! {
    /// Global Host Control register flags.
    pub struct GhcFlags: u32 {
        /// AHCI Enable.
        const AE = 0x80000000;
        /// Interrupt Enable.
        const IE = 0x00000002;
        /// HBA Reset.
        const HR = 0x00000001;
    }
}

bitflags! {
    /// Port Command and Status register flags.
    pub struct PortCmdFlags: u32 {
        /// FIS Receive Enable.
        const FRE = 0x0010;
        /// FIS Receive Running.
        const FR = 0x4000;
        /// Start.
        const ST = 0x0001;
        /// Command List Running.
        const CR = 0x8000;
        /// FIS Receive Running.
        const FRR = 0x4000;
        /// Power On Device.
        const POD = 0x0002;
        /// Spin-Up Device.
        const SUD = 0x0004;
        /// Cold Presence Detection.
        const CLO = 0x0008;
    }
}

// ---------------------------------------------------------------------------
// FIS Types
// ---------------------------------------------------------------------------

/// FIS (Frame Information Structure) types.
pub const FIS_TYPE_REG_H2D: u8 = 0x27;   // Register FIS - Host to Device
pub const FIS_TYPE_REG_D2H: u8 = 0x34;   // Register FIS - Device to Host
pub const FIS_TYPE_DMA_ACT: u8 = 0x39;   // DMA Activate
pub const FIS_TYPE_DMA_SETUP: u8 = 0x41; // DMA Setup
pub const FIS_TYPE_DATA: u8 = 0x46;      // Data
pub const FIS_TYPE_BIST: u8 = 0x58;      // BIST
pub const FIS_TYPE_PIO_SETUP: u8 = 0x5F; // PIO Setup
pub const FIS_TYPE_DEV_BITS: u8 = 0xA1;  // Set Device Bits

// ---------------------------------------------------------------------------
// ATA Commands
// ---------------------------------------------------------------------------

pub const ATA_CMD_READ_DMA: u8 = 0xC8;
pub const ATA_CMD_READ_DMA_EXT: u8 = 0x25;
pub const ATA_CMD_WRITE_DMA: u8 = 0xCA;
pub const ATA_CMD_WRITE_DMA_EXT: u8 = 0x35;
pub const ATA_CMD_IDENTIFY: u8 = 0xEC;
pub const ATA_CMD_FLUSH_CACHE: u8 = 0xE7;
pub const ATA_CMD_FLUSH_CACHE_EXT: u8 = 0xEA;
pub const ATA_CMD_PACKET: u8 = 0xA0;
pub const ATA_CMD_READ_FPDMA: u8 = 0x60;   // NCQ read
pub const ATA_CMD_WRITE_FPDMA: u8 = 0x61;  // NCQ write

/// ATA status register bits.
pub const ATA_SR_BSY: u8 = 0x80;  // Busy
pub const ATA_SR_DRDY: u8 = 0x40; // Drive Ready
pub const ATA_SR_DRQ: u8 = 0x08;  // Data Request
pub const ATA_SR_ERR: u8 = 0x01;  // Error

// ---------------------------------------------------------------------------
// Structures
// ---------------------------------------------------------------------------

/// Host to Device Register FIS (7 DWORDs = 28 bytes).
#[repr(C, align(4))]
#[derive(Debug, Clone, Copy)]
pub struct FisRegH2D {
    /// FIS type (0x27) and update control.
    pub fis_type: u8,
    /// Port multiplier and flags.
    pub pmport: u8,
    /// Command register.
    pub command: u8,
    /// Feature register (low).
    pub featurel: u8,
    /// LBA low.
    pub lba0: u8,
    /// LBA mid.
    pub lba1: u8,
    /// LBA high.
    pub lba2: u8,
    /// Device register.
    pub device: u8,
    /// LBA low (upper).
    pub lba3: u8,
    /// LBA mid (upper).
    pub lba4: u8,
    /// LBA high (upper).
    pub lba5: u8,
    /// Feature register (high).
    pub featureh: u8,
    /// Sector count (low).
    pub countl: u16,
    /// Sector count (high).
    pub counth: u16,
    /// Reserved.
    pub icc: u8,
    /// Control register.
    pub control: u8,
    /// Reserved.
    pub reserved: [u8; 4],
}

impl FisRegH2D {
    /// Create a new H2D FIS.
    pub fn new() -> Self {
        FisRegH2D {
            fis_type: FIS_TYPE_REG_H2D,
            pmport: 1 << 7, // Set 'C' bit for command
            command: 0,
            featurel: 0,
            lba0: 0,
            lba1: 0,
            lba2: 0,
            device: 0,
            lba3: 0,
            lba4: 0,
            lba5: 0,
            featureh: 0,
            countl: 0,
            counth: 0,
            icc: 0,
            control: 0,
            reserved: [0; 4],
        }
    }
}

/// AHCI Command Header (8 DWORDs = 32 bytes).
#[repr(C, align(128))]
#[derive(Debug, Clone, Copy)]
pub struct AhciCmdHeader {
    /// Command FIS length and flags.
    pub flags: u16,
    /// PRD table length (number of entries).
    pub prdtl: u16,
    /// Physical region descriptor byte count.
    pub prdbc: u32,
    /// Command table base address (lower 32 bits).
    pub ctba: u32,
    /// Command table base address (upper 32 bits).
    pub ctbau: u32,
    /// Reserved.
    pub reserved: [u32; 4],
}

/// Physical Region Descriptor (PRD) entry.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AhciPrdtEntry {
    /// Data base address (lower 32 bits).
    pub dba: u32,
    /// Data base address (upper 32 bits).
    pub dbau: u32,
    /// Reserved.
    pub reserved: u32,
    /// Byte count and interrupt flag.
    pub dbc: u32,
}

/// AHCI Command Table.
#[repr(C, align(128))]
#[derive(Debug)]
pub struct AhciCmdTable {
    /// Command FIS (up to 64 bytes).
    pub cfis: [u8; 64],
    /// ATAPI command (up to 16 bytes).
    pub acmd: [u8; 16],
    /// Reserved.
    pub reserved: [u8; 48],
    /// PRD entries (variable, at least 1).
    pub prdt: [AhciPrdtEntry; 1],
}

// ---------------------------------------------------------------------------
// Port Signature
// ---------------------------------------------------------------------------

/// Device signatures detected on AHCI ports.
pub const SIG_SATA: u32 = 0x00000101;
pub const SIG_SATAPI: u32 = 0xEB140101;
pub const SIG_SEMB: u32 = 0xC33C0101;
pub const SIG_PM: u32 = 0x96690101;

// ---------------------------------------------------------------------------
// AHCI Port State
// ---------------------------------------------------------------------------

/// State for a single AHCI port.
pub struct AhciPort {
    /// Port number.
    pub number: u8,
    /// Port signature (device type).
    pub signature: u32,
    /// Base address of port registers.
    pub regs: usize,
    /// Command list base (virtual address).
    pub cmd_list: usize,
    /// Received FIS base (virtual address).
    pub fis_base: usize,
    /// Command table base (virtual address).
    pub cmd_table: usize,
    /// Whether the port is active.
    pub active: bool,
}

impl AhciPort {
    /// Create a new port state.
    pub fn new(number: u8, hba_base: usize) -> Self {
        AhciPort {
            number,
            signature: 0,
            regs: hba_base + regs::PORT_BASE + (number as usize) * regs::PORT_SIZE,
            cmd_list: 0,
            fis_base: 0,
            cmd_table: 0,
            active: false,
        }
    }

    /// Read a 32-bit register from this port.
    ///
    /// # Safety
    /// The port registers must be memory-mapped and accessible.
    pub unsafe fn read_reg(&self, offset: usize) -> u32 {
        ptr::read_volatile((self.regs + offset) as *const u32)
    }

    /// Write a 32-bit register to this port.
    ///
    /// # Safety
    /// The port registers must be memory-mapped and accessible.
    pub unsafe fn write_reg(&mut self, offset: usize, value: u32) {
        ptr::write_volatile((self.regs + offset) as *mut u32, value);
    }

    /// Check if a device is present on this port.
    pub unsafe fn is_present(&self) -> bool {
        let ssts = self.read_reg(regs::port::SSTS);
        let ipm = (ssts >> 8) & 0x0F; // Interface Power Management
        let det = ssts & 0x0F;         // Device Detection
        det == 3 && (ipm == 1 || ipm == 3)
    }

    /// Start command processing on this port.
    ///
    /// # Safety
    /// The port must be properly configured with a command list and FIS buffer.
    pub unsafe fn start(&mut self) {
        // Wait for CR (Command List Running) to clear.
        while self.read_reg(regs::port::CMD) & PortCmdFlags::CR.bits() != 0 {
            core::hint::spin_loop();
        }

        // Set FRE (FIS Receive Enable) and ST (Start).
        let mut cmd = self.read_reg(regs::port::CMD);
        cmd |= PortCmdFlags::FRE.bits() | PortCmdFlags::ST.bits();
        self.write_reg(regs::port::CMD, cmd);
    }

    /// Stop command processing on this port.
    ///
    /// # Safety
    /// All commands must have completed.
    pub unsafe fn stop(&mut self) {
        // Clear ST (Start) and FRE (FIS Receive Enable).
        let mut cmd = self.read_reg(regs::port::CMD);
        cmd &= !(PortCmdFlags::ST.bits() | PortCmdFlags::FRE.bits());
        self.write_reg(regs::port::CMD, cmd);

        // Wait for FR (FIS Receive Running) and CR to clear.
        while self.read_reg(regs::port::CMD) &
            (PortCmdFlags::FR.bits() | PortCmdFlags::CR.bits()) != 0 {
            core::hint::spin_loop();
        }
    }

    /// Issue a command and wait for completion.
    ///
    /// # Safety
    /// The command must be properly set up in the command list.
    pub unsafe fn issue_cmd(&mut self, slot: u8) {
        // Wait for the port to be ready.
        while (self.read_reg(regs::port::TFD) & (ATA_SR_BSY | ATA_SR_DRQ) as u32) != 0 {
            core::hint::spin_loop();
        }

        // Issue the command by setting the bit in CI.
        self.write_reg(regs::port::CI, 1u32 << slot);

        // Wait for completion.
        while self.read_reg(regs::port::CI) & (1u32 << slot) != 0 {
            core::hint::spin_loop();
        }
    }

    /// Read sectors from the device using DMA.
    ///
    /// # Arguments
    /// * `lba` - Starting Logical Block Address
    /// * `count` - Number of sectors to read
    /// * `buffer` - Destination buffer (must be at least count * 512 bytes)
    ///
    /// # Safety
    /// The buffer must be valid, writable, and large enough.
    pub unsafe fn read_sectors(&mut self, lba: u64, count: u16, buffer: &mut [u8]) -> Result<(), &'static str> {
        if count == 0 || count > 128 {
            return Err("Invalid sector count");
        }

        let slot = 0; // Use command slot 0 for simplicity.

        // Set up the command header.
        let cmd_header = &mut *(self.cmd_list as *mut AhciCmdHeader).add(slot);
        cmd_header.flags = (core::mem::size_of::<FisRegH2D>() / 4) as u16; // CFIS length in DWORDs
        cmd_header.prdtl = 1; // One PRD entry for now.
        cmd_header.prdbc = 0;
        cmd_header.ctba = self.cmd_table as u32;
        cmd_header.ctbau = (self.cmd_table >> 32) as u32;

        // Set up the command FIS.
        let cmd_table = &mut *(self.cmd_table as *mut AhciCmdTable);
        let cfis = &mut *(cmd_table.cfis.as_mut_ptr() as *mut FisRegH2D);
        *cfis = FisRegH2D::new();
        cfis.command = ATA_CMD_READ_DMA_EXT;
        cfis.device = 1 << 6; // LBA mode

        // Set up LBA.
        cfis.lba0 = (lba & 0xFF) as u8;
        cfis.lba1 = ((lba >> 8) & 0xFF) as u8;
        cfis.lba2 = ((lba >> 16) & 0xFF) as u8;
        cfis.lba3 = ((lba >> 24) & 0xFF) as u8;
        cfis.lba4 = ((lba >> 32) & 0xFF) as u8;
        cfis.lba5 = ((lba >> 40) & 0xFF) as u8;

        // Set sector count.
        cfis.countl = count;

        // Set up PRD.
        let prd = &mut cmd_table.prdt[0];
        prd.dba = buffer.as_mut_ptr() as u32;
        prd.dbau = (buffer.as_mut_ptr() as u64 >> 32) as u32;
        prd.dbc = ((count as u32 * 512) - 1) | 0x80000000; // Set interrupt bit.

        // Issue the command.
        self.issue_cmd(slot);

        Ok(())
    }

    /// Write sectors to the device using DMA.
    ///
    /// # Safety
    /// Same as `read_sectors` but the buffer must be readable.
    pub unsafe fn write_sectors(&mut self, lba: u64, count: u16, buffer: &[u8]) -> Result<(), &'static str> {
        if count == 0 || count > 128 {
            return Err("Invalid sector count");
        }

        let slot = 0;

        let cmd_header = &mut *(self.cmd_list as *mut AhciCmdHeader).add(slot);
        cmd_header.flags = (core::mem::size_of::<FisRegH2D>() / 4) as u16;
        cmd_header.prdtl = 1;
        cmd_header.prdbc = 0;
        cmd_header.ctba = self.cmd_table as u32;
        cmd_header.ctbau = (self.cmd_table >> 32) as u32;

        let cmd_table = &mut *(self.cmd_table as *mut AhciCmdTable);
        let cfis = &mut *(cmd_table.cfis.as_mut_ptr() as *mut FisRegH2D);
        *cfis = FisRegH2D::new();
        cfis.command = ATA_CMD_WRITE_DMA_EXT;
        cfis.device = 1 << 6;

        cfis.lba0 = (lba & 0xFF) as u8;
        cfis.lba1 = ((lba >> 8) & 0xFF) as u8;
        cfis.lba2 = ((lba >> 16) & 0xFF) as u8;
        cfis.lba3 = ((lba >> 24) & 0xFF) as u8;
        cfis.lba4 = ((lba >> 32) & 0xFF) as u8;
        cfis.lba5 = ((lba >> 40) & 0xFF) as u8;

        cfis.countl = count;

        let prd = &mut cmd_table.prdt[0];
        prd.dba = buffer.as_ptr() as u32;
        prd.dbau = (buffer.as_ptr() as u64 >> 32) as u32;
        prd.dbc = ((count as u32 * 512) - 1) | 0x80000000;

        self.issue_cmd(slot);

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// AHCI Controller (HBA)
// ---------------------------------------------------------------------------

/// The AHCI Host Bus Adapter (controller).
pub struct AhciController {
    /// Base address of HBA memory-mapped registers.
    pub base: usize,
    /// Number of ports implemented.
    pub num_ports: u8,
    /// Port states.
    pub ports: [Option<AhciPort>; 32],
}

impl AhciController {
    /// Create a new AHCI controller at the given memory-mapped base address.
    ///
    /// # Safety
    /// The base address must point to valid AHCI HBA registers.
    pub unsafe fn new(mmio_base: usize) -> Self {
        let pi = ptr::read_volatile((mmio_base + regs::PI) as *const u32);
        let num_ports = pi.count_ones() as u8;

        AhciController {
            base: mmio_base,
            num_ports,
            ports: core::array::from_fn(|_| None),
        }
    }

    /// Initialize the AHCI controller.
    ///
    /// Performs BIOS/OS handoff, resets the controller if needed,
    /// and probes all implemented ports for connected devices.
    ///
    /// # Safety
    /// The HBA must be memory-mapped and accessible.
    pub unsafe fn init(&mut self) {
        // Read the capabilities register.
        let cap = ptr::read_volatile((self.base + regs::CAP) as *const u32);
        let num_cmd_slots = ((cap >> 8) & 0x1F) as u8 + 1;
        let num_ports = (cap & 0x1F) as u8 + 1;

        // Enable AHCI mode.
        let ghc = ptr::read_volatile((self.base + regs::GHC) as *const u32);
        ptr::write_volatile((self.base + regs::GHC) as *mut u32, ghc | GhcFlags::AE.bits());

        // Probe each implemented port.
        let pi = ptr::read_volatile((self.base + regs::PI) as *const u32);
        for i in 0..num_ports {
            if pi & (1 << i) != 0 {
                let mut port = AhciPort::new(i, self.base);

                if port.is_present() {
                    // Read device signature.
                    port.signature = port.read_reg(regs::port::SIG);

                    // Configure the port for DMA.
                    port.stop();

                    // In a real implementation, we would:
                    // 1. Allocate physical memory for the command list (1KB, aligned to 1KB)
                    // 2. Allocate physical memory for the received FIS (4KB, aligned to 256B)
                    // 3. Allocate physical memory for command tables
                    // 4. Program CLB/FB registers with physical addresses
                    // 5. Call port.start()

                    // For now, we just note the device is present.
                    match port.signature {
                        SIG_SATA => {
                            // TODO: Initialize SATA device.
                            port.active = true;
                        }
                        SIG_SATAPI => {
                            // ATAPI device (optical drive).
                            port.active = true;
                        }
                        _ => {}
                    }
                }

                self.ports[i as usize] = Some(port);
            }
        }
    }

    /// Get a reference to a port.
    pub fn port(&self, index: usize) -> Option<&AhciPort> {
        self.ports.get(index).and_then(|p| p.as_ref())
    }

    /// Get a mutable reference to a port.
    pub fn port_mut(&mut self, index: usize) -> Option<&mut AhciPort> {
        self.ports.get_mut(index).and_then(|p| p.as_mut())
    }
}

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

/// Initialize the AHCI subsystem.
/// Scans PCI for AHCI controllers and initializes them.
pub fn init() {
    // In a real implementation, this would:
    // 1. Use pci::devices() to find AHCI controllers (class=0x01, subclass=0x06)
    // 2. Read BAR5 for the memory-mapped HBA base address
    // 3. Map the HBA registers into virtual memory
    // 4. Create and initialize AhciController instances
}
