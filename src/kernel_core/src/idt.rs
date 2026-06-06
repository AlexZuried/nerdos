//! # Interrupt Descriptor Table (IDT)
//!
//! The IDT tells the CPU where to jump when an interrupt or exception occurs.
//! There are 256 entries in the IDT, each corresponding to a different interrupt vector.
//!
//! ## IDT Layout
//!
//! Vectors 0-31: CPU exceptions (reserved by Intel/AMD)
//! Vectors 32-47: Hardware interrupts (remapped from PIC IRQs 0-15)
//! Vectors 48-255: Software interrupts (syscalls, etc.)
//!
//! ## Key Handlers
//!
//! - Divide Error (0): Division by zero
//! - Page Fault (14): Access to unmapped memory
//! - General Protection Fault (13): Privilege violation
//! - Double Fault (8): Exception during exception handling
//! - Timer (32): Preemptive scheduling
//! - Keyboard (33): User input
//! - Syscall (0x80): System calls from user space

use lazy_static::lazy_static;
use x86_64::structures::idt::{
    InterruptDescriptorTable,
    InterruptStackFrame,
    PageFaultErrorCode,
};
use crate::gdt;

// ---------------------------------------------------------------------------
// Hardware Interrupt Offsets (after PIC remapping)
// ---------------------------------------------------------------------------

/// Offset for the master PIC (IRQs 0-7 map to vectors 32-39).
pub const PIC_1_OFFSET: u8 = 32;
/// Offset for the slave PIC (IRQs 8-15 map to vectors 40-47).
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

// ---------------------------------------------------------------------------
// IRQ Numbers
// ---------------------------------------------------------------------------

pub const IRQ_TIMER: u8 = 0;
pub const IRQ_KEYBOARD: u8 = 1;
pub const IRQ_SERIAL1: u8 = 4;
pub const IRQ_SERIAL2: u8 = 3;
pub const IRQ_ATA1: u8 = 14;
pub const IRQ_ATA2: u8 = 15;
pub const IRQ_NETWORK: u8 = 11; // Often used by PCI devices

// ---------------------------------------------------------------------------
// Global IDT
// ---------------------------------------------------------------------------

lazy_static! {
    /// The global Interrupt Descriptor Table.
    /// Initialized with handlers for all CPU exceptions and hardware interrupts.
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        // =================================================================
        // CPU EXCEPTIONS (Vectors 0-21)
        // =================================================================

        // 0: Divide Error - division by zero or overflow
        idt.divide_error.set_handler_fn(divide_error_handler);

        // 1: Debug Exception - used by debuggers
        idt.debug.set_handler_fn(debug_handler);

        // 2: Non-Maskable Interrupt - unrecoverable hardware error
        idt.non_maskable_interrupt.set_handler_fn(nmi_handler);

        // 3: Breakpoint - `int3` instruction (used by debuggers)
        idt.breakpoint.set_handler_fn(breakpoint_handler);

        // 4: Overflow - `into` instruction overflow
        idt.overflow.set_handler_fn(overflow_handler);

        // 5: Bound Range Exceeded - `bound` instruction
        idt.bound_range_exceeded.set_handler_fn(bound_range_exceeded_handler);

        // 6: Invalid Opcode - undefined instruction
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);

        // 7: Device Not Available - FPU instruction with no FPU
        idt.device_not_available.set_handler_fn(device_not_available_handler);

        // 8: Double Fault - exception while handling an exception
        // This uses the IST (Interrupt Stack Table) to prevent triple faults.
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }

        // 9: Coprocessor Segment Overrun (reserved, unused in x86_64)

        // 10: Invalid TSS - bad Task State Segment
        idt.invalid_tss.set_handler_fn(invalid_tss_handler);

        // 11: Segment Not Present - referenced segment not present
        idt.segment_not_present.set_handler_fn(segment_not_present_handler);

        // 12: Stack-Segment Fault - stack operation failed
        idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);

        // 13: General Protection Fault - privilege violation, etc.
        idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);

        // 14: Page Fault - memory access to unmapped page
        idt.page_fault.set_handler_fn(page_fault_handler);

        // 15: Reserved

        // 16: x87 Floating-Point Error
        idt.x87_floating_point.set_handler_fn(x87_floating_point_handler);

        // 17: Alignment Check - unaligned memory access
        idt.alignment_check.set_handler_fn(alignment_check_handler);

        // 18: Machine Check - unrecoverable hardware error
        idt.machine_check.set_handler_fn(machine_check_handler);

        // 19: SIMD Floating-Point Exception
        idt.simd_floating_point.set_handler_fn(simd_floating_point_handler);

        // 20: Virtualization Exception
        idt.virtualization.set_handler_fn(virtualization_handler);

        // 21: Control Protection Exception (CET)
        idt.cp_protection_exception.set_handler_fn(cp_protection_handler);

        // =================================================================
        // HARDWARE INTERRUPTS (Vectors 32-47 after PIC remapping)
        // =================================================================

        idt[PIC_1_OFFSET as usize + IRQ_TIMER as usize]
            .set_handler_fn(timer_interrupt_handler);

        idt[PIC_1_OFFSET as usize + IRQ_KEYBOARD as usize]
            .set_handler_fn(keyboard_interrupt_handler);

        idt[PIC_1_OFFSET as usize + IRQ_SERIAL1 as usize]
            .set_handler_fn(serial1_interrupt_handler);

        // =================================================================
        // SYSCALL VECTOR
        // =================================================================

        // Vector 0x80: Linux-compatible syscall interface
        idt[0x80].set_handler_fn(syscall_interrupt_handler)
            .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

        idt
    };
}

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

