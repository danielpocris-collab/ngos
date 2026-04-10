use crate::surface_agent::SurfaceRect;
use alloc::vec::Vec;
use ngos_gfx_translate::{DrawOp, FontFamily, RgbaColor};

pub struct ChromeStyle {
    pub title_height: u32,
    pub corner_radius: u32,
    pub shadow_blur: u32,
    pub shadow_color: RgbaColor,
    pub title_color: RgbaColor,
    pub frame_color: RgbaColor,
    pub focus_accent: RgbaColor,
}

impl ChromeStyle {
    pub fn default_window() -> Self {
        ChromeStyle {
            title_height: 28,
            corner_radius: 10,
            shadow_blur: 16,
            shadow_color: RgbaColor {
                r: 0x00,
                g: 0x00,
                b: 0x00,
                a: 0x44,
            },
            title_color: RgbaColor {
                r: 0x1e,
                g: 0x28,
                b: 0x3c,
                a: 0xff,
            },
            frame_color: RgbaColor {
                r: 0xfa,
                g: 0xfd,
                b: 0xff,
                a: 0x10,
            },
            focus_accent: RgbaColor {
                r: 0x4b,
                g: 0x92,
                b: 0xe8,
                a: 0xff,
            },
        }
    }

    pub fn focused_window() -> Self {
        ChromeStyle {
            title_height: 28,
            corner_radius: 10,
            shadow_blur: 22,
            shadow_color: RgbaColor {
                r: 0x00,
                g: 0x00,
                b: 0x00,
                a: 0x66,
            },
            title_color: RgbaColor {
                r: 0x22,
                g: 0x30,
                b: 0x48,
                a: 0xff,
            },
            frame_color: RgbaColor {
                r: 0xfa,
                g: 0xfd,
                b: 0xff,
                a: 0x16,
            },
            focus_accent: RgbaColor {
                r: 0x6a,
                g: 0xb2,
                b: 0xff,
                a: 0xff,
            },
        }
    }
}

pub fn chrome_ops_for_window(
    rect: SurfaceRect,
    style: &ChromeStyle,
    focused: bool,
    title: Option<&str>,
) -> Vec<DrawOp> {
    let mut ops = Vec::new();
    let shadow_inset = style.shadow_blur / 2;

    ops.push(DrawOp::ShadowRect {
        x: rect.x.saturating_sub(shadow_inset),
        y: rect.y.saturating_sub(shadow_inset),
        width: rect.width + shadow_inset * 2,
        height: rect.height + shadow_inset * 2,
        blur: style.shadow_blur,
        color: style.shadow_color,
    });

    ops.push(DrawOp::RoundedRect {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
        radius: style.corner_radius,
        color: style.frame_color,
    });

    if rect.height >= style.title_height + 4 {
        ops.push(DrawOp::Backdrop {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: style.title_height,
            opacity: if focused { 0x44 } else { 0x28 },
        });
        ops.push(DrawOp::Rect {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: style.title_height,
            color: style.title_color,
        });
        if focused {
            ops.push(DrawOp::Rect {
                x: rect.x,
                y: rect.y,
                width: 4,
                height: style.title_height,
                color: style.focus_accent,
            });
        }
        if let Some(title) = title {
            ops.push(DrawOp::Text {
                text: title.into(),
                x: rect.x + 18,
                y: rect.y + (style.title_height / 2).saturating_sub(7),
                size: 14,
                color: RgbaColor {
                    r: 0xf5,
                    g: 0xf7,
                    b: 0xfb,
                    a: 0xff,
                },
                font: FontFamily::SansSerif,
            });
        }
        let button_size = 12;
        let button_gap = 8;
        let buttons_y = rect.y + (style.title_height.saturating_sub(button_size)) / 2;
        let close_x = rect.x + rect.width.saturating_sub(button_size + 14);
        let maximize_x = close_x.saturating_sub(button_size + button_gap);
        let minimize_x = maximize_x.saturating_sub(button_size + button_gap);
        ops.push(DrawOp::RoundedRect {
            x: minimize_x,
            y: buttons_y,
            width: button_size,
            height: button_size,
            radius: button_size / 2,
            color: RgbaColor {
                r: 0xff,
                g: 0xbd,
                b: 0x2e,
                a: 0xe8,
            },
        });
        ops.push(DrawOp::RoundedRect {
            x: maximize_x,
            y: buttons_y,
            width: button_size,
            height: button_size,
            radius: button_size / 2,
            color: RgbaColor {
                r: 0x28,
                g: 0xca,
                b: 0x41,
                a: 0xe8,
            },
        });
        ops.push(DrawOp::RoundedRect {
            x: close_x,
            y: buttons_y,
            width: button_size,
            height: button_size,
            radius: button_size / 2,
            color: RgbaColor {
                r: 0xff,
                g: 0x5f,
                b: 0x57,
                a: 0xe8,
            },
        });
    }

    ops
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect_800_600() -> SurfaceRect {
        SurfaceRect {
            x: 100,
            y: 80,
            width: 800,
            height: 600,
        }
    }

    #[test]
    fn chrome_ops_include_shadow_and_frame() {
        let style = ChromeStyle::default_window();
        let ops = chrome_ops_for_window(rect_800_600(), &style, false, Some("Window"));
        assert!(ops.iter().any(|op| matches!(op, DrawOp::ShadowRect { .. })));
        assert!(
            ops.iter()
                .any(|op| matches!(op, DrawOp::RoundedRect { .. }))
        );
        assert!(ops.iter().any(|op| matches!(op, DrawOp::Rect { .. })));
    }

    #[test]
    fn chrome_focused_includes_accent_bar() {
        let style = ChromeStyle::focused_window();
        let ops = chrome_ops_for_window(rect_800_600(), &style, true, Some("Focused"));
        let rect_ops: Vec<_> = ops
            .iter()
            .filter(|op| matches!(op, DrawOp::Rect { .. }))
            .collect();
        assert!(
            rect_ops.len() >= 2,
            "focused chrome must have title bar + accent bar"
        );
    }

    #[test]
    fn chrome_unfocused_no_accent_bar() {
        let style = ChromeStyle::default_window();
        let ops = chrome_ops_for_window(rect_800_600(), &style, false, Some("Unfocused"));
        let rect_ops: Vec<_> = ops
            .iter()
            .filter(|op| matches!(op, DrawOp::Rect { .. }))
            .collect();
        assert_eq!(rect_ops.len(), 1, "unfocused chrome has only title bar");
    }

    #[test]
    fn chrome_small_window_skips_titlebar() {
        let style = ChromeStyle::default_window();
        let small = SurfaceRect {
            x: 0,
            y: 0,
            width: 50,
            height: 20,
        };
        let ops = chrome_ops_for_window(small, &style, true, Some("Tiny"));
        assert!(!ops.iter().any(|op| matches!(op, DrawOp::Rect { .. })));
    }
}
