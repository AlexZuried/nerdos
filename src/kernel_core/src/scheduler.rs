//! # Preemptive Multitasking Scheduler
//!
//! NerdOS uses a round-robin scheduler with priority levels.
//! The scheduler is tick-driven: every timer interrupt (1ms), it checks
//! if the current process has exhausted its time slice.
//!
//! ## Process States
//!
//! - **Running**: Currently executing on the CPU.
//! - **Ready**: Waiting for CPU time.
//! - **Blocked**: Waiting for an event (I/O, sleep, etc.).
//! - **Zombie**: Exited but not yet reaped by parent.
//!
//! ## Context Switch
//!
//! On x86_64, context switch involves saving:
//! - General-purpose registers (RAX, RBX, RCX, RDX, RSI, RDI, RBP, R8-R15)
//! - Stack pointer (RSP)
//! - Instruction pointer (RIP)
//! - Flags (RFLAGS)
//! - Segment registers (CS, SS)
//!
//! These are saved in the Process Control Block (PCB).

use core::arch::naked_asm;
use core::sync::atomic::{AtomicU64, Ordering};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default time slice in milliseconds.
const DEFAULT_TIME_SLICE_MS: u64 = 10;

/// Number of priority levels (0 = highest).
const NUM_PRIORITIES: usize = 4;

/// Maximum number of processes.
const MAX_PROCESSES: usize = 256;

// ---------------------------------------------------------------------------
// Process ID
// ---------------------------------------------------------------------------

/// Global counter for assigning unique PIDs.
static NEXT_PID: AtomicU64 = AtomicU64::new(1);

