//! # NerdOS Kernel Core
//!
//! The heart of the NerdOS operating system. This crate provides:
//! - Boot entry point with Multiboot2 support
//! - Global Descriptor Table (GDT) setup
//! - Interrupt Descriptor Table (IDT) with handler stubs
//! - Physical and virtual memory management
//! - Preemptive multitasking scheduler
//! - System call interface
//! - Kernel logging and panic handler
//!
//! This crate is `#![no_std]` and `#![no_main]` as it runs in a bare-metal
//! environment without the Rust standard library.

#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(const_mut_refs)]
#![feature(alloc_error_handler)]
#![feature(naked_functions)]

// ---------------------------------------------------------------------------
// External Crates
// ---------------------------------------------------------------------------

extern crate alloc;

use core::panic::PanicInfo;
use multiboot2::BootInformation;

// ---------------------------------------------------------------------------
// Internal Modules
// ---------------------------------------------------------------------------

pub mod gdt;
pub mod idt;
pub mod interrupts;
pub mod memory;
pub mod scheduler;
pub mod syscall;
pub mod tty;
pub mod serial;
pub mod vga;
pub mod clock;

// ---------------------------------------------------------------------------
// Module Re-exports for Convenience
// ---------------------------------------------------------------------------

use memory::PhysicalMemoryManager;
use scheduler::Scheduler;

// ---------------------------------------------------------------------------
// Global Kernel State
// ---------------------------------------------------------------------------

/// The kernel's physical memory manager singleton.
/// Initialized during early boot from the Multiboot memory map.
static mut PMM: Option<PhysicalMemoryManager> = None;

/// The kernel's scheduler singleton.
/// Initialized after memory management is set up.
static mut SCHEDULER: Option<Scheduler> = None;

// ---------------------------------------------------------------------------
// Kernel Version Constants
// ---------------------------------------------------------------------------

pub const KERNEL_NAME: &str = "NerdOS";
pub const KERNEL_VERSION: &str = "0.1.0";
pub const KERNEL_GIT_REV: &str = env!("GIT_HASH", "unknown");

// ---------------------------------------------------------------------------
// Heap Configuration
// ---------------------------------------------------------------------------

/// Start of the kernel heap (after the kernel image).
/// 1MB heap size is sufficient for early boot; the MM can expand later.
pub const HEAP_START: usize = 0o_000_001_000_000_0000;
pub const HEAP_SIZE: usize = 1024 * 1024; // 1 MiB

// ---------------------------------------------------------------------------
// Boot Entry Point
// ---------------------------------------------------------------------------

