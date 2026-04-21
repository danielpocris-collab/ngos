//! Top Bar Renderer
//!
//! Renders the top menu bar (like macOS)

extern crate alloc;

use alloc::{vec, vec::Vec};
use ngos_gfx_translate::{DrawOp, RgbaColor};

/// Top Bar Menu Item
#[derive(Debug, Clone)]
pub struct TopBarMenu {
    pub label: &'static str,
    pub items: Vec<&'static str>,
}

/// Top Bar Renderer
pub struct TopBar {
    width: u32,
    height: u32,
    menus: Vec<TopBarMenu>,
}

impl TopBar {
    pub fn new(width: u32, _height: u32) -> Self {
        let mut topbar = TopBar {
            width,
            height: 48,
            menus: Vec::new(),
        };

        // Add default menus (like HTML preview)
        topbar.menus.push(TopBarMenu {
            label: "File",
            items: vec!["New", "Open", "Save", "Close"],
        });
        topbar.menus.push(TopBarMenu {
            label: "Edit",
            items: vec!["Undo", "Redo", "Cut", "Copy", "Paste"],
        });
        topbar.menus.push(TopBarMenu {
            label: "View",
            items: vec!["Zoom In", "Zoom Out", "Full Screen"],
        });
        topbar.menus.push(TopBarMenu {
            label: "Window",
            items: vec!["Minimize", "Maximize", "Close"],
        });
        topbar.menus.push(TopBarMenu {
            label: "Help",
            items: vec!["About NGOS", "Documentation", "Check for Updates"],
        });

        topbar
    }

    fn scale(&self, value: u32, width: u32, height: u32) -> u32 {
        let scale = ((width as f32 / 1920.0).min(height as f32 / 1080.0)).max(0.7);
        (((value as f32) * scale) + 0.5) as u32
    }

    /// Render top bar
    pub fn render(&self) -> Vec<DrawOp> {
        self.render_scaled(self.width, self.height)
    }

    /// Render top bar with viewport scaling.
    pub fn render_scaled(&self, width: u32, height: u32) -> Vec<DrawOp> {
        let mut ops = Vec::new();
        let bar_height = self.scale(self.height, width, height).max(36);
        let scale = ((width as f32 / 1920.0).min(height as f32 / 1080.0)).max(0.7);

        // Background (rgba(0x1a, 0x23, 0x33) from HTML)
        ops.push(DrawOp::Rect {
            x: 0,
            y: 0,
            width,
            height: bar_height,
            color: RgbaColor {
                r: 0x1a,
                g: 0x23,
                b: 0x33,
                a: 0xff,
            },
        });

        // Render menus
        let mut x = self.scale(80, width, height).max(56); // After NGOS logo
        let menu_width = self.scale(60, width, height).max(44);
        let menu_gap = self.scale(70, width, height).max(menu_width + 8);
        for menu in &self.menus {
            // Menu background (hover effect would go here)
            ops.push(DrawOp::Rect {
                x,
                y: 0,
                width: menu_width,
                height: bar_height,
                color: RgbaColor {
                    r: 0x00,
                    g: 0x00,
                    b: 0x00,
                    a: 0x00,
                },
            });
            ops.push(DrawOp::Text {
                text: menu.label.into(),
                x: x + 10,
                y: 10,
                size: (((12.0 * scale) + 0.5) as u32),
                color: RgbaColor {
                    r: 0xff,
                    g: 0xff,
                    b: 0xff,
                    a: 0xff,
                },
                font: ngos_gfx_translate::FontFamily::SansSerif,
            });

            x += menu_gap;
        }

        ops
    }

    /// Get top bar height
    pub fn height(&self) -> u32 {
        self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topbar_creation() {
        let topbar = TopBar::new(1920, 1080);
        assert_eq!(topbar.height(), 48);
        assert_eq!(topbar.menus.len(), 5);
    }

    #[test]
    fn topbar_renders() {
        let topbar = TopBar::new(1920, 1080);
        let ops = topbar.render();
        assert!(!ops.is_empty());
    }
}
