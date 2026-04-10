//! Sidebar Renderer
//!
//! Renders the left sidebar (like Finder)

extern crate alloc;

use alloc::vec::Vec;
use ngos_gfx_translate::{DrawOp, RgbaColor};

/// Sidebar Item
#[derive(Debug, Clone)]
pub struct SidebarItem {
    pub icon: char,
    pub label: &'static str,
    pub active: bool,
}

/// Sidebar Renderer
pub struct Sidebar {
    width: u32,
    height: u32,
    items: Vec<SidebarItem>,
}

impl Sidebar {
    pub fn new(_width: u32, height: u32) -> Self {
        let mut sidebar = Sidebar {
            width: 280,
            height,
            items: Vec::new(),
        };

        // Add default items (like HTML preview)
        sidebar.items.push(SidebarItem {
            icon: '◆',
            label: "Dashboard",
            active: true,
        });
        sidebar.items.push(SidebarItem {
            icon: '◧',
            label: "Files",
            active: false,
        });
        sidebar.items.push(SidebarItem {
            icon: '◫',
            label: "Apps",
            active: false,
        });
        sidebar.items.push(SidebarItem {
            icon: '⚙',
            label: "Settings",
            active: false,
        });

        sidebar
    }

    fn scale(&self, value: u32, width: u32, height: u32) -> u32 {
        let scale = ((width as f32 / 1920.0).min(height as f32 / 1080.0)).max(0.7);
        (((value as f32) * scale) + 0.5) as u32
    }

    /// Render sidebar
    pub fn render(&self) -> Vec<DrawOp> {
        self.render_scaled(self.width, self.height)
    }

    /// Render sidebar with viewport scaling.
    pub fn render_scaled(&self, width: u32, height: u32) -> Vec<DrawOp> {
        let mut ops = Vec::new();
        let scale = ((width as f32 / 1920.0).min(height as f32 / 1080.0)).max(0.7);
        let top_bar_height = self.scale(48, width, height).max(36);
        let dock_height = self.scale(80, width, height).max(56);
        let sidebar_width = self.scale(self.width, width, height).max(220);
        let item_x = self.scale(10, width, height).max(8);
        let item_width = sidebar_width.saturating_sub(item_x * 2);
        let mut y = top_bar_height + self.scale(20, width, height).max(12);
        let item_height = self.scale(40, width, height).max(32);
        let item_gap = self.scale(50, width, height).max(item_height + 8);

        // Background (rgba(0x13, 0x1b, 0x28) from HTML)
        ops.push(DrawOp::Rect {
            x: 0,
            y: top_bar_height, // Below top bar
            width: sidebar_width,
            height: height.saturating_sub(top_bar_height + dock_height),
            color: RgbaColor {
                r: 0x13,
                g: 0x1b,
                b: 0x28,
                a: 0xff,
            },
        });

        // Render items
        for item in &self.items {
            // Item background (active state)
            let bg_color = if item.active {
                RgbaColor {
                    r: 0x00,
                    g: 0xd4,
                    b: 0xff,
                    a: 0x30,
                }
            } else {
                RgbaColor {
                    r: 0x00,
                    g: 0x00,
                    b: 0x00,
                    a: 0x00,
                }
            };

            ops.push(DrawOp::Rect {
                x: item_x,
                y,
                width: item_width,
                height: item_height,
                color: bg_color,
            });
            ops.push(DrawOp::Text {
                text: item.label.into(),
                x: item_x + 22,
                y: y + 12,
                size: (((12.0 * scale) + 0.5) as u32),
                color: RgbaColor {
                    r: 0xff,
                    g: 0xff,
                    b: 0xff,
                    a: 0xff,
                },
                font: ngos_gfx_translate::FontFamily::SansSerif,
            });

            y += item_gap;
        }

        ops
    }

    /// Get sidebar width
    pub fn width(&self) -> u32 {
        self.width
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidebar_creation() {
        let sidebar = Sidebar::new(1920, 1080);
        assert_eq!(sidebar.width(), 280);
        assert_eq!(sidebar.items.len(), 4);
    }

    #[test]
    fn sidebar_renders() {
        let sidebar = Sidebar::new(1920, 1080);
        let ops = sidebar.render();
        assert!(!ops.is_empty());
    }
}