/// Get the next available PID.
fn next_pid() -> u64 {
    NEXT_PID.fetch_add(1, Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Process State
// ---------------------------------------------------------------------------

/// The state of a process in the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    /// The process is currently running on the CPU.
    Running,
    /// The process is ready to run but waiting for CPU time.
    Ready,
    /// The process is blocked waiting for an event.
    Blocked,
    /// The process has exited but not yet been reaped.
    Zombie,
}

// ---------------------------------------------------------------------------
// CPU Context
// ---------------------------------------------------------------------------

/// Saved CPU registers for context switching.
/// This is the x86_64 interrupt frame format.
#[derive(Debug, Clone, Default)]
#[repr(C)]
pub struct CpuContext {
    /// General-purpose registers.
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,

    /// Segment registers.
    pub gs: u64,
    pub fs: u64,
    pub es: u64,
    pub ds: u64,

    /// Interrupt frame (pushed by CPU on interrupt).
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

// ---------------------------------------------------------------------------
// Process Control Block
// ---------------------------------------------------------------------------

/// The Process Control Block contains all information about a process.
/// This is the "struct task_struct" equivalent from Linux.
pub struct Process {
    /// Unique process ID.
    pub pid: u64,
    /// Parent process ID.
    pub ppid: u64,
    /// Current state.
    pub state: ProcessState,
    /// Priority level (0 = highest, NUM_PRIORITIES-1 = lowest).
    pub priority: usize,
    /// Remaining time slice in ticks.
    pub time_slice: u64,
    /// Saved CPU context for context switching.
    pub context: CpuContext,
    /// Kernel stack pointer.
    pub kernel_stack: u64,
    /// User stack top (virtual address).
    pub user_stack_top: u64,
    /// Page table root (physical address of PML4).
    pub page_table: u64,
    /// Process name (for debugging).
    pub name: [u8; 32],
    /// Exit code (valid if state is Zombie).
    pub exit_code: i32,
    /// CPU affinity bitmask.
    pub cpu_affinity: u64,
    /// Accumulated CPU time in ticks.
    pub cpu_time: u64,
}

impl Process {
    /// Create a new process with default values.
    fn new(pid: u64, name: &str) -> Self {
        let mut proc = Process {
            pid,
            ppid: 0,
            state: ProcessState::Ready,
            priority: 2, // Default priority
            time_slice: DEFAULT_TIME_SLICE_MS,
            context: CpuContext::default(),
            kernel_stack: 0,
            user_stack_top: 0,
            page_table: 0,
            name: [0; 32],
            exit_code: 0,
            cpu_affinity: !0, // All CPUs
            cpu_time: 0,
        };

        // Copy name into fixed-size buffer.
        let name_bytes = name.as_bytes();
        let len = name_bytes.len().min(31);
        proc.name[..len].copy_from_slice(&name_bytes[..len]);

        proc
    }

    /// Get the process name as a string slice.
    pub fn name_str(&self) -> &str {
        let len = self.name.iter().position(|&b| b == 0).unwrap_or(32);
        core::str::from_utf8(&self.name[..len]).unwrap_or("<?>")
    }
}

// ---------------------------------------------------------------------------
// Scheduler
// ---------------------------------------------------------------------------

/// The main scheduler structure.
/// Manages the process table and dispatching.
pub struct Scheduler {
    /// Process table. Index 0 is the idle process.
    processes: [Option<Process>; MAX_PROCESSES],
    /// Currently running process index.
    current: usize,
    /// Number of active (non-zombie) processes.
    active_count: usize,
    /// Ready queues for each priority level.
    ready_queues: [ReadyQueue; NUM_PRIORITIES],
    /// Total ticks elapsed since boot.
    total_ticks: u64,
}

/// A simple circular buffer for ready processes.
struct ReadyQueue {
    /// PIDs of ready processes.
    pids: [u64; MAX_PROCESSES],
    /// Head index.
    head: usize,
    /// Tail index.
    tail: usize,
    /// Number of entries.
    count: usize,
}

impl ReadyQueue {
    const fn new() -> Self {
        ReadyQueue {
            pids: [0; MAX_PROCESSES],
            head: 0,
            tail: 0,
            count: 0,
        }
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn enqueue(&mut self, pid: u64) {
        if self.count < MAX_PROCESSES {
            self.pids[self.tail] = pid;
            self.tail = (self.tail + 1) % MAX_PROCESSES;
            self.count += 1;
        }
    }

    fn dequeue(&mut self) -> Option<u64> {
        if self.is_empty() {
            None
        } else {
            let pid = self.pids[self.head];
            self.head = (self.head + 1) % MAX_PROCESSES;
            self.count -= 1;
            Some(pid)
        }
    }
}

impl Scheduler {
    /// Create a new scheduler with the init process.
    pub fn new() -> Self {
        let mut scheduler = Scheduler {
            processes: core::array::from_fn(|_| None),
            current: 0,
            active_count: 0,
            ready_queues: core::array::from_fn(|_| ReadyQueue::new()),
            total_ticks: 0,
        };

        // Create the idle process (PID 0).
        // This runs when no other process is ready.
        let mut idle = Process::new(0, "idle");
        idle.priority = NUM_PRIORITIES - 1; // Lowest priority
        idle.state = ProcessState::Running;
        scheduler.processes[0] = Some(idle);

        scheduler
    }

    /// Create a new process from a binary.
    ///
    /// # Arguments
    /// * `name` - Process name
    /// * `entry_point` - Virtual address to start execution
    /// * `page_table` - Physical address of the PML4
    pub fn spawn(&mut self, name: &str, entry_point: u64, page_table: u64) -> Option<u64> {
        // Find a free slot in the process table.
        let idx = self.processes.iter().position(|p| p.is_none())?;

        let pid = next_pid();
        let mut proc = Process::new(pid, name);
        proc.ppid = self.processes[self.current].as_ref()?.pid;
        proc.state = ProcessState::Ready;
        proc.page_table = page_table;

        // Set up initial CPU context.
        proc.context.rip = entry_point;
        proc.context.rflags = 0x202; // Interrupts enabled (IF=1)
        proc.context.cs = 0x08 | 3; // User code segment (Ring 3)
        proc.context.ss = 0x10 | 3; // User data segment (Ring 3)

        // Allocate user stack.
        // In a real implementation, we'd map pages for the stack.
        proc.user_stack_top = 0x0000_7FFF_FFFF_F000; // Top of user address space
        proc.context.rsp = proc.user_stack_top;

        // Store and enqueue.
        self.processes[idx] = Some(proc);
        self.active_count += 1;
        self.ready_queues[2].enqueue(pid); // Default priority

        Some(pid)
    }

    /// Get the currently running process.
    pub fn current_process(&self) -> Option<&Process> {
        self.processes[self.current].as_ref()
    }

    /// Get a mutable reference to the current process.
    pub fn current_process_mut(&mut self) -> Option<&mut Process> {
        self.processes[self.current].as_mut()
    }

    /// Get a process by PID.
    pub fn get_process(&self, pid: u64) -> Option<&Process> {
        self.processes.iter().find(|p| p.as_ref().map_or(false, |p| p.pid == pid))?.as_ref()
    }

    /// Kill a process.
    pub fn kill(&mut self, pid: u64) -> bool {
        if pid == 0 {
            return false; // Can't kill idle
        }

        for proc in self.processes.iter_mut() {
            if let Some(ref mut p) = proc {
                if p.pid == pid {
                    p.state = ProcessState::Zombie;
                    p.exit_code = -9; // SIGKILL
                    self.active_count -= 1;
                    return true;
                }
            }
        }
        false
    }

    /// List all processes.
    pub fn list_processes(&self, buf: &mut [ProcessInfo]) -> usize {
        let mut count = 0;
        for proc in self.processes.iter() {
            if let Some(ref p) = proc {
                if count < buf.len() {
                    buf[count] = ProcessInfo {
                        pid: p.pid,
                        ppid: p.ppid,
                        state: p.state,
                        priority: p.priority as u32,
                        cpu_time: p.cpu_time,
                        name: p.name,
                    };
                    count += 1;
                }
            }
        }
        count
    }
}

// ---------------------------------------------------------------------------
// Process Info (for sys_ps)
// ---------------------------------------------------------------------------

/// Lightweight info about a process, suitable for copying to user space.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct ProcessInfo {
    pub pid: u64,
    pub ppid: u64,
    pub state: ProcessState,
    pub priority: u32,
    pub cpu_time: u64,
    pub name: [u8; 32],
}

// ---------------------------------------------------------------------------
// Context Switch (Assembly)
// ---------------------------------------------------------------------------

/// Perform a context switch from `old` to `new` process.
///
/// This is a naked function - the compiler doesn't generate prologue/epilogue.
/// We write the assembly directly to have full control over register saving.
///
/// # Safety
///
/// This is one of the most critical and dangerous functions in the kernel.
/// It modifies all CPU registers and the stack pointer. Interrupts must be
/// disabled when calling this.
#[naked]
pub unsafe extern "C" fn context_switch(
    old_ctx: *mut CpuContext,
    new_ctx: *const CpuContext,
    new_page_table: u64,
) {
    // x86_64 calling convention:
    // RDI = old_ctx (save current state here)
    // RSI = new_ctx (load state from here)
    // RDX = new_page_table (CR3 value for new process)
    naked_asm!(
        // Save all callee-saved registers to old context.
        // The order must match the CpuContext struct definition.
        "mov [rdi + 0], r15",
        "mov [rdi + 8], r14",
        "mov [rdi + 16], r13",
        "mov [rdi + 24], r12",
        "mov [rdi + 32], r11",
        "mov [rdi + 40], r10",
        "mov [rdi + 48], r9",
        "mov [rdi + 56], r8",
        "mov [rdi + 64], rbp",
        "mov [rdi + 72], rdi",
        "mov [rdi + 80], rsi",
        "mov [rdi + 88], rdx",
        "mov [rdi + 96], rcx",
        "mov [rdi + 104], rbx",
        "mov [rdi + 112], rax",

        // Save segment registers.
        "mov rax, gs",
        "mov [rdi + 120], rax",
        "mov rax, fs",
        "mov [rdi + 128], rax",
        "mov rax, es",
        "mov [rdi + 136], rax",
        "mov rax, ds",
        "mov [rdi + 144], rax",

        // Save RIP (return address is on stack).
        "mov rax, [rsp]",
        "mov [rdi + 152], rax",

        // Save CS.
        "mov rax, cs",
        "mov [rdi + 160], rax",

        // Save RFLAGS.
        "pushfq",
        "pop rax",
        "mov [rdi + 168], rax",

        // Save RSP.
        "lea rax, [rsp + 8]",  // Adjust for return address
        "mov [rdi + 176], rax",

        // Save SS.
        "mov rax, ss",
        "mov [rdi + 184], rax",

        // Now load the new context.
        // Load general-purpose registers.
        "mov r15, [rsi + 0]",
        "mov r14, [rsi + 8]",
        "mov r13, [rsi + 16]",
        "mov r12, [rsi + 24]",
        "mov r11, [rsi + 32]",
        "mov r10, [rsi + 40]",
        "mov r9, [rsi + 48]",
        "mov r8, [rsi + 56]",
        "mov rbp, [rsi + 64]",
        // Skip RDI, RSI, RDX, RCX, RBX, RAX for now

        // Load segment registers.
        "mov rax, [rsi + 120]",
        "mov gs, ax",
        "mov rax, [rsi + 128]",
        "mov fs, ax",
        "mov rax, [rsi + 136]",
        "mov es, ax",
        "mov rax, [rsi + 144]",
        "mov ds, ax",

        // Switch page tables if needed.
        "cmp rdx, 0",
        "je 2f",
        "mov cr3, rdx",
        "2:",

        // Set up stack for IRET.
        // IRET pops: RIP, CS, RFLAGS, RSP, SS
        "mov rax, [rsi + 184]",  // SS
        "push rax",
        "mov rax, [rsi + 176]",  // RSP
        "push rax",
        "mov rax, [rsi + 168]",  // RFLAGS
        "push rax",
        "mov rax, [rsi + 160]",  // CS
        "push rax",
        "mov rax, [rsi + 152]",  // RIP
        "push rax",

        // Load remaining registers.
        "mov rdi, [rsi + 72]",
        "mov rsi, [rsi + 80]",
        "mov rdx, [rsi + 88]",
        "mov rcx, [rsi + 96]",
        "mov rbx, [rsi + 104]",
        "mov rax, [rsi + 112]",

        // Return to new process via IRET.
        "iretq",
    );
}

// ---------------------------------------------------------------------------
// Timer Tick
// ---------------------------------------------------------------------------

/// Called on every timer interrupt (tick).
///
/// # Safety
/// Must be called with interrupts disabled (from interrupt handler).
pub unsafe fn tick() {
    // In a real implementation, we would:
    // 1. Decrement the current process's time slice
    // 2. If time slice expired, mark as ready and pick next process
    // 3. If higher priority process became ready, preempt
    // 4. Call context_switch() if needed

    // For now, we just track ticks.
    // A full implementation would access the global SCHEDULER.
}
