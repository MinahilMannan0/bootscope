# BootScope - The Six Cs

## 1. Communication

- **On screen:** an 11-step boot log where each `[ OK ]` line appears only after the step succeeded;
  a live status bar; clear red error messages with usage hints; a panic screen that explains what failed.
- **Commands explain themselves:** `help` groups 25 commands with one-line descriptions.
- **Documentation:** `README.md` (setup and use), `ARCHITECTURE.md` (boot flow, components,
  interrupts, memory, with diagrams), `TESTING.md` (test cases), `PROGRESS.md` (status and limits).
- **Honest labelling:** simulated or limited features are marked as such (e.g. "EDUCATIONAL DEMO",
  "QEMU only", "lost on reboot").

## 2. Creativity

- A distinctive BootScope identity: ASCII banner, consistent dark colour palette, cyan accents.
- Boot progress bar and step-by-step boot animation.
- Live status bar like a real desktop taskbar (time, uptime, tasks, heap).
- Interactive demos that make invisible OS concepts visible: `spawn primes` (multitasking),
  `irq` (interrupt counters), `trap` (handled exception), `crash yes` (panic screen),
  `integrity simulate` (tamper detection).

## 3. Coding

- Written in **Rust** for the bare-metal target `x86_64-unknown-none` with `#![no_std]`.
- 15 kernel modules with clear responsibilities (see ARCHITECTURE.md).
- Real low-level code: framebuffer drawing, I/O ports, interrupt descriptor table, PIC and timer
  programming, CMOS clock, keyboard scancodes, heap allocation, scheduler, file system.
- Error handling: `Option`/`Result` instead of crashes; input limits; unknown commands handled.
- `unsafe` limited to small, commented functions.
- Version control with Git: one commit per milestone.

## 4. Cyber Security

- **Memory safety:** Rust prevents buffer overflows and use-after-free in safe code. All hardware
  access that Rust cannot check is isolated in small `unsafe` blocks with `SAFETY:` comments.
- **Bounds checks:** the input buffer (64 characters), file sizes, file names and the keyboard queue
  all have fixed limits, so long input cannot overflow memory.
- **Boot integrity (educational demo):** the `integrity` command fingerprints the kernel image at boot
  with FNV-1a and compares it later; `build.rs` prints the known-good hash on Windows for comparison;
  `integrity simulate` shows that changing one bit changes the hash completely.
- **What real systems do instead:** Secure Boot verifies cryptographic signatures (e.g. SHA-256 + RSA/ECDSA)
  in the firmware **before** the bootloader and kernel run, and a TPM records measurements. BootScope's
  check runs inside the kernel it is checking and uses a non-cryptographic hash, so an attacker could
  bypass it. It demonstrates the idea only; **it is not Secure Boot**.
- **Fault isolation:** CPU exceptions (page fault, general protection fault, double fault) are caught
  and reported instead of failing silently.

## 5. Cloud

- Source code and full commit history are hosted on GitHub:
  https://github.com/MinahilMannan0/bootscope
- GitHub serves as off-site backup, version history from Milestone 1 to v0.4, and the place where
  the documentation is published (Markdown and diagrams render on GitHub).
- The kernel itself needs **no** network or cloud connection to boot or run.

## 6. Co-Intelligence

AI assistance (Claude, by Anthropic) was used throughout development:

| How AI was used | What I did |
|---|---|
| Choosing a toolchain that works on Windows 10 (Rust nightly, `bootloader` crate, QEMU) | Installed Rust, the Visual Studio build tools, QEMU and Git; verified versions |
| Step-by-step instructions and explanations of each concept | Created the project, typed in the first milestones (first boot, console, keyboard, shell) |
| Generating code for later versions (v0.2-v0.4) | Installed each version, built it, ran every test, reported errors back |
| Diagnosing errors (missing `.cargo/config.toml`, network error, deleted helper functions, slow boot) | Read the compiler output, applied the fixes, re-tested |
| Drafting documentation | Reviewed and corrected it to match what I tested |

**What I tested and understood personally:** *(complete this in your own words before submitting)*

- I can explain: ...
- I tested: ...
- The part I found hardest: ...

AI did not run or test the kernel; every build and test result in this project comes from my own
machine.
