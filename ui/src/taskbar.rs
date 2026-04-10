use alloc::vec::Vec;
use ngos_gfx_translate::{DrawOp, RgbaColor};

use ngos_input_translate::mouse_agent::MouseButton;

/// Taskbar item
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskbarItem {
    pub id: u8,
    pub icon: char,
    pub tooltip: &'static str,
    pub active: bool,
    pub pinned: bool,
}

/// Taskbar Renderer
pub struct Taskbar {
    width: u32,
    height: u32,
    items: Vec<TaskbarItem>,
}

impl Taskbar {
    pub fn new(width: u32, _height: u32) -> Self {
        let mut taskbar = Taskbar {
            width,
            height: 80,
            items: Vec::new(),
        };

        // Add default items
        taskbar.items.push(TaskbarItem {
            id: 0,
            icon: '🏠',
            tooltip: "Start",
            active: false,
            pinned: false,
        });
        taskbar.items.push(TaskbarItem {
            id: 1,
            icon: '📁',
            tooltip: "File Explorer",
            active: false,
            pinned: true,
        });
        taskbar.items.push(TaskbarItem {
            id: 2,
            icon: '🌐',
            tooltip: "Browser",
            active: false,
            pinned: true,
        });
        taskbar.items.push(TaskbarItem {
            id: 3,
            icon: '🧮',
            tooltip: "Calculator",
            active: false,
            pinned: true,
        });
        taskbar.items.push(TaskbarItem {
            id: 4,
            icon: '💻',
            tooltip: "Terminal",
            active: false,
            pinned: true,
        });
        taskbar.items.push(TaskbarItem {
            id: 5,
            icon: '🎵',
            tooltip: "Media",
            active: false,
            pinned: true,
        });

        taskbar
    }

    /// Render taskbar
    pub fn render(&self) -> Vec<DrawOp> {
        self.render_scaled(self.width, self.height)
    }