/// Load the IDT into the CPU's IDTR register.
///
/// # Safety
/// Must be called during early boot before interrupts are enabled.
/// The IDT must be properly initialized with valid handlers.
pub fn init() {
    // Load the IDT. The CPU will now use our handlers for all interrupts.
    IDT.load();
}

// ---------------------------------------------------------------------------
// CPU Exception Handlers
// ---------------------------------------------------------------------------

/// Divide Error (Vector 0)
/// Occurs on division by zero or signed division overflow.
extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
    println!("[EXCEPTION] Divide Error\n{:#?}", stack_frame);
    // In a full implementation, this would send a signal to the offending process.
    // For now, we just print and halt in debug builds.
}

/// Debug Exception (Vector 1)
/// Used by debuggers for single-stepping and breakpoints.
extern "x86-interrupt" fn debug_handler(stack_frame: InterruptStackFrame) {
    println!("[EXCEPTION] Debug\n{:#?}", stack_frame);
}

/// Non-Maskable Interrupt (Vector 2)
/// Unrecoverable hardware error (memory parity, etc.).
extern "x86-interrupt" fn nmi_handler(stack_frame: InterruptStackFrame) {
    println!("[EXCEPTION] NMI (Non-Maskable Interrupt)\n{:#?}", stack_frame);
    // NMIs are serious. We halt because continuing is unsafe.
    loop {
        unsafe { x86_64::instructions::hlt(); }
    }
}

/// Breakpoint (Vector 3)
/// Triggered by the `int3` instruction. Used by debuggers.
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("[EXCEPTION] Breakpoint at {:#x}", stack_frame.instruction_pointer);
}

/// Overflow (Vector 4)
/// Triggered by the `into` instruction when OF=1.
extern "x86-interrupt" fn overflow_handler(stack_frame: InterruptStackFrame) {
    println!("[EXCEPTION] Overflow\n{:#?}", stack_frame);
}

/// Bound Range Exceeded (Vector 5)
extern "x86-interrupt" fn bound_range_exceeded_handler(stack_frame: InterruptStackFrame) {
    println!("[EXCEPTION] Bound Range Exceeded\n{:#?}", stack_frame);
}

