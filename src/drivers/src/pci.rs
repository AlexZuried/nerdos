//! # PCI Bus Driver
//!
//! The Peripheral Component Interconnect (PCI) bus is the standard
//! expansion bus for x86 PCs. This module provides:
//! - PCI configuration space access
//! - Device enumeration across all buses
//! - BAR (Base Address Register) decoding
//! - MSI/MSI-X interrupt setup (future)
//!
//! ## PCI Configuration Space
//!
//! Each PCI device has 256 bytes (or 4096 for PCIe) of configuration
//! space containing device identification, control registers, and BARs.
//!
//! ```
//! Offset  | Contents
//! --------|----------
//! 0x00    | Vendor ID (16-bit)
//! 0x02    | Device ID (16-bit)
//! 0x04    | Command (16-bit)
//! 0x06    | Status (16-bit)
//! 0x08    | Revision ID (8-bit), Class Code (24-bit)
//! 0x0C    | Cache line, Latency, Header, BIST
//! 0x10    | BAR0 (32/64-bit)
//! 0x14    | BAR1
//! ...     | BAR2-5
//! ```
//!
//! ## Access Method
//!
//! We use the legacy Configuration Space Access Mechanism #1:
//! - Write bus/device/function/offset to port 0xCF8
//! - Read/write data from/to port 0xCFC

use core::fmt;

// ---------------------------------------------------------------------------
// PCI I/O Ports
// ---------------------------------------------------------------------------

/// Configuration Address Register (selects device and register).
const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
/// Configuration Data Register (reads/writes the selected register).
const PCI_CONFIG_DATA: u16 = 0xCFC;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum number of PCI buses.
pub const MAX_BUSES: u8 = 256;
/// Maximum number of devices per bus.
pub const MAX_DEVICES: u8 = 32;
/// Maximum number of functions per device.
pub const MAX_FUNCTIONS: u8 = 8;

/// Invalid vendor ID (returns when no device is present).
pub const INVALID_VENDOR: u16 = 0xFFFF;

// ---------------------------------------------------------------------------
// Configuration Space Offsets
// ---------------------------------------------------------------------------

pub const PCI_VENDOR_ID: u8 = 0x00;
pub const PCI_DEVICE_ID: u8 = 0x02;
pub const PCI_COMMAND: u8 = 0x04;
pub const PCI_STATUS: u8 = 0x06;
pub const PCI_REVISION_ID: u8 = 0x08;
pub const PCI_PROG_IF: u8 = 0x09;
pub const PCI_SUBCLASS: u8 = 0x0A;
pub const PCI_CLASS: u8 = 0x0B;
pub const PCI_CACHE_LINE_SIZE: u8 = 0x0C;
pub const PCI_LATENCY_TIMER: u8 = 0x0D;
pub const PCI_HEADER_TYPE: u8 = 0x0E;
pub const PCI_BIST: u8 = 0x0F;
pub const PCI_BAR0: u8 = 0x10;
pub const PCI_BAR1: u8 = 0x14;
pub const PCI_BAR2: u8 = 0x18;
pub const PCI_BAR3: u8 = 0x1C;
pub const PCI_BAR4: u8 = 0x20;
pub const PCI_BAR5: u8 = 0x24;
pub const PCI_SECONDARY_BUS: u8 = 0x19;

// ---------------------------------------------------------------------------
// PCI Command Register Bits
// ---------------------------------------------------------------------------

pub const PCI_COMMAND_IO: u16 = 0x01;
pub const PCI_COMMAND_MEMORY: u16 = 0x02;
pub const PCI_COMMAND_MASTER: u16 = 0x04;
pub const PCI_COMMAND_SPECIAL: u16 = 0x08;
pub const PCI_COMMAND_INVALIDATE: u16 = 0x10;
pub const PCI_COMMAND_VGA_PALETTE: u16 = 0x20;
pub const PCI_COMMAND_PARITY: u16 = 0x40;
pub const PCI_COMMAND_WAIT: u16 = 0x80;
pub const PCI_COMMAND_SERR: u16 = 0x100;
pub const PCI_COMMAND_FAST_BACK: u16 = 0x200;
pub const PCI_COMMAND_INTX_DISABLE: u16 = 0x400;

