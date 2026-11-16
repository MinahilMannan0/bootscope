//! The status bar at the top of the screen.
//!
//! During boot it shows a progress bar. After boot, the clock task redraws it
//! every second with live information: date, time, uptime, tasks and heap use.

use crate::allocator;
use crate::console::{Color, Console, CHAR_WIDTH, MARGIN, STATUS_BAR_HEIGHT};
use crate::rtc::Time;
use alloc::format;

const TITLE: &str = concat!("BootScope v", env!("CARGO_PKG_VERSION"));
const TEXT_Y: usize = 5;
const MIDDLE_X: usize = 260;

fn draw_background(console: &mut Console) {
    let width = console.width();
    console.fill_rect(0, 0, width, STATUS_BAR_HEIGHT, Color::PANEL);
    console.fill_rect(0, STATUS_BAR_HEIGHT - 2, width, 2, Color::ACCENT); // bottom line
    console.draw_text_at(MARGIN, TEXT_Y, TITLE, Color::ACCENT, Color::PANEL, true);
}

/// Boot progress: `done` of `total` steps finished. Uses no heap memory,
/// because it is drawn before the heap exists.
pub fn draw_boot(console: &mut Console, done: usize, total: usize) {
    draw_background(console);
    console.draw_text_at(MIDDLE_X, TEXT_Y, "Booting...", Color::TEXT, Color::PANEL, false);

    const BAR_WIDTH: usize = 320;
    let x = console.width() - MARGIN - BAR_WIDTH;
    console.fill_rect(x, 9, BAR_WIDTH, 12, Color::BACKGROUND);
    let filled = BAR_WIDTH * done.min(total) / total.max(1);
    console.fill_rect(x, 9, filled, 12, Color::OK);
}

/// Live status bar, redrawn once per second by the clock task.
pub fn draw_live(console: &mut Console, now: Time, up: u64, task_count: usize) {
    draw_background(console);

    let (heap_used, _, _) = allocator::stats();
    let middle = format!("tasks: {}   heap: {} KiB used", task_count, heap_used / 1024);
    console.draw_text_at(MIDDLE_X, TEXT_Y, &middle, Color::TEXT, Color::PANEL, false);

    let right = format!(
        "{:04}-{:02}-{:02}  {:02}:{:02}:{:02}   up {}:{:02}:{:02}",
        now.year, now.month, now.day, now.hour, now.minute, now.second,
        up / 3600, (up / 60) % 60, up % 60
    );
    let x = console.width().saturating_sub(MARGIN + right.len() * CHAR_WIDTH);
    console.draw_text_at(x, TEXT_Y, &right, Color::TEXT, Color::PANEL, false);
}
