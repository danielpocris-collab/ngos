use core::fmt;
use core::ptr;

use platform_x86_64::FramebufferInfo;

const GLYPH_WIDTH: usize = 5;
const GLYPH_HEIGHT: usize = 7;
const CELL_WIDTH: usize = GLYPH_WIDTH + 1;
const CELL_HEIGHT: usize = GLYPH_HEIGHT + 1;
const PADDING_X: usize = 8;
const PADDING_Y: usize = 8;

const DEFAULT_FOREGROUND: (u8, u8, u8) = (0xf2, 0xf4, 0xf8);
const DEFAULT_BACKGROUND: (u8, u8, u8) = (0x08, 0x0b, 0x12);
const STDERR_FOREGROUND: (u8, u8, u8) = (0xff, 0xc8, 0xa2);
const ALERT_FOREGROUND: (u8, u8, u8) = (0xff, 0xf1, 0xf1);
const ALERT_BACKGROUND: (u8, u8, u8) = (0x38, 0x07, 0x10);

static mut FRAMEBUFFER_CONSOLE: Option<FramebufferConsole> = None;

pub fn init(framebuffer: FramebufferInfo, physical_memory_offset: u64) {
    let Some(virtual_start) = framebuffer
        .physical_start
        .checked_add(physical_memory_offset)
    else {
        return;
    };
    let Some(console) = FramebufferConsole::new(virtual_start as *mut u8, framebuffer) else {
        return;
    };
    unsafe {
        FRAMEBUFFER_CONSOLE = Some(console);
        if let Some(console) = FRAMEBUFFER_CONSOLE.as_mut() {
            console.clear();
        }
    }
}

pub fn is_available() -> bool {
    unsafe { FRAMEBUFFER_CONSOLE.is_some() }
}

pub fn print(args: fmt::Arguments<'_>) {
    unsafe {
        if let Some(console) = FRAMEBUFFER_CONSOLE.as_mut() {
            let _ = fmt::write(console, args);
        }
    }
}

pub fn write_bytes(bytes: &[u8]) {
    unsafe {
        if let Some(console) = FRAMEBUFFER_CONSOLE.as_mut() {
            console.write_bytes(bytes);
        }
    }
}

pub fn write_stderr_bytes(bytes: &[u8]) {
    unsafe {
        if let Some(console) = FRAMEBUFFER_CONSOLE.as_mut() {
            console.write_bytes_colored(bytes, STDERR_FOREGROUND, console.background);
        }
    }
}

pub fn alert_banner(title: &str) {
    unsafe {
        if let Some(console) = FRAMEBUFFER_CONSOLE.as_mut() {
            console.clear_with_palette(ALERT_FOREGROUND, ALERT_BACKGROUND);
            let _ = console.write_colored(title, ALERT_FOREGROUND, ALERT_BACKGROUND);
            let _ = console.write_colored("\n\n", ALERT_FOREGROUND, ALERT_BACKGROUND);
            console.set_palette(DEFAULT_FOREGROUND, ALERT_BACKGROUND);
        }
    }
}

pub fn status_banner(title: &str) {
    unsafe {
        if let Some(console) = FRAMEBUFFER_CONSOLE.as_mut() {
            console.clear_with_palette(DEFAULT_FOREGROUND, DEFAULT_BACKGROUND);
            let _ = console.write_colored(title, DEFAULT_FOREGROUND, DEFAULT_BACKGROUND);
            let _ = console.write_colored("\n\n", DEFAULT_FOREGROUND, DEFAULT_BACKGROUND);
        }
    }
}

struct FramebufferConsole {
    base: *mut u8,
    width: usize,
    height: usize,
    pitch: usize,
    bytes_per_pixel: usize,
    cols: usize,
    rows: usize,
    cursor_col: usize,
    cursor_row: usize,
    red_mask_size: u8,
    red_mask_shift: u8,
    green_mask_size: u8,
    green_mask_shift: u8,
    blue_mask_size: u8,
    blue_mask_shift: u8,
    foreground: (u8, u8, u8),
    background: (u8, u8, u8),
}

