//! # Interrupt Controller Management
//!
//! This module handles the Programmable Interrupt Controllers (PIC).
//! In modern systems, the APIC (Advanced PIC) is preferred, but the legacy
//! 8259 PIC is simpler to set up and sufficient for single-core operation.
//!
//! ## PIC Cascade
//!
//! The x86 has two PIC chips cascaded together:
//! - Master PIC: handles IRQs 0-7, connected to CPU's INT pin
//! - Slave PIC: handles IRQs 8-15, cascaded through IRQ2 of master
//!
//! We remap them so that IRQs don't overlap with CPU exceptions (0-31).

use spin::Mutex;

// ---------------------------------------------------------------------------
// PIC Constants
// ---------------------------------------------------------------------------

/// I/O port for the master PIC command register.
pub const PIC1_COMMAND: u8 = 0x20;
/// I/O port for the master PIC data register.
pub const PIC1_DATA: u8 = 0x21;
/// I/O port for the slave PIC command register.
pub const PIC2_COMMAND: u8 = 0xA0;
/// I/O port for the slave PIC data register.
pub const PIC2_DATA: u8 = 0xA1;

/// Command to initialize the PIC (ICW1).
const CMD_INIT: u8 = 0x11;
/// Command to acknowledge/end an interrupt (EOI).
const CMD_END_OF_INTERRUPT: u8 = 0x20;

/// Mode: 8086/88 (MCS-80/85) mode (ICW4).
const MODE_8086: u8 = 0x01;

// ---------------------------------------------------------------------------
// Global PIC Instance
// ---------------------------------------------------------------------------

/// The global PIC controller, protected by a spinlock for thread safety.
pub static PICS: Mutex<ChainedPics> = Mutex::new(unsafe {
    // Safety: ChainedPics::new_contiguous is const and the offsets are valid.
    ChainedPics::new_contiguous(super::idt::PIC_1_OFFSET, super::idt::PIC_2_OFFSET)
});

// ---------------------------------------------------------------------------
// ChainedPics Structure
// ---------------------------------------------------------------------------

/// Represents the two PIC chips in their cascaded configuration.
pub struct ChainedPics {
    /// Offset added to master PIC interrupts.
    offset1: u8,
    /// Offset added to slave PIC interrupts.
    offset2: u8,
    /// Cached mask for master PIC.
    master_mask: u8,
    /// Cached mask for slave PIC.
    slave_mask: u8,
}

impl ChainedPics {
    /// Create a new ChainedPics instance.
    ///
    /// # Safety
    ///
    /// The offsets must be in the range 32..=215 and must not overlap.
    pub const unsafe fn new_contiguous(offset1: u8, offset2: u8) -> ChainedPics {
        ChainedPics {
            offset1,
            offset2,
            master_mask: 0,
            slave_mask: 0,
        }
    }

    /// Initialize and remap both PICs.
    ///
    /// The default mapping has master's IRQ0 at vector 8, which conflicts
    /// with CPU exceptions. We remap so that:
    /// - Master IRQs 0-7 → vectors offset1..offset1+7
    /// - Slave IRQs 8-15 → vectors offset2..offset2+7
    ///
    /// The initialization sequence (ICW = Initialization Command Word):
    /// 1. ICW1: Tell PIC we want to initialize + expect ICW4
    /// 2. ICW2: Set the vector offset
    /// 3. ICW3: Tell master about slave cascade, slave about cascade ID
    /// 4. ICW4: Set 8086 mode
    pub fn initialize(&mut self) {
        use x86_64::instructions::port::Port;

        // Safety: These ports are well-defined and exclusive to the PIC.
        let mut master_cmd: Port<u8> = Port::new(PIC1_COMMAND as u16);
        let mut master_data: Port<u8> = Port::new(PIC1_DATA as u16);
        let mut slave_cmd: Port<u8> = Port::new(PIC2_COMMAND as u16);
        let mut slave_data: Port<u8> = Port::new(PIC2_DATA as u16);

        // Save current masks so we can restore them after init.
        let saved_master_mask: u8 = unsafe { master_data.read() };
        let saved_slave_mask: u8 = unsafe { slave_data.read() };

        // ICW1: Start initialization, expect ICW4
        unsafe {
            master_cmd.write(CMD_INIT);
            slave_cmd.write(CMD_INIT);
        }

        // ICW2: Set vector offsets
        unsafe {
            master_data.write(self.offset1);
            slave_data.write(self.offset2);
        }

        // ICW3: Configure cascade
        // Master: bit mask showing which IRQ has slave (IRQ2 = bit 2 = 0x04)
        // Slave: cascade identity (connected to IRQ2 on master)
        unsafe {
            master_data.write(0x04); // Tell master slave is at IRQ2
            slave_data.write(0x02);  // Tell slave its cascade identity
        }

        // ICW4: Set 8086 mode
        unsafe {
            master_data.write(MODE_8086);
            slave_data.write(MODE_8086);
        }

        // Restore saved masks
        unsafe {
            master_data.write(saved_master_mask);
            slave_data.write(saved_slave_mask);
        }

        // Cache the masks
        self.master_mask = saved_master_mask;
        self.slave_mask = saved_slave_mask;
    }

