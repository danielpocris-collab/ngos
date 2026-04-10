use crate::logo::NGOSLogo;
use alloc::vec::Vec;
use ngos_gfx_translate::{DrawOp, RgbaColor};

/// Boot stages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootStage {
    Loading,
    Initializing,
    Mounting,
    Starting,
    Complete,
}

/// Boot Screen Renderer
pub struct BootScreen {
    width: u32,
    height: u32,
    progress: u8,
    stage: BootStage,
    logo: NGOSLogo,
    messages: [&'static str; 5],
    current_message: usize,
}

impl BootScreen {
    pub fn new(width: u32, height: u32) -> Self {
        BootScreen {
            width,
            height,
            progress: 0,
            stage: BootStage::Loading,
            logo: NGOSLogo::new(150),
            messages: [
                "✓ Loading kernel core...",
                "✓ Initializing graphics subsystem...",
                "✓ Mounting file system...",
                "✓ Starting user interface...",
                "✓ Welcome to NGOS",
            ],
            current_message: 0,
        }
    }

    /// Update boot progress
    pub fn update(&mut self, delta_ms: u32) {
        if self.progress < 100 {
            let increment = (delta_ms / 30) as u8;
            self.progress = (self.progress + increment).min(100);
        }

        // Update stage based on progress
        self.stage = match self.progress {
            0..=20 => BootStage::Loading,
            21..=40 => BootStage::Initializing,
            41..=60 => BootStage::Mounting,
            61..=80 => BootStage::Starting,
            _ => BootStage::Complete,
        };

        // Update messages
        self.current_message = match self.progress {
            0..=20 => 0,
            21..=40 => 1,
            41..=60 => 2,
            61..=80 => 3,
            _ => 4,
        };
    }

    /// Get boot progress (0-100)
    pub fn progress(&self) -> u8 {
        self.progress
    }

    /// Check if boot is complete
    pub fn is_complete(&self) -> bool {
        self.progress >= 100
    }

    /// Get current boot stage
    pub fn stage(&self) -> BootStage {
        self.stage
    }

