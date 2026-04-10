//! NGOS UI Framebuffer Renderer
//!
//! Renders UI primitives (rounded rects, text, icons) to framebuffer

#![cfg_attr(target_os = "none", allow(dead_code))]

extern crate alloc;

use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
use ngos_gfx_translate::{DrawOp, FontFamily, RgbaColor};

/// Framebuffer wrapper for UI rendering
pub struct FramebufferRenderer<'a> {
    base: &'a mut [u8],
    width: u32,
    height: u32,
    pitch: u32,
    bpp: u32,
}

impl<'a> FramebufferRenderer<'a> {
    /// Create new renderer from framebuffer info
    pub fn new(base: &'a mut [u8], width: u32, height: u32, pitch: u32, bpp: u32) -> Self {
        FramebufferRenderer {
            base,
            width,
            height,
            pitch,
            bpp,
        }
    }

    /// Render a single pixel
    #[inline]
    pub fn put_pixel(&mut self, x: u32, y: u32, color: RgbaColor) {
        if x >= self.width || y >= self.height {
            return;
        }

        let offset = ((y * self.pitch) + (x * (self.bpp / 8))) as usize;

        // BGRA format (typical for framebuffer)
        if offset + 3 < self.base.len() {
            self.base[offset] = color.b;
            self.base[offset + 1] = color.g;
            self.base[offset + 2] = color.r;
            self.base[offset + 3] = color.a;
        }
    }

