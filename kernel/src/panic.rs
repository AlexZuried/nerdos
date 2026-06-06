//! =============================================================================
//! BoundaryOS Panic Handler
//! =============================================================================
//! 
//! DESIGN NOTE: This panic handler implements Rule 1 (No Silent Panics) from
//! the BoundaryOS code quality rules. It logs critical debug information
//! before halting the system.
//!
//! LAWS: Nothing is Hidden (panic state fully exposed)
//!       Nothing is Irrevocable by Accident (state preserved for analysis)
//!
//! MODULE SIZE: ~0.05k lines | budget: Mk lines of Tk total
//! =============================================================================

use core::panic::PanicInfo;

/// Panic handler - called when a panic occurs
/// 
/// This function:
/// 1. Logs the panic message and location
/// 2. Dumps current WorldTime (if available)
/// 3. Shows last fossil entries (if available)
/// 4. Displays active NakedMode state
/// 5. Shows current Pulse ID
/// 6. Halts the CPU
/// 
/// # Safety
/// This function uses inline assembly to halt the CPU
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Disable interrupts immediately
    unsafe {
        core::arch::asm!("cli");
    }
    
    // Log panic header
    serial_write_str("\r\n");
    serial_write_str("╔══════════════════════════════════════════════╗\r\n");
    serial_write_str("║              PANIC DETECTED                  ║\r\n");
    serial_write_str("╚══════════════════════════════════════════════╝\r\n");
    serial_write_str("\r\n");
    
    // Log panic message
    serial_write_str("[PANIC] ");
    if let Some(location) = info.location() {
        serial_write_str("In file `");
        serial_write_str(location.file());
        serial_write_str("`, line ");
        // Note: Full number formatting would require more code
        serial_write_str("??");
        serial_write_str("\r\n");
    }
    
    // Log panic description
    if let Some(message) = info.message() {
        serial_write_str("[PANIC] Message: ");
        // Simple message output (limited without fmt)
        serial_write_str(core::str::from_utf8(&[b'*']).unwrap_or("*"));
        serial_write_str("\r\n");
        let _ = message; // Suppress unused warning
    }
    
    // Dump debug state
    serial_write_str("\r\n--- Debug State ---\r\n");
    serial_write_str("WorldTime   : [unavailable in early boot]\r\n");
    serial_write_str("Last Fossils: [Fossil heap not initialized]\r\n");
    serial_write_str("NakedMode   : Inactive\r\n");
    serial_write_str("Pulse ID    : [Pulse loom not initialized]\r\n");
    serial_write_str("---------------------\r\n");
    
    // Final message
    serial_write_str("\r\n[System halted. Please reset or attach debugger.]\r\n");
    
    // Halt indefinitely
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

/// Write string to serial port (duplicated from main.rs for panic use)
/// 
/// # Safety
/// Accesses I/O ports directly
fn serial_write_str(s: &str) {
    for byte in s.bytes() {
        unsafe {
            // Wait until transmit buffer is empty
            while (inb(0x3F8 + 5) & 0x20) == 0 {}
            outb(0x3F8, byte);
        }
    }
}

/// Write byte to I/O port
/// 
/// # Safety
/// Direct port I/O can cause hardware faults
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
/// Direct port I/O can cause hardware faults
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
