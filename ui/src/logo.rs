use alloc::vec::Vec;
use ngos_gfx_translate::{DrawOp, RgbaColor};

/// NGOS Logo sizes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogoSize {
    Small,  // 32x32 (taskbar)
    Medium, // 80x80 (terminal)
    Large,  // 150x150 (boot screen)
}

/// NGOS Logo Renderer
pub struct NGOSLogo {
    size: u32,
    gradient_start: RgbaColor,
    gradient_end: RgbaColor,
}

impl NGOSLogo {
    pub fn new(size: u32) -> Self {
        NGOSLogo {
            size,
            gradient_start: RgbaColor {
                r: 0x00,
                g: 0xd4,
                b: 0xff,
                a: 0xff,
            }, // Cyan
            gradient_end: RgbaColor {
                r: 0x7b,
                g: 0x2c,
                b: 0xbf,
                a: 0xff,
            }, // Purple
        }
    }

    pub fn with_size(mut self, size: LogoSize) -> Self {
        self.size = match size {
            LogoSize::Small => 32,
            LogoSize::Medium => 80,
            LogoSize::Large => 150,
        };
        self
    }

    /// Render logo at position (x, y)
    pub fn render(&self, x: u32, y: u32) -> Vec<DrawOp> {
        let mut ops = Vec::new();

        // Rounded square background with gradient
        ops.push(DrawOp::GradientRect {
            x,
            y,
            width: self.size,
            height: self.size,
            top_left: self.gradient_start,
            top_right: self.gradient_end,
            bottom_left: self.gradient_end,
            bottom_right: self.gradient_start,
        });

        // Glow effect (simplified as larger semi-transparent rects)
        ops.push(DrawOp::Rect {
            x: x - 5,
            y: y - 5,
            width: self.size + 10,
            height: self.size + 10,
            color: RgbaColor {
                r: 0x00,
                g: 0xd4,
                b: 0xff,
                a: 0x40,
            },
        });

        // Letter "N" (simplified as a rectangle for now)
        let n_width = self.size / 3;
        let n_height = self.size - 20;
        let n_x = x + (self.size - n_width) / 2;
        let n_y = y + 10;

        ops.push(DrawOp::Rect {
            x: n_x,
            y: n_y,
            width: n_width,
            height: n_height,
            color: RgbaColor {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            },
        });

        ops
    }

    /// Render logo with text "NGOS" below
    pub fn render_with_text(&self, x: u32, y: u32) -> Vec<DrawOp> {
        let mut ops = self.render(x, y);

        // "NGOS" text (simplified - in real implementation would use font)
        let text_y = y + self.size + 20;
        let text_height = 40;

        // Draw each letter as a rectangle (placeholder for real font rendering)
        let letter_width = 30;
        let gap = 10;
        let total_width = 4 * letter_width + 3 * gap;
        let start_x = x + (self.size - total_width) / 2;

        for i in 0..4 {
            ops.push(DrawOp::GradientRect {
                x: start_x + (i as u32 * (letter_width + gap)),
                y: text_y,
                width: letter_width,
                height: text_height,
                top_left: self.gradient_start,
                top_right: self.gradient_end,
                bottom_left: self.gradient_end,
                bottom_right: self.gradient_start,
            });
        }

        ops
    }

    /// Render animated boot logo (pulsing effect using simple oscillation)
    pub fn render_boot(&self, x: u32, y: u32, time_ms: u32) -> Vec<DrawOp> {
        let mut ops = self.render(x, y);

        // Pulse effect - oscillate between 10 and 15
        let pulse_size = 10 + (((time_ms / 200) % 5) as u32);

        ops.push(DrawOp::Rect {
            x: x.saturating_sub(pulse_size),
            y: y.saturating_sub(pulse_size),
            width: self.size + pulse_size * 2,
            height: self.size + pulse_size * 2,
            color: RgbaColor {
                r: 0x00,
                g: 0xd4,
                b: 0xff,
                a: 0x30,
            },
        });

        ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logo_creation() {
        let logo = NGOSLogo::new(150);
        assert_eq!(logo.size, 150);
    }

    #[test]
    fn logo_with_size_enum() {
        let logo_small = NGOSLogo::new(32).with_size(LogoSize::Small);
        assert_eq!(logo_small.size, 32);

        let logo_large = NGOSLogo::new(32).with_size(LogoSize::Large);
        assert_eq!(logo_large.size, 150);
    }

    #[test]
    fn logo_renders_ops() {
        let logo = NGOSLogo::new(150);
        let ops = logo.render(100, 100);
        assert!(!ops.is_empty());
    }

    #[test]
    fn logo_with_text_renders_more_ops() {
        let logo = NGOSLogo::new(150);
        let ops_basic = logo.render(100, 100);
        let ops_text = logo.render_with_text(100, 100);
        assert!(ops_text.len() > ops_basic.len());
    }
}