/// The kernel entry point called by the bootloader (GRUB2 via Multiboot2).
///
/// # Safety
///
/// This function is called exactly once by the assembly bootstrap code.
/// It receives:
/// - `eax`: Multiboot2 magic value (must be 0x36d76289)
/// - `ebx`: Physical address of the Multiboot2 information structure
///
/// This function must not return. It initializes all kernel subsystems and
/// then hands control to the scheduler.
#[no_mangle]
pub extern "C" fn kernel_main(multiboot_magic: u32, multiboot_info_ptr: usize) -> ! {
    // ---------------------------------------------------------------
    // Early Boot: Validate Multiboot2 magic number
    // ---------------------------------------------------------------
    // GRUB2 sets EAX to 0x36d76289 to indicate a valid Multiboot2 boot.
    // If this doesn't match, something went terribly wrong in the bootloader.
    if multiboot_magic != 0x36d76289 {
        // We can't print yet, so we just halt. In a real scenario,
        // we might write to the serial port directly if we knew it was safe.
        unsafe { core::arch::asm!("cli; hlt", options(noreturn)) };
    }

    // ---------------------------------------------------------------
    // Parse the Multiboot2 information structure
    // ---------------------------------------------------------------
    // The multiboot_info_ptr is a physical address. We need to convert
    // it to a usable pointer. At this early stage, we rely on identity
    // mapping (physical = virtual) set up by the bootloader.
    let boot_info = unsafe {
        // Safety: The bootloader provides a valid Multiboot2 structure
        // at the given physical address. We trust GRUB2 here.
        multiboot2::BootInformation::load(multiboot_info_ptr as *const u8)
            .expect("Invalid Multiboot2 structure")
    };

    // ---------------------------------------------------------------
    // Initialize Serial Output (for early debugging)
    // ---------------------------------------------------------------
    serial::init();
    serial_println!("[NERDOS] Serial output initialized");
    serial_println!("[NERDOS] {} v{}", KERNEL_NAME, KERNEL_VERSION);

    // ---------------------------------------------------------------
    // Initialize VGA Text Mode (for on-screen output)
    // ---------------------------------------------------------------
    vga::init();
    println!("NerdOS v{} booting...", KERNEL_VERSION);

    // ---------------------------------------------------------------
    // Print Boot Information
    // ---------------------------------------------------------------
    println!("Bootloader: {}", boot_info.boot_loader_name().unwrap_or("unknown"));

    // ---------------------------------------------------------------
    // Set up the Global Descriptor Table (GDT)
    // ---------------------------------------------------------------
    // The GDT defines memory segments for the CPU. In a modern x86_64
    // OS, we mostly use a flat segmentation model with base=0 and limit=max,
    // relying on paging for actual memory protection.
    gdt::init();
    println!("[OK] GDT initialized");

    // ---------------------------------------------------------------
    // Set up the Interrupt Descriptor Table (IDT)
    // ---------------------------------------------------------------
    // The IDT tells the CPU where to jump when interrupts/exceptions occur.
    // This must be done early because we need to catch CPU exceptions
    // (like page faults) during memory initialization.
    idt::init();
    println!("[OK] IDT initialized");

    // ---------------------------------------------------------------
    // Initialize Physical Memory Manager
    // ---------------------------------------------------------------
    // We parse the memory map from Multiboot2 to know which physical
    // frames are available for allocation. We mark the kernel image
    // and any reserved regions as unavailable.
    println!("[INFO] Initializing physical memory manager...");
    unsafe {
        // Safety: This is early boot, we're the only code running.
        // We construct the PMM from the Multiboot memory map.
        let pmm = PhysicalMemoryManager::from_multiboot(&boot_info);
        PMM = Some(pmm);
    }
    println!("[OK] Physical memory manager initialized");

    // ---------------------------------------------------------------
    // Initialize Virtual Memory (Paging)
    // ---------------------------------------------------------------
    // Set up a page table structure that identity-maps the kernel
    // and provides a heap region. This transitions us from the bootloader's
    // page tables to our own.
    println!("[INFO] Initializing paging...");
    unsafe {
        // Safety: PMM is initialized. We're still on the bootloader's
        // stack in identity-mapped memory.
        memory::init_paging(HEAP_START, HEAP_SIZE);
    }
    println!("[OK] Paging initialized");

    // ---------------------------------------------------------------
    // Initialize Heap Allocator
    // ---------------------------------------------------------------
    // Now that paging is set up, we can initialize the global allocator
    // for `alloc` types like Box, Vec, String, etc.
    unsafe {
        memory::allocator::init_heap(HEAP_START, HEAP_SIZE);
    }
    println!("[OK] Heap allocator initialized ({} KiB)", HEAP_SIZE / 1024);

    // ---------------------------------------------------------------
    // Initialize Interrupt Controllers (PIC/APIC)
    // ---------------------------------------------------------------
    // Remap the Programmable Interrupt Controllers (PIC) so that
    // hardware interrupts don't overlap with CPU exceptions.
    // Eventually we may switch to APIC for SMP support.
    unsafe { interrupts::init_pic(); }
    println!("[OK] PIC remapped (IRQs 32-47)");

    // ---------------------------------------------------------------
    // Initialize the Clock (PIT)
    // ---------------------------------------------------------------
    // The Programmable Interval Timer provides regular interrupts
    // for preemptive multitasking.
    clock::init();
    println!("[OK] PIT initialized ({} Hz)", clock::TICK_FREQUENCY);

    // ---------------------------------------------------------------
    // Initialize Syscall Interface
    // ---------------------------------------------------------------
    syscall::init();
    println!("[OK] Syscall interface initialized (syscall/sysret)");

    // ---------------------------------------------------------------
    // Enable Interrupts
    // ---------------------------------------------------------------
    // From this point on, we are interruptible. The timer interrupt
    // will drive the scheduler.
    unsafe { x86_64::instructions::interrupts::enable(); }
    println!("[OK] Interrupts enabled");

    // ---------------------------------------------------------------
    // Initialize Scheduler
    // ---------------------------------------------------------------
    // Create the init process and any kernel threads.
    unsafe {
        SCHEDULER = Some(Scheduler::new());
    }
    println!("[OK] Scheduler initialized");

    // ---------------------------------------------------------------
    // Mount Root Filesystem
    // ---------------------------------------------------------------
    // Attempt to mount the root filesystem from the boot device.
    // For now, we support initrd (initial ramdisk) as a fallback.
    println!("[INFO] Mounting root filesystem...");
    // vfs::init() would go here once drivers are loaded

    // ---------------------------------------------------------------
    // Print Welcome Message
    // ---------------------------------------------------------------
    println!("\n============================================");
    println!("  {} v{} - The Hacker's Operating System", KERNEL_NAME, KERNEL_VERSION);
    println!("  ");
    println!("  Type 'help' for available commands");
    println!("  All config is in TOML. Everything is Rust.");
    println!("============================================\n");

    // ---------------------------------------------------------------
    // Enter Idle Loop (or yield to scheduler)
    // ---------------------------------------------------------------
    // The scheduler will take over from here. We enter an idle state
    // waiting for interrupts. The timer IRQ will switch to user tasks.
    loop {
        unsafe {
            // Halt the CPU until the next interrupt.
            // This is more power-efficient than a busy loop.
            x86_64::instructions::hlt();
        }
    }
}

