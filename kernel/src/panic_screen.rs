//! Kernel panic screen.
//!
//! When the kernel hits an unrecoverable error (a Rust panic, or a fatal CPU
//! exception), we show a clear error screen instead of freezing silently.

use crate::console::{Color, Console};
use bootloader_api::info::FrameBufferInfo;
use core::fmt::Write;
use core::panic::PanicInfo;
use spin::Mutex;

struct Screen {
    address: usize,
    len: usize,
    info: FrameBufferInfo,
}

static SCREEN: Mutex<Option<Screen>> = Mutex::new(None);

/// Remember where the framebuffer is, so the panic handler can reach it.
pub fn register(buffer: &mut [u8], info: FrameBufferInfo) {
    *SCREEN.lock() = Some(Screen { address: buffer.as_mut_ptr() as usize, len: buffer.len(), info });
}

pub fn show(panic: &PanicInfo) {
    x86_64::instructions::interrupts::disable(); // nothing may interrupt us now
    let Some(guard) = SCREEN.try_lock() else { return };
    let Some(screen) = guard.as_ref() else { return };

    // SAFETY: the kernel is stopping; nothing else will draw to the screen
    // again, so taking over the framebuffer here is acceptable.
    let buffer = unsafe { core::slice::from_raw_parts_mut(screen.address as *mut u8, screen.len) };
    let mut console = Console::new(buffer, screen.info);
    console.clear_all();
    console.fill_rect(0, 0, console.width(), 6, Color::ERROR);

    console.set_color(Color::ERROR);
    let _ = writeln!(console, "\n  KERNEL PANIC - BootScope has stopped\n");
    console.set_color(Color::TEXT);
    let _ = writeln!(console, "  {}", panic.message());
    if let Some(location) = panic.location() {
        console.set_color(Color::DIM);
        let _ = writeln!(console, "\n  at {}:{}", location.file(), location.line());
    }
    console.set_color(Color::DIM);
    let _ = writeln!(console, "\n  The CPU has been halted. Close the QEMU window and run 'cargo run'");
    let _ = writeln!(console, "  to start BootScope again.");
}
