//! BootScope command shell.
//!
//! The shell collects typed characters into a line. When Enter is pressed it
//! splits the line into a command and its arguments, then runs the command.
//! We have no heap (no dynamic memory) yet, so everything uses fixed-size arrays.

use crate::console::{Color, Console};
use crate::keyboard::Key;
use core::fmt::Write;

const MAX_INPUT: usize = 64; // longest command line we accept
const HISTORY_SIZE: usize = 10; // how many past commands we remember

/// One line of text stored in a fixed-size array.
#[derive(Clone, Copy)]
struct Line {
    bytes: [u8; MAX_INPUT],
    len: usize,
}

impl Line {
    const EMPTY: Line = Line { bytes: [0; MAX_INPUT], len: 0 };

    fn as_str(&self) -> &str {
        // Our keyboard only produces ASCII, so this cannot fail; "" is a safe fallback.
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}

pub struct Shell {
    input: Line,
    history: [Line; HISTORY_SIZE],
    history_count: usize,
}

impl Shell {
    pub fn new() -> Self {
        Shell { input: Line::EMPTY, history: [Line::EMPTY; HISTORY_SIZE], history_count: 0 }
    }

    pub fn prompt(&self, console: &mut Console) {
        console.set_color(Color::ACCENT);
        let _ = write!(console, "BootScope> ");
        console.set_color(Color::TEXT);
    }

    /// React to one key press.
    pub fn handle_key(&mut self, key: Key, console: &mut Console) {
        match key {
            Key::Char(c) => {
                // Buffer limit: extra characters are ignored instead of overflowing.
                if c.is_ascii() && self.input.len < MAX_INPUT {
                    self.input.bytes[self.input.len] = c as u8;
                    self.input.len += 1;
                    let _ = write!(console, "{c}");
                }
            }
            Key::Backspace => {
                // Only erase what the user typed, never the prompt.
                if self.input.len > 0 {
                    self.input.len -= 1;
                    console.backspace();
                }
            }
            Key::Enter => {
                let _ = writeln!(console);
                let line = self.input; // copy the typed line
                self.input = Line::EMPTY; // and reset for the next command
                self.execute(line.as_str().trim(), console);
                self.prompt(console);
            }
        }
    }

    /// Split the line into command + arguments and run it.
    fn execute(&mut self, text: &str, console: &mut Console) {
        if text.is_empty() {
            return; // empty line: just show a new prompt
        }
        self.save_to_history(text);

        let (command, args) = match text.split_once(' ') {
            Some((command, args)) => (command, args.trim()),
            None => (text, ""),
        };

        match command {
            "help" => cmd_help(console),
            "about" => cmd_about(console),
            "version" => {
                let _ = writeln!(console, "{} v{}", crate::OS_NAME, crate::OS_VERSION);
            }
            "echo" => {
                let _ = writeln!(console, "{args}");
            }
            "clear" => console.clear(),
            "history" => self.cmd_history(console),
            _ => {
                console.set_color(Color::ERROR);
                let _ = writeln!(console, "Unknown command: '{command}'. Type 'help' for a list.");
                console.set_color(Color::TEXT);
            }
        }
    }

    fn save_to_history(&mut self, text: &str) {
        let mut line = Line::EMPTY;
        line.bytes[..text.len()].copy_from_slice(text.as_bytes());
        line.len = text.len();

        if self.history_count < HISTORY_SIZE {
            self.history[self.history_count] = line;
            self.history_count += 1;
        } else {
            // Full: drop the oldest entry by shifting everything one place left.
            self.history.copy_within(1.., 0);
            self.history[HISTORY_SIZE - 1] = line;
        }
    }

    fn cmd_history(&self, console: &mut Console) {
        for (i, line) in self.history[..self.history_count].iter().enumerate() {
            let _ = writeln!(console, "  {:>2}  {}", i + 1, line.as_str());
        }
    }
}

fn cmd_help(console: &mut Console) {
    let commands = [
        ("help", "Show this list of commands"),
        ("about", "About BootScope"),
        ("version", "Show the BootScope version"),
        ("echo <text>", "Print the text back"),
        ("clear", "Clear the screen"),
        ("history", "Show the last 10 commands"),
    ];
    let _ = writeln!(console, "Available commands:");
    for (name, description) in commands {
        console.set_color(Color::ACCENT);
        let _ = write!(console, "  {name:<14}");
        console.set_color(Color::TEXT);
        let _ = writeln!(console, "{description}");
    }
}

fn cmd_about(console: &mut Console) {
    let _ = writeln!(console, "{} v{}", crate::OS_NAME, crate::OS_VERSION);
    let _ = writeln!(console, "  A small educational operating system written in Rust.");
    let _ = writeln!(console, "  Boots in QEMU via the bootloader crate; runs bare metal (no_std).");
    let _ = writeln!(console, "  Operating Systems course project by: Minahil Mannan");
}