// ---------------------------------------------------------------------------
// PCI Device ID
// ---------------------------------------------------------------------------

/// A unique identifier for a PCI device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PciDeviceId {
    /// PCI bus number (0-255).
    pub bus: u8,
    /// Device number on the bus (0-31).
    pub device: u8,
    /// Function number (0-7).
    pub function: u8,
}

impl PciDeviceId {
    /// Create a new PCI device ID.
    pub const fn new(bus: u8, device: u8, function: u8) -> Self {
        PciDeviceId { bus, device, function }
    }

    /// Format as BDF (Bus:Device.Function) string.
    pub fn bdf_string(&self) -> [u8; 12] {
        // Format: "XX:XX.X\0"
        let mut buf = [0u8; 12];
        let bus_hex = hex_bytes(self.bus);
        let dev_hex = hex_bytes(self.device);
        let func_hex = hex_byte(self.function & 0x0F);

        buf[0] = bus_hex[0];
        buf[1] = bus_hex[1];
        buf[2] = b':';
        buf[3] = dev_hex[0];
        buf[4] = dev_hex[1];
        buf[5] = b'.';
        buf[6] = func_hex;

        buf
    }
}

// Helper to convert a byte to two hex digits.
const fn hex_bytes(byte: u8) -> [u8; 2] {
    const HEX: &[u8] = b"0123456789ABCDEF";
    [HEX[(byte >> 4) as usize], HEX[(byte & 0x0F) as usize]]
}

const fn hex_byte(nibble: u8) -> u8 {
    const HEX: &[u8] = b"0123456789ABCDEF";
    HEX[(nibble & 0x0F) as usize]
}

// ---------------------------------------------------------------------------
// PCI Device Info
// ---------------------------------------------------------------------------

/// Information about a discovered PCI device.
#[derive(Debug, Clone, Copy)]
pub struct PciDeviceInfo {
    /// Device location.
    pub id: PciDeviceId,
    /// Vendor ID.
    pub vendor_id: u16,
    /// Device ID.
    pub device_id: u16,
    /// Revision ID.
    pub revision: u8,
    /// Programming interface.
    pub prog_if: u8,
    /// Subclass code.
    pub subclass: u8,
    /// Class code.
    pub class: u8,
    /// Header type.
    pub header_type: u8,
}

impl PciDeviceInfo {
    /// Get the class name string.
    pub fn class_name(&self) -> &'static str {
        match self.class {
            0x00 => "Unclassified",
            0x01 => "Mass Storage Controller",
            0x02 => "Network Controller",
            0x03 => "Display Controller",
            0x04 => "Multimedia Controller",
            0x05 => "Memory Controller",
            0x06 => "Bridge",
            0x07 => "Simple Communication Controller",
            0x08 => "Base System Peripheral",
            0x09 => "Input Device Controller",
            0x0A => "Docking Station",
            0x0B => "Processor",
            0x0C => "Serial Bus Controller",
            0x0D => "Wireless Controller",
            0x0E => "Intelligent Controller",
            0x0F => "Satellite Communications Controller",
            0x10 => "Encryption Controller",
            0x11 => "Signal Processing Controller",
            0x12 => "Processing Accelerator",
            0x13 => "Non-Essential Instrumentation",
            0x40 => "Coprocessor",
            _ => "Unknown",
        }
    }

    /// Check if this is a multi-function device.
    pub fn is_multifunction(&self) -> bool {
        (self.header_type & 0x80) != 0
    }

    /// Get the header type (masked to lower 7 bits).
    pub fn header_type(&self) -> u8 {
        self.header_type & 0x7F
    }

    /// Get a human-readable description.
    pub fn description(&self) -> [u8; 64] {
        let mut buf = [0u8; 64];
        let bdf = self.id.bdf_string();

        // Write BDF
        for (i, &b) in bdf.iter().enumerate() {
            if b == 0 { break; }
            buf[i] = b;
        }

        // Write vendor/device info
        let pos = bdf.iter().position(|&b| b == 0).unwrap_or(11);
        let info = vendor_device_string(self.vendor_id, self.device_id);

        buf[pos] = b' ';
        buf[pos + 1] = b'-';
        buf[pos + 2] = b' ';

        for (i, &b) in info.iter().enumerate() {
            if b == 0 { break; }
            if pos + 3 + i < 64 {
                buf[pos + 3 + i] = b;
            }
        }

        buf
    }
}