    /// Render boot screen
    pub fn render(&self, _stage: BootStage) -> Vec<DrawOp> {
        let mut ops = Vec::new();

        // Background gradient
        ops.push(DrawOp::GradientRect {
            x: 0,
            y: 0,
            width: self.width,
            height: self.height,
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
        ops.push(DrawOp::Rect {
            x: 0,
            y: 0,
            width: self.width,
            height: self.height,
            color: RgbaColor {
                r: 0x00,
                g: 0x00,
                b: 0x00,
                a: 0x08,
            },
        });

        // Atmospheric orbs similar to the desktop background.
        ops.push(DrawOp::RoundedRect {
            x: self.width / 8,
            y: self.height / 10,
            width: self.width / 2,
            height: self.height / 2,
            radius: self.width.max(self.height) / 4,
            color: RgbaColor {
                r: 0x00,
                g: 0xd4,
                b: 0xff,
                a: 0x20,
            },
        });
        ops.push(DrawOp::RoundedRect {
            x: self.width.saturating_sub(self.width / 3),
            y: self.height / 6,
            width: self.width / 3,
            height: self.height / 3,
            radius: self.width.max(self.height) / 6,
            color: RgbaColor {
                r: 0x7b,
                g: 0x2c,
                b: 0xbf,
                a: 0x18,
            },
        });

        // Center logo
        let logo_x = self.width.saturating_sub(150) / 2;
        let logo_y = self.height.saturating_sub(150) / 2;

        // Pulsing glow effect (simple oscillation without sin)
        let pulse = 15 + (((self.progress as u32 / 5) % 5) as u32);
        ops.push(DrawOp::Rect {
            x: logo_x.saturating_sub(pulse),
            y: logo_y.saturating_sub(pulse),
            width: 150 + pulse * 2,
            height: 150 + pulse * 2,
            color: RgbaColor {
                r: 0x00,
                g: 0xd4,
                b: 0xff,
                a: 0x40,
            },
        });

        // Render logo
        ops.extend(self.logo.render(logo_x, logo_y));
        ops.push(DrawOp::Rect {
            x: logo_x.saturating_sub(12),
            y: logo_y.saturating_sub(12),
            width: 174,
            height: 174,
            color: RgbaColor {
                r: 0x00,
                g: 0xd4,
                b: 0xff,
                a: 0x12,
            },
        });

        // Title block
        let text_y = logo_y + 180;
        let text_x = self.width.saturating_sub(260) / 2;
        ops.push(DrawOp::Text {
            text: "NGOS".into(),
            x: text_x + 78,
            y: text_y,
            size: 28,
            color: RgbaColor {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            },
            font: ngos_gfx_translate::FontFamily::SansSerif,
        });
        ops.push(DrawOp::Text {
            text: "Next Gen OS".into(),
            x: text_x + 42,
            y: text_y + 32,
            size: 12,
            color: RgbaColor {
                r: 0x94,
                g: 0xa3,
                b: 0xb8,
                a: 0xff,
            },
            font: ngos_gfx_translate::FontFamily::SansSerif,
        });

        // "NEXT GEN OS" subtitle
        let subtitle_y = text_y + 60;
        let subtitle_x = self.width.saturating_sub(150) / 2;

        ops.push(DrawOp::Text {
            text: "Loading desktop environment".into(),
            x: subtitle_x.saturating_sub(20),
            y: subtitle_y,
            size: 12,
            color: RgbaColor {
                r: 0x94,
                g: 0xa3,
                b: 0xb8,
                a: 0xff,
            },
            font: ngos_gfx_translate::FontFamily::SansSerif,
        });

        ops.push(DrawOp::Text {
            text: "Booting NGOS core services".into(),
            x: subtitle_x.saturating_sub(44),
            y: subtitle_y + 18,
            size: 11,
            color: RgbaColor {
                r: 0xa8,
                g: 0xb7,
                b: 0xc7,
                a: 0xff,
            },
            font: ngos_gfx_translate::FontFamily::SansSerif,
        });

        // Progress bar
        let bar_width = self.width.min(400);
        let bar_height = 6;
        let bar_x = self.width.saturating_sub(bar_width) / 2;
        let bar_y = logo_y + 290;

        // Progress bar background
        ops.push(DrawOp::Rect {
            x: bar_x,
            y: bar_y,
            width: bar_width,
            height: bar_height,
            color: RgbaColor {
                r: 0x30,
                g: 0x30,
                b: 0x50,
                a: 0xff,
            },
        });

        // Progress bar fill
        let fill_width = (bar_width * self.progress as u32) / 100;
        ops.push(DrawOp::GradientRect {
            x: bar_x,
            y: bar_y,
            width: fill_width,
            height: bar_height,
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

        // Progress bar glow
        ops.push(DrawOp::Rect {
            x: bar_x,
            y: bar_y - 2,
            width: bar_width,
            height: bar_height + 4,
            color: RgbaColor {
                r: 0x00,
                g: 0xd4,
                b: 0xff,
                a: 0x30,
            },
        });

        // Boot messages
        let messages_y = bar_y + 40;
        let message_height = 20;

        for (i, &_message) in self.messages.iter().enumerate() {
            if i <= self.current_message {
                let alpha = if i == self.current_message {
                    0xff
                } else {
                    0x99
                };

                // Message background
                ops.push(DrawOp::Rect {
                    x: self.width.saturating_sub(350) / 2,
                    y: messages_y + (i as u32 * message_height),
                    width: 350,
                    height: message_height,
                    color: RgbaColor {
                        r: 0x00,
                        g: 0x00,
                        b: 0x00,
                        a: alpha / 3,
                    },
                });

                // Message text (simplified as rectangle)
                ops.push(DrawOp::Rect {
                    x: self.width.saturating_sub(300) / 2,
                    y: messages_y + (i as u32 * message_height),
                    width: 300,
                    height: message_height,
                    color: RgbaColor {
                        r: 0x94,
                        g: 0xa3,
                        b: 0xb8,
                        a: alpha,
                    },
                });
            }
        }

        ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_screen_creation() {
        let screen = BootScreen::new(1920, 1080);
        assert_eq!(screen.progress(), 0);
        assert!(!screen.is_complete());
    }

    #[test]
    fn boot_screen_update() {
        let mut screen = BootScreen::new(1920, 1080);
        screen.update(1000);
        assert!(screen.progress() > 0);
    }

    #[test]
    fn boot_screen_stages() {
        let mut screen = BootScreen::new(1920, 1080);

        // Initial stage is Loading
        assert_eq!(screen.stage(), BootStage::Loading);

        // After enough updates, should progress
        for _ in 0..10 {
            screen.update(1000);
        }
        // Should be at least Initializing or further
        assert!(screen.progress() > 0);
    }

    #[test]
    fn boot_screen_renders() {
        let screen = BootScreen::new(1920, 1080);
        let ops = screen.render(BootStage::Loading);
        assert!(!ops.is_empty());
    }

    #[test]
    fn boot_screen_complete() {
        let mut screen = BootScreen::new(1920, 1080);

        // Simulate full boot
        for _ in 0..100 {
            screen.update(30);
        }

        assert!(screen.is_complete());
        assert_eq!(screen.stage(), BootStage::Complete);
    }
}
