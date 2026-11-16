//! Framebuffer text console.
//!
//! The framebuffer is memory where every few bytes are one pixel. There is no
//! "print" on bare metal, so we draw each letter pixel by pixel using a bitmap font.
//!
//! Screen layout:
//!   rows 0 .. STATUS_BAR_HEIGHT  -> status bar (drawn by the clock task)
//!   rows below that              -> scrolling text area

use bootloader_api::info::{FrameBufferInfo, PixelFormat};
use core::fmt;
use noto_sans_mono_bitmap::{get_raster, get_raster_width, FontWeight, RasterHeight};

const FONT_SIZE: RasterHeight = RasterHeight::Size20;
pub const CHAR_WIDTH: usize = get_raster_width(FontWeight::Regular, FONT_SIZE);
const CHAR_HEIGHT: usize = 20; // matches RasterHeight::Size20
const LINE_HEIGHT: usize = CHAR_HEIGHT + 2;
pub const MARGIN: usize = 14;
pub const STATUS_BAR_HEIGHT: usize = 30;
const TEXT_TOP: usize = STATUS_BAR_HEIGHT + 10;

/// A simple RGB color.
#[derive(Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const BACKGROUND: Color = Color { r: 12, g: 18, b: 28 };
    pub const PANEL: Color = Color { r: 26, g: 38, b: 58 };
    pub const TEXT: Color = Color { r: 220, g: 224, b: 230 };
    pub const ACCENT: Color = Color { r: 80, g: 200, b: 230 };
    pub const OK: Color = Color { r: 90, g: 210, b: 120 };
    pub const DIM: Color = Color { r: 130, g: 140, b: 155 };
    pub const ERROR: Color = Color { r: 235, g: 95, b: 95 };
    pub const WARN: Color = Color { r: 235, g: 190, b: 80 };
}

pub struct Console<'a> {
    buffer: &'a mut [u8],
    info: FrameBufferInfo,
    x: usize,
    y: usize,
    fg: Color,
    cursor_visible: bool,
}

impl<'a> Console<'a> {
    pub fn new(buffer: &'a mut [u8], info: FrameBufferInfo) -> Self {
        Console { buffer, info, x: MARGIN, y: TEXT_TOP, fg: Color::TEXT, cursor_visible: false }
    }

    pub fn width(&self) -> usize {
        self.info.width
    }

    pub fn set_color(&mut self, color: Color) {
        self.fg = color;
    }

    /// Paint the whole screen (status bar included) with the background.
    pub fn clear_all(&mut self) {
        self.fill_rect(0, 0, self.info.width, self.info.height, Color::BACKGROUND);
        self.x = MARGIN;
        self.y = TEXT_TOP;
        self.cursor_visible = false;
    }

    /// Clear only the text area; the status bar stays.
    pub fn clear(&mut self) {
        let h = self.info.height - STATUS_BAR_HEIGHT;
        self.fill_rect(0, STATUS_BAR_HEIGHT, self.info.width, h, Color::BACKGROUND);
        self.x = MARGIN;
        self.y = TEXT_TOP;
        self.cursor_visible = false;
    }

