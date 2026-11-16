# BootScope - Progress

## Completed milestones

| Version | Milestone | Main OS concepts |
|---|---|---|
| 0.1 | First boot: kernel colours the screen | Bare-metal target, `no_std`, bootloader handoff |
| 0.1 | Framebuffer console, banner, boot messages | Direct hardware memory access, font rendering |
| 0.1 | PS/2 keyboard input (polling) | I/O ports, scancodes |
| 0.1 | Shell: `help`, `about`, `version`, `echo`, `clear`, `history` | Input buffer, parsing, command dispatch |
| 0.2 | `reboot`, `shutdown`, `info`, `mem`, `time`, `uptime`, `integrity` | Memory map, CMOS clock, power control, integrity measurement |
| 0.3 | Heap allocator | Dynamic memory allocation |
| 0.3 | RAM file system | Files, names, create/read/write/delete |
| 0.3 | Cooperative scheduler (`ps`, `spawn`, `kill`) | Tasks, round-robin scheduling |
| 0.3 | Status bar, larger font, boot progress, headings, cursor | User interface |
| 0.4 | Interrupts: IDT, PIC, PIT timer, keyboard IRQ | Interrupt handling, hardware timer, idle `hlt` |
| 0.4 | Exception handlers and panic screen (`trap`, `crash yes`) | CPU exceptions, fault reporting |

Mid-lab requirements (all met): builds, boots in QEMU, startup screen, console output, keyboard input,
shell prompt, at least four commands, OS concepts explained, modular source code, setup and testing docs.

## Known limitations

- **Multitasking is cooperative:** tasks run one step at a time and are never interrupted mid-step.
  There is no context switching, no separate address spaces and no user mode.
- **File system is RAM-only:** files are lost on reboot; no disk driver; no directories.
- **Memory:** the kernel uses the bootloader's page tables; it does not manage virtual memory itself.
- **Shutdown is QEMU-specific:** real hardware would need an ACPI driver.
- **Keyboard:** US layout only; arrow keys, Ctrl, Alt and function keys are ignored.
- **Double fault on stack overflow** has no separate stack (no TSS/IST set up), so it may reset the VM.
- **Integrity check is educational:** FNV-1a is not cryptographic and the check runs inside the kernel
  it is checking; real systems use Secure Boot with signatures verified by firmware.
- **Clock:** CMOS year is assumed to be 20xx.

## Possible future work

1. Preemptive multitasking: switch tasks from the timer interrupt (save/restore registers and stacks).
2. GDT with TSS and a separate double-fault stack.
3. Own page-table management (map/unmap pages, guard pages).
4. Disk driver (ATA) and a FAT file system.
5. Directories in the file system.
6. Command-line editing with arrow keys and history recall.
7. User-mode programs and system calls.
