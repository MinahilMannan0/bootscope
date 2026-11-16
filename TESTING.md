# BootScope - Testing

All tests are run manually in QEMU on Windows 10 with `cargo run`.
Before submission, run every test again from a clean build and tick the last column.

**Clean build:** `cargo clean` then `cargo run`.

## Build and boot

| # | Test | Expected result | Result |
|---|---|---|---|
| B1 | `cargo run` after `cargo clean` | Builds without errors, QEMU opens | ☐ |
| B2 | Boot sequence | Banner, progress bar fills, 11 `[ OK ]` lines, `BootScope>` prompt | ☐ |
| B3 | PowerShell output | Disk image path, known-good kernel hash and kernel size printed | ☐ |
| B4 | Status bar | Date, time and uptime update every second | ☐ |
| B5 | Cursor | Blinks after the prompt | ☐ |

## Keyboard and shell

| # | Input | Expected result | Result |
|---|---|---|---|
| K1 | Type `hello` | Letters appear as typed | ☐ |
| K2 | Shift + letters | Capital letters | ☐ |
| K3 | Backspace | Last character removed | ☐ |
| K4 | Backspace on empty line | Prompt is not erased | ☐ |
| K5 | Enter on empty line | New prompt, no error | ☐ |
| K6 | Hold a key (more than 64 characters) | Input stops at 64 characters, no crash | ☐ |
| K7 | `abcxyz` | Red "Unknown command" message, new prompt | ☐ |
| K8 | `help` | 5 groups of commands with headings | ☐ |
| K9 | `about`, `version` | Name, version 0.4.0, description | ☐ |
| K10 | `echo hello world` | `hello world` | ☐ |
| K11 | `history` | Numbered list of recent commands (max 10) | ☐ |
| K12 | `clear` | Text area cleared, status bar stays | ☐ |

## System information

| # | Input | Expected result | Result |
|---|---|---|---|
| S1 | `info` | Architecture, boot method, display, kernel image, RAM, heap, tasks, interrupts, boot time | ☐ |
| S2 | `mem` | Memory region table, totals, heap size/allocated/free | ☐ |
| S3 | `time` | Current local date and time | ☐ |
| S4 | `uptime` | Time since boot with hundredths of a second | ☐ |
| S5 | `irq` | Timer count grows about 100 per second; keyboard count grows with key presses | ☐ |

## File system

| # | Input | Expected result | Result |
|---|---|---|---|
| F1 | `ls` | `readme.txt` listed | ☐ |
| F2 | `write notes.txt Hello` then `cat notes.txt` | `Hello` | ☐ |
| F3 | `append notes.txt World` then `cat notes.txt` | Two lines | ☐ |
| F4 | `touch empty.txt` then `ls` | `empty.txt` with 0 bytes | ☐ |
| F5 | `touch empty.txt` again | Error: file already exists | ☐ |
| F6 | `cat missing.txt` | Error: file not found | ☐ |
| F7 | `rm notes.txt` then `ls` | File gone | ☐ |
| F8 | `touch bad/name` | Error: invalid name | ☐ |
| F9 | `write notes.txt` (no text) | Usage message | ☐ |
| F10 | `reboot` then `ls` | Only `readme.txt` (RAM file system is cleared) | ☐ |

## Tasks

| # | Input | Expected result | Result |
|---|---|---|---|
| T1 | `ps` | kernel (0), shell (1), clock, cursor | ☐ |
| T2 | `spawn primes`, then `ps` twice | Prime count increases; shell stays responsive | ☐ |
| T3 | `kill <pid of primes>` | Task stopped, gone from `ps` | ☐ |
| T4 | `kill <pid of clock>` | Status bar stops updating | ☐ |
| T5 | `spawn clock` | Status bar updates again | ☐ |
| T6 | `kill 0`, `kill 99`, `kill abc` | Error messages, no crash | ☐ |
| T7 | `spawn xyz` | Error: unknown task | ☐ |

## Interrupts and exceptions

| # | Input | Expected result | Result |
|---|---|---|---|
| I1 | `trap` | Breakpoint handled, kernel continues, count increases | ☐ |
| I2 | `crash` (without `yes`) | Warning only, nothing happens | ☐ |
| I3 | `crash yes` | Red KERNEL PANIC screen with PAGE FAULT and address | ☐ |

## Security demo

| # | Input | Expected result | Result |
|---|---|---|---|
| X1 | `integrity` | MATCH | ☐ |
| X2 | Compare "Hash at boot" with PowerShell "Known-good kernel hash" | Same value | ☐ |
| X3 | `integrity simulate` | MISMATCH, completely different hash | ☐ |

## Power

| # | Input | Expected result | Result |
|---|---|---|---|
| P1 | `reboot` | BootScope boots again | ☐ |
| P2 | `shutdown` | QEMU window closes | ☐ |

## Known issues found during testing

| Issue | Status |
|---|---|
| Boot animation was very slow with a busy-wait pause | Fixed in v0.4: pause uses timer ticks |
| Backspace cannot move back across a wrapped line | Known limitation (input is limited to 64 characters) |