// ---------------------------------------------------------------------------
// Known PCI Vendors and Devices
// ---------------------------------------------------------------------------

/// Get a string describing the vendor and device.
fn vendor_device_string(vendor: u16, device: u16) -> [u8; 32] {
    let mut buf = [0u8; 32];

    // Known vendor strings (first 6 chars).
    let vendor_str = match vendor {
        0x8086 => b"Intel",
        0x1022 => b"AMD",
        0x10EC => b"Realtek",
        0x1B36 => b"QEMU",
        0x1234 => b"Bochs",
        0x80EE => b"VirtualBox",
        0x15AD => b"VMware",
        0x10DE => b"NVIDIA",
        0x1002 => b"AMD/ATI",
        0x1AF4 => b"VirtIO",
        _ => b"Unknown",
    };

    // Copy vendor string.
    let mut pos = 0;
    for &b in vendor_str.iter() {
        buf[pos] = b;
        pos += 1;
    }

    buf[pos] = b' ';
    pos += 1;

    // Add device ID in hex.
    let dev_hex = hex_bytes((device >> 8) as u8);
    buf[pos] = dev_hex[0];
    buf[pos + 1] = dev_hex[1];
    let dev_hex2 = hex_bytes(device as u8);
    buf[pos + 2] = dev_hex2[0];
    buf[pos + 3] = dev_hex2[1];

    buf
}

// ---------------------------------------------------------------------------
// PCI Access Functions
// ---------------------------------------------------------------------------

/// Read a 32-bit value from PCI configuration space.
///
/// # Arguments
/// * `bus` - PCI bus number (0-255)
/// * `device` - Device number (0-31)
/// * `function` - Function number (0-7)
/// * `offset` - Register offset (must be 4-byte aligned)
///
/// # Safety
/// This function uses I/O ports. It must not be called concurrently
/// from multiple CPUs without synchronization.
pub unsafe fn pci_read32(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    use x86_64::instructions::port::Port;

    // Build the configuration address:
    // Bit 31: Enable bit (must be 1)
    // Bits 30-24: Reserved (0)
    // Bits 23-16: Bus number
    // Bits 15-11: Device number
    // Bits 10-8: Function number
    // Bits 7-2: Register offset (dword-aligned)
    // Bits 1-0: Always 0
    let address: u32 =
        (1u32 << 31) |
        ((bus as u32) << 16) |
        ((device as u32) << 11) |
        ((function as u32) << 8) |
        ((offset as u32) & 0xFC);

    let mut address_port: Port<u32> = Port::new(PCI_CONFIG_ADDRESS);
    let mut data_port: Port<u32> = Port::new(PCI_CONFIG_DATA);

    address_port.write(address);
    data_port.read()
}

/// Write a 32-bit value to PCI configuration space.
///
/// # Safety
/// Same as `pci_read32`.
pub unsafe fn pci_write32(bus: u8, device: u8, function: u8, offset: u8, value: u32) {
    use x86_64::instructions::port::Port;

    let address: u32 =
        (1u32 << 31) |
        ((bus as u32) << 16) |
        ((device as u32) << 11) |
        ((function as u32) << 8) |
        ((offset as u32) & 0xFC);

    let mut address_port: Port<u32> = Port::new(PCI_CONFIG_ADDRESS);
    let mut data_port: Port<u32> = Port::new(PCI_CONFIG_DATA);

    address_port.write(address);
    data_port.write(value);
}

