//! NGOS Browser Rendering Engine
//!
//! FrameScript renderer for NGOS GPU - 100% Proprietary
//!
//! Canonical subsystem role:
//! - subsystem: browser paint support
//! - owner layer: application support layer
//! - semantic owner: `browser-paint`
//! - truth path role: browser-facing paint and presentation support over
//!   rendering translation surfaces
//!
//! Canonical contract families defined here:
//! - browser renderer contracts
//! - browser frame presentation contracts
//! - browser paint support contracts
//!
//! This crate may define browser paint support behavior, but it must not
//! redefine kernel, runtime, or product-level OS truth.

pub use browser_core::{BrowserError, BrowserResult};
pub use browser_layout::{LayoutNode, LayoutTree, Rect};
use ngos_gfx_translate::{EncodedFrame, FrameScript};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentedFrame {
    pub script: FrameScript,
    pub encoded: EncodedFrame,
}

/// Renderer interface
pub trait Renderer {
    fn render(&mut self, tree: &LayoutTree) -> BrowserResult<()>;
    fn get_output(&self) -> &str;
    fn present(&mut self) -> BrowserResult<()>;
}

/// FrameScript Renderer for NGOS GPU
pub struct FrameScriptRenderer {
    output: String,
    width: u32,
    height: u32,
    frame_count: u64,
    last_presented: Option<PresentedFrame>,
}

