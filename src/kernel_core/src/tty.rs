//! # Terminal (TTY) Layer
//!
//! The TTY layer provides:
//! - Keyboard input handling (scancode decoding)
//! - Input line editing (backspace, echo)
//! - cooked/canonical mode (line-buffered input)
//! - Session management for multiple virtual terminals
//!
//! ## Architecture
//!
//! ```text
//! User Process (read/write)
//!       |
//!   TTY Layer (line discipline)
//!       |
//!   Keyboard Driver ----> VGA Output
//!       ^
//!   Hardware IRQ1
//! ```

use spin::Mutex;

// ---------------------------------------------------------------------------
// Keyboard Sub-module
// ---------------------------------------------------------------------------

pub mod keyboard {
    //! # PS/2 Keyboard Driver
    //!
    //! Handles scan code set 1 (the default set used by most PCs).
    //! Supports:
    //! - Regular key presses and releases
    //! - Shift, Ctrl, Alt modifiers
    //! - Extended keys (arrows, function keys)
    //! - Numeric keypad

    use super::*;

    // Current modifier state.
    static SHIFT_PRESSED: Mutex<bool> = Mutex::new(false);
    static CTRL_PRESSED: Mutex<bool> = Mutex::new(false);
    static ALT_PRESSED: Mutex<bool> = Mutex::new(false);

    /// Extended scancode flag (sent before extended key codes).
    const SCANCODE_PREFIX_E0: u8 = 0xE0;
    /// Release flag (OR'd with the make code).
    const SCANCODE_RELEASE: u8 = 0x80;

    /// Standard US QWERTY keymap (unshifted).
    const KEYMAP_US: [char; 128] = [
        // 0x00 - 0x07
        '\0', '\x1B', '1', '2', '3', '4', '5', '6',
        // 0x08 - 0x0F
        '7', '8', '9', '0', '-', '=', '\x08', '\t',
        // 0x10 - 0x17
        'q', 'w', 'e', 'r', 't', 'y', 'u', 'i',
        // 0x18 - 0x1F
        'o', 'p', '[', ']', '\n', '\0', 'a', 's',
        // 0x20 - 0x27
        'd', 'f', 'g', 'h', 'j', 'k', 'l', ';',
        // 0x28 - 0x2F
        '\'', '`', '\0', '\\', 'z', 'x', 'c', 'v',
        // 0x30 - 0x37
        'b', 'n', 'm', ',', '.', '/', '\0', '\0',
        // 0x38 - 0x3F
        '\0', ' ', '\0', '\0', '\0', '\0', '\0', '\0',
        // 0x40 - 0x47
        '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
        // 0x48 - 0x4F
        '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
        // 0x50 - 0x57
        '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
        // 0x58 - 0x5F
        '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
        // 0x60 - 0x67
        '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
        // 0x68 - 0x6F
        '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
        // 0x70 - 0x77
        '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
        // 0x78 - 0x7F
        '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
    ];

    /// Shifted US QWERTY keymap.
    const KEYMAP_US_SHIFT: [char; 128] = [
        // 0x00 - 0x07
        '\0', '\x1B', '!', '@', '#', '$', '%', '^',
        // 0x08 - 0x0F
        '&', '*', '(', ')', '_', '+', '\x08', '\t',
        // 0x10 - 0x17
        'Q', 'W', 'E', 'R', 'T', 'Y', 'U', 'I',
        // 0x18 - 0x1F
        'O', 'P', '{', '}', '\n', '\0', 'A', 'S',
        // 0x20 - 0x27
        'D', 'F', 'G', 'H', 'J', 'K', 'L', ':',
        // 0x28 - 0x2F
        '"', '~', '\0', '|', 'Z', 'X', 'C', 'V',
        // 0x30 - 0x37
        'B', 'N', 'M', '<', '>', '?', '\0', '\0',
        // 0x38 - 0x3F
        '\0', ' ', '\0', '\0', '\0', '\0', '\0', '\0',
        // Rest is same as unshifted
        '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
        '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
        '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
        '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
        '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
        '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
        '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
        '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
        '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
    ];

    /// Extended scancode keymap (for arrow keys, etc.).
    #[derive(Debug, Clone, Copy)]
    pub enum ExtendedKey {
        /// Not an extended key.
        None,
        /// Home key.
        Home,
        /// End key.
        End,
        /// Arrow up.
        Up,
        /// Arrow down.
        Down,
        /// Arrow left.
        Left,
        /// Arrow right.
        Right,
        /// Page up.
        PageUp,
        /// Page down.
        PageDown,
        /// Insert key.
        Insert,
        /// Delete key.
        Delete,
        /// Unknown extended key.
        Unknown(u8),
    }

