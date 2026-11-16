//! Framebuffer text console.
//! The framebuffer is memory where every few bytes are one pixel.
//! There is no "print" on bare metal, so we draw each letter pixel by pixel.

use bootloader_api::info::{FrameBufferInfo, PixelFormat};
use core::fmt;
use noto_sans_mono_bitmap::{get_raster, get_raster_width, FontWeight, RasterHeight};

const FONT_WEIGHT: FontWeight = FontWeight::Regular;
const FONT_SIZE: RasterHeight = RasterHeight::Size16;
const CHAR_WIDTH: usize = get_raster_width(FONT_WEIGHT, FONT_SIZE);
const CHAR_HEIGHT: usize = 16;
const LINE_HEIGHT: usize = CHAR_HEIGHT + 2;
const MARGIN: usize = 10;

#[derive(Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const BACKGROUND: Color = Color { r: 12, g: 18, b: 28 };
    pub const TEXT: Color = Color { r: 220, g: 224, b: 230 };
    pub const ACCENT: Color = Color { r: 80, g: 200, b: 230 };
    pub const OK: Color = Color { r: 90, g: 210, b: 120 };
    pub const DIM: Color = Color { r: 130, g: 140, b: 155 };
}

pub struct Console<'a> {
    buffer: &'a mut [u8],
    info: FrameBufferInfo,
    x: usize,
    y: usize,
    fg: Color,
}

impl<'a> Console<'a> {
    pub fn new(buffer: &'a mut [u8], info: FrameBufferInfo) -> Self {
        Console { buffer, info, x: MARGIN, y: MARGIN, fg: Color::TEXT }
    }

    pub fn set_color(&mut self, color: Color) {
        self.fg = color;
    }

    /// Paint the whole screen with the background color and go to the top.
    pub fn clear(&mut self) {
        for y in 0..self.info.height {
            for x in 0..self.info.width {
                self.put_pixel(x, y, Color::BACKGROUND);
            }
        }
        self.x = MARGIN;
        self.y = MARGIN;
    }

    fn newline(&mut self) {
        self.x = MARGIN;
        self.y += LINE_HEIGHT;
        if self.y + LINE_HEIGHT > self.info.height {
            self.scroll_up();
        }
    }

    /// Move everything up by one text line, like a terminal does.
    fn scroll_up(&mut self) {
        let row_bytes = self.info.stride * self.info.bytes_per_pixel;
        let shift = LINE_HEIGHT * row_bytes;
        let used = (self.info.height * row_bytes).min(self.buffer.len());
        if shift < used {
            self.buffer.copy_within(shift..used, 0);
        }
        for y in self.info.height.saturating_sub(LINE_HEIGHT)..self.info.height {
            for x in 0..self.info.width {
                self.put_pixel(x, y, Color::BACKGROUND);
            }
        }
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
                // Unknown characters become '?' instead of crashing.
                let glyph = get_raster(c, FONT_WEIGHT, FONT_SIZE)
                    .or_else(|| get_raster('?', FONT_WEIGHT, FONT_SIZE));
                if let Some(glyph) = glyph {
                    for (row, line) in glyph.raster().iter().enumerate() {
                        for (col, intensity) in line.iter().enumerate() {
                            let color = blend(self.fg, Color::BACKGROUND, *intensity);
                            self.put_pixel(self.x + col, self.y + row, color);
                        }
                    }
                }
                self.x += CHAR_WIDTH;
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
        for c in s.chars() {
            self.write_char(c);
        }
        Ok(())
    }
}