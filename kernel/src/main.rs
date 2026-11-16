#![no_std]
#![no_main]

// `alloc` gives us Vec, String and Box once the heap is set up.
extern crate alloc;

mod allocator;
mod console;
mod fs;
mod integrity;
mod keyboard;
mod port;
mod power;
mod rtc;
mod shell;
mod statusbar;
mod sysinfo;
mod task;

use alloc::boxed::Box;
use bootloader_api::config::{BootloaderConfig, Mapping};
use bootloader_api::info::{MemoryRegion, PixelFormat};
use bootloader_api::{entry_point, BootInfo};
use console::{Color, Console};
use core::fmt::{self, Write};
use keyboard::Keyboard;
use shell::{Kernel, Shell};
use sysinfo::SystemInfo;
use task::{ClockTask, CursorTask, Scheduler};

const OS_NAME: &str = "BootScope";
const OS_VERSION: &str = env!("CARGO_PKG_VERSION"); // taken from kernel/Cargo.toml

const BANNER: &str = r#"
 ____              _   ____
| __ )  ___   ___ | |_/ ___|  ___ ___  _ __   ___
|  _ \ / _ \ / _ \| __\___ \ / __/ _ \| '_ \ / _ \
| |_) | (_) | (_) | |_ ___) | (_| (_) | |_) |  __/
|____/ \___/ \___/ \__|____/ \___\___/| .__/ \___|
                                      |_|
"#;

/// Number of boot steps, for the progress bar.
const BOOT_STEPS: usize = 10;
/// Short pause after each boot step, so people can read the boot log.
/// Increase it for a slower boot animation, decrease it for a faster one.
const BOOT_PAUSE: u64 = 1_000_000;

/// Settings for the bootloader. We ask it to map all physical memory into our
/// address space: the heap and the integrity check need to reach physical RAM.
pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

/// Prints boot messages and advances the progress bar.
struct BootLog {
    step: usize,
}

impl BootLog {
    fn ok(&mut self, console: &mut Console, message: fmt::Arguments) {
        self.line(console, Color::OK, " OK ", message);
    }

    fn fail(&mut self, console: &mut Console, message: fmt::Arguments) {
        self.line(console, Color::ERROR, "FAIL", message);
    }

