//! Architecture-specific code for x86_64
//! 
//! This module contains all hardware-specific initialization and drivers
//! for the x86_64 architecture.
//!
//! MODULE SIZE: ~0.1k lines | budget: 10k lines of 100k total

pub mod gdt;
pub mod idt;
pub mod interrupts;
pub mod paging;
pub mod apic;
pub mod pit;
pub mod tsc;
pub mod iommu;
pub mod syscall;

pub use gdt::init as gdt_init;
pub use idt::init as idt_init;
pub use paging::init as paging_init;
pub use apic::init as apic_init;
pub use pit::init as pit_init;
pub use tsc::calibrate as tsc_calibrate;
pub use iommu::init as iommu_init;

/// Parse Multiboot2 information structure
/// 
/// # Safety
/// The pointer must be valid and provided by the bootloader
pub unsafe fn parse_multiboot(info: *const u32) -> alloc::vec::Vec<MemoryRegion> {
    if info.is_null() {
        return alloc::vec![];
    }
    
    // TODO: Implement proper multiboot parsing
    // For now, return a dummy memory region
    alloc::vec![MemoryRegion {
        base: 0x100000,
        size: 0x1000000,
        region_type: MemoryRegionType::Available,
    }]
}

/// Memory region from bootloader
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub base: u64,
    pub size: u64,
    pub region_type: MemoryRegionType,
}

/// Type of memory region
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionType {
    Available,
    Reserved,
    ACPI,
    NVS,
    Bad,
}
