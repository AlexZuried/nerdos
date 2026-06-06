//! Serial port driver (COM1)
//! 
//! Provides debug output via serial port.

use spin::Mutex;
use core::fmt::{Write, Result as fmtResult};

/// Serial port writer
pub static SERIAL: Mutex<SerialPort> = Mutex::new(SerialPort);

/// COM1 I/O port
const SERIAL_PORT: u16 = 0x3F8;

/// Serial port driver
pub struct SerialPort;

impl SerialPort {
    /// Initialize serial port
    pub fn init(&mut self) {
        unsafe {
            // Disable interrupts
            outb(SERIAL_PORT + 1, 0x00);
            // Enable DLAB (set baud rate divisor)
            outb(SERIAL_PORT + 3, 0x80);
            // Set divisor to 3 (38400 baud)
            outb(SERIAL_PORT + 0, 0x03);
            outb(SERIAL_PORT + 1, 0x00);
            // 8 bits, no parity, one stop bit
            outb(SERIAL_PORT + 3, 0x03);
            // Enable FIFO
            outb(SERIAL_PORT + 2, 0xC7);
            // Enable IRQs, RTS/DSR
            outb(SERIAL_PORT + 4, 0x0B);
        }
    }
    
    /// Check if serial port is ready to transmit
    fn is_transmit_empty(&self) -> bool {
        unsafe {
            (inb(SERIAL_PORT + 5) & 0x20) != 0
        }
    }
    
    /// Write a byte to serial port
    fn write_byte(&self, byte: u8) {
        while !self.is_transmit_empty() {}
        unsafe {
            outb(SERIAL_PORT, byte);
        }
    }
}

impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmtResult {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

/// Initialize serial port driver
pub fn init() {
    SERIAL.lock().init();
    log!("Serial port initialized (COM1)");
}

/// Port I/O read
unsafe fn inb(port: u16) -> u8 {
    let ret: u8;
    core::arch::asm!("in al, dx", out("al") ret, in("dx") port, options(nomem, nostack, preserves_flags));
    ret
}

/// Port I/O write
unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
}
