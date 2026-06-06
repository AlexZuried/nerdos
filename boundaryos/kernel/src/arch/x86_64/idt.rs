//! Interrupt Descriptor Table (IDT) setup
//! 
//! Sets up exception and interrupt handlers.

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptHandler};
use spin::Mutex;

/// Kernel IDT
static IDT: Mutex<InterruptDescriptorTable> = Mutex::new(InterruptDescriptorTable::new());

/// Initialize the IDT
/// 
/// # Safety
/// This function modifies CPU interrupt handling and must only be called once during boot.
pub unsafe fn init() {
    let mut idt = IDT.lock();
    
    // Set up exception handlers
    idt.breakpoint.set_handler_fn(breakpoint_handler);
    idt.double_fault.set_handler_fn(double_fault_handler);
    idt.invalid_tss.set_handler_fn(invalid_tss_handler);
    idt.segment_not_present.set_handler_fn(segment_not_present_handler);
    idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);
    idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);
    idt.page_fault.set_handler_fn(page_fault_handler);
    
    // Load IDT
    idt.load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut x86_64::structures::idt::InterruptStackFrame) {
    log!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: &mut x86_64::structures::idt::InterruptStackFrame, _error_code: u64) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn invalid_tss_handler(stack_frame: &mut x86_64::structures::idt::InterruptStackFrame, _error_code: u64) {
    log!("EXCEPTION: INVALID TSS\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn segment_not_present_handler(stack_frame: &mut x86_64::structures::idt::InterruptStackFrame, _error_code: u64) {
    log!("EXCEPTION: SEGMENT NOT PRESENT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn stack_segment_fault_handler(stack_frame: &mut x86_64::structures::idt::InterruptStackFrame, _error_code: u64) {
    log!("EXCEPTION: STACK SEGMENT FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn general_protection_fault_handler(stack_frame: &mut x86_64::structures::idt::InterruptStackFrame, error_code: u64) {
    log!("EXCEPTION: GENERAL PROTECTION FAULT (error: {})\n{:#?}", error_code, stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(stack_frame: &mut x86_64::structures::idt::InterruptStackFrame, error_code: x86_64::structures::idt::PageFaultErrorCode) {
    use x86_64::registers::control::Cr2;
    log!("EXCEPTION: PAGE FAULT (error: {:?}) at address {:?}", error_code, Cr2::read());
    log!("Stack frame: {:#?}", stack_frame);
}
