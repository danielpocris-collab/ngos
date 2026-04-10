use alloc::{format, string::String, vec, vec::Vec};
use ngos_effects_pipeline::{
    AccentPulse, BackdropSpec, GradientDirection, GradientSpec, GradientStop, ShadowSpec,
    TemporalSpec, TemporalState, TranslucencySpec,
};
use ngos_gfx_translate::{BlendMode, DrawOp};

pub type SurfaceId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SurfaceRole {
    Background = 0,
    Panel = 1,
    Window = 2,
    Overlay = 3,
    Cursor = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Surface {
    pub id: SurfaceId,
    pub role: SurfaceRole,
    pub rect: SurfaceRect,
    pub alpha: u8,
    pub visible: bool,
    pub focused: bool,
    pub title: Option<String>,
    pub blend_mode: BlendMode,
    pub pass_name: String,
    pub corner_radius: u32,
    pub backdrop_opacity: Option<u8>,
    pub shadow_enabled: bool,
    pub material: SurfaceMaterial,
    pub content: Vec<DrawOp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceMaterial {
    pub gradient: Option<GradientSpec>,
    pub translucency: Option<TranslucencySpec>,
    pub backdrop: Option<BackdropSpec>,
    pub shadow: Option<ShadowSpec>,
    pub temporal: Option<TemporalSpec>,
    pub accent_line: Option<ngos_gfx_translate::RgbaColor>,
}

impl SurfaceMaterial {
    fn default_for_role(role: SurfaceRole) -> Self {
        match role {
            SurfaceRole::Background => SurfaceMaterial {
                gradient: Some(GradientSpec {
                    direction: GradientDirection::DiagonalTopLeft,
                    stops: vec![
                        GradientStop {
                            position: 0,
                            color: ngos_gfx_translate::RgbaColor {
                                r: 0x0b,
                                g: 0x12,
                                b: 0x1e,
                                a: 0xff,
                            },
                        },
                        GradientStop {
                            position: 160,
                            color: ngos_gfx_translate::RgbaColor {
                                r: 0x12,
                                g: 0x1d,
                                b: 0x32,
                                a: 0xff,
                            },
                        },
                        GradientStop {
                            position: 255,
                            color: ngos_gfx_translate::RgbaColor {
                                r: 0x06,
                                g: 0x0c,
                                b: 0x15,
                                a: 0xff,
                            },
                        },
                    ],
                }),
                translucency: None,
                backdrop: None,
                shadow: None,
                temporal: None,
                accent_line: None,
            },
            SurfaceRole::Panel => SurfaceMaterial {
                gradient: Some(GradientSpec {
                    direction: GradientDirection::Vertical,
                    stops: vec![
                        GradientStop {
                            position: 0,
                            color: ngos_gfx_translate::RgbaColor {
                                r: 0xf8,
                                g: 0xfb,
                                b: 0xff,
                                a: 0x10,
                            },
                        },
                        GradientStop {
                            position: 255,
                            color: ngos_gfx_translate::RgbaColor {
                                r: 0xc8,
                                g: 0xd7,
                                b: 0xf2,
                                a: 0x06,
                            },
                        },
                    ],
                }),
                translucency: Some(TranslucencySpec {
                    opacity: 0x2e,
                    blur_radius: 8,
                }),
                backdrop: None,
                shadow: None,
                temporal: None,
                accent_line: Some(ngos_gfx_translate::RgbaColor {
                    r: 0x7d,
                    g: 0xc8,
                    b: 0xff,
                    a: 0x22,
                }),
            },
            SurfaceRole::Window => SurfaceMaterial {
                gradient: Some(GradientSpec {
                    direction: GradientDirection::Vertical,
                    stops: vec![
                        GradientStop {
                            position: 0,
                            color: ngos_gfx_translate::RgbaColor {
                                r: 0xff,
                                g: 0xff,
                                b: 0xff,
                                a: 0x12,
                            },
                        },
                        GradientStop {
                            position: 255,
                            color: ngos_gfx_translate::RgbaColor {
                                r: 0x8f,
                                g: 0xb8,
                                b: 0xe8,
                                a: 0x05,
                            },
                        },
                    ],
                }),
                translucency: Some(TranslucencySpec {
                    opacity: 0x3a,
                    blur_radius: 10,
                }),
                backdrop: Some(BackdropSpec {
                    blur_radius: 12,
                    tint_color: ngos_gfx_translate::RgbaColor {
                        r: 0x11,
                        g: 0x18,
                        b: 0x29,
                        a: 0xff,
                    },
                    tint_opacity: 0x24,
                }),
                shadow: Some(ShadowSpec {
                    offset_x: 0,
                    offset_y: 10,
                    blur: 26,
                    spread: 10,
                    color: ngos_gfx_translate::RgbaColor {
                        r: 0x03,
                        g: 0x06,
                        b: 0x10,
                        a: 0x34,
                    },
                }),
                temporal: Some(TemporalSpec {
                    state: TemporalState {
                        tick: 32,
                        stride: 2,
                    },
                    color: ngos_gfx_translate::RgbaColor {
                        r: 0x6f,
                        g: 0xc4,
                        b: 0xff,
                        a: 0x00,
                    },
                    accent: AccentPulse {
                        base_alpha: 0x06,
                        pulse_range: 0x10,
                    },
                }),
                accent_line: Some(ngos_gfx_translate::RgbaColor {
                    r: 0x81,
                    g: 0xc9,
                    b: 0xff,
                    a: 0x34,
                }),
            },
            SurfaceRole::Overlay => SurfaceMaterial {
                gradient: Some(GradientSpec {
                    direction: GradientDirection::Vertical,
                    stops: vec![
                        GradientStop {
                            position: 0,
                            color: ngos_gfx_translate::RgbaColor {
                                r: 0xff,
                                g: 0xff,
                                b: 0xff,
                                a: 0x16,
                            },
                        },
                        GradientStop {
                            position: 255,
                            color: ngos_gfx_translate::RgbaColor {
                                r: 0x9d,
                                g: 0xd0,
                                b: 0xff,
                                a: 0x08,
                            },
                        },
                    ],
                }),
                translucency: Some(TranslucencySpec {
                    opacity: 0x48,
                    blur_radius: 12,
                }),
                backdrop: Some(BackdropSpec {
                    blur_radius: 14,
                    tint_color: ngos_gfx_translate::RgbaColor {
                        r: 0x10,
                        g: 0x16,
                        b: 0x24,
                        a: 0xff,
                    },
                    tint_opacity: 0x2e,
                }),
                shadow: Some(ShadowSpec {
                    offset_x: 0,
                    offset_y: 12,
                    blur: 30,
                    spread: 12,
                    color: ngos_gfx_translate::RgbaColor {
                        r: 0x02,
                        g: 0x06,
                        b: 0x10,
                        a: 0x3a,
                    },
                }),
                temporal: Some(TemporalSpec {
                    state: TemporalState {
                        tick: 96,
                        stride: 3,
                    },
                    color: ngos_gfx_translate::RgbaColor {
                        r: 0x9c,
                        g: 0xe1,
                        b: 0xff,
                        a: 0x00,
                    },
                    accent: AccentPulse {
                        base_alpha: 0x08,
                        pulse_range: 0x14,
                    },
                }),
                accent_line: Some(ngos_gfx_translate::RgbaColor {
                    r: 0xa4,
                    g: 0xda,
                    b: 0xff,
                    a: 0x3e,
                }),
            },
            SurfaceRole::Cursor => SurfaceMaterial {
                gradient: None,
                translucency: None,
                backdrop: None,
                shadow: None,
                temporal: None,
                accent_line: None,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceError {
    ZeroDimensions { id: SurfaceId },
    DuplicateId { id: SurfaceId },
    NotFound { id: SurfaceId },
}

impl SurfaceError {
    pub fn describe(&self) -> String {
        match self {
            Self::ZeroDimensions { id } => format!("surface {} has zero dimensions", id),
            Self::DuplicateId { id } => format!("surface {} already exists in stack", id),
            Self::NotFound { id } => format!("surface {} not found in stack", id),
        }
    }
}

impl Surface {
    pub fn new(id: SurfaceId, role: SurfaceRole, rect: SurfaceRect) -> Result<Self, SurfaceError> {
        if rect.width == 0 || rect.height == 0 {
            return Err(SurfaceError::ZeroDimensions { id });
        }
        Ok(Surface {
            id,
            role,
            rect,
            alpha: 255,
            visible: true,
            focused: false,
            title: None,
            blend_mode: match role {
                SurfaceRole::Background => BlendMode::SourceOver,
                SurfaceRole::Panel => BlendMode::Screen,
                SurfaceRole::Window => BlendMode::SourceOver,
                SurfaceRole::Overlay => BlendMode::Overlay,
                SurfaceRole::Cursor => BlendMode::Additive,
            },
            pass_name: match role {
                SurfaceRole::Background => String::from("background"),
                SurfaceRole::Panel => String::from("panel"),
                SurfaceRole::Window => String::from("window"),
                SurfaceRole::Overlay => String::from("overlay"),
                SurfaceRole::Cursor => String::from("cursor"),
            },
            corner_radius: match role {
                SurfaceRole::Background => 0,
                SurfaceRole::Panel => 18,
                SurfaceRole::Window => 16,
                SurfaceRole::Overlay => 20,
                SurfaceRole::Cursor => 0,
            },
            backdrop_opacity: match role {
                SurfaceRole::Background => None,
                SurfaceRole::Panel => Some(0x38),
                SurfaceRole::Window => Some(0x46),
                SurfaceRole::Overlay => Some(0x52),
                SurfaceRole::Cursor => None,
            },
            shadow_enabled: matches!(role, SurfaceRole::Window | SurfaceRole::Overlay),
            material: SurfaceMaterial::default_for_role(role),
            content: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_new_valid() {
        let s = Surface::new(
            1,
            SurfaceRole::Window,
            SurfaceRect {
                x: 0,
                y: 0,
                width: 800,
                height: 600,
            },
        );
        assert!(s.is_ok());
        let s = s.unwrap();
        assert_eq!(s.id, 1);
        assert_eq!(s.role, SurfaceRole::Window);
        assert!(s.visible);
        assert!(!s.focused);
        assert_eq!(s.alpha, 255);
    }

    #[test]
    fn surface_rejects_zero_width() {
        let err = Surface::new(
            2,
            SurfaceRole::Window,
            SurfaceRect {
                x: 0,
                y: 0,
                width: 0,
                height: 100,
            },
        )
        .unwrap_err();
        assert!(matches!(err, SurfaceError::ZeroDimensions { id: 2 }));
        assert!(err.describe().contains("zero dimensions"));
    }

    #[test]
    fn surface_rejects_zero_height() {
        let err = Surface::new(
            3,
            SurfaceRole::Panel,
            SurfaceRect {
                x: 0,
                y: 0,
                width: 100,
                height: 0,
            },
        )
        .unwrap_err();
        assert!(matches!(err, SurfaceError::ZeroDimensions { id: 3 }));
    }

    #[test]
    fn surface_role_order() {
        assert!(SurfaceRole::Background < SurfaceRole::Panel);
        assert!(SurfaceRole::Panel < SurfaceRole::Window);
        assert!(SurfaceRole::Window < SurfaceRole::Overlay);
        assert!(SurfaceRole::Overlay < SurfaceRole::Cursor);
    }
}