    /// Process a raw scancode from the keyboard.
    ///
    /// This is called from the keyboard interrupt handler.
    /// It decodes the scancode and buffers the resulting character.
    pub fn handle_scancode(scancode: u8) {
        // Check for extended prefix (E0).
        if scancode == SCANCODE_PREFIX_E0 {
            // Next byte will be the extended scancode.
            // For now, we just ignore the prefix and handle the next byte.
            // A full implementation would track extended state.
            return;
        }

        // Check if this is a key release (bit 7 set).
        let is_release = (scancode & SCANCODE_RELEASE) != 0;
        let make_code = scancode & !SCANCODE_RELEASE;

        // Handle modifier keys.
        match make_code {
            0x2A | 0x36 => {
                // Left shift (0x2A) or Right shift (0x36)
                *SHIFT_PRESSED.lock() = !is_release;
                return;
            }
            0x1D => {
                // Left Ctrl
                *CTRL_PRESSED.lock() = !is_release;
                return;
            }
            0x38 => {
                // Left Alt
                *ALT_PRESSED.lock() = !is_release;
                return;
            }
            _ => {}
        }

        // If it's a release, ignore for now (no key repeat).
        if is_release {
            return;
        }

        // Look up the character.
        let shift = *SHIFT_PRESSED.lock();
        let ctrl = *CTRL_PRESSED.lock();
        let keymap = if shift { &KEYMAP_US_SHIFT } else { &KEYMAP_US };

        if make_code < 128 {
            let ch = keymap[make_code as usize];
            if ch != '\0' {
                // Handle Ctrl+letter combinations (e.g., Ctrl+C = 0x03).
                let out = if ctrl && ch.is_ascii_lowercase() {
                    (ch as u8 - b'a' + 1) as char
                } else if ctrl && ch.is_ascii_uppercase() {
                    (ch as u8 - b'A' + 1) as char
                } else {
                    ch
                };

                // Buffer the character for reading.
                buffer_char(out);

                // Echo to screen (in canonical mode).
                if out == '\n' || out == '\r' {
                    crate::println!();
                } else if out == '\x08' {
                    // Backspace
                    crate::print!("\x08 \x08");
                } else {
                    crate::print!("{}", out);
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Input Buffer
    // -----------------------------------------------------------------------

    /// Size of the input buffer.
    const INPUT_BUFFER_SIZE: usize = 256;

    /// The input buffer for keyboard data.
    static INPUT_BUFFER: Mutex<InputBuffer> = Mutex::new(InputBuffer::new());

    /// A simple circular buffer for keyboard input.
    struct InputBuffer {
        data: [u8; INPUT_BUFFER_SIZE],
        head: usize,
        tail: usize,
        count: usize,
    }

    impl InputBuffer {
        const fn new() -> Self {
            InputBuffer {
                data: [0; INPUT_BUFFER_SIZE],
                head: 0,
                tail: 0,
                count: 0,
            }
        }

        fn push(&mut self, byte: u8) -> bool {
            if self.count >= INPUT_BUFFER_SIZE {
                return false; // Buffer full
            }
            self.data[self.tail] = byte;
            self.tail = (self.tail + 1) % INPUT_BUFFER_SIZE;
            self.count += 1;
            true
        }

        fn pop(&mut self) -> Option<u8> {
            if self.count == 0 {
                return None;
            }
            let byte = self.data[self.head];
            self.head = (self.head + 1) % INPUT_BUFFER_SIZE;
            self.count -= 1;
            Some(byte)
        }
    }

    /// Buffer a character from the keyboard.
    fn buffer_char(ch: char) {
        let mut buffer = INPUT_BUFFER.lock();
        buffer.push(ch as u8);
    }

    /// Read a character from the input buffer (non-blocking).
    pub fn read_char() -> Option<char> {
        let mut buffer = INPUT_BUFFER.lock();
        buffer.pop().map(|b| b as char)
    }

    /// Read a character from the input buffer (blocking).
    pub fn read_char_blocking() -> char {
        loop {
            if let Some(ch) = read_char() {
                return ch;
            }
            // Halt until next interrupt (keyboard or timer).
            unsafe { x86_64::instructions::hlt(); }
        }
    }
}

// ---------------------------------------------------------------------------
// TTY Structure
// ---------------------------------------------------------------------------

/// A terminal session.
/// Manages the input/output state for a single user session.
pub struct Tty {
    /// TTY number (0 = console, 1-7 = virtual terminals).
    pub number: usize,
    /// Foreground process group ID.
    pub pgid: u64,
    /// Session ID.
    pub sid: u64,
    /// Current foreground color for output.
    foreground_color: crate::vga::Color,
    /// Whether canonical (line-buffered) mode is active.
    canonical_mode: bool,
    /// Whether echo is enabled.
    echo: bool,
    /// Raw input buffer (before line discipline processing).
    raw_buffer: [u8; 256],
    /// Position in raw buffer.
    raw_pos: usize,
    /// Cooked line buffer (after line discipline).
    cooked_buffer: [u8; 256],
    /// Position in cooked buffer.
    cooked_pos: usize,
    /// Size of the cooked line (for read).
    cooked_size: usize,
}

impl Tty {
    /// Create a new TTY.
    pub const fn new(number: usize) -> Self {
        Tty {
            number,
            pgid: 0,
            sid: 0,
            foreground_color: crate::vga::Color::LightGray,
            canonical_mode: true,
            echo: true,
            raw_buffer: [0; 256],
            raw_pos: 0,
            cooked_buffer: [0; 256],
            cooked_pos: 0,
            cooked_size: 0,
        }
    }

    /// Set canonical mode.
    pub fn set_canonical(&mut self, enabled: bool) {
        self.canonical_mode = enabled;
    }

    /// Set echo mode.
    pub fn set_echo(&mut self, enabled: bool) {
        self.echo = enabled;
    }

    /// Write a byte to the TTY output (VGA).
    pub fn output(&mut self, byte: u8) {
        match byte {
            b'\r' => crate::print!("\r"),
            b'\n' => crate::println!(),
            b'\t' => crate::print!("\t"),
            b'\x08' => crate::print!("\x08 \x08"),
            // Ring the bell
            b'\x07' => { /* Bell - no-op in text mode */ }
            // Escape sequences - handle cursor movement
            0x1B => { /* ESC - start of escape sequence */ }
            // Printable ASCII
            0x20..=0x7E => crate::print!("{}", byte as char),
            // Non-printable
            _ => crate::print!("?"),
        }
    }

    /// Write a string to the TTY output.
    pub fn output_str(&mut self, s: &str) {
        for byte in s.bytes() {
            self.output(byte);
        }
    }

    /// Read a line from the TTY input (blocking, canonical mode).
    pub fn read_line(&mut self, buf: &mut [u8]) -> usize {
        let mut pos = 0;

        while pos < buf.len() {
            let ch = keyboard::read_char_blocking();

            if ch == '\n' || ch == '\r' {
                buf[pos] = b'\n';
                pos += 1;
                self.output(b'\n');
                break;
            } else if ch == '\x08' || ch == '\x7F' {
                // Backspace
                if pos > 0 {
                    pos -= 1;
                    self.output(b'\x08');
                }
            } else if ch.is_ascii() {
                if pos < buf.len() {
                    buf[pos] = ch as u8;
                    pos += 1;
                    if self.echo {
                        self.output(ch as u8);
                    }
                }
            }
        }

        pos
    }

    /// Process a raw input byte through the line discipline.
    /// In canonical mode, buffers until newline. In raw mode, passes through.
    fn process_input(&mut self, byte: u8) {
        if self.canonical_mode {
            // Canonical mode: buffer until newline.
            if byte == b'\n' || byte == b'\r' {
                // Deliver the line.
                self.cooked_buffer[self.cooked_pos] = b'\n';
                self.cooked_pos += 1;
                self.cooked_size = self.cooked_pos;
                self.cooked_pos = 0;
            } else if byte == b'\x08' || byte == 0x7F {
                // Backspace
                if self.cooked_pos > 0 {
                    self.cooked_pos -= 1;
                }
            } else {
                self.cooked_buffer[self.cooked_pos] = byte;
                self.cooked_pos += 1;
            }
        } else {
            // Raw mode: pass through immediately.
            self.cooked_buffer[0] = byte;
            self.cooked_size = 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Global TTY
// ---------------------------------------------------------------------------

/// The current active TTY.
static ACTIVE_TTY: Mutex<Tty> = Mutex::new(Tty::new(0));

/// Write to the current TTY.
pub fn write(data: &[u8]) {
    let mut tty = ACTIVE_TTY.lock();
    for &byte in data {
        tty.output(byte);
    }
}

/// Read a line from the current TTY.
pub fn read_line(buf: &mut [u8]) -> usize {
    let mut tty = ACTIVE_TTY.lock();
    tty.read_line(buf)
}

/// Set the foreground color.
pub fn set_color(color: crate::vga::Color) {
    let mut tty = ACTIVE_TTY.lock();
    tty.foreground_color = color;
    crate::vga::set_color(color);
}
