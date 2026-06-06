//! # Hardware Drivers for NerdOS
//!
//! This crate provides drivers for common x86_64 PC hardware:
//!
//! | Driver  | Device           | Status       |
//! |---------|------------------|--------------|
//! | Keyboard| PS/2 Keyboard    | Working      |
//! | VGA     | VGA Text Mode    | Working      |
//! | Serial  | UART 16550       | Working      |
//! | AHCI    | SATA Controller  | Partial      |
//! | e1000   | Intel Ethernet   | Partial      |
//! | PCI     | PCI Bus          | Enumeration  |
//!
//! ## PCI Device Enumeration
//!
//! The PCI module provides the foundation for all PCI-based drivers
//! (AHCI, e1000, etc.). It scans all PCI buses and catalogs devices.

#![no_std]

pub mod pci;
pub mod ahci;
pub mod e1000;

// Re-export core drivers that are part of kernel_core
// (keyboard, vga, serial are in kernel_core because they're needed early)
