use alloc::{string::String, vec::Vec};
use ngos_compositor::{Surface, SurfaceRect, SurfaceRole};
use ngos_gfx_translate::{DrawOp, RgbaColor};
use ngos_input_translate::mouse_agent::MouseButton;

/// Window states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
}

/// UI Window
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UIWindow {
    pub id: u8,
    pub title: &'static str,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub state: WindowState,
    pub visible: bool,
}

impl UIWindow {
    pub fn new(id: u8, title: &'static str, x: i32, y: i32, width: u32, height: u32) -> Self {
        UIWindow {
            id,
            title,
            x,
            y,
            width,
            height,
            state: WindowState::Normal,
            visible: true,
        }
    }

    fn scaled_bounds(&self, viewport_width: u32, viewport_height: u32) -> (u32, u32, u32, u32) {
        let scale =
            ((viewport_width as f32 / 1920.0).min(viewport_height as f32 / 1080.0)).max(0.7);
        let taskbar_h = (((80.0 * scale) + 0.5) as u32).max(56);
        match self.state {
            WindowState::Maximized => (
                0,
                0,
                viewport_width,
                viewport_height.saturating_sub(taskbar_h),
            ),
            _ => (
                (((self.x as f32) * scale) + 0.5) as u32,
                (((self.y as f32) * scale) + 0.5) as u32,
                (((self.width as f32) * scale) + 0.5) as u32,
                (((self.height as f32) * scale) + 0.5) as u32,
            ),
        }
    }

    fn viewport_scale(viewport_width: u32, viewport_height: u32) -> f32 {
        ((viewport_width as f32 / 1920.0).min(viewport_height as f32 / 1080.0)).max(0.7)
    }

    fn scaled_metric(scale: f32, base: u32, minimum: u32) -> u32 {
        ((((base as f32) * scale) + 0.5) as u32).max(minimum)
    }

    pub fn content_ops_scaled(&self, viewport_width: u32, viewport_height: u32) -> Vec<DrawOp> {
        let mut ops = Vec::new();

        if !self.visible || self.state == WindowState::Minimized {
            return ops;
        }

        let scale = Self::viewport_scale(viewport_width, viewport_height);
        let (x, y, w, h) = self.scaled_bounds(viewport_width, viewport_height);
        let title_height = Self::scaled_metric(scale, 42, 30);
        let inner_margin = Self::scaled_metric(scale, 16, 10);
        let panel_margin = Self::scaled_metric(scale, 28, 16);
        let panel_height = Self::scaled_metric(scale, 36, 26);
        let panel_radius = Self::scaled_metric(scale, 10, 7);
        let shadow_radius = Self::scaled_metric(scale, 14, 10);
        let label_size = Self::scaled_metric(scale, 13, 11);

        ops.push(DrawOp::RoundedRect {
            x: x + inner_margin,
            y: y + title_height + inner_margin,
            width: w.saturating_sub(inner_margin * 2),
            height: h.saturating_sub(title_height + (inner_margin * 2)),
            radius: shadow_radius,
            color: RgbaColor {
                r: 0x00,
                g: 0x00,
                b: 0x00,
                a: 0x20,
            },
        });

        ops.push(DrawOp::RoundedRect {
            x: x + panel_margin,
            y: y + title_height + (inner_margin * 2),
            width: w.saturating_sub(panel_margin * 2),
            height: panel_height,
            radius: panel_radius,
            color: RgbaColor {
                r: 0x30,
                g: 0x30,
                b: 0x50,
                a: 0x36,
            },
        });

        ops.push(DrawOp::Text {
            text: self.title.into(),
            x: x + panel_margin + Self::scaled_metric(scale, 2, 2),
            y: y + title_height + (inner_margin * 2) + Self::scaled_metric(scale, 12, 10),
            size: label_size,
            color: RgbaColor {
                r: 0xe2,
                g: 0xe8,
                b: 0xf0,
                a: 0xff,
            },
            font: ngos_gfx_translate::FontFamily::SansSerif,
        });

        ops
    }

    /// Render window
    pub fn render(&self) -> Vec<DrawOp> {
        self.render_scaled(1920, 1080)
    }

