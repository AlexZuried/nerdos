//! Minimal syscall surface
//! 
//! Provides a stable ABI for userspace programs (future work)

/// Syscall numbers for BoundaryOS
#[repr(u64)]
pub enum SyscallNumber {
    Read = 0,
    Write = 1,
    Open = 2,
    Close = 3,
    Yield = 10,
    Exit = 20,
    Fork = 30,
    Exec = 31,
    Wait = 32,
    GetTime = 100,
    CreateCapability = 200,
    RevokeCapability = 201,
}

/// Initialize syscall handling
pub fn init() {
    log!("Syscall interface initialized (stub)");
    // TODO: Set up syscall entry point (SYSCALL/SYSRET or INT 0x80)
}

/// Handle a syscall
/// 
/// # Safety
/// This function is called from assembly syscall handler with user-controlled registers.
pub unsafe fn handle(syscall_num: u64, arg1: u64, arg2: u64, arg3: u64) -> i64 {
    match syscall_num {
        0 => sys_read(arg1, arg2, arg3),
        1 => sys_write(arg1, arg2, arg3),
        100 => sys_get_time(),
        _ => -1, // Invalid syscall
    }
}

/// Read from file descriptor (stub)
fn sys_read(_fd: u64, _buf: u64, _count: u64) -> i64 {
    // TODO: Implement proper read syscall
    -1
}

/// Write to file descriptor (stub)
fn sys_write(_fd: u64, _buf: u64, _count: u64) -> i64 {
    // TODO: Implement proper write syscall
    -1
}

/// Get current time in nanoseconds
fn sys_get_time() -> i64 {
    crate::arch::tsc::time_ns() as i64
}
