//! PS/2 keyboard input using polling.
//!
//! The keyboard controller has two I/O ports:
//!   0x64 = status port: bit 0 is 1 when a byte is waiting for us
//!   0x60 = data port:   the byte itself (a "scancode")
//! A scancode is a number for a physical key, not a letter. Pressing a key
//! sends one code; releasing it sends the same code + 0x80.
//! "Polling" means we keep asking the status port until a byte arrives.

use core::arch::asm;

const DATA_PORT: u16 = 0x60;
const STATUS_PORT: u16 = 0x64;

/// A key press the shell cares about.
pub enum Key {
    Char(char),
    Enter,
    Backspace,
}

pub struct Keyboard {
    shift: bool, // is a Shift key currently held down?
}

impl Keyboard {
    pub fn new() -> Self {
        Keyboard { shift: false }
    }

    /// Wait until a useful key is pressed, then return it.
    pub fn read_key(&mut self) -> Key {
        loop {
            let scancode = wait_for_scancode();
            if let Some(key) = self.decode(scancode) {
                return key;
            }
        }
    }

    /// Turn a scancode into a Key. Returns None for keys we ignore
    /// (key releases, Ctrl, arrows, F-keys, ...).
    fn decode(&mut self, scancode: u8) -> Option<Key> {
        match scancode {
            0x2A | 0x36 => {
                self.shift = true; // left or right Shift pressed
                return None;
            }
            0xAA | 0xB6 => {
                self.shift = false; // left or right Shift released
                return None;
            }
            0x1C => return Some(Key::Enter),
            0x0E => return Some(Key::Backspace),
            _ => {}
        }
        if scancode & 0x80 != 0 {
            return None; // any other key being released
        }
        let (normal, shifted) = scancode_to_chars(scancode)?;
        Some(Key::Char(if self.shift { shifted } else { normal }))
    }
}

/// Scancode set 1 (US layout): (character without Shift, character with Shift).
fn scancode_to_chars(scancode: u8) -> Option<(char, char)> {
    let pair = match scancode {
        0x02 => ('1', '!'), 0x03 => ('2', '@'), 0x04 => ('3', '#'), 0x05 => ('4', '$'),
        0x06 => ('5', '%'), 0x07 => ('6', '^'), 0x08 => ('7', '&'), 0x09 => ('8', '*'),
        0x0A => ('9', '('), 0x0B => ('0', ')'), 0x0C => ('-', '_'), 0x0D => ('=', '+'),
        0x10 => ('q', 'Q'), 0x11 => ('w', 'W'), 0x12 => ('e', 'E'), 0x13 => ('r', 'R'),
        0x14 => ('t', 'T'), 0x15 => ('y', 'Y'), 0x16 => ('u', 'U'), 0x17 => ('i', 'I'),
        0x18 => ('o', 'O'), 0x19 => ('p', 'P'), 0x1A => ('[', '{'), 0x1B => (']', '}'),
        0x1E => ('a', 'A'), 0x1F => ('s', 'S'), 0x20 => ('d', 'D'), 0x21 => ('f', 'F'),
        0x22 => ('g', 'G'), 0x23 => ('h', 'H'), 0x24 => ('j', 'J'), 0x25 => ('k', 'K'),
        0x26 => ('l', 'L'), 0x27 => (';', ':'), 0x28 => ('\'', '"'), 0x29 => ('`', '~'),
        0x2B => ('\\', '|'),
        0x2C => ('z', 'Z'), 0x2D => ('x', 'X'), 0x2E => ('c', 'C'), 0x2F => ('v', 'V'),
        0x30 => ('b', 'B'), 0x31 => ('n', 'N'), 0x32 => ('m', 'M'), 0x33 => (',', '<'),
        0x34 => ('.', '>'), 0x35 => ('/', '?'),
        0x39 => (' ', ' '),
        _ => return None,
    };
    Some(pair)
}

/// Keep checking the status port until the keyboard has a byte for us.
fn wait_for_scancode() -> u8 {
    loop {
        // SAFETY: ports 0x60/0x64 belong to the PS/2 controller; reading them
        // has no effect on memory.
        let status = unsafe { read_port(STATUS_PORT) };
        if status & 0x01 != 0 {
            return unsafe { read_port(DATA_PORT) };
        }
        core::hint::spin_loop(); // tell the CPU we are busy-waiting
    }
}

/// Read one byte from an I/O port with the x86 `in` instruction.
/// Unsafe because talking to the wrong port could confuse hardware.
unsafe fn read_port(port: u16) -> u8 {
    let value: u8;
    asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack, preserves_flags));
    value
}