    /// Render window for a given viewport.
    pub fn render_scaled(&self, viewport_width: u32, viewport_height: u32) -> Vec<DrawOp> {
        let mut ops = Vec::new();

        if !self.visible || self.state == WindowState::Minimized {
            return ops;
        }

        let scale = Self::viewport_scale(viewport_width, viewport_height);
        let (x, y, w, h) = self.scaled_bounds(viewport_width, viewport_height);
        let shadow_margin = Self::scaled_metric(scale, 8, 6);
        let shadow_blur = Self::scaled_metric(scale, 22, 16);
        let corner_radius = Self::scaled_metric(scale, 16, 12);
        let title_height = Self::scaled_metric(scale, 42, 30);
        let blur_radius = Self::scaled_metric(scale, 8, 6);
        let title_text_inset_x = Self::scaled_metric(scale, 18, 12);
        let title_text_inset_y = Self::scaled_metric(scale, 14, 10);
        let title_text_size = Self::scaled_metric(scale, 14, 11);

        ops.push(DrawOp::ShadowRect {
            x: x.saturating_sub(shadow_margin),
            y: y.saturating_sub(shadow_margin),
            width: w.saturating_add(shadow_margin * 2),
            height: h.saturating_add(shadow_margin * 2),
            blur: shadow_blur,
            color: RgbaColor {
                r: 0x00,
                g: 0x00,
                b: 0x00,
                a: 0x2A,
            },
        });
        ops.push(DrawOp::Backdrop {
            x,
            y,
            width: w,
            height: h,
            opacity: 0x7A,
        });

        // Window background
        ops.push(DrawOp::RoundedRect {
            x,
            y,
            width: w,
            height: h,
            radius: if self.state == WindowState::Maximized {
                0
            } else {
                corner_radius
            },
            color: RgbaColor {
                r: 0x16,
                g: 0x21,
                b: 0x3e,
                a: 0xee,
            },
        });

        // Title bar
        ops.push(DrawOp::Rect {
            x,
            y,
            width: w,
            height: title_height,
            color: RgbaColor {
                r: 0x00,
                g: 0x00,
                b: 0x00,
                a: 0x30,
            },
        });
        ops.push(DrawOp::GaussianBlur {
            x,
            y,
            width: w,
            height: title_height,
            radius: blur_radius,
        });
        ops.push(DrawOp::Rect {
            x,
            y: y + 1,
            width: w,
            height: 1,
            color: RgbaColor {
                r: 0x73,
                g: 0xd5,
                b: 0xff,
                a: 0x40,
            },
        });

        ops.push(DrawOp::Rect {
            x,
            y: y + title_height - 1,
            width: w,
            height: 1,
            color: RgbaColor {
                r: 0x30,
                g: 0x30,
                b: 0x50,
                a: 0xff,
            },
        });

        ops.push(DrawOp::Text {
            text: self.title.into(),
            x: x + title_text_inset_x,
            y: y + title_text_inset_y,
            size: title_text_size,
            color: RgbaColor {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            },
            font: ngos_gfx_translate::FontFamily::SansSerif,
        });

        // Window controls (close, minimize, maximize)
        let control_size = Self::scaled_metric(scale, 14, 10);
        let control_gap = Self::scaled_metric(scale, 8, 5);
        let controls_inset = Self::scaled_metric(scale, 14, 10);
        let control_radius = control_size / 2;
        let controls_x = x + w - control_size - controls_inset;
        let controls_y = y + (title_height - control_size) / 2;

        // Close button (red)
        ops.push(DrawOp::RoundedRect {
            x: controls_x,
            y: controls_y,
            width: control_size,
            height: control_size,
            radius: control_radius,
            color: RgbaColor {
                r: 0xff,
                g: 0x5f,
                b: 0x57,
                a: 0xff,
            },
        });

        // Maximize button (green)
        ops.push(DrawOp::RoundedRect {
            x: controls_x - control_size - control_gap,
            y: controls_y,
            width: control_size,
            height: control_size,
            radius: control_radius,
            color: RgbaColor {
                r: 0x28,
                g: 0xca,
                b: 0x41,
                a: 0xff,
            },
        });

        // Minimize button (yellow)
        ops.push(DrawOp::RoundedRect {
            x: controls_x - (control_size + control_gap) * 2,
            y: controls_y,
            width: control_size,
            height: control_size,
            radius: control_radius,
            color: RgbaColor {
                r: 0xff,
                g: 0xbd,
                b: 0x2e,
                a: 0xff,
            },
        });

        ops.extend(self.content_ops_scaled(viewport_width, viewport_height));

        ops
    }

    pub fn as_surface(&self, viewport_width: u32, viewport_height: u32) -> Option<Surface> {
        if !self.visible || self.state == WindowState::Minimized {
            return None;
        }
        let (x, y, width, height) = self.scaled_bounds(viewport_width, viewport_height);
        let mut surface = Surface::new(
            1_000 + self.id as u32,
            SurfaceRole::Window,
            SurfaceRect {
                x,
                y,
                width,
                height,
            },
        )
        .ok()?;
        surface.focused = true;
        surface.title = Some(self.title.into());
        surface.pass_name = String::from("window-surface");
        surface.content = self.content_ops_scaled(viewport_width, viewport_height);
        Some(surface)
    }
}

