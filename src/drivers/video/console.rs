use core::fmt;

use alloc::vec;
use alloc::vec::Vec;

use noto_sans_mono_bitmap;

use super::fb::FrameBuffer;

#[derive(Copy, Clone)]
struct ScreenChar {
    character: u8
}

pub struct Console {
    font_width: usize,
    font_height: usize,

    width: usize,
    height: usize,
    pos: usize,
    chars: Vec<ScreenChar>,
    fb: FrameBuffer
}

impl Console {
    pub fn new(fb: FrameBuffer) -> Self {
        let font_style = noto_sans_mono_bitmap::FontWeight::Regular;
        let font_height = noto_sans_mono_bitmap::RasterHeight::Size16;
        let font_width = noto_sans_mono_bitmap::get_raster_width(font_style, font_height);

        let width = fb.width / font_width;
        let height = fb.height / font_height.val();
        let chars = vec![ScreenChar { character: b' ' }; width * height];

        Console {
            pos: 0,
            font_height: font_height.val(),
            font_width,
            width,
            height,
            chars,
            fb
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                self.pos += self.width - (self.pos % self.width);
            },
            byte => {
                self.chars[self.pos] = ScreenChar {
                    character: byte,
                };
                self.draw_text(self.pos, self.pos + 1);
                self.pos += 1;
            }
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe),
            }

        }
    }
    pub fn draw_text(&mut self, start: usize, end: usize) {
        let font_style = noto_sans_mono_bitmap::FontWeight::Regular;
        let font_height = noto_sans_mono_bitmap::RasterHeight::Size16;

        let fb = &self.fb;
        for (i, c) in (start..end).zip(self.chars[start..end].iter()) {
            let glyph = noto_sans_mono_bitmap::get_raster(c.character as char, font_style, font_height).or(noto_sans_mono_bitmap::get_raster('?', font_style, font_height)).unwrap();

            let y = (i / self.width) * self.font_height;
            let x = (i % self.width) * self.font_width;

            for row in 0..self.font_height {
                for col in 0..self.font_width {
                    unsafe {
                        let index = ((y + row) * fb.stride + x + col) * fb.bpp / 8;
                        let glyph_pixel = glyph.raster()[row][col];
                        *fb.buffer.add(index) = glyph_pixel;
                        *fb.buffer.add(index + 1) = glyph_pixel;
                        *fb.buffer.add(index + 2) = glyph_pixel;
                    }
                }
            }
        }
    }
}

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}