impl FramebufferConsole {
    fn new(base: *mut u8, framebuffer: FramebufferInfo) -> Option<Self> {
        let width = framebuffer.width as usize;
        let height = framebuffer.height as usize;
        let pitch = framebuffer.pitch as usize;
        let bytes_per_pixel = usize::from(framebuffer.bpp).checked_div(8)?;
        if width == 0 || height == 0 || pitch == 0 || bytes_per_pixel == 0 {
            return None;
        }
        if bytes_per_pixel > 4 || pitch < width.saturating_mul(bytes_per_pixel) {
            return None;
        }
        let cols = width.saturating_sub(PADDING_X * 2) / CELL_WIDTH;
        let rows = height.saturating_sub(PADDING_Y * 2) / CELL_HEIGHT;
        if cols == 0 || rows == 0 {
            return None;
        }
        Some(Self {
            base,
            width,
            height,
            pitch,
            bytes_per_pixel,
            cols,
            rows,
            cursor_col: 0,
            cursor_row: 0,
            red_mask_size: framebuffer.red_mask_size,
            red_mask_shift: framebuffer.red_mask_shift,
            green_mask_size: framebuffer.green_mask_size,
            green_mask_shift: framebuffer.green_mask_shift,
            blue_mask_size: framebuffer.blue_mask_size,
            blue_mask_shift: framebuffer.blue_mask_shift,
            foreground: DEFAULT_FOREGROUND,
            background: DEFAULT_BACKGROUND,
        })
    }

    fn clear(&mut self) {
        self.clear_with_palette(self.foreground, self.background);
    }

    fn clear_with_palette(&mut self, foreground: (u8, u8, u8), background: (u8, u8, u8)) {
        self.foreground = foreground;
        self.background = background;
        let pixel = self.encode_color(background);
        for y in 0..self.height {
            let row = unsafe { self.base.add(y * self.pitch) };
            for x in 0..self.width {
                unsafe { self.write_pixel(row, x, pixel) };
            }
        }
        self.cursor_col = 0;
        self.cursor_row = 0;
    }

    fn set_palette(&mut self, foreground: (u8, u8, u8), background: (u8, u8, u8)) {
        self.foreground = foreground;
        self.background = background;
    }

    fn put_byte(&mut self, byte: u8) {
        self.put_byte_colored(byte, self.foreground, self.background);
    }

    fn put_byte_colored(&mut self, byte: u8, foreground: (u8, u8, u8), background: (u8, u8, u8)) {
        match byte {
            b'\n' => self.new_line(),
            b'\r' => self.cursor_col = 0,
            b'\t' => {
                let spaces = 4 - (self.cursor_col % 4);
                for _ in 0..spaces {
                    self.put_byte_colored(b' ', foreground, background);
                }
            }
            0x20..=0x7e => {
                self.draw_glyph(byte as char, foreground, background);
                self.cursor_col += 1;
                if self.cursor_col >= self.cols {
                    self.new_line();
                }
            }
            _ => {
                self.draw_glyph('?', foreground, background);
                self.cursor_col += 1;
                if self.cursor_col >= self.cols {
                    self.new_line();
                }
            }
        }
    }

    fn new_line(&mut self) {
        self.cursor_col = 0;
        self.cursor_row += 1;
        if self.cursor_row >= self.rows {
            self.scroll_up();
            self.cursor_row = self.rows - 1;
        }
    }

    fn scroll_up(&mut self) {
        let scroll_bytes = self.pitch * CELL_HEIGHT;
        let visible_bytes = self.pitch * self.height;
        unsafe {
            ptr::copy(
                self.base.add(scroll_bytes),
                self.base,
                visible_bytes.saturating_sub(scroll_bytes),
            );
        }
        let background = self.encode_color(self.background);
        for y in self.height.saturating_sub(CELL_HEIGHT)..self.height {
            let row = unsafe { self.base.add(y * self.pitch) };
            for x in 0..self.width {
                unsafe { self.write_pixel(row, x, background) };
            }
        }
    }

    fn draw_glyph(&mut self, ch: char, foreground_rgb: (u8, u8, u8), background_rgb: (u8, u8, u8)) {
        let glyph = glyph_rows(ch);
        let base_x = PADDING_X + self.cursor_col * CELL_WIDTH;
        let base_y = PADDING_Y + self.cursor_row * CELL_HEIGHT;
        let foreground = self.encode_color(foreground_rgb);
        let background = self.encode_color(background_rgb);
        for y in 0..CELL_HEIGHT {
            let row = unsafe { self.base.add((base_y + y) * self.pitch) };
            for x in 0..CELL_WIDTH {
                let on = if y < GLYPH_HEIGHT && x < GLYPH_WIDTH {
                    let bit = 1 << (GLYPH_WIDTH - 1 - x);
                    (glyph[y] & bit) != 0
                } else {
                    false
                };
                unsafe {
                    self.write_pixel(row, base_x + x, if on { foreground } else { background });
                }
            }
        }
    }

