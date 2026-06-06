//! =============================================================================
//! BoundaryOS Kernel Main Entry Point
//! =============================================================================
//! 
//! DESIGN NOTE: This is the Rust entry point called from boot.S after the CPU
//! has been switched to 64-bit long mode. It initializes all kernel subsystems
//! in the exact order specified in the BoundaryOS Master Build Prompt.
//!
//! LAWS: Nothing is Hidden (all initialization steps logged)
//!       Nothing is a Magic Incantation (every step documented)
//!
//! MODULE SIZE: ~0.1k lines | budget: Mk lines of Tk total
//! =============================================================================

#![no_std]
#![no_main]
#![feature(abi_x86_64)]
#![feature(naked_functions)]

// Core Rust imports
use core::panic::PanicInfo;

// Module declarations
mod panic;

/// Multiboot2 information structure
/// Source: https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html
#[repr(C)]
struct MultibootInfo {
    total_size: u32,
    _reserved: u32,
    // Tags follow...
}

/// Kernel entry point called from boot.S
/// 
/// # Arguments
/// * `multiboot_info` - Pointer to Multiboot2 information structure provided by GRUB
/// 
/// # Safety
/// This function is marked unsafe because:
/// - It dereferences a raw pointer from the bootloader
/// - It calls other unsafe initialization functions
/// - It must be called only after proper CPU initialization
/// 
/// DESIGN NOTE: The multiboot_info pointer is passed in RDI per System V AMD64 ABI
#[no_mangle]
pub extern "C" fn kernel_main(multiboot_info: *const MultibootInfo) -> ! {
    // SAFETY: Boot sequence follows exact order from specification
    // Each step is logged via serial before execution
    
    // 1. Initialize serial port for debug output
    // SAFETY: Serial port I/O ports are well-defined (0x3F8-0x3FF)
    unsafe {
        serial_init();
    }
    
    log!("╔══════════════════════════════════════════════╗");
    log!("║     BoundaryOS kernel_main entered           ║");
    log!("╚══════════════════════════════════════════════╝");
    
    // 2. Initialize GDT
    // SAFETY: GDT setup requires inline assembly
    unsafe {
        gdt_init();
    }
    log!("[+] GDT initialized");
    
    // 3. Initialize IDT
    // SAFETY: IDT setup requires inline assembly
    unsafe {
        idt_init();
    }
    log!("[+] IDT initialized");
    
    // 4. Parse Multiboot2 information
    // SAFETY: Pointer validated by bootloader contract
    let mb_info = unsafe { &*multiboot_info };
    log!("[+] Multiboot2 info parsed (size: {} bytes)", mb_info.total_size);
    
    // 5. Initialize physical memory manager
    // SAFETY: Uses memory map from bootloader
    unsafe {
        physical_mm_init(mb_info);
    }
    log!("[+] Physical memory manager initialized");
    
    // 6. Initialize paging
    // SAFETY: Page table manipulation requires privileged instructions
    unsafe {
        paging_init();
    }
    log!("[+] Paging initialized");
    
    // 7. Initialize kernel heap
    // SAFETY: Heap uses reserved memory region from linker script
    unsafe {
        heap_init();
    }
    log!("[+] Kernel heap initialized");
    
    // 8. Print boot banner
    print_boot_banner();
    
    // 9. Enter world event loop (never returns)
    log!("[+] Starting World event loop...");
    world_event_loop()
}

/// Initialize serial port (COM1)
/// 
/// # Safety
/// Accesses I/O ports 0x3F8-0x3FF which are hardware-specific
#[inline(always)]
unsafe fn serial_init() {
    // Disable interrupts
    outb(0x3F9, 0x00);
    
    // Enable DLAB (set baud rate divisor)
    outb(0x3F8 + 3, 0x80);
    
    // Set divisor to 3 (38400 baud)
    outb(0x3F8, 0x03);
    outb(0x3F9, 0x00);
    
    // 8 bits, no parity, one stop bit
    outb(0x3F8 + 3, 0x03);
    
    // Enable FIFO
    outb(0x3F8 + 2, 0xC7);
    
    // Enable interrupts
    outb(0x3F8 + 1, 0x0B);
}

/// Write byte to I/O port
/// 
/// # Safety
/// Direct port I/O can cause hardware faults if port doesn't exist
#[inline(always)]
unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!(
        "outb %al, %dx",
        in("al") value,
        in("dx") port,
        options(nostack, nomem),
    );
}

/// Read byte from I/O port
/// 
/// # Safety
/// Direct port I/O can cause hardware faults if port doesn't exist
#[inline(always)]
unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    core::arch::asm!(
        "inb %dx, %al",
        out("al") value,
        in("dx") port,
        options(nostack, nomem),
    );
    value
}

/// Log message to serial port
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        // Simple serial output (full formatting would require more code)
        $crate::serial_write_str("[BoundaryOS] ");
        $crate::serial_write_str(core::str::from_utf8(&[$($arg)*]).unwrap_or(""));
        $crate::serial_write_str("\r\n");
    }};
}

/// Write string to serial port
fn serial_write_str(s: &str) {
    for byte in s.bytes() {
        unsafe {
            // Wait until transmit buffer is empty
            while (inb(0x3F8 + 5) & 0x20) == 0 {}
            outb(0x3F8, byte);
        }
    }
}

/// GDT initialization stub
unsafe fn gdt_init() {
    // Implemented in Phase 3
}

/// IDT initialization stub
unsafe fn idt_init() {
    // Implemented in Phase 3
}

/// Physical memory manager initialization stub
unsafe fn physical_mm_init(_mb_info: &MultibootInfo) {
    // Implemented in Phase 4
}

/// Paging initialization stub
unsafe fn paging_init() {
    // Implemented in Phase 5
}

/// Heap initialization stub
unsafe fn heap_init() {
    // Implemented in Phase 6
}

/// Print the boot banner
fn print_boot_banner() {
    serial_write_str("\r\n");
    serial_write_str("╔══════════════════════════════════════════════╗\r\n");
    serial_write_str("║          BoundaryOS is awake.                ║\r\n");
    serial_write_str("║                                              ║\r\n");
    serial_write_str("║  objects  : [0]                              ║\r\n");
    serial_write_str("║  invariants: [0]                             ║\r\n");
    serial_write_str("║  capabilities: [1]                           ║\r\n");
    serial_write_str("║  fossils  : [0]                              ║\r\n");
    serial_write_str("║                                              ║\r\n");
    serial_write_str("║  Touch a thing by naming it.                 ║\r\n");
    serial_write_str("║  Suggestions: world  keyboard  screen  time  ║\r\n");
    serial_write_str("╚══════════════════════════════════════════════╝\r\n");
    serial_write_str("\r\n");
}

/// World event loop - never returns
/// 
/// This is the main loop of BoundaryOS. It processes events,
/// updates the World state, and handles user interactions.
fn world_event_loop() -> ! {
    loop {
        // Halt until next interrupt
        unsafe {
            core::arch::asm!("hlt");
        }
        
        // Event processing will be implemented in later phases
    }
}