    /// Send End of Interrupt signal.
    /// Must be called at the end of every hardware interrupt handler.
    pub unsafe fn notify_end_of_interrupt(&mut self, irq: u8) {
        use x86_64::instructions::port::Port;

        // If this is a slave IRQ (8-15), we must also EOI the slave.
        if irq >= self.offset2 && irq < self.offset2 + 8 {
            let mut slave_cmd: Port<u8> = Port::new(PIC2_COMMAND as u16);
            slave_cmd.write(CMD_END_OF_INTERRUPT);
        }

        // Always EOI the master (for both master and slave IRQs).
        let mut master_cmd: Port<u8> = Port::new(PIC1_COMMAND as u16);
        master_cmd.write(CMD_END_OF_INTERRUPT);
    }

    /// Mask (disable) a specific IRQ line.
    pub fn mask_irq(&mut self, irq: u8) {
        use x86_64::instructions::port::Port;

        let irq = irq - self.offset1;

        if irq < 8 {
            self.master_mask |= 1 << irq;
            unsafe {
                let mut master_data: Port<u8> = Port::new(PIC1_DATA as u16);
                master_data.write(self.master_mask);
            }
        } else {
            let irq = irq - 8;
            self.slave_mask |= 1 << irq;
            unsafe {
                let mut slave_data: Port<u8> = Port::new(PIC2_DATA as u16);
                slave_data.write(self.slave_mask);
            }
        }
    }

    /// Unmask (enable) a specific IRQ line.
    pub fn unmask_irq(&mut self, irq: u8) {
        use x86_64::instructions::port::Port;

        let irq = irq - self.offset1;

        if irq < 8 {
            self.master_mask &= !(1 << irq);
            unsafe {
                let mut master_data: Port<u8> = Port::new(PIC1_DATA as u16);
                master_data.write(self.master_mask);
            }
        } else {
            let irq = irq - 8;
            self.slave_mask &= !(1 << irq);
            unsafe {
                let mut slave_data: Port<u8> = Port::new(PIC2_DATA as u16);
                slave_data.write(self.slave_mask);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

/// Initialize and remap the PICs.
///
/// This remaps IRQs so they don't conflict with CPU exceptions:
/// - IRQ 0 (timer) → vector 32
/// - IRQ 1 (keyboard) → vector 33
/// - ...etc
///
/// # Safety
/// Must be called during early boot, after the IDT is set up but before
/// interrupts are enabled.
pub fn init_pic() {
    let mut pics = PICS.lock();
    pics.initialize();

    // Unmask the interrupts we care about.
    // Keep others masked to prevent spurious interrupts from uninitialized devices.
    pics.unmask_irq(super::idt::PIC_1_OFFSET + super::idt::IRQ_TIMER);
    pics.unmask_irq(super::idt::PIC_1_OFFSET + super::idt::IRQ_KEYBOARD);
    pics.unmask_irq(super::idt::PIC_1_OFFSET + super::idt::IRQ_SERIAL1);
}
