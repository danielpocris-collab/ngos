use alloc::vec::Vec;
use ngos_gfx_translate::{DrawOp, RgbaColor};

/// Start menu item
#[derive(Debug, Clone)]
pub struct StartMenuItem {
    pub id: u8,
    pub icon: char,
    pub label: &'static str,
}

/// Start Menu
pub struct StartMenu {
    width: u32,
    height: u32,
    visible: bool,
    items: Vec<StartMenuItem>,
}

impl StartMenu {
    pub fn new(width: u32, height: u32) -> Self {
        let mut menu = StartMenu {
            width,
            height,
            visible: false,
            items: Vec::new(),
        };

        // Add default items
        menu.items.push(StartMenuItem {
            id: 1,
            icon: '📁',
            label: "File Explorer",
        });
        menu.items.push(StartMenuItem {
            id: 2,
            icon: '🌐',
            label: "Browser",
        });
        menu.items.push(StartMenuItem {
            id: 3,
            icon: '🧮',
            label: "Calculator",
        });
        menu.items.push(StartMenuItem {
            id: 4,
            icon: '💻',
            label: "Terminal",
        });
        menu.items.push(StartMenuItem {
            id: 5,
            icon: '🎵',
            label: "Media",
        });
        menu.items.push(StartMenuItem {
            id: 6,
            icon: '⚙',
            label: "Settings",
        });

        menu
    }

    fn scale_value(value: u32, width: u32, height: u32) -> u32 {
        let scale = ((width as f32 / 1920.0).min(height as f32 / 1080.0)).max(0.7);
        (((value as f32) * scale) + 0.5) as u32
    }

    /// Toggle menu visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Check if menu is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Render start menu
    pub fn render(&self) -> Vec<DrawOp> {
        self.render_scaled(self.width, self.height)
    }