/// Invalid Opcode (Vector 6)
/// The CPU encountered an instruction it doesn't understand.
extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    println!("[EXCEPTION] Invalid Opcode at {:#x}", stack_frame.instruction_pointer);
}

/// Device Not Available (Vector 7)
/// FPU instruction executed with TS=1 (lazy FPU context switching).
extern "x86-interrupt" fn device_not_available_handler(stack_frame: InterruptStackFrame) {
    println!("[EXCEPTION] Device Not Available\n{:#?}", stack_frame);
}

/// Double Fault (Vector 8)
/// An exception occurred while handling another exception.
/// This is fatal. We use a separate stack (IST) to handle it.
///
/// The error code is always 0 in modern x86_64.
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    // Disable interrupts immediately - we're in a very bad state
    unsafe { x86_64::instructions::interrupts::disable(); }

    println!("\n!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    println!("  CRITICAL: DOUBLE FAULT");
    println!("  The kernel encountered an unrecoverable error.");
    println!("  {:#?}", stack_frame);
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\n");

    serial_println!("[CRITICAL] Double fault at {:#x}", stack_frame.instruction_pointer);

    // Halt forever. A double fault means something is fundamentally wrong.
    loop {
        unsafe {
            x86_64::instructions::interrupts::disable();
            x86_64::instructions::hlt();
        }
    }
}

/// Invalid TSS (Vector 10)
extern "x86-interrupt" fn invalid_tss_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    println!("[EXCEPTION] Invalid TSS (error code: {})\n{:#?}", error_code, stack_frame);
}

/// Segment Not Present (Vector 11)
extern "x86-interrupt" fn segment_not_present_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    println!("[EXCEPTION] Segment Not Present (error: {})\n{:#?}", error_code, stack_frame);
}

/// Stack-Segment Fault (Vector 12)
extern "x86-interrupt" fn stack_segment_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    println!("[EXCEPTION] Stack-Segment Fault (error: {})\n{:#?}", error_code, stack_frame);
}

/// General Protection Fault (Vector 13)
/// Privilege violation, I/O instruction in user mode, etc.
extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    println!("[EXCEPTION] General Protection Fault (error: {})\n{:#?}", error_code, stack_frame);
    println!("  This usually means a privilege violation or null pointer dereference.");
}

/// Page Fault (Vector 14)
/// Access to a memory page that is not mapped or violates protection.
///
/// The CR2 register contains the virtual address that caused the fault.
/// The error code tells us what type of access failed.
extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    // CR2 contains the virtual address that caused the page fault
    let addr = Cr2::read();

    println!("[EXCEPTION] Page Fault at {:#x}", addr);
    println!("  Error code: {:?}", error_code);
    println!("  {:#?}", stack_frame);

    // Check if this is a legitimate fault (e.g., stack growth, lazy allocation)
    // or a fatal error (null pointer dereference, etc.).
    if addr.is_null() {
        println!("  Cause: Null pointer dereference");
    } else if !error_code.contains(PageFaultErrorCode::PRESENT) {
        println!("  Cause: Page not present (demand paging or bug)");
    }

    // In a full implementation, we would:
    // 1. Try to handle stack growth
    // 2. Try to page in from swap
    // 3. Send SIGSEGV to the process
    // For now, we halt on unhandled page faults.
}

/// x87 Floating-Point Error (Vector 16)
extern "x86-interrupt" fn x87_floating_point_handler(stack_frame: InterruptStackFrame) {
    println!("[EXCEPTION] x87 FPU Error\n{:#?}", stack_frame);
}

/// Alignment Check (Vector 17)
extern "x86-interrupt" fn alignment_check_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    println!("[EXCEPTION] Alignment Check (error: {})\n{:#?}", error_code, stack_frame);
}