    fn write_colored(
        &mut self,
        text: &str,
        foreground: (u8, u8, u8),
        background: (u8, u8, u8),
    ) -> fmt::Result {
        for byte in text.bytes() {
            self.put_byte_colored(byte, foreground, background);
        }
        Ok(())
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.put_byte(byte);
        }
    }

    fn write_bytes_colored(
        &mut self,
        bytes: &[u8],
        foreground: (u8, u8, u8),
        background: (u8, u8, u8),
    ) {
        for &byte in bytes {
            self.put_byte_colored(byte, foreground, background);
        }
    }

    fn encode_color(&self, (red, green, blue): (u8, u8, u8)) -> u32 {
        let mut pixel = 0u32;
        pixel |= pack_channel(red, self.red_mask_size, self.red_mask_shift);
        pixel |= pack_channel(green, self.green_mask_size, self.green_mask_shift);
        pixel |= pack_channel(blue, self.blue_mask_size, self.blue_mask_shift);
        pixel
    }

    unsafe fn write_pixel(&self, row: *mut u8, x: usize, pixel: u32) {
        let offset = x * self.bytes_per_pixel;
        let dst = unsafe { row.add(offset) };
        let bytes = pixel.to_le_bytes();
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), dst, self.bytes_per_pixel);
        }
    }
}

impl fmt::Write for FramebufferConsole {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for byte in text.bytes() {
            self.put_byte(byte);
        }
        Ok(())
    }
}

const fn pack_channel(value: u8, size: u8, shift: u8) -> u32 {
    if size == 0 {
        return 0;
    }
    let raw_max = (1u32 << size) - 1;
    let max = if raw_max < 255 { raw_max } else { 255 };
    let scaled = ((value as u32) * max + 127) / 255;
    scaled << shift
}