    fn line(&mut self, console: &mut Console, color: Color, label: &str, message: fmt::Arguments) {
        console.set_color(Color::DIM);
        let _ = write!(console, "  [");
        console.set_color(color);
        let _ = write!(console, "{label}");
        console.set_color(Color::DIM);
        let _ = write!(console, "] ");
        console.set_color(Color::TEXT);
        let _ = writeln!(console, "{message}");
        self.step += 1;
        statusbar::draw_boot(console, self.step, BOOT_STEPS);
        pause();
    }
}

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    // Reaching this line means the bootloader handed control to us.

    // Copy the values we need from BootInfo before borrowing the framebuffer.
    let memory_regions: &'static [MemoryRegion] = &boot_info.memory_regions;
    let api = &boot_info.api_version;
    let bootloader_api = (api.version_major(), api.version_minor(), api.version_patch());
    let kernel_addr = boot_info.kernel_addr;
    let kernel_len = boot_info.kernel_len;
    let physical_memory_offset = boot_info.physical_memory_offset.as_ref().copied();
    let rsdp_addr = boot_info.rsdp_addr.as_ref().copied();

    // Without a framebuffer we have no screen, so we stop.
    let Some(framebuffer) = boot_info.framebuffer.as_mut() else {
        halt_forever();
    };
    let fb_info = framebuffer.info();
    let mut console = Console::new(framebuffer.buffer_mut(), fb_info);
    console.clear_all();
    statusbar::draw_boot(&mut console, 0, BOOT_STEPS);

    console.set_color(Color::ACCENT);
    let _ = write!(console, "{BANNER}");
    console.set_color(Color::DIM);
    let _ = writeln!(console, "  {OS_NAME} v{OS_VERSION} - educational operating system\n");
    pause();

    let mut log = BootLog { step: 0 };

    // 1-2: bootloader handoff and console
    log.ok(&mut console, format_args!("Bootloader handoff successful (bootloader API v{}.{}.{})",
        bootloader_api.0, bootloader_api.1, bootloader_api.2));
    log.ok(&mut console, format_args!("Console initialized ({} x {} framebuffer)",
        fb_info.width, fb_info.height));

    // 3: clock
    let boot_time = rtc::now();
    log.ok(&mut console, format_args!("Real-time clock read: {:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        boot_time.year, boot_time.month, boot_time.day,
        boot_time.hour, boot_time.minute, boot_time.second));

    let pixel_format = match fb_info.pixel_format {
        PixelFormat::Rgb => "RGB",
        PixelFormat::Bgr => "BGR",
        PixelFormat::U8 => "grayscale",
        _ => "unknown",
    };
    let mut info = SystemInfo {
        bootloader_api,
        screen_width: fb_info.width,
        screen_height: fb_info.height,
        pixel_format,
        bytes_per_pixel: fb_info.bytes_per_pixel,
        kernel_addr,
        kernel_len,
        physical_memory_offset,
        rsdp_addr,
        memory_regions,
        boot_time,
        kernel_hash: None,
    };

    // 4: memory map
    let totals = info.memory_totals();
    log.ok(&mut console, format_args!("Memory map received: {} regions, {} MiB usable RAM",
        memory_regions.len(), totals.usable / (1024 * 1024)));

    // 5: heap - everything after this may use Vec, String and Box.
    match allocator::init(&info) {
        Some((start, size)) => log.ok(&mut console, format_args!(
            "Heap allocator ready: {} MiB at physical address {:#x}", size / (1024 * 1024), start)),
        None => {
            log.fail(&mut console, format_args!("Heap could not be set up - system halted"));
            halt_forever();
        }
    }

    // 6: integrity measurement (educational demo)
    match integrity::kernel_image(&info) {
        Some(image) => {
            let hash = integrity::fnv1a(image);
            info.kernel_hash = Some(hash);
            log.ok(&mut console, format_args!("Kernel image measured: {:#018x} (educational demo)", hash));
        }
        None => log.fail(&mut console, format_args!("Kernel image could not be measured")),
    }

    // 7: file system
    let mut shell = Shell::new(fs::RamFs::new());
    log.ok(&mut console, format_args!("RAM file system mounted ({} file)", shell.file_count()));

    // 8: scheduler with two background tasks
    let mut scheduler = Scheduler::new();
    scheduler.spawn(Box::new(ClockTask::new()));
    scheduler.spawn(Box::new(CursorTask::new()));
    log.ok(&mut console, format_args!("Scheduler started: {} background tasks (cooperative round-robin)",
        scheduler.len()));

    // 9-10: keyboard and shell
    let mut keyboard = Keyboard::new();
    log.ok(&mut console, format_args!("Keyboard ready (PS/2, polling mode)"));
    log.ok(&mut console, format_args!("Command shell ready"));

    console.set_color(Color::ACCENT);
    let _ = writeln!(console, "\n  Welcome to BootScope! Type 'help' to see the commands.\n");
    shell.prompt(&mut console);

    // Main loop: check the keyboard, then give every background task a turn.
    loop {
        if let Some(key) = keyboard.poll_key() {
            let mut kernel = Kernel { console: &mut console, info: &info, scheduler: &mut scheduler };
            shell.handle_key(key, &mut kernel);
        }
        scheduler.run_once(&mut console, &info);
    }
}

/// Busy-wait for a moment (only used for the boot animation).
fn pause() {
    for _ in 0..BOOT_PAUSE {
        core::hint::spin_loop();
    }
}

/// Stop the CPU. `hlt` pauses the processor until an interrupt arrives.
fn halt_forever() -> ! {
    loop {
        // SAFETY: `hlt` only pauses the CPU and does not touch memory.
        unsafe { core::arch::asm!("hlt", options(nomem, nostack, preserves_flags)) };
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    halt_forever();
}