/// Window Manager
pub struct UIManager {
    pub windows: Vec<UIWindow>,
    pub active_window: Option<u8>,
    width: u32,
    height: u32,
}

impl UIManager {
    pub fn new(width: u32, height: u32) -> Self {
        UIManager {
            windows: Vec::new(),
            active_window: None,
            width,
            height,
        }
    }

    /// Open a new window
    pub fn open_window(&mut self, app_id: u8) {
        let title = match app_id {
            1 => "File Explorer",
            2 => "Browser",
            3 => "Calculator",
            4 => "Terminal",
            5 => "Media Player",
            6 => "Settings",
            _ => "Application",
        };

        let window = UIWindow::new(
            app_id,
            title,
            (self.width as i32 / 10) + (self.windows.len() as i32 * 26),
            (self.height as i32 / 10) + (self.windows.len() as i32 * 26),
            (self.width * 3) / 8,
            (self.height * 7) / 15,
        );

        self.windows.push(window);
        self.active_window = Some(app_id);
    }

    /// Render all windows
    pub fn render_all(&self) -> Vec<DrawOp> {
        let mut ops = Vec::new();
        for window in &self.windows {
            ops.extend(window.render());
        }
        ops
    }

    pub fn render_all_scaled(&self, width: u32, height: u32) -> Vec<DrawOp> {
        let mut ops = Vec::new();
        for window in &self.windows {
            ops.extend(window.render_scaled(width, height));
        }
        ops
    }

    pub fn surface_list_scaled(&self, width: u32, height: u32) -> Vec<Surface> {
        let mut surfaces = Vec::new();
        for window in &self.windows {
            if let Some(mut surface) = window.as_surface(width, height) {
                surface.focused = self.active_window == Some(window.id);
                surfaces.push(surface);
            }
        }
        surfaces
    }

    /// Handle mouse input
    pub fn handle_mouse(&mut self, _x: i32, _y: i32, _button: MouseButton, _pressed: bool) {
        // Simplified - would handle window dragging in real implementation
    }

    /// Update windows
    pub fn update(&mut self, _delta_ms: u32) {
        // Animation updates would go here
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_creation() {
        let window = UIWindow::new(1, "Test", 100, 100, 400, 300);
        assert_eq!(window.id, 1);
        assert_eq!(window.title, "Test");
    }

    #[test]
    fn window_renders() {
        let window = UIWindow::new(1, "Test", 100, 100, 400, 300);
        let ops = window.render();
        assert!(!ops.is_empty());
    }

    #[test]
    fn scaled_window_metrics_shrink_for_small_viewports() {
        let window = UIWindow::new(1, "Test", 100, 100, 400, 300);
        let ops = window.render_scaled(1280, 720);

        let title_bar = ops
            .iter()
            .find_map(|op| match op {
                DrawOp::Rect {
                    x,
                    y,
                    width,
                    height,
                    ..
                } if *x == 70 && *y == 70 && *width == 280 => Some(*height),
                _ => None,
            })
            .expect("scaled title bar");
        assert_eq!(title_bar, 30);

        let title_text_size = ops
            .iter()
            .find_map(|op| match op {
                DrawOp::Text { size, .. } => Some(*size),
                _ => None,
            })
            .expect("scaled title text");
        assert_eq!(title_text_size, 11);
    }

    #[test]
    fn render_all_scaled_does_not_apply_window_scaling_twice() {
        let mut wm = UIManager::new(1920, 1080);
        wm.windows
            .push(UIWindow::new(1, "Test", 100, 100, 400, 300));

        let ops = wm.render_all_scaled(1280, 720);
        let title_bar = ops
            .iter()
            .find_map(|op| match op {
                DrawOp::Rect {
                    x,
                    y,
                    width,
                    height,
                    ..
                } if *x == 70 && *y == 70 => Some((*width, *height)),
                _ => None,
            })
            .expect("single-scaled title bar");

        assert_eq!(title_bar, (280, 30));
    }

    #[test]
    fn window_manager_creation() {
        let wm = UIManager::new(1920, 1080);
        assert!(wm.windows.is_empty());
    }

    #[test]
    fn window_manager_opens_window() {
        let mut wm = UIManager::new(1920, 1080);
        wm.open_window(1);
        assert_eq!(wm.windows.len(), 1);
    }
}
