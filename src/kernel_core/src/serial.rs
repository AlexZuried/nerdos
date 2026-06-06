//! # Serial Port Driver (UART 16550)
//!
//! Provides output to COM1 (serial port 1) for debugging.
//! This is invaluable during kernel development because it works even
//! when the VGA buffer or display drivers aren't functional.
//!
//! ## Serial Port Addresses
//!
//! | Port    | I/O Base |
//! |---------|----------|
//! | COM1    | 0x3F8    |
//! | COM2    | 0x2F8    |
//! | COM3    | 0x3E8    |
//! | COM4    | 0x2E8    |
//!
//! ## Register Map (relative to base)
//!
//! | Offset | Name              | Access |
//! |--------|-------------------|--------|
//! | +0     | Data (DLAB=0)     | R/W    |
//! | +1     | Interrupt Enable  | W      |
//! | +2     | Interrupt ID      | R      |
//! | +3     | Line Control      | W      |
//! | +4     | Modem Control     | W      |
//! | +5     | Line Status       | R      |
//! | +6     | Modem Status      | R      |
//! | +7     | Scratch           | R/W    |

use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::port::Port;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Base I/O port for COM1.
const SERIAL_PORT_BASE: u16 = 0x3F8;

/// Baud rate divisor for 115200 baud.
/// Divisor = 115200 / desired_baud. For 115200, divisor = 1.
const BAUD_DIVISOR: u16 = 1;

// Register offsets
const DATA_REG: u16 = 0;      // Data register (R/W)
const IER_REG: u16 = 1;       // Interrupt Enable Register
const IIR_REG: u16 = 2;       // Interrupt Identification Register
const FCR_REG: u16 = 2;       // FIFO Control Register
const LCR_REG: u16 = 3;       // Line Control Register
const MCR_REG: u16 = 4;       // Modem Control Register
const LSR_REG: u16 = 5;       // Line Status Register

// Line Control Register bits
const LCR_DLAB: u8 = 0x80;    // Divisor Latch Access Bit
const LCR_8N1: u8 = 0x03;     // 8 data bits, no parity, 1 stop bit

// FIFO Control Register bits
const FCR_ENABLE: u8 = 0x01;  // Enable FIFO
const FCR_CLEAR: u8 = 0x06;   // Clear both FIFOs

// Modem Control Register bits
const MCR_DTR: u8 = 0x01;     // Data Terminal Ready
const MCR_RTS: u8 = 0x02;     // Request To Send
const MCR_OUT2: u8 = 0x08;    // Auxiliary output 2 (required for interrupts)

// Line Status Register bits
const LSR_EMPTY: u8 = 0x20;   // Transmitter holding register empty
const LSR_DATA: u8 = 0x01;    // Data ready

// ---------------------------------------------------------------------------
// Serial Port Structure
// ---------------------------------------------------------------------------

/// A UART 16550 serial port.
pub struct SerialPort {
    /// Base I/O port.
    base: u16,
}

impl SerialPort {
    /// Create a new serial port at the given base address.
    ///
    /// # Safety
    /// The base address must point to a valid UART 16550 serial port.
    pub const unsafe fn new(base: u16) -> Self {
        SerialPort { base }
    }

    /// Initialize the serial port.
    ///
    /// Configures:
    /// - 115200 baud
    /// - 8 data bits, no parity, 1 stop bit (8N1)
    /// - FIFOs enabled with 14-byte threshold
    /// - Interrupts enabled
    pub fn init(&mut self) {
        unsafe {
            // Disable interrupts during initialization.
            self.write_reg(IER_REG, 0x00);

            // Enable DLAB to set baud rate divisor.
            self.write_reg(LCR_REG, LCR_DLAB);

            // Set divisor (low byte, then high byte).
            self.write_reg(DATA_REG, (BAUD_DIVISOR & 0xFF) as u8);
            self.write_reg(IER_REG, ((BAUD_DIVISOR >> 8) & 0xFF) as u8);

            // 8 bits, no parity, one stop bit. Clear DLAB.
            self.write_reg(LCR_REG, LCR_8N1);

            // Enable FIFO, clear them, with 14-byte threshold.
            self.write_reg(FCR_REG, FCR_ENABLE | FCR_CLEAR | 0xC0);

            // Enable IRQs, RTS, DTR.
            self.write_reg(MCR_REG, MCR_DTR | MCR_RTS | MCR_OUT2);

            // Enable interrupts (data available, transmitter empty).
            self.write_reg(IER_REG, 0x01);
        }
    }

    /// Check if the transmitter is ready to accept a byte.
    fn is_transmit_empty(&self) -> bool {
        unsafe {
            (self.read_reg(LSR_REG) & LSR_EMPTY) != 0
        }
    }

    /// Wait until the transmitter is ready, then send a byte.
    pub fn send(&mut self, data: u8) {
        // Wait for the transmitter holding register to be empty.
        while !self.is_transmit_empty() {
            core::hint::spin_loop();
        }

        unsafe {
            self.write_reg(DATA_REG, data);
        }
    }

    /// Check if data is available to read.
    pub fn data_available(&self) -> bool {
        unsafe {
            (self.read_reg(LSR_REG) & LSR_DATA) != 0
        }
    }

    /// Read a byte from the serial port.
    /// Returns None if no data is available.
    pub fn receive(&mut self) -> Option<u8> {
        if self.data_available() {
            Some(unsafe { self.read_reg(DATA_REG) })
        } else {
            None
        }
    }

    /// Write a string to the serial port.
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            self.send(byte);
        }
    }

    // Low-level register access.

    /// Read from a register.
    ///
    /// # Safety
    /// The register offset must be valid (0-7).
    unsafe fn read_reg(&self, reg: u16) -> u8 {
        let mut port: Port<u8> = Port::new(self.base + reg);
        port.read()
    }

    /// Write to a register.
    ///
    /// # Safety
    /// The register offset must be valid (0-7).
    unsafe fn write_reg(&mut self, reg: u16, value: u8) {
        let mut port: Port<u8> = Port::new(self.base + reg);
        port.write(value);
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Global Serial Port
// ---------------------------------------------------------------------------

/// Global serial port instance (COM1), protected by a spinlock.
lazy_static! {
    static ref SERIAL1: Mutex<SerialPort> = Mutex::new(unsafe {
        // Safety: COM1 at 0x3F8 is standard on x86 PCs.
        SerialPort::new(SERIAL_PORT_BASE)
    });
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Initialize the serial port for debugging output.
pub fn init() {
    SERIAL1.lock().init();
}

/// Print formatted text to the serial port.
///
/// This is the internal implementation called by the `serial_print!` macros.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    SERIAL1.lock().write_fmt(args).unwrap();
}

/// Handle a serial port interrupt.
/// Called from the serial interrupt handler.
pub fn handle_interrupt() {
    let mut serial = SERIAL1.lock();
    while serial.data_available() {
        if let Some(byte) = serial.receive() {
            // In a real implementation, this would be buffered for reading
            // by user processes via sys_read(STDIN).
            // For now, echo it back.
            serial.send(byte);
        }
    }
}

/// Read a byte from the serial port (blocking).
///
/// # Safety
/// This spins until data is available. Only use during early boot
/// or when interrupts are disabled.
pub fn read_byte_blocking() -> u8 {
    let mut serial = SERIAL1.lock();
    loop {
        if let Some(byte) = serial.receive() {
            return byte;
        }
        core::hint::spin_loop();
    }
}
