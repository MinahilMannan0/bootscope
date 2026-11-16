//! BootScope command shell.
//!
//! The shell collects typed characters into a line. When Enter is pressed it
//! splits the line into a command and its arguments, then runs the command.

use crate::console::{Color, Console};
use crate::fs::RamFs;
use crate::keyboard::Key;
use crate::sysinfo::{region_kind_name, SystemInfo};
use crate::task::{self, Scheduler};
use crate::{allocator, integrity, interrupts, rtc};
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

/// Everything a command may need besides the shell itself.
pub struct Kernel<'a, 'b> {
    pub console: &'a mut Console<'b>,
    pub info: &'a SystemInfo,
    pub scheduler: &'a mut Scheduler,
}

pub struct Shell {
    input: Line,
    history: [Line; HISTORY_SIZE],
    history_count: usize,
    fs: RamFs,
}

impl Shell {
    pub fn new(fs: RamFs) -> Self {
        Shell { input: Line::EMPTY, history: [Line::EMPTY; HISTORY_SIZE], history_count: 0, fs }
    }

    pub fn file_count(&self) -> usize {
        self.fs.files().len()
    }

    pub fn prompt(&self, console: &mut Console) {
        console.set_color(Color::ACCENT);
        let _ = write!(console, "BootScope> ");
        console.set_color(Color::TEXT);
    }

    /// React to one key press.
    pub fn handle_key(&mut self, key: Key, k: &mut Kernel) {
        match key {
            Key::Char(c) => {
                // Buffer limit: extra characters are ignored instead of overflowing.
                if c.is_ascii() && self.input.len < MAX_INPUT {
                    self.input.bytes[self.input.len] = c as u8;
                    self.input.len += 1;
                    let _ = write!(k.console, "{c}");
                }
            }
            Key::Backspace => {
                // Only erase what the user typed, never the prompt.
                if self.input.len > 0 {
                    self.input.len -= 1;
                    k.console.backspace();
                }
            }
            Key::Enter => {
                let _ = writeln!(k.console);
                let line = self.input; // copy the typed line
                self.input = Line::EMPTY; // and reset for the next command
                self.execute(line.as_str().trim(), k);
                self.prompt(k.console);
            }
        }
    }

