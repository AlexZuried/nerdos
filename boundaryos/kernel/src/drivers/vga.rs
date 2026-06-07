//! VGA text buffer driver
//! 
//! Provides text output to VGA display.

use spin::Mutex;
use core::fmt::{Write, Result as fmtResult};

/// VGA buffer dimensions
const VGA_WIDTH: usize = 80;
const VGA_HEIGHT: usize = 25;

/// VGA text mode color codes
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

/// VGA writer with cursor position
pub static WRITER: Mutex<VgaWriter> = Mutex::new(VgaWriter {
    column_position: 0,
    color_code: ColorCode::new(Color::White, Color::Black),
    buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
});

/// Color code for VGA text
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    const fn new(foreground: Color, background: Color) -> Self {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

/// VGA character with color
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

/// VGA buffer layout
#[repr(transparent)]
struct Buffer {
    chars: [[ScreenChar; VGA_WIDTH]; VGA_HEIGHT],
}

/// VGA text writer
pub struct VgaWriter {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl VgaWriter {
    /// Write a byte to VGA buffer
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= VGA_WIDTH {
                    self.new_line();
                }
                
                let row = VGA_HEIGHT - 1;
                let col = self.column_position;
                
                self.buffer.chars[row][col] = ScreenChar {
                    ascii_character: byte,
                    color_code: self.color_code,
                };
                self.column_position += 1;
            }
        }
    }
    
    /// Move to next line
    fn new_line(&mut self) {
        // TODO: Implement scrolling
        self.column_position = 0;
    }
}

impl Write for VgaWriter {
    fn write_str(&mut self, s: &str) -> fmtResult {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

/// Initialize VGA driver
pub fn init() {
    log!("VGA text buffer initialized");
    // Clear screen
    let mut writer = WRITER.lock();
    for row in 0..VGA_HEIGHT {
        for col in 0..VGA_WIDTH {
            writer.buffer.chars[row][col] = ScreenChar {
                ascii_character: b' ',
                color_code: writer.color_code,
            };
        }
    }
}