impl FrameScriptRenderer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            output: String::new(),
            width,
            height,
            frame_count: 0,
            last_presented: None,
        }
    }

    pub fn with_size(width: u32, height: u32) -> Self {
        Self::new(width, height)
    }

    pub fn last_presented(&self) -> Option<&PresentedFrame> {
        self.last_presented.as_ref()
    }

    fn rgba(r: u8, g: u8, b: u8, a: u8) -> String {
        format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
    }

    fn clamp_dimension(value: f32) -> u32 {
        value.max(0.0).round() as u32
    }

    fn pulse_alpha(&self, depth: usize) -> u8 {
        let phase = ((self.frame_count + depth as u64 * 3) % 10) as u8;
        0x30 + phase.saturating_mul(6)
    }

    fn push_rect(&mut self, x: u32, y: u32, width: u32, height: u32, color: &str) {
        self.output
            .push_str(&format!("rect={x},{y},{width},{height},{color}\n"));
    }

    fn push_gradient_rect(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        tl: &str,
        tr: &str,
        bl: &str,
        br: &str,
    ) {
        self.output.push_str(&format!(
            "gradient-rect={x},{y},{width},{height},{tl},{tr},{bl},{br}\n"
        ));
    }

    fn push_rounded_rect(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        radius: u32,
        color: &str,
    ) {
        self.output.push_str(&format!(
            "rounded-rect={x},{y},{width},{height},{radius},{color}\n"
        ));
    }

    fn push_shadow_rect(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        blur: u32,
        color: &str,
    ) {
        self.output.push_str(&format!(
            "shadow-rect={x},{y},{width},{height},{blur},{color}\n"
        ));
    }

    fn render_root_backdrop(&mut self) {
        let width = self.width;
        let height = self.height;
        let pulse = self.pulse_alpha(0);
        self.output
            .push_str(&format!("clear={}\n", Self::rgba(0x08, 0x10, 0x1C, 0xFF)));
        self.push_gradient_rect(
            0,
            0,
            width,
            height,
            &Self::rgba(0x0A, 0x16, 0x25, 0xFF),
            &Self::rgba(0x14, 0x28, 0x3D, 0xFF),
            &Self::rgba(0x07, 0x0E, 0x18, 0xFF),
            &Self::rgba(0x10, 0x1B, 0x30, 0xFF),
        );
        self.push_rect(
            width / 16,
            height / 8,
            width * 7 / 10,
            height / 20,
            &Self::rgba(0xA0, 0xD6, 0xFF, pulse),
        );
        self.push_rect(
            width * 11 / 20,
            height * 2 / 3,
            width / 4,
            height / 24,
            &Self::rgba(0x5B, 0xFF, 0xC7, pulse.saturating_sub(6)),
        );
    }

    fn render_node_surface(&mut self, node: &LayoutNode, depth: usize) {
        let rect = &node.rect;
        let x = Self::clamp_dimension(rect.x);
        let y = Self::clamp_dimension(rect.y);
        let width = Self::clamp_dimension(rect.width).max(8);
        let height = Self::clamp_dimension(rect.height).max(8);
        let depth_u32 = depth as u32;
        let radius = 18u32.saturating_sub(depth_u32.min(10));
        let blur = 28u32.saturating_sub(depth_u32.min(12) * 2).max(10);
        let title_height = (height / 7).clamp(18, 42);
        let accent_width = (width / 32).clamp(4, 10);
        let pulse = self.pulse_alpha(depth);
        let shell = 0x8E_u8.saturating_sub((depth as u8).saturating_mul(9));
        let overlay = 0xC8_u8.saturating_sub((depth as u8).saturating_mul(8));
        let accent = if depth % 3 == 0 {
            Self::rgba(0x73, 0xD5, 0xFF, pulse)
        } else if depth % 3 == 1 {
            Self::rgba(0x79, 0xFF, 0xD8, pulse)
        } else {
            Self::rgba(0xFF, 0xB8, 0x6B, pulse)
        };

        self.push_shadow_rect(
            x.saturating_sub(6),
            y.saturating_sub(6),
            width.saturating_add(12),
            height.saturating_add(14),
            blur,
            &Self::rgba(0x00, 0x00, 0x00, 0x34),
        );
        self.push_rounded_rect(
            x,
            y,
            width,
            height,
            radius,
            &Self::rgba(0x14, 0x1F, 0x2E, shell),
        );
        self.push_gradient_rect(
            x,
            y,
            width,
            title_height,
            &Self::rgba(0x26, 0x3D, 0x58, overlay),
            &Self::rgba(0x32, 0x4F, 0x74, overlay),
            &Self::rgba(0x1B, 0x2D, 0x45, overlay.saturating_sub(10)),
            &Self::rgba(0x28, 0x3D, 0x59, overlay.saturating_sub(10)),
        );
        self.push_rect(x + 10, y + 10, 10, 10, &Self::rgba(0xFF, 0x7B, 0x7B, 0xC8));
        self.push_rect(x + 26, y + 10, 10, 10, &Self::rgba(0xFF, 0xCF, 0x67, 0xC8));
        self.push_rect(x + 42, y + 10, 10, 10, &Self::rgba(0x6D, 0xF2, 0x97, 0xC8));
        self.push_rect(
            x + 8,
            y + 8,
            accent_width,
            height.saturating_sub(16),
            &accent,
        );

        let content_x = x + accent_width + 18;
        let content_y = y + title_height + 10;
        let content_width = width.saturating_sub(accent_width + 30);
        let content_height = height.saturating_sub(title_height + 22);
        if content_width > 24 && content_height > 24 {
            self.push_rounded_rect(
                content_x,
                content_y,
                content_width,
                content_height,
                radius.saturating_sub(6).max(8),
                &Self::rgba(0xF4, 0xFA, 0xFF, 0x12),
            );
            self.push_rect(
                content_x + 12,
                content_y + 12,
                content_width / 3,
                4,
                &Self::rgba(0xD5, 0xEA, 0xFF, 0x74),
            );
            self.push_rect(
                content_x + 12,
                content_y + 24,
                content_width.saturating_sub(24),
                2,
                &Self::rgba(0xC2, 0xDC, 0xF8, 0x28),
            );
            self.push_rect(
                content_x + 12,
                content_y + content_height.saturating_sub(20),
                content_width / 2,
                3,
                &accent,
            );
        }
    }

    fn render_node(&mut self, node: &LayoutNode, depth: usize) {
        if depth == 0 {
            self.render_root_backdrop();
        }
        if node.rect.width > 2.0 && node.rect.height > 2.0 {
            self.render_node_surface(node, depth);
        }

        for child in &node.children {
            self.render_node(child, depth + 1);
        }
    }
}