    /// Fill a rectangle with one color (clipped to the screen).
    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: Color) {
        for py in y..(y + h).min(self.info.height) {
            for px in x..(x + w).min(self.info.width) {
                self.put_pixel(px, py, color);
            }
        }
    }

    /// Draw text at a fixed pixel position without moving the text cursor.
    /// Used for the status bar and headings.
    pub fn draw_text_at(&mut self, x: usize, y: usize, text: &str, fg: Color, bg: Color, bold: bool) {
        let weight = if bold { FontWeight::Bold } else { FontWeight::Regular };
        let char_width = get_raster_width(weight, FONT_SIZE);
        let mut cx = x;
        for c in text.chars() {
            self.draw_glyph(cx, y, c, fg, bg, weight);
            cx += char_width;
        }
    }

    /// A coloured heading strip across the screen, like a panel title.
    pub fn header(&mut self, title: &str) {
        self.hide_cursor();
        if self.x != MARGIN {
            self.newline();
        }
        let y = self.y;
        let width = self.info.width - 2 * MARGIN;
        self.fill_rect(MARGIN, y, width, LINE_HEIGHT, Color::PANEL);
        self.fill_rect(MARGIN, y, 4, LINE_HEIGHT, Color::ACCENT); // accent edge
        self.draw_text_at(MARGIN + 12, y + 1, title, Color::ACCENT, Color::PANEL, true);
        self.newline();
    }

    /// Erase the last character on the current line.
    pub fn backspace(&mut self) {
        self.hide_cursor();
        if self.x >= MARGIN + CHAR_WIDTH {
            self.x -= CHAR_WIDTH;
            self.fill_rect(self.x, self.y, CHAR_WIDTH, LINE_HEIGHT, Color::BACKGROUND);
        }
    }

    /// Show or hide the blinking cursor (a small bar under the next character).
    pub fn set_cursor(&mut self, visible: bool) {
        if visible != self.cursor_visible {
            let color = if visible { Color::ACCENT } else { Color::BACKGROUND };
            self.fill_rect(self.x, self.y + CHAR_HEIGHT - 2, CHAR_WIDTH, 3, color);
            self.cursor_visible = visible;
        }
    }

    fn hide_cursor(&mut self) {
        self.set_cursor(false);
    }

    fn newline(&mut self) {
        self.x = MARGIN;
        self.y += LINE_HEIGHT;
        if self.y + LINE_HEIGHT > self.info.height {
            self.scroll_up();
        }
    }

    /// Move the text area up by one line (the status bar does not move).
    fn scroll_up(&mut self) {
        let row_bytes = self.info.stride * self.info.bytes_per_pixel;
        let start = TEXT_TOP * row_bytes;
        let shift = LINE_HEIGHT * row_bytes;
        let end = (self.info.height * row_bytes).min(self.buffer.len());
        if start + shift < end {
            self.buffer.copy_within(start + shift..end, start);
        }
        let bottom = self.info.height.saturating_sub(LINE_HEIGHT);
        self.fill_rect(0, bottom, self.info.width, LINE_HEIGHT, Color::BACKGROUND);
        self.y -= LINE_HEIGHT;
    }

    fn write_char(&mut self, c: char) {
        match c {
            '\n' => self.newline(),
            '\r' => self.x = MARGIN,
            c => {
                if self.x + CHAR_WIDTH + MARGIN > self.info.width {
                    self.newline(); // wrap long lines
                }
                self.draw_glyph(self.x, self.y, c, self.fg, Color::BACKGROUND, FontWeight::Regular);
                self.x += CHAR_WIDTH;
            }
        }
    }

    /// Draw one letter. Unknown characters become '?' instead of crashing.
    fn draw_glyph(&mut self, x: usize, y: usize, c: char, fg: Color, bg: Color, weight: FontWeight) {
        let glyph = get_raster(c, weight, FONT_SIZE).or_else(|| get_raster('?', weight, FONT_SIZE));
        if let Some(glyph) = glyph {
            for (row, line) in glyph.raster().iter().enumerate() {
                for (col, intensity) in line.iter().enumerate() {
                    self.put_pixel(x + col, y + row, blend(fg, bg, *intensity));
                }
            }
        }
    }

    /// Write one pixel. Screens store colors as RGB or BGR, so we check.
    fn put_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x >= self.info.width || y >= self.info.height {
            return; // never write outside the screen
        }
        let bpp = self.info.bytes_per_pixel;
        let offset = (y * self.info.stride + x) * bpp;
        let bytes: [u8; 4] = match self.info.pixel_format {
            PixelFormat::Rgb => [color.r, color.g, color.b, 0],
            PixelFormat::Bgr => [color.b, color.g, color.r, 0],
            PixelFormat::U8 => {
                let gray = ((color.r as u16 + color.g as u16 + color.b as u16) / 3) as u8;
                [gray, 0, 0, 0]
            }
            _ => [color.r, color.g, color.b, 0],
        };
        let n = bpp.min(4);
        if let Some(dst) = self.buffer.get_mut(offset..offset + n) {
            dst.copy_from_slice(&bytes[..n]);
        }
    }
}

/// Mix text color and background for smooth letter edges.
fn blend(fg: Color, bg: Color, intensity: u8) -> Color {
    let mix = |f: u8, b: u8| -> u8 {
        ((f as u16 * intensity as u16 + b as u16 * (255 - intensity as u16)) / 255) as u8
    };
    Color { r: mix(fg.r, bg.r), g: mix(fg.g, bg.g), b: mix(fg.b, bg.b) }
}

/// Lets us use write! and writeln! on the console.
impl fmt::Write for Console<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.hide_cursor();
        for c in s.chars() {
            self.write_char(c);
        }
        Ok(())
    }
}
