//! Time Stamp Counter (TSC) management
//! 
//! Provides high-precision timing

use spin::Mutex;

/// TSC frequency in Hz (calibrated at boot)
static TSC_FREQUENCY: Mutex<u64> = Mutex::new(0);

/// Calibrate the TSC against PIT
pub fn calibrate() {
    log!("TSC calibrated (stub)");
    // TODO: Implement proper TSC calibration
    *TSC_FREQUENCY.lock() = 1_000_000_000; // Assume 1 GHz for now
}

/// Get current TSC value
#[inline]
pub fn read() -> u64 {
    unsafe {
        core::arch::x86_64::_rdtsc() as u64
    }
}

/// Get time in nanoseconds since boot
pub fn time_ns() -> u64 {
    let tsc = read();
    let freq = *TSC_FREQUENCY.lock();
    if freq == 0 {
        return 0;
    }
    tsc * 1_000_000_000 / freq
}
