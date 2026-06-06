//! BoundaryOS Kernel Entry Point
//! 
//! This is the main entry point for the BoundaryOS kernel.
//! It initializes all subsystems in the correct order and starts
//! the World event loop.
//!
//! # Safety
//! This function is called from assembly boot code and must:
//! - Never return
//! - Handle all initialization errors gracefully
//! - Set up a valid panic handler before doing anything else

#![no_std]
#![no_main]
#![feature(abi_x86_64_sysv)]
#![feature(naked_functions)]

extern crate alloc;

use core::panic::PanicInfo;

// Module declarations
mod arch;
mod memory;
mod security;
mod world;
mod drivers;
mod runtime;
mod image;

// Re-export key types
pub use memory::physical::PhysicalMemoryManager;
pub use world::object::WorldObject;
pub use security::unforgeable::UnforgeableThread;

/// Kernel entry point called from boot.S
/// 
/// # Arguments
/// * `multiboot_info` - Pointer to Multiboot2 information structure
/// 
/// # Safety
/// This function must never return. It takes ownership of the hardware
/// and initializes the entire system.
#[no_mangle]
pub extern "C" fn kernel_main(multiboot_info: *const u32) -> ! {
    // Initialize serial port for debug output (before anything else!)
    drivers::serial_init();
    
    log!("BoundaryOS kernel_main entered");
    log!("Multiboot info at: {:?}", multiboot_info);
    
    // Phase 1: Architecture initialization
    arch::gdt_init();
    log!("GDT initialized");
    
    arch::idt_init();
    log!("IDT initialized");
    
    // Parse multiboot information
    let memory_map = arch::parse_multiboot(multiboot_info);
    log!("Memory map parsed: {} entries", memory_map.len());
    
    // Phase 2: Memory initialization
    memory::physical_mm_init(&memory_map);
    log!("Physical memory manager initialized");
    
    memory::paging_init();
    log!("Page tables initialized");
    
    memory::heap_init();
    log!("Kernel heap initialized");
    
    // Phase 3: Hardware initialization
    arch::iommu_init();
    log!("IOMMU initialized (stub)");
    
    arch::tsc_calibrate();
    log!("TSC calibrated");
    
    arch::apic_init();
    log!("APIC initialized");
    
    // Phase 4: Security initialization
    security::capability_table_init();
    log!("Capability table initialized");
    
    // Phase 5: Device enumeration
    drivers::pci_scan();
    log!("PCI devices scanned");
    
    drivers::exo_layer_init();
    log!("Exo-layers initialized");
    
    drivers::myth_layer_init();
    log!("Mythic objects created");
    
    // Phase 6: World initialization
    world::fossil_heap_init();
    log!("Fossil heap initialized");
    
    world::world_init();
    log!("World initialized");
    
    security::security_init();
    log!("Security subsystem initialized");
    
    // Phase 7: Runtime initialization
    runtime::pulse_loom_init();
    log!("Pulse loom initialized");
    
    // Phase 8: Interaction surface
    drivers::interaction_surface_init();
    log!("Interaction surface initialized");
    
    // Print boot banner
    print_boot_banner();
    
    // Start the World event loop (does not return)
    world::event_loop()
}

/// Print the BoundaryOS boot banner
fn print_boot_banner() {
    use core::fmt::Write;
    
    let banner = r#"
╔══════════════════════════════════════════════╗
║          BoundaryOS is awake.                ║
║                                              ║
║  objects  : [0]                              ║
║  invariants: [0]                             ║
║  capabilities: [1]                           ║
║  fossils  : [0]                              ║
║                                              ║
║  Touch a thing by naming it.                 ║
║  Suggestions: world  keyboard  screen  time  ║
╚══════════════════════════════════════════════╝
"#;
    
    // Write to VGA text buffer
    let mut writer = drivers::vga::WRITER.lock();
    let _ = writer.write_str(banner);
}

/// Panic handler - logs to serial and VGA before halting
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use core::fmt::Write;
    
    // Log to serial first (most reliable)
    log!("PANIC: {}", info);
    
    // Also write to VGA
    let mut writer = drivers::vga::WRITER.lock();
    let _ = writer.write_str("\n\x1b[31m"); // Red color
    let _ = writer.write_str("KERNEL PANIC\n");
    let _ = writer.write_str(info.to_string().as_str());
    
    // Halt
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

/// Logging macro for kernel messages
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let mut writer = $crate::drivers::serial::SERIAL.lock();
        let _ = writeln!(writer, $($arg)*);
    }};
}