const fn glyph_rows(ch: char) -> [u8; GLYPH_HEIGHT] {
    match ch {
        'A' => [0x0e, 0x11, 0x11, 0x1f, 0x11, 0x11, 0x11],
        'B' => [0x1e, 0x11, 0x11, 0x1e, 0x11, 0x11, 0x1e],
        'C' => [0x0e, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0e],
        'D' => [0x1e, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1e],
        'E' => [0x1f, 0x10, 0x10, 0x1e, 0x10, 0x10, 0x1f],
        'F' => [0x1f, 0x10, 0x10, 0x1e, 0x10, 0x10, 0x10],
        'G' => [0x0f, 0x10, 0x10, 0x17, 0x11, 0x11, 0x0f],
        'H' => [0x11, 0x11, 0x11, 0x1f, 0x11, 0x11, 0x11],
        'I' => [0x1f, 0x04, 0x04, 0x04, 0x04, 0x04, 0x1f],
        'J' => [0x01, 0x01, 0x01, 0x01, 0x11, 0x11, 0x0e],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1f],
        'M' => [0x11, 0x1b, 0x15, 0x15, 0x11, 0x11, 0x11],
        'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        'O' => [0x0e, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0e],
        'P' => [0x1e, 0x11, 0x11, 0x1e, 0x10, 0x10, 0x10],
        'Q' => [0x0e, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0d],
        'R' => [0x1e, 0x11, 0x11, 0x1e, 0x14, 0x12, 0x11],
        'S' => [0x0f, 0x10, 0x10, 0x0e, 0x01, 0x01, 0x1e],
        'T' => [0x1f, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0e],
        'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0a, 0x04],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0a],
        'X' => [0x11, 0x11, 0x0a, 0x04, 0x0a, 0x11, 0x11],
        'Y' => [0x11, 0x11, 0x0a, 0x04, 0x04, 0x04, 0x04],
        'Z' => [0x1f, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1f],
        'a' => [0x00, 0x00, 0x0e, 0x01, 0x0f, 0x11, 0x0f],
        'b' => [0x10, 0x10, 0x1e, 0x11, 0x11, 0x11, 0x1e],
        'c' => [0x00, 0x00, 0x0e, 0x11, 0x10, 0x11, 0x0e],
        'd' => [0x01, 0x01, 0x0f, 0x11, 0x11, 0x11, 0x0f],
        'e' => [0x00, 0x00, 0x0e, 0x11, 0x1f, 0x10, 0x0e],
        'f' => [0x03, 0x04, 0x04, 0x0f, 0x04, 0x04, 0x04],
        'g' => [0x00, 0x0f, 0x11, 0x11, 0x0f, 0x01, 0x0e],
        'h' => [0x10, 0x10, 0x1e, 0x11, 0x11, 0x11, 0x11],
        'i' => [0x04, 0x00, 0x0c, 0x04, 0x04, 0x04, 0x0e],
        'j' => [0x02, 0x00, 0x06, 0x02, 0x02, 0x12, 0x0c],
        'k' => [0x10, 0x10, 0x12, 0x14, 0x18, 0x14, 0x12],
        'l' => [0x0c, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0e],
        'm' => [0x00, 0x00, 0x1a, 0x15, 0x15, 0x15, 0x15],
        'n' => [0x00, 0x00, 0x1e, 0x11, 0x11, 0x11, 0x11],
        'o' => [0x00, 0x00, 0x0e, 0x11, 0x11, 0x11, 0x0e],
        'p' => [0x00, 0x00, 0x1e, 0x11, 0x1e, 0x10, 0x10],
        'q' => [0x00, 0x00, 0x0f, 0x11, 0x0f, 0x01, 0x01],
        'r' => [0x00, 0x00, 0x16, 0x19, 0x10, 0x10, 0x10],
        's' => [0x00, 0x00, 0x0f, 0x10, 0x0e, 0x01, 0x1e],
        't' => [0x04, 0x04, 0x0f, 0x04, 0x04, 0x04, 0x03],
        'u' => [0x00, 0x00, 0x11, 0x11, 0x11, 0x13, 0x0d],
        'v' => [0x00, 0x00, 0x11, 0x11, 0x11, 0x0a, 0x04],
        'w' => [0x00, 0x00, 0x11, 0x15, 0x15, 0x15, 0x0a],
        'x' => [0x00, 0x00, 0x11, 0x0a, 0x04, 0x0a, 0x11],
        'y' => [0x00, 0x00, 0x11, 0x11, 0x0f, 0x01, 0x0e],
        'z' => [0x00, 0x00, 0x1f, 0x02, 0x04, 0x08, 0x1f],
        '0' => [0x0e, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0e],
        '1' => [0x04, 0x0c, 0x14, 0x04, 0x04, 0x04, 0x1f],
        '2' => [0x0e, 0x11, 0x01, 0x06, 0x08, 0x10, 0x1f],
        '3' => [0x1f, 0x02, 0x04, 0x06, 0x01, 0x11, 0x0e],
        '4' => [0x02, 0x06, 0x0a, 0x12, 0x1f, 0x02, 0x02],
        '5' => [0x1f, 0x10, 0x1e, 0x01, 0x01, 0x11, 0x0e],
        '6' => [0x06, 0x08, 0x10, 0x1e, 0x11, 0x11, 0x0e],
        '7' => [0x1f, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0e, 0x11, 0x11, 0x0e, 0x11, 0x11, 0x0e],
        '9' => [0x0e, 0x11, 0x11, 0x0f, 0x01, 0x02, 0x0c],
        '-' => [0x00, 0x00, 0x00, 0x1f, 0x00, 0x00, 0x00],
        '_' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1f],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0c, 0x0c],
        ',' => [0x00, 0x00, 0x00, 0x00, 0x0c, 0x0c, 0x08],
        ':' => [0x00, 0x0c, 0x0c, 0x00, 0x0c, 0x0c, 0x00],
        ';' => [0x00, 0x0c, 0x0c, 0x00, 0x0c, 0x0c, 0x08],
        '/' => [0x01, 0x02, 0x02, 0x04, 0x08, 0x08, 0x10],
        '\\' => [0x10, 0x08, 0x08, 0x04, 0x02, 0x02, 0x01],
        '(' => [0x02, 0x04, 0x08, 0x08, 0x08, 0x04, 0x02],
        ')' => [0x08, 0x04, 0x02, 0x02, 0x02, 0x04, 0x08],
        '[' => [0x0e, 0x08, 0x08, 0x08, 0x08, 0x08, 0x0e],
        ']' => [0x0e, 0x02, 0x02, 0x02, 0x02, 0x02, 0x0e],
        '{' => [0x02, 0x04, 0x04, 0x18, 0x04, 0x04, 0x02],
        '}' => [0x08, 0x04, 0x04, 0x03, 0x04, 0x04, 0x08],
        '=' => [0x00, 0x00, 0x1f, 0x00, 0x1f, 0x00, 0x00],
        '+' => [0x00, 0x04, 0x04, 0x1f, 0x04, 0x04, 0x00],
        '*' => [0x00, 0x11, 0x0a, 0x04, 0x0a, 0x11, 0x00],
        '!' => [0x04, 0x04, 0x04, 0x04, 0x04, 0x00, 0x04],
        '?' => [0x0e, 0x11, 0x01, 0x02, 0x04, 0x00, 0x04],
        '"' => [0x0a, 0x0a, 0x0a, 0x00, 0x00, 0x00, 0x00],
        '\'' => [0x04, 0x04, 0x08, 0x00, 0x00, 0x00, 0x00],
        '#' => [0x0a, 0x0a, 0x1f, 0x0a, 0x1f, 0x0a, 0x0a],
        '$' => [0x04, 0x0f, 0x14, 0x0e, 0x05, 0x1e, 0x04],
        '%' => [0x19, 0x19, 0x02, 0x04, 0x08, 0x13, 0x13],
        '&' => [0x0c, 0x12, 0x14, 0x08, 0x15, 0x12, 0x0d],
        '@' => [0x0e, 0x11, 0x17, 0x15, 0x17, 0x10, 0x0e],
        '^' => [0x04, 0x0a, 0x11, 0x00, 0x00, 0x00, 0x00],
        '<' => [0x02, 0x04, 0x08, 0x10, 0x08, 0x04, 0x02],
        '>' => [0x08, 0x04, 0x02, 0x01, 0x02, 0x04, 0x08],
        '|' => [0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        '~' => [0x00, 0x00, 0x09, 0x16, 0x00, 0x00, 0x00],
        '`' => [0x08, 0x04, 0x02, 0x00, 0x00, 0x00, 0x00],
        ' ' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        _ => [0x1f, 0x11, 0x02, 0x04, 0x08, 0x11, 0x1f],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_framebuffer_info() -> FramebufferInfo {
        FramebufferInfo {
            physical_start: 0,
            width: 96,
            height: 32,
            pitch: 96 * 4,
            bpp: 32,
            red_mask_size: 8,
            red_mask_shift: 16,
            green_mask_size: 8,
            green_mask_shift: 8,
            blue_mask_size: 8,
            blue_mask_shift: 0,
        }
    }

    #[test]
    fn channel_packing_respects_mask_size() {
        assert_eq!(pack_channel(0xff, 8, 16), 0x00ff_0000);
        assert_eq!(pack_channel(0xff, 5, 11), 0x0000_f800);
        assert_eq!(pack_channel(0x80, 6, 5), 0x0000_0400);
    }

    #[test]
    fn console_geometry_requires_visible_text_area() {
        let mut info = test_framebuffer_info();
        info.width = 8;
        info.height = 8;
        assert!(FramebufferConsole::new(ptr::null_mut(), info).is_none());
    }

    #[test]
    fn glyph_table_covers_boot_log_characters() {
        for ch in "ngos/x86_64: framebuffer ready".chars() {
            let glyph = glyph_rows(ch);
            assert_eq!(glyph.len(), GLYPH_HEIGHT);
        }
    }

    #[test]
    fn colored_write_updates_palette() {
        let mut pixels = [0u8; 96 * 32 * 4];
        let mut console =
            FramebufferConsole::new(pixels.as_mut_ptr(), test_framebuffer_info()).expect("console");
        console.clear_with_palette(ALERT_FOREGROUND, ALERT_BACKGROUND);
        let _ = console.write_colored("FAULT", ALERT_FOREGROUND, ALERT_BACKGROUND);
        assert_eq!(console.foreground, ALERT_FOREGROUND);
        assert_eq!(console.background, ALERT_BACKGROUND);
    }

    #[test]
    fn raw_byte_output_tracks_newline_and_tab_layout() {
        let mut pixels = [0u8; 96 * 32 * 4];
        let mut console =
            FramebufferConsole::new(pixels.as_mut_ptr(), test_framebuffer_info()).expect("console");
        console.write_bytes(b"hi\n\tx");
        assert_eq!(console.cursor_row, 1);
        assert_eq!(console.cursor_col, 5);
    }

    #[test]
    fn colored_byte_output_preserves_default_palette() {
        let mut pixels = [0u8; 96 * 32 * 4];
        let mut console =
            FramebufferConsole::new(pixels.as_mut_ptr(), test_framebuffer_info()).expect("console");
        console.write_bytes_colored(b"err", STDERR_FOREGROUND, DEFAULT_BACKGROUND);
        assert_eq!(console.foreground, DEFAULT_FOREGROUND);
        assert_eq!(console.background, DEFAULT_BACKGROUND);
        assert_eq!(console.cursor_col, 3);
    }
}
