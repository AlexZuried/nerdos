//! # VGA Text Mode Driver
//!
//! Provides output to the VGA text buffer, which is mapped at 0xB8000.
//! This is the classic 80x25 character display used by BIOS and bootloaders.
//!
//! ## VGA Text Buffer Format
//!
//! Each character cell is 2 bytes:
//! - Byte 0: ASCII character
//! - Byte 1: Attribute (foreground color | background color << 4)
//!
//! The buffer is at physical address 0xB8000 and is 4000 bytes
/// (80 columns * 25 rows * 2 bytes).

use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Physical address of the VGA text buffer.
const VGA_BUFFER_ADDR: usize = 0xB8000;

/// Screen dimensions.
const SCREEN_WIDTH: usize = 80;
const SCREEN_HEIGHT: usize = 25;

// ---------------------------------------------------------------------------
// Colors
// ---------------------------------------------------------------------------

/// Available colors for the VGA text mode.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// A color code combines foreground and background colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    /// Create a new color code from foreground and background colors.
    const fn new(foreground: Color, background: Color) -> Self {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

// ---------------------------------------------------------------------------
// Screen Character
// ---------------------------------------------------------------------------

/// A single character on the VGA screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    /// The ASCII character to display.
    ascii_character: u8,
    /// The color attribute.
    color_code: ColorCode,
}

// ---------------------------------------------------------------------------
// VGA Buffer
// ---------------------------------------------------------------------------

/// The VGA text buffer is a 2D array of screen characters.
/// We use `Volatile` to ensure the compiler doesn't optimize away writes.
struct VgaBuffer {
    chars: [[Volatile<ScreenChar>; SCREEN_WIDTH]; SCREEN_HEIGHT],
}

/// Global VGA writer instance, protected by a spinlock.
lazy_static! {
    static ref VGA_WRITER: Mutex<VgaWriter> = Mutex::new(VgaWriter {
        column_position: 0,
        color_code: ColorCode::new(Color::LightGray, Color::Black),
        buffer: unsafe { &mut *(VGA_BUFFER_ADDR as *mut VgaBuffer) },
    });
}

// ---------------------------------------------------------------------------
// VGA Writer
// ---------------------------------------------------------------------------

/// The VGA writer maintains the cursor position and color.
/// It implements `fmt::Write` so we can use the `write!` and `writeln!` macros.
pub struct VgaWriter {
    /// Current column position on the current row.
    column_position: usize,
    /// Current foreground/background color.
    color_code: ColorCode,
    /// Reference to the VGA buffer.
    buffer: &'static mut VgaBuffer,
}

impl VgaWriter {
    /// Write a single byte to the VGA buffer.
    ///
    /// Special characters:
    /// - `\n`: Newline (move to start of next line)
    /// - `\r`: Carriage return (move to start of current line)
    /// - `\t`: Tab (advance to next 8-column boundary)
    /// - `\x08`: Backspace (move cursor back one position)
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            b'\r' => self.column_position = 0,
            b'\t' => {
                // Advance to next 8-column tab stop
                let next_tab = (self.column_position + 8) & !7;
                while self.column_position < next_tab && self.column_position < SCREEN_WIDTH {
                    self.write_raw_byte(b' ');
                }
            }
            b'\x08' => {
                // Backspace
                if self.column_position > 0 {
                    self.column_position -= 1;
                    self.write_raw_byte_at(b' ', self.column_position, SCREEN_HEIGHT - 1);
                }
            }
            byte => {
                if self.column_position >= SCREEN_WIDTH {
                    self.new_line();
                }
                self.write_raw_byte_at(byte, self.column_position, SCREEN_HEIGHT - 1);
                self.column_position += 1;
            }
        }
    }

    /// Write a raw byte at a specific position.
    fn write_raw_byte_at(&mut self, byte: u8, col: usize, row: usize) {
        let color_code = self.color_code;
        self.buffer.chars[row][col].write(ScreenChar {
            ascii_character: byte,
            color_code,
        });
    }

    /// Write a raw byte at the current position (no wrapping).
    fn write_raw_byte(&mut self, byte: u8) {
        self.write_raw_byte_at(byte, self.column_position, SCREEN_HEIGHT - 1);
        self.column_position += 1;
    }

    /// Write a string to the VGA buffer.
    ///
    /// Non-ASCII characters and control characters (except \n, \r, \t, \x08)
    /// are replaced with a placeholder.
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // Printable ASCII or newline
                0x20..=0x7E | b'\n' | b'\r' | b'\t' | b'\x08' => self.write_byte(byte),
                // Non-ASCII: print as placeholder
                _ => self.write_byte(0xFE),
            }
        }
    }

    /// Move to a new line, scrolling if necessary.
    fn new_line(&mut self) {
        // Move everything up by one row.
        for row in 1..SCREEN_HEIGHT {
            for col in 0..SCREEN_WIDTH {
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(character);
            }
        }

        // Clear the last row.
        self.clear_row(SCREEN_HEIGHT - 1);
        self.column_position = 0;
    }

    /// Clear a specific row with spaces.
    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..SCREEN_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }

    /// Clear the entire screen.
    pub fn clear_screen(&mut self) {
        for row in 0..SCREEN_HEIGHT {
            self.clear_row(row);
        }
        self.column_position = 0;
    }

    /// Set the foreground color.
    pub fn set_foreground_color(&mut self, color: Color) {
        self.color_code = ColorCode::new(color, Color::Black);
    }

    /// Set the full color (foreground + background).
    pub fn set_color(&mut self, foreground: Color, background: Color) {
        self.color_code = ColorCode::new(foreground, background);
    }
}

impl fmt::Write for VgaWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Initialize the VGA driver.
/// Clears the screen and sets default colors.
pub fn init() {
    let mut writer = VGA_WRITER.lock();
    writer.clear_screen();
    writer.set_foreground_color(Color::LightGreen);
}

/// Print formatted text to the VGA buffer.
///
/// This is the internal implementation called by the `print!` and `println!` macros.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    VGA_WRITER.lock().write_fmt(args).unwrap();
}

/// Clear the screen.
pub fn clear_screen() {
    VGA_WRITER.lock().clear_screen();
}

/// Set the text color.
pub fn set_color(foreground: Color) {
    VGA_WRITER.lock().set_foreground_color(foreground);
}
