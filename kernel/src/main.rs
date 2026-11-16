#![no_std]
#![no_main]

mod console;
mod keyboard;
mod shell;

use bootloader_api::info::PixelFormat;
use bootloader_api::{entry_point, BootInfo};
use console::{Color, Console};
use core::fmt::Write;
use keyboard::Keyboard;
use shell::Shell;

const OS_NAME: &str = "BootScope";
const OS_VERSION: &str = "0.1.0";

const BANNER: &str = r#"
 ____              _   ____
| __ )  ___   ___ | |_/ ___|  ___ ___  _ __   ___
|  _ \ / _ \ / _ \| __\___ \ / __/ _ \| '_ \ / _ \
| |_) | (_) | (_) | |_ ___) | (_| (_) | |_) |  __/
|____/ \___/ \___/ \__|____/ \___\___/| .__/ \___|
                                      |_|
"#;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    // Reaching this line means the bootloader handed control to us.

    // Without a framebuffer we have no screen, so we stop.
    let Some(framebuffer) = boot_info.framebuffer.as_mut() else {
        halt_forever();
    };
    let info = framebuffer.info();
    let mut console = Console::new(framebuffer.buffer_mut(), info);
    console.clear();

    console.set_color(Color::ACCENT);
    let _ = write!(console, "{BANNER}");
    console.set_color(Color::DIM);
    let _ = writeln!(console, "  {OS_NAME} v{OS_VERSION} - educational operating system\n");

    ok(&mut console, "BootScope boot sequence started");
    ok(&mut console, "Bootloader handoff successful");
    ok(&mut console, "Kernel initialization started");
    ok(&mut console, "Console initialized (framebuffer)");

    // Real values from the bootloader - nothing invented.
    let format = match info.pixel_format {
        PixelFormat::Rgb => "RGB",
        PixelFormat::Bgr => "BGR",
        PixelFormat::U8 => "grayscale",
        _ => "unknown",
    };
    console.set_color(Color::TEXT);
    let _ = writeln!(console, "\n  Display : {} x {} pixels, {}, {} bytes/pixel",
        info.width, info.height, format, info.bytes_per_pixel);
    let _ = writeln!(console, "  Target  : x86_64, bare metal (no_std)\n");

    ok(&mut console, "Keyboard ready (PS/2, polling mode)");

    let mut keyboard = Keyboard::new();
    let mut shell = Shell::new();
    ok(&mut console, "Command shell ready");

    console.set_color(Color::ACCENT);
    let _ = writeln!(console, "\n  Welcome to BootScope! Type 'help' to see the commands.\n");
    shell.prompt(&mut console);

    // Main loop: wait for a key, give it to the shell, repeat forever.
    loop {
        let key = keyboard.read_key();
        shell.handle_key(key, &mut console);
    }
}

/// Print a green "[ OK ]" status line.
fn ok(console: &mut Console, message: &str) {
    console.set_color(Color::OK);
    let _ = write!(console, "  [ OK ] ");
    console.set_color(Color::TEXT);
    let _ = writeln!(console, "{message}");
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