/// Read a 16-bit value from PCI configuration space.
pub unsafe fn pci_read16(bus: u8, device: u8, function: u8, offset: u8) -> u16 {
    let value = pci_read32(bus, device, function, offset);
    if offset & 2 != 0 {
        (value >> 16) as u16
    } else {
        value as u16
    }
}

/// Read an 8-bit value from PCI configuration space.
pub unsafe fn pci_read8(bus: u8, device: u8, function: u8, offset: u8) -> u8 {
    let value = pci_read32(bus, device, function, offset);
    let shift = (offset & 3) * 8;
    (value >> shift) as u8
}

// ---------------------------------------------------------------------------
// BAR Decoding
// ---------------------------------------------------------------------------

/// A decoded Base Address Register.
#[derive(Debug, Clone, Copy)]
pub enum Bar {
    /// Memory-mapped BAR with address and size.
    Memory { address: u64, size: u64, prefetchable: bool },
    /// I/O-mapped BAR with address and size.
    Io { address: u16, size: u32 },
    /// Invalid or unimplemented BAR.
    None,
}

/// Read and decode a BAR from PCI configuration space.
///
/// BARs can be either:
/// - 32-bit memory BAR: bits 0-1 = 00, bit 3 = prefetchable
/// - 64-bit memory BAR: bits 0-1 = 10, uses BAR+4 for upper 32 bits
/// - I/O BAR: bit 0 = 1
///
/// # Arguments
/// * `id` - PCI device location
/// * `bar_index` - BAR index (0-5)
///
/// # Safety
/// Accesses PCI configuration space.
pub unsafe fn read_bar(id: PciDeviceId, bar_index: u8) -> Bar {
    let offset = PCI_BAR0 + bar_index * 4;
    let bar_value = pci_read32(id.bus, id.device, id.function, offset);

    // Check if BAR is implemented.
    if bar_value == 0 {
        return Bar::None;
    }

    // Determine BAR type.
    if bar_value & 0x01 != 0 {
        // I/O BAR
        let address = (bar_value & !0x03) as u16;

        // Calculate size by writing all 1s and reading back.
        pci_write32(id.bus, id.device, id.function, offset, 0xFFFFFFFF);
        let size_mask = pci_read32(id.bus, id.device, id.function, offset);
        let size = (!(size_mask & !0x03)).wrapping_add(1) as u32;

        // Restore original value.
        pci_write32(id.bus, id.device, id.function, offset, bar_value);

        Bar::Io { address, size }
    } else {
        // Memory BAR
        let prefetchable = (bar_value & 0x08) != 0;
        let addr_type = (bar_value >> 1) & 0x03;

        let address = if addr_type == 0x02 {
            // 64-bit BAR - read upper 32 bits.
            let upper = pci_read32(id.bus, id.device, id.function, offset + 4);
            ((upper as u64) << 32) | (bar_value & !0x0F) as u64
        } else {
            // 32-bit BAR.
            (bar_value & !0x0F) as u64
        };

        // Calculate size.
        pci_write32(id.bus, id.device, id.function, offset, 0xFFFFFFFF);
        let size_mask = pci_read32(id.bus, id.device, id.function, offset);
        let size = (!(size_mask & !0x0F)).wrapping_add(1) as u64;

        // Restore original value.
        pci_write32(id.bus, id.device, id.function, offset, bar_value);

        Bar::Memory { address, size, prefetchable }
    }
}

// ---------------------------------------------------------------------------
// Device Enumeration
// ---------------------------------------------------------------------------

/// A catalog of discovered PCI devices.
pub struct PciDeviceList {
    devices: [Option<PciDeviceInfo>; 256],
    count: usize,
}

impl PciDeviceList {
    /// Create an empty device list.
    pub const fn new() -> Self {
        PciDeviceList {
            devices: [None; 256],
            count: 0,
        }
    }

