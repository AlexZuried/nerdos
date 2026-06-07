//! Global Descriptor Table (GDT) setup
//! 
//! Sets up segment descriptors for x86_64 long mode.
//! 
//! # Safety
//! Direct manipulation of CPU segment registers.

use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor};
use x86_64::structures::tss::TaskStateSegment;
use spin::Mutex;

/// Kernel GDT
static GDT: Mutex<GlobalDescriptorTable> = Mutex::new(GlobalDescriptorTable::new());

/// Task State Segment for interrupt stack tables
pub static TSS: Mutex<TaskStateSegment> = Mutex::new(TaskStateSegment::new());

/// Initialize the GDT
/// 
/// # Safety
/// This function modifies CPU segment registers and must only be called once during boot.
pub unsafe fn init() {
    let mut gdt = GDT.lock();
    
    // Add code segment descriptor
    gdt.add_entry(Descriptor::kernel_code_segment());
    
    // Add data segment descriptor  
    gdt.add_entry(Descriptor::kernel_data_segment());
    
    // Add TSS descriptor
    // gdt.add_entry(Descriptor::tss_segment(&TSS.lock()));
    
    gdt.load();
}
