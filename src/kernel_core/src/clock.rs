//! # Programmable Interval Timer (PIT) Driver
//!
//! The PIT is a legacy timer chip (Intel 8253/8254) that provides
//! periodic interrupts for preemptive multitasking.
//!
//! While modern systems prefer the Local APIC timer or HPET,
//! the PIT is simple, universally available, and sufficient for
//! a single-core kernel.
//!
//! ## Configuration
//!
//! The PIT has 3 channels:
//! - Channel 0: IRQ0 (timer interrupt) - the one we use
//! - Channel 1: DRAM refresh (obsolete, don't use)
//! - Channel 2: PC speaker
//!
//! We configure Channel 0 in Mode 3 (square wave generator) to
//! produce periodic interrupts.

use x86_64::instructions::port::Port;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// PIT base frequency in Hz (1.193182 MHz).
const PIT_BASE_FREQUENCY: u64 = 1_193_182;

/// Desired timer frequency in Hz (1000 = 1ms tick).
/// This gives us 1ms resolution for scheduling.
pub const TICK_FREQUENCY: u64 = 1000;

/// The divisor to program into the PIT.
/// divisor = base_freq / desired_freq
const PIT_DIVISOR: u16 = (PIT_BASE_FREQUENCY / TICK_FREQUENCY) as u16;

// I/O Ports
const PIT_CHANNEL0: u16 = 0x40;  // Channel 0 data port
const PIT_COMMAND: u16 = 0x43;   // Command/mode register

// Command byte format:
// Bits 6-7: Select channel (00 = channel 0)
// Bits 4-5: Access mode (11 = lobyte/hibyte)
// Bits 1-3: Operating mode (011 = square wave generator)
// Bit 0: BCD mode (0 = binary)
const PIT_COMMAND_BYTE: u8 = 0b0011_0110; // Channel 0, lobyte/hibyte, mode 3, binary

// ---------------------------------------------------------------------------
// Tick Counter
// ---------------------------------------------------------------------------

/// Global tick counter, incremented by the timer interrupt handler.
/// Use `get_ticks()` for safe access.
static mut TICKS: u64 = 0;

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

/// Initialize the PIT for periodic interrupts.
///
/// This configures Channel 0 to generate interrupts at TICK_FREQUENCY Hz.
///
/// # Safety
/// Must be called during early boot, before interrupts are enabled.
pub fn init() {
    unsafe {
        // Safety: These are well-defined I/O ports for the PIT.
        let mut command: Port<u8> = Port::new(PIT_COMMAND);
        let mut channel0: Port<u8> = Port::new(PIT_CHANNEL0);

        // Send the command byte to configure the PIT.
        command.write(PIT_COMMAND_BYTE);

        // Send the divisor (low byte first, then high byte).
        channel0.write((PIT_DIVISOR & 0xFF) as u8);
        channel0.write(((PIT_DIVISOR >> 8) & 0xFF) as u8);
    }
}

// ---------------------------------------------------------------------------
// Tick Management
// ---------------------------------------------------------------------------

/// Increment the tick counter.
/// Called from the timer interrupt handler.
///
/// # Safety
/// Must only be called from the timer interrupt handler.
/// Uses raw mutable access to static.
pub unsafe fn increment_ticks() {
    TICKS += 1;
}

/// Get the current number of ticks since boot.
///
/// Each tick represents 1 millisecond.
pub fn get_ticks() -> u64 {
    // Safety: Reading a u64 on x86_64 is atomic for aligned accesses.
    unsafe { TICKS }
}

/// Get the time since boot in milliseconds.
pub fn uptime_ms() -> u64 {
    get_ticks()
}

/// Get the time since boot in seconds.
pub fn uptime_secs() -> u64 {
    get_ticks() / 1000
}

/// Sleep for a number of milliseconds (busy-wait).
///
/// This is a simple busy-wait sleep. It consumes CPU cycles.
/// For efficient sleeping, use the scheduler's sleep functionality
/// which yields the CPU instead.
pub fn busy_wait_ms(ms: u64) {
    let target = get_ticks() + ms;
    while get_ticks() < target {
        core::hint::spin_loop();
    }
}

// ---------------------------------------------------------------------------
// Timestamp Conversion
// ---------------------------------------------------------------------------

/// Convert a tick count to seconds.
pub const fn ticks_to_secs(ticks: u64) -> u64 {
    ticks / TICK_FREQUENCY
}

/// Convert a tick count to milliseconds.
pub const fn ticks_to_ms(ticks: u64) -> u64 {
    ticks
}

/// Convert seconds to tick count.
pub const fn secs_to_ticks(secs: u64) -> u64 {
    secs * TICK_FREQUENCY
}

/// Convert milliseconds to tick count.
pub const fn ms_to_ticks(ms: u64) -> u64 {
    ms
}