    /// Render a solid rectangle
    pub fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: RgbaColor) {
        for dy in 0..h {
            for dx in 0..w {
                self.put_pixel(x + dx, y + dy, color);
            }
        }
    }

    /// Render a rounded rectangle
    pub fn fill_rounded_rect(
        &mut self,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        radius: u32,
        color: RgbaColor,
    ) {
        // Clamp radius
        let radius = radius.min(w / 2).min(h / 2);

        for dy in 0..h {
            for dx in 0..w {
                let px = x + dx;
                let py = y + dy;

                // Check if pixel is inside rounded rect
                if self.is_inside_rounded_rect(px, py, x, y, w, h, radius) {
                    self.put_pixel(px, py, color);
                }
            }
        }
    }

    /// Check if pixel is inside rounded rectangle
    fn is_inside_rounded_rect(
        &self,
        px: u32,
        py: u32,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        r: u32,
    ) -> bool {
        // Center of corner circles
        let r_f = r as f32;

        // Top-left corner
        if px < x + r && py < y + r {
            let dx = (px - (x + r)) as f32;
            let dy = (py - (y + r)) as f32;
            return dx * dx + dy * dy <= r_f * r_f;
        }

        // Top-right corner
        if px >= x + w - r && py < y + r {
            let dx = (px - (x + w - r - 1)) as f32;
            let dy = (py - (y + r)) as f32;
            return dx * dx + dy * dy <= r_f * r_f;
        }

        // Bottom-left corner
        if px < x + r && py >= y + h - r {
            let dx = (px - (x + r)) as f32;
            let dy = (py - (y + h - r - 1)) as f32;
            return dx * dx + dy * dy <= r_f * r_f;
        }

        // Bottom-right corner
        if px >= x + w - r && py >= y + h - r {
            let dx = (px - (x + w - r - 1)) as f32;
            let dy = (py - (y + h - r - 1)) as f32;
            return dx * dx + dy * dy <= r_f * r_f;
        }

        // Inside main rectangle
        true
    }

    /// Render text using bitmap font
    pub fn render_text(
        &mut self,
        text: &str,
        x: u32,
        y: u32,
        size: u32,
        color: RgbaColor,
        font: FontFamily,
    ) {
        // Use built-in bitmap font
        let font_data = BitmapFont::get_font(font);

        let mut cursor_x = x;
        for ch in text.chars() {
            let glyph = font_data.get_glyph(ch);
            self.render_glyph(&glyph, cursor_x, y, size, color);
            cursor_x += glyph.advance * size / font_data.size;
        }
    }

    /// Render a single glyph
    fn render_glyph(&mut self, glyph: &GlyphBitmap, x: u32, y: u32, size: u32, color: RgbaColor) {
        let scale = size / 16; // Base glyph size is 16px

        for gy in 0..glyph.height {
            for gx in 0..glyph.width {
                let alpha = glyph.data[(gy * glyph.width + gx) as usize];
                if alpha > 0 {
                    // Blend with background
                    let px = x + gx * scale;
                    let py = y + gy * scale;

                    // Simple alpha blending
                    for sy in 0..scale {
                        for sx in 0..scale {
                            self.put_pixel(
                                px + sx,
                                py + sy,
                                RgbaColor {
                                    r: color.r,
                                    g: color.g,
                                    b: color.b,
                                    a: (alpha as u32 * color.a as u32 / 255) as u8,
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    /// Render an icon (Unicode character)
    pub fn render_icon(&mut self, icon: char, x: u32, y: u32, size: u32, color: RgbaColor) {
        // For now, render as text - would use emoji font in real implementation
        self.render_text(&icon.to_string(), x, y, size, color, FontFamily::System);
    }

    /// Render a DrawOp
    pub fn render_op(&mut self, op: &DrawOp) {
        match op {
            DrawOp::Clear { color } => {
                self.fill_rect(0, 0, self.width, self.height, *color);
            }
            DrawOp::Rect {
                x,
                y,
                width,
                height,
                color,
            } => {
                self.fill_rect(*x, *y, *width, *height, *color);
            }
            DrawOp::RoundedRect {
                x,
                y,
                width,
                height,
                radius,
                color,
            } => {
                self.fill_rounded_rect(*x, *y, *width, *height, *radius, *color);
            }
            DrawOp::Text {
                text,
                x,
                y,
                size,
                color,
                font,
            } => {
                self.render_text(text, *x, *y, *size, *color, *font);
            }
            DrawOp::Icon {
                icon,
                x,
                y,
                size,
                color,
            } => {
                self.render_icon(*icon, *x, *y, *size, *color);
            }
            DrawOp::GradientRect {
                x,
                y,
                width,
                height,
                top_left,
                top_right,
                bottom_left,
                bottom_right,
            } => {
                self.fill_gradient_rect(
                    *x,
                    *y,
                    *width,
                    *height,
                    *top_left,
                    *top_right,
                    *bottom_left,
                    *bottom_right,
                );
            }
            // Other ops would be implemented here
            _ => {
                // Placeholder for unimplemented ops
            }
        }
    }

    /// Render gradient rectangle
    fn fill_gradient_rect(
        &mut self,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        tl: RgbaColor,
        tr: RgbaColor,
        bl: RgbaColor,
        br: RgbaColor,
    ) {
        for dy in 0..h {
            let v = if h > 1 {
                dy as f32 / (h - 1) as f32
            } else {
                0.0
            };

            for dx in 0..w {
                let u = if w > 1 {
                    dx as f32 / (w - 1) as f32
                } else {
                    0.0
                };

                // Bilinear interpolation
                let r = Self::lerp_2d(tl.r, tr.r, bl.r, br.r, u, v);
                let g = Self::lerp_2d(tl.g, tr.g, bl.g, br.g, u, v);
                let b = Self::lerp_2d(tl.b, tr.b, bl.b, br.b, u, v);
                let a = Self::lerp_2d(tl.a, tr.a, bl.a, br.a, u, v);

                self.put_pixel(x + dx, y + dy, RgbaColor { r, g, b, a });
            }
        }
    }

    #[inline]
    fn lerp(a: u8, b: u8, t: f32) -> u8 {
        (a as f32 + (b as f32 - a as f32) * t) as u8
    }

    #[inline]
    fn lerp_2d(tl: u8, tr: u8, bl: u8, br: u8, u: f32, v: f32) -> u8 {
        let top = Self::lerp(tl, tr, u);
        let bot = Self::lerp(bl, br, u);
        Self::lerp(top, bot, v)
    }

    /// Render multiple DrawOps
    pub fn render_ops(&mut self, ops: &[DrawOp]) {
        for op in ops {
            self.render_op(op);
        }
    }
}

/// Simple bitmap font for UI rendering
pub struct BitmapFont {
    pub size: u32,
    pub glyphs: alloc::collections::BTreeMap<char, GlyphBitmap>,
}

#[derive(Debug, Clone)]
pub struct GlyphBitmap {
    pub width: u32,
    pub height: u32,
    pub advance: u32,
    pub data: Vec<u8>, // Alpha channel only
}

impl BitmapFont {
    /// Get built-in font for family
    pub fn get_font(family: FontFamily) -> &'static BitmapFont {
        match family {
            FontFamily::System => &FONT_SYSTEM,
            FontFamily::Monospace => &FONT_MONO,
            FontFamily::SansSerif => &FONT_SANS,
            FontFamily::Serif => &FONT_SYSTEM,
        }
    }

    pub fn get_glyph(&self, ch: char) -> GlyphBitmap {
        self.glyphs
            .get(&ch)
            .cloned()
            .unwrap_or_else(|| GlyphBitmap {
                width: 8,
                height: 16,
                advance: 8,
                data: vec![255; 8 * 16],
            })
    }
}

// Built-in system font (simplified - would be full font data in real implementation)
static FONT_SYSTEM: BitmapFont = BitmapFont {
    size: 16,
    glyphs: alloc::collections::BTreeMap::new(), // Would contain actual glyph data
};

static FONT_MONO: BitmapFont = BitmapFont {
    size: 16,
    glyphs: alloc::collections::BTreeMap::new(),
};

static FONT_SANS: BitmapFont = BitmapFont {
    size: 16,
    glyphs: alloc::collections::BTreeMap::new(),
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_creation() {
        let mut buffer = [0u8; 1920 * 1080 * 4];
        let renderer = FramebufferRenderer::new(&mut buffer, 1920, 1080, 1920 * 4, 32);
        assert_eq!(renderer.width, 1920);
        assert_eq!(renderer.height, 1080);
    }

    #[test]
    fn fill_rect() {
        let mut buffer = [0u8; 100 * 100 * 4];
        let mut renderer = FramebufferRenderer::new(&mut buffer, 100, 100, 100 * 4, 32);

        renderer.fill_rect(
            10,
            10,
            20,
            20,
            RgbaColor {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            },
        );

        // Check a pixel inside the rect
        let offset = ((20 * 100 + 20) * 4) as usize;
        assert_eq!(buffer[offset], 0); // B
        assert_eq!(buffer[offset + 1], 0); // G
        assert_eq!(buffer[offset + 2], 255); // R
    }

    #[test]
    fn rounded_rect() {
        let mut buffer = [0u8; 100 * 100 * 4];
        let mut renderer = FramebufferRenderer::new(&mut buffer, 100, 100, 100 * 4, 32);

        renderer.fill_rounded_rect(
            10,
            10,
            40,
            40,
            8,
            RgbaColor {
                r: 0,
                g: 255,
                b: 0,
                a: 255,
            },
        );

        // Should have drawn something
        assert_ne!(buffer[0], 0); // At least one pixel should be drawn
    }

    #[test]
    fn is_inside_rounded_rect_center() {
        let mut buffer = [0u8; 100 * 100 * 4];
        let renderer = FramebufferRenderer::new(&mut buffer, 100, 100, 100 * 4, 32);

        // Center pixel should be inside
        assert!(renderer.is_inside_rounded_rect(30, 30, 20, 20, 40, 40, 8));
    }

    #[test]
    fn gradient_render() {
        let mut buffer = [0u8; 100 * 100 * 4];
        let mut renderer = FramebufferRenderer::new(&mut buffer, 100, 100, 100 * 4, 32);

        renderer.fill_gradient_rect(
            0,
            0,
            100,
            100,
            RgbaColor {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            RgbaColor {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            },
            RgbaColor {
                r: 0,
                g: 255,
                b: 0,
                a: 255,
            },
            RgbaColor {
                r: 0,
                g: 0,
                b: 255,
                a: 255,
            },
        );

        // Check corners have different colors
        let tl_offset = 0;
        let tr_offset = (99 * 4) as usize;

        assert_ne!(buffer[tl_offset + 2], buffer[tr_offset + 2]); // R should differ
    }
}