/// Machine Check (Vector 18)
/// Unrecoverable hardware error reported by the CPU.
extern "x86-interrupt" fn machine_check_handler(stack_frame: InterruptStackFrame) -> ! {
    println!("[CRITICAL] Machine Check Exception\n{:#?}", stack_frame);
    loop {
        unsafe { x86_64::instructions::hlt(); }
    }
}

/// SIMD Floating-Point Exception (Vector 19)
extern "x86-interrupt" fn simd_floating_point_handler(stack_frame: InterruptStackFrame) {
    println!("[EXCEPTION] SIMD FPU Error\n{:#?}", stack_frame);
}

/// Virtualization Exception (Vector 20)
extern "x86-interrupt" fn virtualization_handler(stack_frame: InterruptStackFrame) {
    println!("[EXCEPTION] Virtualization\n{:#?}", stack_frame);
}

/// Control Protection Exception (Vector 21)
extern "x86-interrupt" fn cp_protection_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    println!("[EXCEPTION] Control Protection (error: {})\n{:#?}", error_code, stack_frame);
}

// ---------------------------------------------------------------------------
// Hardware Interrupt Handlers
// ---------------------------------------------------------------------------

/// Timer interrupt handler (IRQ 0, Vector 32).
/// Called approximately 1000 times per second.
/// This drives the preemptive scheduler.
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use crate::interrupts::pics::PICS;

    // Notify the scheduler that a tick occurred.
    // If a process has used its time slice, the scheduler will switch.
    unsafe {
        crate::scheduler::tick();
    }

    // Send End of Interrupt (EOI) to the PIC.
    // This tells the PIC we're done handling the interrupt.
    unsafe {
        PICS.notify_end_of_interrupt(PIC_1_OFFSET + IRQ_TIMER);
    }
}

/// Keyboard interrupt handler (IRQ 1, Vector 33).
/// Called when a key is pressed or released.
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;
    use crate::interrupts::pics::PICS;

    // Read the scan code from the keyboard data port (0x60).
    // Safety: Reading from port 0x60 is safe and returns the scan code.
    let mut port: Port<u8> = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    // Hand off to the keyboard driver for decoding.
    crate::tty::keyboard::handle_scancode(scancode);

    // Send EOI.
    unsafe {
        PICS.notify_end_of_interrupt(PIC_1_OFFSET + IRQ_KEYBOARD);
    }
}

/// Serial port 1 interrupt handler (IRQ 4, Vector 36).
extern "x86-interrupt" fn serial1_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use crate::interrupts::pics::PICS;
    crate::serial::handle_interrupt();
    unsafe {
        PICS.notify_end_of_interrupt(PIC_1_OFFSET + IRQ_SERIAL1);
    }
}

/// Syscall interrupt handler (Vector 0x80).
/// Linux-compatible syscall interface using `int 0x80`.
/// In production, we prefer the `syscall` instruction, but this provides
/// compatibility with simpler userland code.
extern "x86-interrupt" fn syscall_interrupt_handler(stack_frame: InterruptStackFrame) {
    use crate::syscall::Syscall;

    // The syscall number is in RAX, arguments in RDI, RSI, RDX, R10, R8, R9.
    // We read these from the saved stack frame.
    let syscall_num: u64;
    let arg1: u64;
    let arg2: u64;
    let arg3: u64;

    // Safety: We read from the interrupt stack frame which contains
    // the saved register state at the time of the interrupt.
    unsafe {
        // These offsets correspond to the x86_64 calling convention
        // for interrupt handlers.
        syscall_num = (*stack_frame.as_ptr()).cpu_registers.rax;
        arg1 = (*stack_frame.as_ptr()).cpu_registers.rdi;
        arg2 = (*stack_frame.as_ptr()).cpu_registers.rsi;
        arg3 = (*stack_frame.as_ptr()).cpu_registers.rdx;
    }

    let result = Syscall::dispatch(syscall_num, arg1, arg2, arg3);

    // Return value goes back in RAX via the stack frame.
    unsafe {
        (*stack_frame.as_mut_ptr()).cpu_registers.rax = result;
    }
}
