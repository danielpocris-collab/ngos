//! UI Renderer for NGOS Boot Screen
//!
//! Renders UI elements (logo, progress bar, text) directly to framebuffer

use ngos_gfx_translate::{DrawOp, RgbaColor};
use ngos_ui::boot_screen::{BootScreen, BootStage};

/// UI Renderer state
pub struct UIRenderer {
    width: u32,
    height: u32,
}

impl UIRenderer {
    pub fn new(width: u32, height: u32) -> Self {
        UIRenderer { width, height }
    }

    /// Render a DrawOp to framebuffer
    pub fn render_op(&self, op: &DrawOp) {
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
            _ => {
                // Other ops (text, circles) would be implemented here
            }
        }
    }

    /// Render multiple DrawOps
    pub fn render_ops(&self, ops: &[DrawOp]) {
        for op in ops {
            self.render_op(op);
        }
    }

    /// Fill rectangle with solid color
    fn fill_rect(&self, x: u32, y: u32, w: u32, h: u32, color: RgbaColor) {
        // Simplified - would call actual framebuffer drawing in real implementation
        // This is a placeholder for the actual implementation
        let _ = (x, y, w, h, color);
    }

    /// Fill rectangle with gradient
    fn fill_gradient_rect(
        &self,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        tl: RgbaColor,
        tr: RgbaColor,
        bl: RgbaColor,
        br: RgbaColor,
    ) {
        // Bilinear interpolation for gradient
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

                // Interpolate top edge
                let top_r = Self::lerp(tl.r, tr.r, u);
                let top_g = Self::lerp(tl.g, tr.g, u);
                let top_b = Self::lerp(tl.b, tr.b, u);
                let top_a = Self::lerp(tl.a, tr.a, u);

                // Interpolate bottom edge
                let bot_r = Self::lerp(bl.r, br.r, u);
                let bot_g = Self::lerp(bl.g, br.g, u);
                let bot_b = Self::lerp(bl.b, br.b, u);
                let bot_a = Self::lerp(bl.a, br.a, u);

                // Interpolate vertically
                let r = Self::lerp(top_r, bot_r, v);
                let g = Self::lerp(top_g, bot_g, v);
                let b = Self::lerp(top_b, bot_b, v);
                let a = Self::lerp(top_a, bot_a, v);

                let color = RgbaColor { r, g, b, a };
                // Would set pixel here in real implementation
                let _ = (x + dx, y + dy, color);
            }
        }
    }

    /// Linear interpolation
    fn lerp(a: u8, b: u8, t: f32) -> u8 {
        (a as f32 + (b as f32 - a as f32) * t) as u8
    }

    /// Draw text (placeholder - would use font in real implementation)
    pub fn draw_text(&self, text: &str, x: u32, y: u32, size: u32, color: RgbaColor) {
        let _ = (text, x, y, size, color);
        // Would render actual text with font here
    }

    /// Draw NGOS logo
    pub fn draw_logo(&self, x: u32, y: u32, size: u32) {
        // Logo rendering would go here
        let _ = (x, y, size);
    }
}

/// Boot screen renderer
pub fn render_boot_screen(renderer: &UIRenderer, progress: u8) {
    let mut boot = BootScreen::new(renderer.width, renderer.height);

    // Simulate boot progress
    for _ in 0..progress {
        boot.update(30);
    }

    let ops = boot.render(BootStage::Loading);
    renderer.render_ops(&ops);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_renderer_creation() {
        let renderer = UIRenderer::new(1920, 1080);
        assert_eq!(renderer.width, 1920);
        assert_eq!(renderer.height, 1080);
    }

    #[test]
    fn lerp_test() {
        assert_eq!(UIRenderer::lerp(0, 100, 0.5), 50);
        assert_eq!(UIRenderer::lerp(100, 200, 0.25), 125);
    }
}