// ---------------------------------------------------------------------------
// Panic Handler
// ---------------------------------------------------------------------------

/// The kernel panic handler.
///
/// This is called when a `panic!` occurs anywhere in the kernel.
/// We dump useful information and halt the CPU.
///
/// # Safety
///
/// This function never returns. It halts the CPU.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Disable interrupts to prevent further damage
    unsafe { x86_64::instructions::interrupts::disable(); }

    // Try to print panic info to both VGA and serial
    println!("\n!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    println!("  KERNEL PANIC");
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");

    if let Some(location) = info.location() {
        println!("  Location: {}:{}", location.file(), location.line());
    }
    if let Some(message) = info.message() {
        // We can't easily format the message without alloc, but we can try
        println!("  Message: {:?}", message);
    }

    serial_println!("[PANIC] {}", info);

    // Halt the CPU forever
    loop {
        unsafe {
            x86_64::instructions::interrupts::disable();
            x86_64::instructions::hlt();
        }
    }
}

// ---------------------------------------------------------------------------
// Allocation Error Handler
// ---------------------------------------------------------------------------

/// Called when the global allocator fails to allocate memory.
/// This is a critical failure in the kernel.
#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error: {:?}", layout);
}

// ---------------------------------------------------------------------------
// Macros
// ---------------------------------------------------------------------------

/// Print to the VGA text buffer (like `print!` in std).
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga::_print(format_args!($($arg)*)));
}

/// Print with newline to the VGA text buffer (like `println!` in std).
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

/// Print to the serial port (for debugging).
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => ($crate::serial::_print(format_args!($($arg)*)));
}

/// Print with newline to the serial port.
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => ($crate::serial_print!("{}\n", format_args!($($arg)*)));
}
