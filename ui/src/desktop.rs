//! Desktop Renderer
//!
//! Renders the desktop surface, shortcuts, and status affordances.

extern crate alloc;

use alloc::vec::Vec;
use ngos_gfx_translate::{DrawOp, RgbaColor};

/// Desktop Icon
#[derive(Debug, Clone)]
pub struct DesktopIcon {
    pub id: u8,
    pub icon: char,
    pub label: &'static str,
    pub x: u32,
    pub y: u32,
    pub accent: RgbaColor,
}

/// Desktop Renderer
pub struct Desktop {
    width: u32,
    height: u32,
    icons: Vec<DesktopIcon>,
}

impl Desktop {
    pub fn new(width: u32, height: u32) -> Self {
        let mut desktop = Desktop {
            width,
            height,
            icons: Vec::new(),
        };

        // Left column shortcuts, matching a real desktop launch surface.
        desktop.icons.push(DesktopIcon {
            id: 1,
            icon: '📁',
            label: "File Explorer",
            x: 42,
            y: 120,
            accent: RgbaColor {
                r: 0x00,
                g: 0xd4,
                b: 0xff,
                a: 0xff,
            },
        });
        desktop.icons.push(DesktopIcon {
            id: 2,
            icon: '🌐',
            label: "Browser",
            x: 42,
            y: 210,
            accent: RgbaColor {
                r: 0x7b,
                g: 0x2c,
                b: 0xbf,
                a: 0xff,
            },
        });
        desktop.icons.push(DesktopIcon {
            id: 3,
            icon: '🧮',
            label: "Calculator",
            x: 42,
            y: 300,
            accent: RgbaColor {
                r: 0x2e,
                g: 0xd5,
                b: 0x73,
                a: 0xff,
            },
        });
        desktop.icons.push(DesktopIcon {
            id: 4,
            icon: '💻',
            label: "Terminal",
            x: 42,
            y: 390,
            accent: RgbaColor {
                r: 0xff,
                g: 0xa5,
                b: 0x02,
                a: 0xff,
            },
        });
        desktop.icons.push(DesktopIcon {
            id: 5,
            icon: '🎵',
            label: "Media",
            x: 42,
            y: 480,
            accent: RgbaColor {
                r: 0xfb,
                g: 0x56,
                b: 0x07,
                a: 0xff,
            },
        });
        desktop.icons.push(DesktopIcon {
            id: 6,
            icon: '⚙',
            label: "Settings",
            x: 42,
            y: 570,
            accent: RgbaColor {
                r: 0xec,
                g: 0x48,
                b: 0x99,
                a: 0xff,
            },
        });

        desktop
    }

    /// Render desktop background
    pub fn render_background(&self) -> Vec<DrawOp> {
        self.render_background_scaled(self.width, self.height)
    }