    pub fn render_scaled(&self, width: u32, height: u32) -> Vec<DrawOp> {
        if !self.visible {
            return Vec::new();
        }

        let mut ops = Vec::new();

        let menu_width = Self::scale_value(650, width, height).max(520);
        let menu_height = Self::scale_value(540, width, height).max(420);
        let bottom_offset = Self::scale_value(90, width, height).max(72);
        let x = width.saturating_sub(menu_width) / 2;
        let y = height.saturating_sub(menu_height + bottom_offset);

        // Menu background
        ops.push(DrawOp::Backdrop {
            x,
            y,
            width: menu_width,
            height: menu_height,
            opacity: 0x82,
        });
        ops.push(DrawOp::RoundedRect {
            x,
            y,
            width: menu_width,
            height: menu_height,
            radius: 20,
            color: RgbaColor {
                r: 0x1a,
                g: 0x1a,
                b: 0x2e,
                a: 0xee,
            },
        });
        ops.push(DrawOp::GaussianBlur {
            x: x + 4,
            y: y + 4,
            width: menu_width.saturating_sub(8),
            height: menu_height.saturating_sub(8),
            radius: 12,
        });

        ops.push(DrawOp::Text {
            text: "NGOS".into(),
            x: x + 30,
            y: y + 24,
            size: 22,
            color: RgbaColor {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            },
            font: ngos_gfx_translate::FontFamily::SansSerif,
        });
        ops.push(DrawOp::Text {
            text: "Complete Next Gen Desktop UI".into(),
            x: x + 30,
            y: y + 50,
            size: 12,
            color: RgbaColor {
                r: 0xc8,
                g: 0xd2,
                b: 0xe1,
                a: 0xff,
            },
            font: ngos_gfx_translate::FontFamily::SansSerif,
        });

        // Search bar
        let search_height = Self::scale_value(50, width, height).max(40);
        ops.push(DrawOp::RoundedRect {
            x: x + 30,
            y: y + 84,
            width: menu_width - 60,
            height: search_height,
            radius: 14,
            color: RgbaColor {
                r: 0x00,
                g: 0x00,
                b: 0x00,
                a: 0x30,
            },
        });
        ops.push(DrawOp::Text {
            text: "Search apps, files, settings...".into(),
            x: x + 48,
            y: y + 100,
            size: 13,
            color: RgbaColor {
                r: 0xa8,
                g: 0xb7,
                b: 0xc7,
                a: 0xff,
            },
            font: ngos_gfx_translate::FontFamily::SansSerif,
        });

        ops.push(DrawOp::Text {
            text: "Pinned".into(),
            x: x + 30,
            y: y + 154,
            size: 13,
            color: RgbaColor {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            },
            font: ngos_gfx_translate::FontFamily::SansSerif,
        });

        let grid_y = y + 178;
        let item_size = Self::scale_value(80, width, height).max(64);
        let gap = Self::scale_value(20, width, height).max(12);
        let label_size = Self::scale_value(11, width, height).max(10);

        for (i, item) in self.items.iter().enumerate() {
            let row = i / 4;
            let col = i % 4;
            let item_x = x + 30 + (col as u32 * (item_size + gap));
            let item_y = grid_y + (row as u32 * (item_size + gap));

            ops.push(DrawOp::RoundedRect {
                x: item_x,
                y: item_y,
                width: item_size,
                height: item_size,
                radius: 16,
                color: RgbaColor {
                    r: 0x30,
                    g: 0x30,
                    b: 0x50,
                    a: 0x50,
                },
            });
            ops.push(DrawOp::Icon {
                icon: item.icon,
                x: item_x + (item_size / 2).saturating_sub(14),
                y: item_y + 14,
                size: Self::scale_value(28, width, height).max(22),
                color: RgbaColor {
                    r: 0xff,
                    g: 0xff,
                    b: 0xff,
                    a: 0xff,
                },
            });
            ops.push(DrawOp::Text {
                text: item.label.into(),
                x: item_x + 10,
                y: item_y + item_size.saturating_sub(22),
                size: label_size,
                color: RgbaColor {
                    r: 0xff,
                    g: 0xff,
                    b: 0xff,
                    a: 0xff,
                },
                font: ngos_gfx_translate::FontFamily::SansSerif,
            });
        }

        ops.push(DrawOp::Text {
            text: "Recent".into(),
            x: x + 30,
            y: y + 280,
            size: 13,
            color: RgbaColor {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            },
            font: ngos_gfx_translate::FontFamily::SansSerif,
        });
        let recent = [
            "Project Proposal.docx",
            "Budget 2026.xlsx",
            "Design Mockup.png",
        ];
        for (i, item) in recent.iter().enumerate() {
            let top = y + 312 + (i as u32 * 44);
            ops.push(DrawOp::RoundedRect {
                x: x + 30,
                y: top,
                width: menu_width - 60,
                height: 36,
                radius: 10,
                color: RgbaColor {
                    r: 0x30,
                    g: 0x30,
                    b: 0x50,
                    a: 0x50,
                },
            });
            ops.push(DrawOp::Text {
                text: (*item).into(),
                x: x + 48,
                y: top + 10,
                size: 11,
                color: RgbaColor {
                    r: 0xff,
                    g: 0xff,
                    b: 0xff,
                    a: 0xff,
                },
                font: ngos_gfx_translate::FontFamily::SansSerif,
            });
        }

        ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_menu_creation() {
        let menu = StartMenu::new(1920, 1080);
        assert!(!menu.is_visible());
        assert!(!menu.items.is_empty());
    }

    #[test]
    fn start_menu_toggle() {
        let mut menu = StartMenu::new(1920, 1080);
        assert!(!menu.is_visible());
        menu.toggle();
        assert!(menu.is_visible());
        menu.toggle();
        assert!(!menu.is_visible());
    }

    #[test]
    fn start_menu_render_when_hidden() {
        let menu = StartMenu::new(1920, 1080);
        let ops = menu.render();
        assert!(ops.is_empty());
    }

    #[test]
    fn start_menu_render_when_visible() {
        let mut menu = StartMenu::new(1920, 1080);
        menu.toggle();
        let ops = menu.render();
        assert!(!ops.is_empty());
    }
}
