# BootScope

**BootScope** is a small educational operating system written in Rust for my Operating Systems course.
It is a real kernel: it boots in the QEMU emulator, talks directly to the (emulated) hardware, and
provides an interactive command-line shell. It is not a simulation and not a normal Windows program.

Version: **0.4.0**

## Features

| Area | What BootScope does |
|---|---|
| Boot | Boots from a BIOS disk image via the `bootloader` crate; 11-step boot log with a progress bar |
| Console | Framebuffer text console: every letter is drawn pixel by pixel; scrolling, colours, headings |
| Status bar | Live date, time, uptime, task count and heap usage, redrawn every second |
| Interrupts | IDT with CPU-exception handlers; 8259 PIC remapped to vectors 32-47 |
| Timer | PIT programmed to 100 Hz (IRQ 0); the kernel sleeps with `hlt` when idle |
| Keyboard | PS/2 keyboard driven by IRQ 1; scancodes queued in a lock-free ring buffer |
| Memory | Reads the bootloader's physical memory map; 16 MiB kernel heap (`Vec`, `String`, `Box`) |
| File system | RAM file system: create, write, append, read, list, delete files (lost on reboot) |
| Multitasking | Cooperative round-robin scheduler with background tasks (`ps`, `spawn`, `kill`) |
| Clock | Reads date/time from the CMOS real-time clock chip |
| Security demo | Educational kernel integrity check (FNV-1a hash), clearly labelled as not Secure Boot |
| Errors | Unknown commands handled; CPU exceptions show a kernel panic screen |
| Power | `reboot` (keyboard-controller reset) and `shutdown` (QEMU power-off port) |

## Commands (25)

| Group | Commands |
|---|---|
| General | `help`, `about`, `version`, `echo <text>`, `clear`, `history` |
| System | `info`, `mem`, `time`, `uptime`, `integrity` (`integrity simulate`), `irq`, `trap`, `crash yes` |
| Files | `ls`, `cat <file>`, `touch <file>`, `write <file> <text>`, `append <file> <text>`, `rm <file>` |
| Tasks | `ps`, `spawn <primes/clock/cursor>`, `kill <pid>` |
| Power | `reboot`, `shutdown` |

## Requirements (Windows 10)

- **Rust** installed with `rustup` (with the Visual Studio C++ Build Tools it offers)
- **QEMU** for Windows, with `C:\Program Files\qemu` added to PATH
- **Git**
- Internet for the first build (downloads nightly Rust and the crates)

The file `rust-toolchain.toml` selects nightly Rust and the bare-metal target automatically.

## Build and run

```
git clone https://github.com/MinahilMannan0/bootscope.git
cd bootscope
cargo run
```

- Click inside the QEMU window to type. Press **Ctrl+Alt+G** to release the mouse.
- Full screen for presentations: `cargo run -- --fullscreen` (leave with **Ctrl+Alt+F**).

## Demo sequence

```
help
info
mem
write notes.txt Hello from BootScope
append notes.txt Second line
cat notes.txt
ls
spawn primes
ps
irq
uptime
integrity
integrity simulate
trap
crash yes
```

## Project structure

| Path | Responsibility |
|---|---|
| `kernel/src/main.rs` | Entry point, bootloader configuration, boot sequence, main loop |
| `kernel/src/console.rs` | Framebuffer text console (pixels, font, scrolling, cursor, headings) |
| `kernel/src/statusbar.rs` | Boot progress bar and live status bar |
| `kernel/src/interrupts.rs` | IDT, exception handlers, PIC, PIT timer, keyboard IRQ and scancode queue |
| `kernel/src/keyboard.rs` | Scancode-to-key translation (US layout, Shift, Enter, Backspace) |
| `kernel/src/shell.rs` | Input buffer, parsing, history and all commands |
| `kernel/src/allocator.rs` | Kernel heap set-up from the memory map |
| `kernel/src/fs.rs` | RAM file system |
| `kernel/src/task.rs` | Cooperative scheduler and background tasks |
| `kernel/src/sysinfo.rs` | System information collected at boot |
| `kernel/src/rtc.rs` | CMOS real-time clock |
| `kernel/src/integrity.rs` | Educational integrity check |
| `kernel/src/power.rs` | Reboot and shutdown |
| `kernel/src/port.rs` | x86 I/O port access (`in` / `out` instructions) |
| `kernel/src/panic_screen.rs` | Kernel panic screen |
| `build.rs` | Runs on Windows: builds the bootable disk image, computes the known-good kernel hash |
| `src/main.rs` | Runs on Windows: starts QEMU with the disk image |

## Documentation

- [ARCHITECTURE.md](ARCHITECTURE.md): boot flow, components, main loop, design decisions
- [TESTING.md](TESTING.md): test cases and results
- [PROGRESS.md](PROGRESS.md): milestones, limitations and future work
- [SIX_CS.md](SIX_CS.md): how the project addresses the six Cs

## Not implemented (outside the current scope)

Preemptive multitasking and context switching, separate processes and user mode, virtual memory
management (paging beyond what the bootloader sets up), disk drivers and on-disk file systems,
networking, USB, SMP (multiple CPU cores), real Secure Boot. See [PROGRESS.md](PROGRESS.md).