impl Renderer for FrameScriptRenderer {
    fn render(&mut self, tree: &LayoutTree) -> BrowserResult<()> {
        self.output.clear();
        self.frame_count += 1;
        self.last_presented = None;

        // FrameScript header
        self.output.push_str(&format!(
            "surface={}x{}\n\
             frame=browser-{}\n\
             queue=graphics\n\
             present-mode=mailbox\n\
             completion=wait-complete\n",
            self.width, self.height, self.frame_count
        ));

        // Render layout tree
        if let Some(ref root) = tree.root {
            self.render_node(root, 0);
        }

        Ok(())
    }

    fn get_output(&self) -> &str {
        &self.output
    }

    fn present(&mut self) -> BrowserResult<()> {
        let script = FrameScript::parse(&self.output).map_err(|error| {
            BrowserError::Render(format!("invalid framescript: {}", error.describe()))
        })?;
        let encoded = script.encode("browser-paint/framescript");
        self.last_presented = Some(PresentedFrame { script, encoded });
        Ok(())
    }
}

/// ASCII Renderer for debugging
pub struct AsciiRenderer {
    output: String,
    width: usize,
    height: usize,
}

impl AsciiRenderer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            output: String::new(),
            width,
            height,
        }
    }
}

impl Renderer for AsciiRenderer {
    fn render(&mut self, tree: &LayoutTree) -> BrowserResult<()> {
        self.output.clear();

        // Create ASCII buffer
        let mut buffer = vec![vec![' '; self.width]; self.height];

        // Draw border
        for x in 0..self.width {
            buffer[0][x] = '-';
            buffer[self.height - 1][x] = '-';
        }
        for y in 0..self.height {
            buffer[y][0] = '|';
            buffer[y][self.width - 1] = '|';
        }

        // Render nodes as boxes
        if let Some(ref root) = tree.root {
            self.render_node_ascii(&mut buffer, root, 0);
        }

        // Convert to string
        for row in buffer {
            self.output.extend(row);
            self.output.push('\n');
        }

        Ok(())
    }

    fn get_output(&self) -> &str {
        &self.output
    }

    fn present(&mut self) -> BrowserResult<()> {
        println!("{}", self.output);
        Ok(())
    }
}

impl AsciiRenderer {
    fn render_node_ascii(&self, buffer: &mut Vec<Vec<char>>, node: &LayoutNode, depth: usize) {
        let rect = &node.rect;
        let x = rect.x as usize % self.width;
        let y = rect.y as usize % self.height;
        let w = (rect.width as usize).min(self.width - x - 1);
        let h = (rect.height as usize).min(self.height - y - 1);

        // Draw box corners
        if w > 1 && h > 1 {
            buffer[y][x] = '+';
            buffer[y][x + w] = '+';
            buffer[y + h][x] = '+';
            buffer[y + h][x + w] = '+';

            // Draw edges
            for i in 1..w {
                buffer[y][x + i] = '-';
                buffer[y + h][x + i] = '-';
            }
            for i in 1..h {
                buffer[y + i][x] = '|';
                buffer[y + i][x + w] = '|';
            }
        }

        // Render children
        for child in &node.children {
            self.render_node_ascii(buffer, child, depth + 1);
        }
    }
}

#[cfg(feature = "skia")]
pub struct SkiaRenderer {
    width: u32,
    height: u32,
    frame_count: u64,
}