    pub fn render_scaled(&self, width: u32, height: u32) -> Vec<DrawOp> {
        let mut ops = Vec::new();

        let scale = ((width as f32 / 1920.0).min(height as f32 / 1080.0)).max(0.7);
        let taskbar_h = (((self.height as f32 * scale) + 0.5) as u32).max(56);
        let y = height.saturating_sub(taskbar_h);

        // Taskbar background
        ops.push(DrawOp::Rect {
            x: 0,
            y: y.saturating_sub(((8.0 * scale) + 0.5) as u32),
            width,
            height: (((8.0 * scale) + 0.5) as u32),
            color: RgbaColor {
                r: 0x00,
                g: 0xd4,
                b: 0xff,
                a: 0x18,
            },
        });
        ops.push(DrawOp::GradientRect {
            x: 0,
            y,
            width,
            height: taskbar_h,
            top_left: RgbaColor {
                r: 0x1a,
                g: 0x1a,
                b: 0x2e,
                a: 0xcc,
            },
            top_right: RgbaColor {
                r: 0x16,
                g: 0x21,
                b: 0x3e,
                a: 0xcc,
            },
            bottom_left: RgbaColor {
                r: 0x16,
                g: 0x21,
                b: 0x3e,
                a: 0xcc,
            },
            bottom_right: RgbaColor {
                r: 0x1a,
                g: 0x1a,
                b: 0x2e,
                a: 0xcc,
            },
        });
        ops.push(DrawOp::Backdrop {
            x: 0,
            y,
            width,
            height: taskbar_h,
            opacity: 0x68,
        });
        ops.push(DrawOp::Rect {
            x: 0,
            y,
            width,
            height: taskbar_h,
            color: RgbaColor {
                r: 0x00,
                g: 0x00,
                b: 0x00,
                a: 0x12,
            },
        });

        // Render items
        let item_width = ((56.0 * scale) + 0.5) as u32;
        let gap = ((8.0 * scale) + 0.5) as u32;
        let start_x = ((82.0 * scale) + 0.5) as u32; // After start button

        for (i, item) in self.items.iter().enumerate() {
            let x = start_x + (i as u32 * (item_width + gap));

            // Item background
            let bg_color = if item.active {
                RgbaColor {
                    r: 0x30,
                    g: 0x30,
                    b: 0x50,
                    a: 0xff,
                }
            } else {
                RgbaColor {
                    r: 0x00,
                    g: 0x00,
                    b: 0x00,
                    a: 0x00,
                }
            };

            ops.push(DrawOp::RoundedRect {
                x,
                y: y + (((13.0 * scale) + 0.5) as u32),
                width: item_width,
                height: (((54.0 * scale) + 0.5) as u32),
                radius: (((14.0 * scale) + 0.5) as u32),
                color: bg_color,
            });
            if item.pinned {
                ops.push(DrawOp::Rect {
                    x,
                    y: y + (((66.0 * scale) + 0.5) as u32),
                    width: item_width,
                    height: (((2.0 * scale) + 0.5) as u32),
                    color: RgbaColor {
                        r: 0x00,
                        g: 0xd4,
                        b: 0xff,
                        a: 0x42,
                    },
                });
            }

            ops.push(DrawOp::Icon {
                icon: item.icon,
                x: x + (((16.0 * scale) + 0.5) as u32),
                y: y + (((19.0 * scale) + 0.5) as u32),
                size: (((24.0 * scale) + 0.5) as u32),
                color: RgbaColor {
                    r: 0xff,
                    g: 0xff,
                    b: 0xff,
                    a: 0xff,
                },
            });

            if item.pinned {
                ops.push(DrawOp::Text {
                    text: item.tooltip.into(),
                    x: x.saturating_sub(((2.0 * scale) + 0.5) as u32),
                    y: y + (((68.0 * scale) + 0.5) as u32),
                    size: (((10.0 * scale) + 0.5) as u32),
                    color: RgbaColor {
                        r: 0xc8,
                        g: 0xd2,
                        b: 0xe1,
                        a: 0xff,
                    },
                    font: ngos_gfx_translate::FontFamily::SansSerif,
                });
            }

            // Active indicator
            if item.active {
                ops.push(DrawOp::RoundedRect {
                    x: x + item_width / 2 - 3,
                    y: y + (((62.0 * scale) + 0.5) as u32),
                    width: (((6.0 * scale) + 0.5) as u32),
                    height: (((6.0 * scale) + 0.5) as u32),
                    radius: (((3.0 * scale) + 0.5) as u32),
                    color: RgbaColor {
                        r: 0x00,
                        g: 0xd4,
                        b: 0xff,
                        a: 0xff,
                    },
                });
            }
        }

        ops.push(DrawOp::RoundedRect {
            x: width.saturating_sub(((196.0 * scale) + 0.5) as u32),
            y: y + (((15.0 * scale) + 0.5) as u32),
            width: (((166.0 * scale) + 0.5) as u32),
            height: (((48.0 * scale) + 0.5) as u32),
            radius: (((14.0 * scale) + 0.5) as u32),
            color: RgbaColor {
                r: 0x00,
                g: 0x00,
                b: 0x00,
                a: 0x24,
            },
        });
        ops.push(DrawOp::Rect {
            x: width.saturating_sub(((196.0 * scale) + 0.5) as u32),
            y: y + (((15.0 * scale) + 0.5) as u32),
            width: (((166.0 * scale) + 0.5) as u32),
            height: 1,
            color: RgbaColor {
                r: 0x00,
                g: 0xd4,
                b: 0xff,
                a: 0x30,
            },
        });
        ops.push(DrawOp::GaussianBlur {
            x: width.saturating_sub(((196.0 * scale) + 0.5) as u32),
            y: y + (((15.0 * scale) + 0.5) as u32),
            width: (((166.0 * scale) + 0.5) as u32),
            height: (((48.0 * scale) + 0.5) as u32),
            radius: 6,
        });
        ops.push(DrawOp::Text {
            text: "12:00 PM".into(),
            x: width.saturating_sub(((176.0 * scale) + 0.5) as u32),
            y: y + (((25.0 * scale) + 0.5) as u32),
            size: (((16.0 * scale) + 0.5) as u32),
            color: RgbaColor {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            },
            font: ngos_gfx_translate::FontFamily::SansSerif,
        });
        ops.push(DrawOp::Text {
            text: "3/31/2026".into(),
            x: width.saturating_sub(((176.0 * scale) + 0.5) as u32),
            y: y + (((42.0 * scale) + 0.5) as u32),
            size: (((11.0 * scale) + 0.5) as u32),
            color: RgbaColor {
                r: 0xc8,
                g: 0xd2,
                b: 0xe1,
                a: 0xff,
            },
            font: ngos_gfx_translate::FontFamily::SansSerif,
        });

        ops
    }

    /// Handle mouse click on taskbar
    pub fn handle_click(
        &self,
        x: i32,
        y: i32,
        _button: MouseButton,
        _pressed: bool,
    ) -> Option<super::TaskbarAction> {
        // Check if click is in taskbar area
        let scale = ((self.width as f32 / 1920.0).min(self.height as f32 / 1080.0)).max(0.7);
        let taskbar_h = ((self.height as f32 * scale) + 0.5) as i32;
        let taskbar_top = self.height as i32 - taskbar_h;
        if y >= taskbar_top {
            // Check start button
            let start_button_w = ((50.0 * scale) + 0.5) as i32;
            let start_button_x = ((10.0 * scale) + 0.5) as i32;
            if x >= start_button_x && x <= start_button_x + start_button_w {
                return Some(super::TaskbarAction::OpenStart);
            }

            // Check app items
            let item_width = ((56.0 * scale) + 0.5) as i32;
            let gap = ((8.0 * scale) + 0.5) as i32;
            let start_x = ((82.0 * scale) + 0.5) as i32;

            for (i, item) in self.items.iter().enumerate() {
                let item_x = start_x + (i as i32 * (item_width + gap));
                if x >= item_x && x <= item_x + item_width as i32 {
                    return Some(super::TaskbarAction::OpenApp(item.id));
                }
            }
        }

        None
    }

    /// Get taskbar height
    pub fn height(&self) -> u32 {
        self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taskbar_creation() {
        let taskbar = Taskbar::new(1920, 1080);
        assert_eq!(taskbar.height(), 80);
        assert!(!taskbar.items.is_empty());
    }

    #[test]
    fn taskbar_renders() {
        let taskbar = Taskbar::new(1920, 1080);
        let ops = taskbar.render();
        assert!(!ops.is_empty());
    }
}
