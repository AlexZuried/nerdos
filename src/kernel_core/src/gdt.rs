//! # Global Descriptor Table (GDT)
//!
//! The GDT defines the memory segments used by the CPU.
//! In x86_64, we use a flat segmentation model where all segments
//! span the entire address space. Memory protection is handled by paging,
//! not segmentation. However, we still need the GDT for:
//! - Code and data segment descriptors (required by the CPU)
//! - Task State Segment (TSS) for interrupt stack switching
//! - Setting up the syscall/sysret instructions
//!
//! ## Structure
//!
//! Our GDT contains:
//! | Index | Descriptor    | Purpose                        |
//! |-------|--------------|--------------------------------|
//! | 0     | Null         | Required null descriptor       |
//! | 1     | Kernel Code  | Ring 0 code segment (64-bit)   |
//! | 2     | Kernel Data  | Ring 0 data segment            |
//! | 3     | User Code    | Ring 3 code segment (64-bit)   |
//! | 4     | User Data    | Ring 3 data segment            |
//! | 5     | TSS          | Task State Segment (64-bit)    |

use lazy_static::lazy_static;
use x86_64::VirtAddr;
use x86_64::structures::tss::TaskStateSegment;
use x86_64::structures::gdt::{
    GlobalDescriptorTable, Descriptor, SegmentSelector
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// The index of the double fault Interrupt Stack Table (IST) entry.
/// When a double fault occurs, the CPU switches to this stack.
/// This prevents triple faults when the normal stack is corrupted.
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

/// Size of the double fault stack in pages (4 KiB each).
const STACK_SIZE: usize = 4096 * 5;

// ---------------------------------------------------------------------------
// Static Stack for Double Fault Handler
// ---------------------------------------------------------------------------

/// A dedicated stack for the double fault handler.
/// This is allocated as a static array to avoid heap allocation
/// during a critical exception.
static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

// ---------------------------------------------------------------------------
// Global GDT and Selectors (initialized at boot)
// ---------------------------------------------------------------------------

lazy_static! {
    /// The global Task State Segment.
    /// Contains the IST (Interrupt Stack Table) for handling exceptions
    /// that occur on corrupted stacks.
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();

        // Set up the IST entry for double fault handling.
        // Safety: We're writing to a static mut during lazy initialization.
        // This is safe because lazy_static guarantees this runs exactly once.
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            // Stack grows downward, so we use the top of the stack
            stack_start + STACK_SIZE as u64
        };

        tss
    };

    /// The Global Descriptor Table containing all segment descriptors.
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();

        // 0: Null descriptor (required by x86 architecture)
        // The CPU expects the first entry to be null.

        // 1: Kernel code segment (Ring 0)
        // This is a 64-bit code segment descriptor with DPL=0 (kernel).
        let code_selector = gdt.append(Descriptor::kernel_code_segment());

        // 2: Kernel data segment (Ring 0)
        let data_selector = gdt.append(Descriptor::kernel_data_segment());

        // 3: User data segment (Ring 3)
        // DPL=3 allows user-mode code to use this segment.
        let user_data_selector = gdt.append(Descriptor::user_data_segment());

        // 4: User code segment (Ring 3)
        let user_code_selector = gdt.append(Descriptor::user_code_segment());

        // 5: Task State Segment (TSS)
        // The TSS contains the IST and I/O permission bitmap.
        // It's a system segment, so it uses a different descriptor format.
        let tss_selector = gdt.append(Descriptor::tss_segment(&TSS))
            .expect("Failed to add TSS to GDT");

        (gdt, Selectors {
            code_selector,
            data_selector,
            user_code_selector,
            user_data_selector,
            tss_selector,
        })
    };
}

// ---------------------------------------------------------------------------
// Selector Struct
// ---------------------------------------------------------------------------

/// Holds the segment selectors (indices into the GDT) for each segment.
/// These are used to load the segment registers (CS, DS, SS, etc.).
pub struct Selectors {
    /// Kernel code segment selector (used for CS in Ring 0).
    pub code_selector: SegmentSelector,
    /// Kernel data segment selector (used for DS, SS in Ring 0).
    pub data_selector: SegmentSelector,
    /// User code segment selector (used for CS in Ring 3).
    pub user_code_selector: SegmentSelector,
    /// User data segment selector (used for DS, SS in Ring 3).
    pub user_data_selector: SegmentSelector,
    /// TSS segment selector (used with LTR instruction).
    pub tss_selector: SegmentSelector,
}

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

/// Initialize the GDT and load it into the CPU.
///
/// This function:
/// 1. Loads the GDT into the GDTR register
/// 2. Reloads segment registers (CS, DS, SS, ES)
/// 3. Loads the TSS into the TR register
///
/// # Safety
///
/// This function uses `unsafe` because it modifies critical CPU state.
/// It must be called exactly once during early boot, before interrupts are enabled.
pub fn init() {
    use x86_64::instructions::tables::load_tss;
    use x86_64::instructions::segmentation::*;

    let (gdt_ref, selectors) = &*GDT;

    // Load the GDT into the GDTR register.
    // The CPU will now use our descriptors for segment checks.
    gdt_ref.load();

    // Safety: We just loaded a valid GDT. Now we reload segment registers
    // to point to our new descriptors.
    unsafe {
        // Reload CS (code segment) using a far return trick.
        // We can't directly write to CS, so we use a far jump.
        set_cs(selectors.code_selector);

        // Reload data segments.
        load_ss(selectors.data_selector);
        set_ds(selectors.data_selector);
        set_es(selectors.data_selector);
        set_fs(selectors.data_selector);
        set_gs(selectors.data_selector);

        // Load the Task State Segment.
        // This tells the CPU where to find the IST for stack switching.
        load_tss(selectors.tss_selector);
    }
}

/// Returns the kernel code segment selector.
/// Used when setting up interrupt handlers that run in Ring 0.
pub fn kernel_code_selector() -> SegmentSelector {
    GDT.1.code_selector
}

/// Returns the user code and data segment selectors.
/// Used when creating new user processes.
pub fn user_segments() -> (SegmentSelector, SegmentSelector) {
    (GDT.1.user_code_selector, GDT.1.user_data_selector)
}