#[cfg(feature = "skia")]
impl SkiaRenderer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            frame_count: 0,
        }
    }

    fn rgba(r: u8, g: u8, b: u8, a: u8) -> skia_safe::Color {
        skia_safe::Color::from_argb(a, r, g, b)
    }

    fn clamp_f32(value: f32) -> f32 {
        value.max(0.0)
    }

    fn draw_root_backdrop(&self, canvas: &skia_safe::Canvas) {
        use skia_safe::{Paint, PaintStyle, Rect};

        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_style(PaintStyle::Fill);
        canvas.clear(Self::rgba(0x08, 0x10, 0x1C, 0xFF));

        paint.set_color(Self::rgba(0x0A, 0x16, 0x25, 0xFF));
        canvas.draw_rect(
            Rect::from_xywh(0.0, 0.0, self.width as f32, self.height as f32),
            &paint,
        );

        paint.set_color(Self::rgba(0x14, 0x28, 0x3D, 0xA0));
        canvas.draw_rect(
            Rect::from_xywh(
                self.width as f32 * 0.18,
                0.0,
                self.width as f32 * 0.64,
                self.height as f32,
            ),
            &paint,
        );

        paint.set_color(Self::rgba(0xA0, 0xD6, 0xFF, 0x30));
        canvas.draw_rect(
            Rect::from_xywh(
                self.width as f32 / 16.0,
                self.height as f32 / 8.0,
                self.width as f32 * 0.70,
                self.height as f32 / 20.0,
            ),
            &paint,
        );

        paint.set_color(Self::rgba(0x5B, 0xFF, 0xC7, 0x26));
        canvas.draw_rect(
            Rect::from_xywh(
                self.width as f32 * 11.0 / 20.0,
                self.height as f32 * 2.0 / 3.0,
                self.width as f32 / 4.0,
                self.height as f32 / 24.0,
            ),
            &paint,
        );
    }

    fn draw_node(&self, canvas: &skia_safe::Canvas, node: &LayoutNode, depth: usize) {
        use skia_safe::{Font, MaskFilter, Paint, PaintStyle, Rect};

        let rect = &node.rect;
        let x = Self::clamp_f32(rect.x);
        let y = Self::clamp_f32(rect.y);
        let width = Self::clamp_f32(rect.width).max(8.0);
        let height = Self::clamp_f32(rect.height).max(8.0);
        let depth_u32 = depth as u32;
        let radius = 18u32.saturating_sub(depth_u32.min(10)) as f32;
        let blur = 28u32.saturating_sub(depth_u32.min(12) * 2).max(10) as f32;
        let title_height = (height / 7.0).clamp(18.0, 42.0);
        let accent_width = (width / 32.0).clamp(4.0, 10.0);
        let shell = 0x8E_u8.saturating_sub((depth as u8).saturating_mul(9));
        let overlay = 0xC8_u8.saturating_sub((depth as u8).saturating_mul(8));
        let accent = if depth % 3 == 0 {
            Self::rgba(0x73, 0xD5, 0xFF, 0xD0)
        } else if depth % 3 == 1 {
            Self::rgba(0x79, 0xFF, 0xD8, 0xD0)
        } else {
            Self::rgba(0xFF, 0xB8, 0x6B, 0xD0)
        };

        let mut shadow_paint = Paint::default();
        shadow_paint.set_anti_alias(true);
        shadow_paint.set_style(PaintStyle::Fill);
        shadow_paint.set_color(Self::rgba(0x00, 0x00, 0x00, 0x34));
        shadow_paint.set_mask_filter(MaskFilter::blur(skia_safe::BlurStyle::Normal, blur, None));
        canvas.draw_round_rect(
            Rect::from_xywh(x - 6.0, y - 6.0, width + 12.0, height + 14.0),
            radius + 2.0,
            radius + 2.0,
            &shadow_paint,
        );

        let mut shell_paint = Paint::default();
        shell_paint.set_anti_alias(true);
        shell_paint.set_style(PaintStyle::Fill);
        shell_paint.set_color(Self::rgba(0x14, 0x1F, 0x2E, shell));
        canvas.draw_round_rect(
            Rect::from_xywh(x, y, width, height),
            radius,
            radius,
            &shell_paint,
        );

        let mut overlay_paint = Paint::default();
        overlay_paint.set_anti_alias(true);
        overlay_paint.set_style(PaintStyle::Fill);
        overlay_paint.set_color(Self::rgba(0x26, 0x3D, 0x58, overlay));
        canvas.draw_rect(Rect::from_xywh(x, y, width, title_height), &overlay_paint);

        let mut accent_paint = Paint::default();
        accent_paint.set_anti_alias(true);
        accent_paint.set_style(PaintStyle::Fill);
        accent_paint.set_color(accent);
        canvas.draw_rect(
            Rect::from_xywh(x + 8.0, y + 8.0, accent_width, height - 16.0),
            &accent_paint,
        );

        let content_x = x + accent_width + 18.0;
        let content_y = y + title_height + 10.0;
        let content_width = width - accent_width - 30.0;
        let content_height = height - title_height - 22.0;
        if content_width > 24.0 && content_height > 24.0 {
            let mut content_paint = Paint::default();
            content_paint.set_anti_alias(true);
            content_paint.set_style(PaintStyle::Fill);
            content_paint.set_color(Self::rgba(0xF4, 0xFA, 0xFF, 0x12));
            canvas.draw_round_rect(
                Rect::from_xywh(content_x, content_y, content_width, content_height),
                (radius - 6.0).max(8.0),
                (radius - 6.0).max(8.0),
                &content_paint,
            );

            content_paint.set_color(Self::rgba(0xD5, 0xEA, 0xFF, 0x74));
            canvas.draw_rect(
                Rect::from_xywh(content_x + 12.0, content_y + 12.0, content_width / 3.0, 4.0),
                &content_paint,
            );

            content_paint.set_color(Self::rgba(0xC2, 0xDC, 0xF8, 0x28));
            canvas.draw_rect(
                Rect::from_xywh(
                    content_x + 12.0,
                    content_y + 24.0,
                    content_width - 24.0,
                    2.0,
                ),
                &content_paint,
            );

            content_paint.set_color(accent);
            canvas.draw_rect(
                Rect::from_xywh(
                    content_x + 12.0,
                    content_y + content_height - 20.0,
                    content_width / 2.0,
                    3.0,
                ),
                &content_paint,
            );
        }

        let mut dot_paint = Paint::default();
        dot_paint.set_anti_alias(true);
        dot_paint.set_style(PaintStyle::Fill);
        dot_paint.set_color(Self::rgba(0xFF, 0x7B, 0x7B, 0xC8));
        canvas.draw_rect(Rect::from_xywh(x + 10.0, y + 10.0, 10.0, 10.0), &dot_paint);
        dot_paint.set_color(Self::rgba(0xFF, 0xCF, 0x67, 0xC8));
        canvas.draw_rect(Rect::from_xywh(x + 26.0, y + 10.0, 10.0, 10.0), &dot_paint);
        dot_paint.set_color(Self::rgba(0x6D, 0xF2, 0x97, 0xC8));
        canvas.draw_rect(Rect::from_xywh(x + 42.0, y + 10.0, 10.0, 10.0), &dot_paint);

        let mut font = Font::default();
        font.set_size((depth_u32.max(1) as f32 * 1.2).max(18.0));
        let mut text_paint = Paint::default();
        text_paint.set_anti_alias(true);
        text_paint.set_color(Self::rgba(0xE8, 0xF2, 0xFF, 0xFF));
        let label = format!("node-{}", depth);
        canvas.draw_str(
            label.as_str(),
            (content_x + 12.0, content_y + 18.0),
            &font,
            &text_paint,
        );
        font.set_size(12.0);
        canvas.draw_str(
            node.node.borrow().name.as_str(),
            (content_x + 12.0, content_y + 36.0),
            &font,
            &text_paint,
        );
    }

    fn draw_node_recursive(&self, canvas: &skia_safe::Canvas, node: &LayoutNode, depth: usize) {
        if depth == 0 {
            self.draw_root_backdrop(canvas);
        }
        if node.rect.width > 2.0 && node.rect.height > 2.0 {
            self.draw_node(canvas, node, depth);
        }
        for child in &node.children {
            self.draw_node_recursive(canvas, child, depth + 1);
        }
    }

    pub fn render_to_png(
        &mut self,
        tree: &LayoutTree,
        path: impl AsRef<std::path::Path>,
    ) -> BrowserResult<()> {
        use skia_safe::{EncodedImageFormat, surfaces};
        let mut surface = surfaces::raster_n32_premul((self.width as i32, self.height as i32))
            .ok_or_else(|| BrowserError::Render("failed to create skia surface".into()))?;
        self.frame_count += 1;
        let canvas = surface.canvas();
        self.draw_node_recursive(
            canvas,
            tree.root
                .as_ref()
                .ok_or_else(|| BrowserError::Render("empty layout tree".into()))?,
            0,
        );
        let image = surface.image_snapshot();
        let data = image
            .encode(None, EncodedImageFormat::PNG, None)
            .ok_or_else(|| BrowserError::Render("failed to encode png".into()))?;
        std::fs::write(path, data.as_bytes())
            .map_err(|error| BrowserError::Render(format!("failed to write png: {error:?}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use browser_dom::{NodeData, NodeType};
    use browser_layout::{LayoutTree, Rect};

    fn create_test_tree() -> LayoutTree {
        LayoutTree {
            root: Some(LayoutNode {
                node: std::rc::Rc::new(std::cell::RefCell::new(NodeData::new(
                    NodeType::Element,
                    "div",
                ))),
                rect: Rect::new(0.0, 0.0, 100.0, 100.0),
                children: Vec::new(),
            }),
        }
    }

    #[test]
    fn create_frame_script_renderer() {
        let renderer = FrameScriptRenderer::new(1920, 1080);
        assert_eq!(renderer.width, 1920);
        assert_eq!(renderer.height, 1080);
    }

    #[test]
    fn render_simple_tree() {
        let mut renderer = FrameScriptRenderer::new(800, 600);
        let tree = create_test_tree();

        let result = renderer.render(&tree);
        assert!(result.is_ok());

        let output = renderer.get_output();
        assert!(output.contains("surface=800x600"));
        assert!(output.contains("frame=browser-"));
        assert!(output.contains("gradient-rect="));
        assert!(output.contains("rounded-rect="));
        assert!(output.contains("shadow-rect="));
    }

    #[test]
    fn render_uses_temporal_pulse_between_frames() {
        let mut renderer = FrameScriptRenderer::new(640, 480);
        let tree = create_test_tree();

        renderer.render(&tree).expect("first frame should render");
        let first = renderer.get_output().to_string();

        renderer.render(&tree).expect("second frame should render");
        let second = renderer.get_output().to_string();

        assert_ne!(first, second);
    }

    #[test]
    fn present_validates_and_encodes_framescript_output() {
        let mut renderer = FrameScriptRenderer::new(800, 600);
        let tree = create_test_tree();

        renderer.render(&tree).expect("frame should render");
        renderer.present().expect("frame should present");

        let presented = renderer
            .last_presented()
            .expect("presented frame metadata should be stored");
        assert_eq!(presented.script.width, 800);
        assert_eq!(presented.script.height, 600);
        assert_eq!(presented.encoded.queue, "graphics");
        assert_eq!(presented.encoded.present_mode, "mailbox");
        assert!(presented.encoded.payload.contains("ngos-gfx-translate/v1"));
        assert!(
            presented
                .encoded
                .payload
                .contains("profile=browser-paint/framescript")
        );
    }

    #[test]
    fn present_rejects_invalid_framescript_output() {
        let mut renderer = FrameScriptRenderer::new(800, 600);
        renderer.output = String::from(
            "surface=800x600\nframe=broken-1\nqueue=graphics\npresent-mode=mailbox\ncompletion=wait-complete\n",
        );

        let err = renderer
            .present()
            .expect_err("present must reject a frame without draw ops");
        match err {
            BrowserError::Render(message) => {
                assert!(message.contains("missing field draw-op"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
        assert!(renderer.last_presented().is_none());
    }

    #[test]
    fn create_ascii_renderer() {
        let renderer = AsciiRenderer::new(80, 24);
        assert_eq!(renderer.width, 80);
        assert_eq!(renderer.height, 24);
    }
}