    pub fn render_background_scaled(&self, width: u32, height: u32) -> Vec<DrawOp> {
        let mut ops = Vec::new();

        // Main atmospheric gradient.
        ops.push(DrawOp::GradientRect {
            x: 0,
            y: 0,
            width,
            height,
            top_left: RgbaColor {
                r: 0x0a,
                g: 0x0a,
                b: 0x0f,
                a: 0xff,
            },
            top_right: RgbaColor {
                r: 0x1a,
                g: 0x1a,
                b: 0x2e,
                a: 0xff,
            },
            bottom_left: RgbaColor {
                r: 0x16,
                g: 0x21,
                b: 0x3e,
                a: 0xff,
            },
            bottom_right: RgbaColor {
                r: 0x0a,
                g: 0x0a,
                b: 0x0f,
                a: 0xff,
            },
        });

        // Secondary depth layer so the desktop does not feel flat.
        ops.push(DrawOp::GradientRect {
            x: 0,
            y: 0,
            width,
            height,
            top_left: RgbaColor {
                r: 0x00,
                g: 0xd4,
                b: 0xff,
                a: 0x12,
            },
            top_right: RgbaColor {
                r: 0x7b,
                g: 0x2c,
                b: 0xbf,
                a: 0x10,
            },
            bottom_left: RgbaColor {
                r: 0x0f,
                g: 0x20,
                b: 0x27,
                a: 0x08,
            },
            bottom_right: RgbaColor {
                r: 0x2c,
                g: 0x53,
                b: 0x64,
                a: 0x08,
            },
        });

        // Floating color orbs to mimic the more atmospheric HTML preview.
        let orb_specs = [
            (
                width / 8,
                height / 8,
                width / 3,
                height / 3,
                RgbaColor {
                    r: 0x00,
                    g: 0xd4,
                    b: 0xff,
                    a: 0x34,
                },
            ),
            (
                width.saturating_sub(width / 4),
                height / 6,
                width / 4,
                height / 4,
                RgbaColor {
                    r: 0x7b,
                    g: 0x2c,
                    b: 0xbf,
                    a: 0x2e,
                },
            ),
            (
                width / 2,
                height / 2,
                width / 4,
                height / 4,
                RgbaColor {
                    r: 0x00,
                    g: 0xf5,
                    b: 0xd4,
                    a: 0x26,
                },
            ),
            (
                width.saturating_sub(width / 3),
                height.saturating_sub(height / 4),
                width / 4,
                height / 4,
                RgbaColor {
                    r: 0xfb,
                    g: 0x56,
                    b: 0x07,
                    a: 0x28,
                },
            ),
        ];
        for (x, y, w, h, color) in orb_specs {
            ops.push(DrawOp::RoundedRect {
                x: x.saturating_sub(w / 2),
                y: y.saturating_sub(h / 2),
                width: w,
                height: h,
                radius: w.max(h) / 2,
                color,
            });
        }

        // Subtle grain layer for depth.
        let grain_step = (width.max(height) / 28).max(24);
        let mut gx = 0;
        while gx < width {
            let mut gy = 0;
            while gy < height {
                let alpha = if (gx / grain_step + gy / grain_step) % 2 == 0 {
                    0x05
                } else {
                    0x03
                };
                ops.push(DrawOp::Rect {
                    x: gx,
                    y: gy,
                    width: 2,
                    height: 2,
                    color: RgbaColor {
                        r: 0xff,
                        g: 0xff,
                        b: 0xff,
                        a: alpha,
                    },
                });
                gy = gy.saturating_add(grain_step);
            }
            gx = gx.saturating_add(grain_step);
        }

        // Backdrop-style card behind the desktop icon column.
        ops.push(DrawOp::RoundedRect {
            x: 24,
            y: 92,
            width: 140,
            height: height.saturating_sub(200),
            radius: 20,
            color: RgbaColor {
                r: 0x13,
                g: 0x1b,
                b: 0x28,
                a: 0xcc,
            },
        });

        // Hero card on the right side to give the desktop a clear focal point.
        let card_x = width.saturating_sub(390);
        ops.push(DrawOp::RoundedRect {
            x: card_x,
            y: 92,
            width: 340,
            height: 220,
            radius: 24,
            color: RgbaColor {
                r: 0x16,
                g: 0x21,
                b: 0x3e,
                a: 0xdd,
            },
        });
        ops.push(DrawOp::GradientRect {
            x: card_x + 20,
            y: 112,
            width: 120,
            height: 120,
            top_left: RgbaColor {
                r: 0x00,
                g: 0xd4,
                b: 0xff,
                a: 0xff,
            },
            top_right: RgbaColor {
                r: 0x7b,
                g: 0x2c,
                b: 0xbf,
                a: 0xff,
            },
            bottom_left: RgbaColor {
                r: 0x7b,
                g: 0x2c,
                b: 0xbf,
                a: 0xff,
            },
            bottom_right: RgbaColor {
                r: 0x00,
                g: 0xd4,
                b: 0xff,
                a: 0xff,
            },
        });
        ops.push(DrawOp::Text {
            text: "NGOS Desktop".into(),
            x: card_x + 160,
            y: 126,
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
            text: "Pinned apps, taskbar, widgets".into(),
            x: card_x + 160,
            y: 162,
            size: 12,
            color: RgbaColor {
                r: 0xc8,
                g: 0xd2,
                b: 0xe1,
                a: 0xff,
            },
            font: ngos_gfx_translate::FontFamily::SansSerif,
        });
        ops.push(DrawOp::Text {
            text: "Windows and settings".into(),
            x: card_x + 160,
            y: 188,
            size: 11,
            color: RgbaColor {
                r: 0xa8,
                g: 0xb7,
                b: 0xc7,
                a: 0xff,
            },
            font: ngos_gfx_translate::FontFamily::SansSerif,
        });

        // Bottom-right status cluster.
        ops.push(DrawOp::RoundedRect {
            x: width.saturating_sub(270),
            y: height.saturating_sub(180),
            width: 240,
            height: 90,
            radius: 18,
            color: RgbaColor {
                r: 0x1a,
                g: 0x1a,
                b: 0x2e,
                a: 0xcc,
            },
        });
        ops.push(DrawOp::Text {
            text: "22° Partly Cloudy".into(),
            x: width.saturating_sub(248),
            y: height.saturating_sub(148),
            size: 16,
            color: RgbaColor {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            },
            font: ngos_gfx_translate::FontFamily::SansSerif,
        });
        ops.push(DrawOp::Text {
            text: "Bucharest, Romania".into(),
            x: width.saturating_sub(248),
            y: height.saturating_sub(122),
            size: 12,
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

    /// Render desktop icons
    pub fn render_icons(&self) -> Vec<DrawOp> {
        self.render_icons_scaled(self.width, self.height)
    }

    pub fn render_icons_scaled(&self, width: u32, height: u32) -> Vec<DrawOp> {
        let mut ops = Vec::new();
        let scale = ((width as f32 / 1920.0).min(height as f32 / 1080.0)).max(0.7);
        let icon_x = ((42.0 * scale) + 0.5) as u32;
        let mut icon_y = ((120.0 * scale) + 0.5) as u32;
        let gap_y = ((90.0 * scale) + 0.5) as u32;

        for icon in &self.icons {
            // Icon body
            ops.push(DrawOp::RoundedRect {
                x: icon_x,
                y: icon_y,
                width: (((56.0 * scale) + 0.5) as u32),
                height: (((56.0 * scale) + 0.5) as u32),
                radius: 16,
                color: RgbaColor {
                    r: 0x30,
                    g: 0x30,
                    b: 0x50,
                    a: 0x72,
                },
            });

            ops.push(DrawOp::RoundedRect {
                x: icon_x + (((8.0 * scale) + 0.5) as u32),
                y: icon_y + (((8.0 * scale) + 0.5) as u32),
                width: (((40.0 * scale) + 0.5) as u32),
                height: (((40.0 * scale) + 0.5) as u32),
                radius: 12,
                color: icon.accent,
            });

            ops.push(DrawOp::Icon {
                icon: icon.icon,
                x: icon_x + (((14.0 * scale) + 0.5) as u32),
                y: icon_y + (((10.0 * scale) + 0.5) as u32),
                size: (((28.0 * scale) + 0.5) as u32),
                color: RgbaColor {
                    r: 0xff,
                    g: 0xff,
                    b: 0xff,
                    a: 0xff,
                },
            });

            ops.push(DrawOp::Text {
                text: icon.label.into(),
                x: icon_x + (((68.0 * scale) + 0.5) as u32),
                y: icon_y + (((16.0 * scale) + 0.5) as u32),
                size: (((13.0 * scale) + 0.5) as u32),
                color: RgbaColor {
                    r: 0xff,
                    g: 0xff,
                    b: 0xff,
                    a: 0xff,
                },
                font: ngos_gfx_translate::FontFamily::SansSerif,
            });

            ops.push(DrawOp::Text {
                text: "Pinned".into(),
                x: icon_x + (((68.0 * scale) + 0.5) as u32),
                y: icon_y + (((36.0 * scale) + 0.5) as u32),
                size: (((10.0 * scale) + 0.5) as u32),
                color: RgbaColor {
                    r: 0xb0,
                    g: 0xbe,
                    b: 0xce,
                    a: 0xff,
                },
                font: ngos_gfx_translate::FontFamily::SansSerif,
            });
            icon_y = icon_y.saturating_add(gap_y);
        }

        ops
    }

    /// Render full desktop (background + icons)
    pub fn render(&self) -> Vec<DrawOp> {
        let mut ops = self.render_background();
        ops.extend(self.render_icons());
        ops
    }

    pub fn render_scaled(&self, width: u32, height: u32) -> Vec<DrawOp> {
        let mut ops = self.render_background_scaled(width, height);
        ops.extend(self.render_icons_scaled(width, height));
        ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_creation() {
        let desktop = Desktop::new(1920, 1080);
        assert_eq!(desktop.icons.len(), 6);
    }

    #[test]
    fn desktop_renders_background() {
        let desktop = Desktop::new(1920, 1080);
        let ops = desktop.render_background();
        assert!(!ops.is_empty());
    }

    #[test]
    fn desktop_renders_icons() {
        let desktop = Desktop::new(1920, 1080);
        let ops = desktop.render_icons();
        assert!(!ops.is_empty());
    }
}