    /// Add a device to the list.
    fn add(&mut self, info: PciDeviceInfo) {
        if self.count < 256 {
            self.devices[self.count] = Some(info);
            self.count += 1;
        }
    }

    /// Get the number of devices.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Check if the list is empty.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get a device by index.
    pub fn get(&self, index: usize) -> Option<&PciDeviceInfo> {
        self.devices.get(index).and_then(|d| d.as_ref())
    }

    /// Find a device by vendor and device ID.
    pub fn find_by_id(&self, vendor_id: u16, device_id: u16) -> Option<&PciDeviceInfo> {
        for i in 0..self.count {
            if let Some(dev) = &self.devices[i] {
                if dev.vendor_id == vendor_id && dev.device_id == device_id {
                    return Some(dev);
                }
            }
        }
        None
    }

    /// Find devices by class code.
    pub fn find_by_class(&self, class: u8, subclass: u8) -> impl Iterator<Item = &PciDeviceInfo> {
        let mut result: [&PciDeviceInfo; 16] = [&DUMMY; 16];
        let mut count = 0;

        for i in 0..self.count {
            if let Some(dev) = &self.devices[i] {
                if dev.class == class && dev.subclass == subclass && count < 16 {
                    result[count] = dev;
                    count += 1;
                }
            }
        }

        result.into_iter().take(count)
    }
}

static DUMMY: PciDeviceInfo = PciDeviceInfo {
    id: PciDeviceId { bus: 0, device: 0, function: 0 },
    vendor_id: 0,
    device_id: 0,
    revision: 0,
    prog_if: 0,
    subclass: 0,
    class: 0,
    header_type: 0,
};

// ---------------------------------------------------------------------------
// Global Device List
// ---------------------------------------------------------------------------

/// Global list of discovered PCI devices.
static mut PCI_DEVICES: PciDeviceList = PciDeviceList::new();

// ---------------------------------------------------------------------------
// Enumeration
// ---------------------------------------------------------------------------

/// Scan all PCI buses and catalog devices.
///
/// This should be called once during early boot.
///
/// # Safety
/// Uses I/O ports. Must not be called concurrently.
pub unsafe fn enumerate() {
    for bus in 0..MAX_BUSES {
        for device in 0..MAX_DEVICES {
            // Read vendor ID to check if a device exists.
            let vendor = pci_read16(bus, device, 0, PCI_VENDOR_ID);

            if vendor == INVALID_VENDOR {
                continue; // No device at this slot.
            }

            // Read header type to determine if multi-function.
            let header_type = pci_read8(bus, device, 0, PCI_HEADER_TYPE);
            let num_functions = if (header_type & 0x80) != 0 { MAX_FUNCTIONS } else { 1 };

            for function in 0..num_functions {
                if function > 0 {
                    // Check if this function exists.
                    let func_vendor = pci_read16(bus, device, function, PCI_VENDOR_ID);
                    if func_vendor == INVALID_VENDOR {
                        continue;
                    }
                }

                let device_id = pci_read16(bus, device, function, PCI_DEVICE_ID);
                let revision = pci_read8(bus, device, function, PCI_REVISION_ID);
                let prog_if = pci_read8(bus, device, function, PCI_PROG_IF);
                let subclass = pci_read8(bus, device, function, PCI_SUBCLASS);
                let class = pci_read8(bus, device, function, PCI_CLASS);
                let ht = pci_read8(bus, device, function, PCI_HEADER_TYPE);

                let info = PciDeviceInfo {
                    id: PciDeviceId::new(bus, device, function),
                    vendor_id,
                    device_id,
                    revision,
                    prog_if,
                    subclass,
                    class,
                    header_type: ht,
                };

                PCI_DEVICES.add(info);
            }
        }
    }
}

/// Get a reference to the global PCI device list.
///
/// # Safety
/// The list must have been populated by `enumerate()`.
pub unsafe fn devices() -> &'static PciDeviceList {
    &PCI_DEVICES
}

/// Initialize the PCI subsystem.
pub fn init() {
    unsafe {
        enumerate();
    }
}