    /// Split the line into command + arguments and run it.
    fn execute(&mut self, text: &str, k: &mut Kernel) {
        if text.is_empty() {
            return; // empty line: just show a new prompt
        }
        self.save_to_history(text);

        let (command, args) = match text.split_once(' ') {
            Some((command, args)) => (command, args.trim()),
            None => (text, ""),
        };
        let console = &mut *k.console;

        match command {
            // General
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
            // System
            "info" => self.cmd_info(k),
            "mem" => cmd_mem(console, k.info),
            "time" => cmd_time(console),
            "uptime" => cmd_uptime(console),
            "irq" => cmd_irq(console),
            "trap" => cmd_trap(console),
            "crash" => cmd_crash(console, args),
            "integrity" => cmd_integrity(console, k.info, args),
            // Files
            "ls" => self.cmd_ls(console),
            "cat" => self.cmd_cat(console, args),
            "touch" => {
                let result = self.fs.create(args);
                report(console, result, "File created");
            }
            "write" | "append" => self.cmd_write(console, args, command == "append"),
            "rm" => {
                let result = self.fs.remove(args);
                report(console, result, "File deleted");
            }
            // Tasks
            "ps" => cmd_ps(k),
            "spawn" => cmd_spawn(k, args),
            "kill" => cmd_kill(k, args),
            // Power
            "reboot" => {
                let _ = writeln!(console, "Rebooting...");
                crate::power::reboot();
            }
            "shutdown" => {
                let _ = writeln!(console, "Powering off...");
                crate::power::shutdown();
            }
            _ => error(console, format_args!("Unknown command: '{command}'. Type 'help' for a list.")),
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
        console.header("Command history");
        for (i, line) in self.history[..self.history_count].iter().enumerate() {
            let _ = writeln!(console, "  {:>2}  {}", i + 1, line.as_str());
        }
    }

    fn cmd_info(&self, k: &mut Kernel) {
        let i = k.info;
        let (major, minor, patch) = i.bootloader_api;
        let (heap_used, heap_free, heap_size) = allocator::stats();
        let console = &mut *k.console;
        console.header("System information");
        let _ = writeln!(console, "  OS             : {} v{}", crate::OS_NAME, crate::OS_VERSION);
        let _ = writeln!(console, "  Architecture   : x86_64 (64-bit long mode)");
        let _ = writeln!(console, "  Kernel target  : x86_64-unknown-none (bare metal, no_std)");
        let _ = writeln!(console, "  Boot method    : BIOS, bootloader crate (API v{major}.{minor}.{patch})");
        let _ = writeln!(console, "  Display        : {} x {} pixels, {}, {} bytes/pixel",
            i.screen_width, i.screen_height, i.pixel_format, i.bytes_per_pixel);
        let _ = writeln!(console, "  Kernel image   : {} KiB at physical address {:#x}",
            i.kernel_len / 1024, i.kernel_addr);
        let _ = writeln!(console, "  Usable RAM     : {} MiB", i.memory_totals().usable / (1024 * 1024));
        let _ = writeln!(console, "  Heap           : {} KiB used, {} KiB free of {} KiB",
            heap_used / 1024, heap_free / 1024, heap_size / 1024);
        let _ = writeln!(console, "  Tasks          : {} background + kernel + shell", k.scheduler.len());
        let _ = writeln!(console, "  Interrupts     : timer {} Hz ({} ticks), keyboard IRQ 1",
            interrupts::TIMER_HZ, interrupts::ticks());
        let b = i.boot_time;
        let _ = writeln!(console, "  Booted at      : {:04}-{:02}-{:02} {:02}:{:02}:{:02} (CMOS clock)",
            b.year, b.month, b.day, b.hour, b.minute, b.second);
        let _ = writeln!(console, "  File system    : RAM fs, {} files, {} bytes",
            self.fs.files().len(), self.fs.total_bytes());
        match i.rsdp_addr {
            Some(addr) => {
                let _ = writeln!(console, "  ACPI tables    : RSDP found at {addr:#x}");
            }
            None => {
                let _ = writeln!(console, "  ACPI tables    : not reported");
            }
        }
        let _ = writeln!(console, "  CPU model/cores: not detected (not implemented)");
    }

    fn cmd_ls(&self, console: &mut Console) {
        console.header("Files (RAM file system)");
        for file in self.fs.files() {
            console.set_color(Color::ACCENT);
            let _ = write!(console, "  {:<26}", file.name);
            console.set_color(Color::TEXT);
            let _ = writeln!(console, "{:>6} bytes", file.data.len());
        }
        console.set_color(Color::DIM);
        let _ = writeln!(console, "  {} files, {} bytes total (stored in RAM, lost on reboot)",
            self.fs.files().len(), self.fs.total_bytes());
        console.set_color(Color::TEXT);
    }

    fn cmd_cat(&self, console: &mut Console, name: &str) {
        match self.fs.read(name) {
            Ok(text) => {
                let _ = writeln!(console, "{text}");
            }
            Err(e) => error(console, format_args!("cat: {name}: {e}")),
        }
    }

    fn cmd_write(&mut self, console: &mut Console, args: &str, append: bool) {
        let Some((name, text)) = args.split_once(' ') else {
            let usage = if append { "append <file> <text>" } else { "write <file> <text>" };
            error(console, format_args!("Usage: {usage}"));
            return;
        };
        let result = self.fs.write(name, text.trim(), append);
        report(console, result, if append { "Text appended" } else { "File written" });
    }
}

fn cmd_help(console: &mut Console) {
    let groups: [(&str, &[(&str, &str)]); 5] = [
        ("General", &[
            ("help", "Show this list"),
            ("about", "About BootScope"),
            ("version", "Show the version"),
            ("echo <text>", "Print the text back"),
            ("clear", "Clear the screen"),
            ("history", "Show the last 10 commands"),
        ]),
        ("System", &[
            ("info", "System and boot information"),
            ("mem", "Memory map and heap usage"),
            ("time / uptime", "Clock time / time since boot"),
            ("integrity", "Kernel integrity check (educational demo)"),
            ("irq", "Interrupt statistics"),
            ("trap / crash yes", "Exception demos: breakpoint / page fault"),
        ]),
        ("Files", &[
            ("ls", "List files"),
            ("cat <file>", "Show a file"),
            ("touch <file>", "Create an empty file"),
            ("write <file> <text>", "Write text to a file (append <file> <text> adds)"),
            ("rm <file>", "Delete a file"),
        ]),
        ("Tasks", &[
            ("ps", "List running tasks"),
            ("spawn <task>", "Start a task: primes, clock, cursor"),
            ("kill <pid>", "Stop a task"),
        ]),
        ("Power", &[
            ("reboot", "Restart the computer"),
            ("shutdown", "Power off (QEMU only)"),
        ]),
    ];
    for (title, commands) in groups {
        console.header(title);
        for (name, description) in commands {
            console.set_color(Color::ACCENT);
            let _ = write!(console, "  {name:<22}");
            console.set_color(Color::TEXT);
            let _ = writeln!(console, "{description}");
        }
    }
}

fn cmd_about(console: &mut Console) {
    console.header("About");
    let _ = writeln!(console, "  {} v{}", crate::OS_NAME, crate::OS_VERSION);
    let _ = writeln!(console, "  A small educational operating system written in Rust.");
    let _ = writeln!(console, "  Boots in QEMU via the bootloader crate; runs bare metal (no_std).");
    let _ = writeln!(console, "  Operating Systems course project by: Minahil Mannan");
}

fn cmd_mem(console: &mut Console, info: &SystemInfo) {
    let regions = info.memory_regions;
    console.header("Physical memory map (from the bootloader)");
    console.set_color(Color::DIM);
    let _ = writeln!(console, "  Start               End                     Size  Type");
    console.set_color(Color::TEXT);
    for region in regions {
        let _ = write!(console, "  {:#018x}  {:#018x}  ", region.start, region.end);
        write_size(console, region.end - region.start);
        let _ = writeln!(console, "  {}", region_kind_name(region.kind));
    }
    let totals = info.memory_totals();
    let _ = write!(console, "  Total usable RAM : ");
    write_size(console, totals.usable);
    let _ = write!(console, "    Bootloader/kernel: ");
    write_size(console, totals.bootloader);
    let _ = write!(console, "    Reserved: ");
    write_size(console, totals.other);
    let _ = writeln!(console);

    let (used, free, size) = allocator::stats();
    console.header("Kernel heap (dynamic memory)");
    let _ = write!(console, "  Heap size: ");
    write_size(console, size as u64);
    let _ = write!(console, "    Allocated: ");
    write_size(console, used as u64);
    let _ = write!(console, "    Free: ");
    write_size(console, free as u64);
    let _ = writeln!(console);
}

fn cmd_time(console: &mut Console) {
    let t = rtc::now();
    let _ = writeln!(console, "{:04}-{:02}-{:02} {:02}:{:02}:{:02} (from the CMOS real-time clock)",
        t.year, t.month, t.day, t.hour, t.minute, t.second);
}

fn cmd_uptime(console: &mut Console) {
    let ticks = interrupts::ticks();
    let seconds = ticks / interrupts::TIMER_HZ;
    let hundredths = ticks % interrupts::TIMER_HZ;
    let _ = writeln!(console, "Uptime: {}h {}m {}.{:02}s", seconds / 3600, (seconds / 60) % 60,
        seconds % 60, hundredths);
    console.set_color(Color::DIM);
    let _ = writeln!(console, "  ({ticks} timer interrupts at {} Hz since the timer started)", interrupts::TIMER_HZ);
    console.set_color(Color::TEXT);
}

fn cmd_irq(console: &mut Console) {
    console.header("Interrupts");
    let _ = writeln!(console, "  IDT            : loaded (CPU exceptions + hardware interrupts)");
    let _ = writeln!(console, "  PIC            : 8259, IRQ 0-15 remapped to vectors 32-47");
    let _ = writeln!(console, "  Timer, IRQ 0   : {} interrupts, PIT at {} Hz", interrupts::ticks(), interrupts::TIMER_HZ);
    let _ = writeln!(console, "  Keyboard, IRQ 1: {} interrupts (one per key press and release)", interrupts::keyboard_irqs());
    let _ = writeln!(console, "  Breakpoints    : {} handled", interrupts::breakpoints());
}

/// Trigger a breakpoint exception on purpose; the handler counts it and returns.
fn cmd_trap(console: &mut Console) {
    let before = interrupts::breakpoints();
    x86_64::instructions::interrupts::int3();
    let after = interrupts::breakpoints();
    console_ok(console, format_args!(
        "Breakpoint exception (int3) handled by the IDT - kernel continued ({before} -> {after})"));
}

/// Read from an unmapped address on purpose: page fault -> panic screen.
fn cmd_crash(console: &mut Console, args: &str) {
    if args != "yes" {
        error(console, format_args!("This stops BootScope with a page fault. Type 'crash yes' to confirm."));
        return;
    }
    // SAFETY: not safe at all - deliberately touching unmapped memory to show
    // how the CPU reports a page fault. The handler stops the kernel.
    let value = unsafe { core::ptr::read_volatile(0xdead_beef_000 as *const u64) };
    let _ = writeln!(console, "unexpected: read {value}");
}

fn cmd_integrity(console: &mut Console, info: &SystemInfo, args: &str) {
    console.header("Kernel integrity check - EDUCATIONAL DEMO");
    console.set_color(Color::WARN);
    let _ = writeln!(console, "  Not Secure Boot and not a cryptographic signature.");
    console.set_color(Color::TEXT);

    let (Some(image), Some(boot_hash)) = (integrity::kernel_image(info), info.kernel_hash) else {
        let _ = writeln!(console, "  Kernel image is not available, so no check is possible.");
        return;
    };

    let simulate = args == "simulate";
    let current = if simulate {
        integrity::fnv1a_with_one_bit_flipped(image)
    } else {
        integrity::fnv1a(image)
    };

    let _ = writeln!(console, "  Kernel image   : {} bytes in memory", image.len());
    let _ = writeln!(console, "  Hash at boot   : {boot_hash:#018x}");
    let _ = writeln!(console, "  Hash now       : {current:#018x}{}",
        if simulate { "  (one bit flipped - SIMULATED)" } else { "" });

    if current == boot_hash {
        console.set_color(Color::OK);
        let _ = writeln!(console, "  Result: MATCH - kernel image unchanged since boot");
    } else {
        console.set_color(Color::ERROR);
        let _ = writeln!(console, "  Result: MISMATCH - kernel image differs from boot measurement");
    }
    console.set_color(Color::DIM);
    let _ = writeln!(console, "  Compare 'Hash at boot' with the known-good hash printed in PowerShell.");
    let _ = writeln!(console, "  Try 'integrity simulate' to see how one changed bit is detected.");
    console.set_color(Color::TEXT);
}

fn cmd_ps(k: &mut Kernel) {
    let summaries = k.scheduler.summaries();
    let console = &mut *k.console;
    console.header("Tasks (cooperative round-robin scheduler)");
    console.set_color(Color::DIM);
    let _ = writeln!(console, "  PID  NAME      TURNS        STATUS");
    console.set_color(Color::TEXT);
    let _ = writeln!(console, "    0  kernel    -            main loop: runs tasks, sleeps (hlt) until an interrupt");
    let _ = writeln!(console, "    1  shell     -            foreground: runs when a key interrupt arrives");
    for s in summaries {
        let _ = writeln!(console, "  {:>3}  {:<8}  {:<11}  {}", s.pid, s.name, s.runs, s.status);
    }
}

fn cmd_spawn(k: &mut Kernel, name: &str) {
    match task::create(name) {
        Some(new_task) => {
            let pid = k.scheduler.spawn(new_task);
            console_ok(k.console, format_args!("Started task '{name}' with PID {pid}"));
        }
        None => error(k.console, format_args!("spawn: unknown task '{name}' (try: primes, clock, cursor)")),
    }
}

fn cmd_kill(k: &mut Kernel, args: &str) {
    let Ok(pid) = args.parse::<u32>() else {
        error(k.console, format_args!("Usage: kill <pid>   (see 'ps' for PIDs)"));
        return;
    };
    if pid < 2 {
        error(k.console, format_args!("kill: PID {pid} is part of the kernel and cannot be stopped"));
        return;
    }
    match k.scheduler.kill(pid) {
        Some(name) => console_ok(k.console, format_args!("Stopped task '{name}' (PID {pid})")),
        None => error(k.console, format_args!("kill: no task with PID {pid}")),
    }
}

/// Print a success message, or the file-system error.
fn report(console: &mut Console, result: Result<(), crate::fs::FsError>, success: &str) {
    match result {
        Ok(()) => console_ok(console, format_args!("{success}")),
        Err(e) => error(console, format_args!("Error: {e}")),
    }
}

fn console_ok(console: &mut Console, message: core::fmt::Arguments) {
    console.set_color(Color::OK);
    let _ = writeln!(console, "{message}");
    console.set_color(Color::TEXT);
}

fn error(console: &mut Console, message: core::fmt::Arguments) {
    console.set_color(Color::ERROR);
    let _ = writeln!(console, "{message}");
    console.set_color(Color::TEXT);
}

/// Print a byte count as B, KiB or MiB, padded to a fixed width.
fn write_size(console: &mut Console, bytes: u64) {
    let _ = if bytes >= 1024 * 1024 {
        write!(console, "{:>6} MiB", bytes / (1024 * 1024))
    } else if bytes >= 1024 {
        write!(console, "{:>6} KiB", bytes / 1024)
    } else {
        write!(console, "{:>6} B  ", bytes)
    };
}
