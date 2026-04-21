use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use crate::FrameScriptError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbaColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Font families for text rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFamily {
    System,    // Default system font
    Monospace, // Terminal/code font
    SansSerif, // UI font (Inter, etc)
    Serif,     // Document font
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    SourceOver,
    Screen,
    Multiply,
    Overlay,
    Additive,
}

impl BlendMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SourceOver => "source-over",
            Self::Screen => "screen",
            Self::Multiply => "multiply",
            Self::Overlay => "overlay",
            Self::Additive => "additive",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderPassClass {
    Background,
    Panel,
    Chrome,
    Content,
    Overlay,
    Effect,
    Presentation,
}

impl RenderPassClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Background => "background",
            Self::Panel => "panel",
            Self::Chrome => "chrome",
            Self::Content => "content",
            Self::Overlay => "overlay",
            Self::Effect => "effect",
            Self::Presentation => "presentation",
        }
    }
}

/// Draw operation classes for categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawOpClass {
    Geometry,
    Composition,
    Effect,
    Presentation,
    Text,
    Image,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawOp {
    // geometry
    Clear {
        color: RgbaColor,
    },
    GradientRect {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        top_left: RgbaColor,
        top_right: RgbaColor,
        bottom_left: RgbaColor,
        bottom_right: RgbaColor,
    },
    Line {
        x0: u32,
        y0: u32,
        x1: u32,
        y1: u32,
        color: RgbaColor,
    },
    Rect {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        color: RgbaColor,
    },
    RoundedRect {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        radius: u32,
        color: RgbaColor,
    },
    Sprite {
        sprite: String,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    Triangle {
        x0: u32,
        y0: u32,
        x1: u32,
        y1: u32,
        x2: u32,
        y2: u32,
        color: RgbaColor,
    },
    Ellipse {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        color: RgbaColor,
    },
    // effect
    ShadowRect {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        blur: u32,
        color: RgbaColor,
    },
    GaussianBlur {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        radius: u32,
    },
    Backdrop {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        opacity: u8,
    },
    // composition
    Blit {
        source: String,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    BeginPass {
        label: String,
        class: RenderPassClass,
    },
    EndPass,
    SetBlendMode {
        mode: BlendMode,
    },
    ClearBlendMode,
    PushLayer {
        opacity: u8,
    },
    PopLayer,
    SetClip {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    ClearClip,
    // presentation
    SetPresentRegion {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    FlipRegion {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    // text (NEW!)
    Text {
        text: String,
        x: u32,
        y: u32,
        size: u32,
        color: RgbaColor,
        font: FontFamily,
    },
    // image (NEW!)
    Image {
        data: Vec<u8>,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    // icon (NEW! - for emoji/unicode icons)
    Icon {
        icon: char,
        x: u32,
        y: u32,
        size: u32,
        color: RgbaColor,
    },
}

impl DrawOp {
    pub fn class(&self) -> DrawOpClass {
        match self {
            Self::Clear { .. }
            | Self::GradientRect { .. }
            | Self::Line { .. }
            | Self::Rect { .. }
            | Self::RoundedRect { .. }
            | Self::Sprite { .. }
            | Self::Triangle { .. }
            | Self::Ellipse { .. } => DrawOpClass::Geometry,

            Self::ShadowRect { .. } | Self::GaussianBlur { .. } | Self::Backdrop { .. } => {
                DrawOpClass::Effect
            }

            Self::Blit { .. }
            | Self::BeginPass { .. }
            | Self::EndPass
            | Self::SetBlendMode { .. }
            | Self::ClearBlendMode
            | Self::PushLayer { .. }
            | Self::PopLayer
            | Self::SetClip { .. }
            | Self::ClearClip => DrawOpClass::Composition,

            Self::SetPresentRegion { .. } | Self::FlipRegion { .. } => DrawOpClass::Presentation,

            Self::Text { .. } => DrawOpClass::Text,

            Self::Image { .. } | Self::Icon { .. } => DrawOpClass::Image,
        }
    }

    pub fn validate_args(&self) -> Result<(), FrameScriptError> {
        match self {
            Self::BeginPass { label, .. } if label.is_empty() => {
                Err(FrameScriptError::InvalidValue {
                    key: String::from("begin-pass.label"),
                    value: label.clone(),
                })
            }
            Self::Ellipse { width, height, .. } if *width == 0 || *height == 0 => {
                Err(FrameScriptError::InvalidValue {
                    key: String::from("ellipse.dimensions"),
                    value: format!("{}x{}", width, height),
                })
            }
            Self::GaussianBlur {
                width,
                height,
                radius,
                ..
            } if *width == 0 || *height == 0 || *radius == 0 => {
                Err(FrameScriptError::InvalidValue {
                    key: String::from("gaussian-blur.args"),
                    value: format!("w={} h={} r={}", width, height, radius),
                })
            }
            Self::Backdrop { width, height, .. } if *width == 0 || *height == 0 => {
                Err(FrameScriptError::InvalidValue {
                    key: String::from("backdrop.dimensions"),
                    value: format!("{}x{}", width, height),
                })
            }
            Self::SetClip { width, height, .. } if *width == 0 || *height == 0 => {
                Err(FrameScriptError::InvalidValue {
                    key: String::from("set-clip.dimensions"),
                    value: format!("{}x{}", width, height),
                })
            }
            Self::SetPresentRegion { width, height, .. } if *width == 0 || *height == 0 => {
                Err(FrameScriptError::InvalidValue {
                    key: String::from("set-present-region.dimensions"),
                    value: format!("{}x{}", width, height),
                })
            }
            Self::FlipRegion { width, height, .. } if *width == 0 || *height == 0 => {
                Err(FrameScriptError::InvalidValue {
                    key: String::from("flip-region.dimensions"),
                    value: format!("{}x{}", width, height),
                })
            }
            _ => Ok(()),
        }
    }

    pub fn encode_line(&self) -> String {
        match self {
            Self::Clear { color } => format!(
                "op=clear rgba={:02x}{:02x}{:02x}{:02x}",
                color.r, color.g, color.b, color.a
            ),
            Self::GradientRect {
                x,
                y,
                width,
                height,
                top_left,
                top_right,
                bottom_left,
                bottom_right,
            } => format!(
                "op=gradient-rect x={} y={} w={} h={} tl={:02x}{:02x}{:02x}{:02x} tr={:02x}{:02x}{:02x}{:02x} bl={:02x}{:02x}{:02x}{:02x} br={:02x}{:02x}{:02x}{:02x}",
                x,
                y,
                width,
                height,
                top_left.r,
                top_left.g,
                top_left.b,
                top_left.a,
                top_right.r,
                top_right.g,
                top_right.b,
                top_right.a,
                bottom_left.r,
                bottom_left.g,
                bottom_left.b,
                bottom_left.a,
                bottom_right.r,
                bottom_right.g,
                bottom_right.b,
                bottom_right.a,
            ),
            Self::Line {
                x0,
                y0,
                x1,
                y1,
                color,
            } => format!(
                "op=line x0={} y0={} x1={} y1={} rgba={:02x}{:02x}{:02x}{:02x}",
                x0, y0, x1, y1, color.r, color.g, color.b, color.a
            ),
            Self::Rect {
                x,
                y,
                width,
                height,
                color,
            } => format!(
                "op=rect x={} y={} w={} h={} rgba={:02x}{:02x}{:02x}{:02x}",
                x, y, width, height, color.r, color.g, color.b, color.a
            ),
            Self::RoundedRect {
                x,
                y,
                width,
                height,
                radius,
                color,
            } => format!(
                "op=rounded-rect x={} y={} w={} h={} radius={} rgba={:02x}{:02x}{:02x}{:02x}",
                x, y, width, height, radius, color.r, color.g, color.b, color.a
            ),
            Self::ShadowRect {
                x,
                y,
                width,
                height,
                blur,
                color,
            } => format!(
                "op=shadow-rect x={} y={} w={} h={} blur={} rgba={:02x}{:02x}{:02x}{:02x}",
                x, y, width, height, blur, color.r, color.g, color.b, color.a
            ),
            Self::Sprite {
                sprite,
                x,
                y,
                width,
                height,
            } => format!(
                "op=sprite id={} x={} y={} w={} h={}",
                sprite, x, y, width, height
            ),
            Self::Blit {
                source,
                x,
                y,
                width,
                height,
            } => format!(
                "op=blit source={} x={} y={} w={} h={}",
                source, x, y, width, height
            ),
            Self::BeginPass { label, class } => {
                format!("op=begin-pass label={} class={}", label, class.as_str())
            }
            Self::EndPass => String::from("op=end-pass"),
            Self::SetBlendMode { mode } => {
                format!("op=set-blend-mode mode={}", mode.as_str())
            }
            Self::ClearBlendMode => String::from("op=clear-blend-mode"),
            Self::Triangle {
                x0,
                y0,
                x1,
                y1,
                x2,
                y2,
                color,
            } => format!(
                "op=triangle x0={} y0={} x1={} y1={} x2={} y2={} rgba={:02x}{:02x}{:02x}{:02x}",
                x0, y0, x1, y1, x2, y2, color.r, color.g, color.b, color.a
            ),
            Self::Ellipse {
                x,
                y,
                width,
                height,
                color,
            } => format!(
                "op=ellipse x={} y={} w={} h={} rgba={:02x}{:02x}{:02x}{:02x}",
                x, y, width, height, color.r, color.g, color.b, color.a
            ),
            Self::PushLayer { opacity } => format!("op=push-layer opacity={}", opacity),
            Self::PopLayer => String::from("op=pop-layer"),
            Self::SetClip {
                x,
                y,
                width,
                height,
            } => format!("op=set-clip x={} y={} w={} h={}", x, y, width, height),
            Self::ClearClip => String::from("op=clear-clip"),
            Self::GaussianBlur {
                x,
                y,
                width,
                height,
                radius,
            } => format!(
                "op=gaussian-blur x={} y={} w={} h={} radius={}",
                x, y, width, height, radius
            ),
            Self::Backdrop {
                x,
                y,
                width,
                height,
                opacity,
            } => format!(
                "op=backdrop x={} y={} w={} h={} opacity={}",
                x, y, width, height, opacity
            ),
            Self::SetPresentRegion {
                x,
                y,
                width,
                height,
            } => format!(
                "op=set-present-region x={} y={} w={} h={}",
                x, y, width, height
            ),
            Self::FlipRegion {
                x,
                y,
                width,
                height,
            } => format!("op=flip-region x={} y={} w={} h={}", x, y, width, height),
            // New primitives - placeholder encoding
            Self::Text {
                text,
                x,
                y,
                size,
                color,
                font,
            } => format!(
                "op=text x={} y={} size={} rgba={:02x}{:02x}{:02x}{:02x} font={:?} text={}",
                x, y, size, color.r, color.g, color.b, color.a, font, text
            ),
            Self::Image {
                x,
                y,
                width,
                height,
                ..
            } => format!("op=image x={} y={} w={} h={}", x, y, width, height),
            Self::Icon {
                icon,
                x,
                y,
                size,
                color,
            } => format!(
                "op=icon x={} y={} size={} rgba={:02x}{:02x}{:02x}{:02x} icon={}",
                x, y, size, color.r, color.g, color.b, color.a, icon
            ),
        }
    }
}

pub fn parse_blend_mode(value: &str) -> Result<BlendMode, FrameScriptError> {
    match value.trim() {
        "source-over" => Ok(BlendMode::SourceOver),
        "screen" => Ok(BlendMode::Screen),
        "multiply" => Ok(BlendMode::Multiply),
        "overlay" => Ok(BlendMode::Overlay),
        "additive" => Ok(BlendMode::Additive),
        _ => Err(FrameScriptError::InvalidValue {
            key: String::from("blend-mode"),
            value: value.to_string(),
        }),
    }
}

pub fn parse_render_pass_class(value: &str) -> Result<RenderPassClass, FrameScriptError> {
    match value.trim() {
        "background" => Ok(RenderPassClass::Background),
        "panel" => Ok(RenderPassClass::Panel),
        "chrome" => Ok(RenderPassClass::Chrome),
        "content" => Ok(RenderPassClass::Content),
        "overlay" => Ok(RenderPassClass::Overlay),
        "effect" => Ok(RenderPassClass::Effect),
        "presentation" => Ok(RenderPassClass::Presentation),
        _ => Err(FrameScriptError::InvalidValue {
            key: String::from("render-pass-class"),
            value: value.to_string(),
        }),
    }
}

pub fn parse_color(value: &str) -> Result<RgbaColor, FrameScriptError> {
    let hex = value.strip_prefix('#').unwrap_or(value);
    let bytes = match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok();
            let g = u8::from_str_radix(&hex[2..4], 16).ok();
            let b = u8::from_str_radix(&hex[4..6], 16).ok();
            match (r, g, b) {
                (Some(r), Some(g), Some(b)) => Some([r, g, b, 0xff]),
                _ => None,
            }
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok();
            let g = u8::from_str_radix(&hex[2..4], 16).ok();
            let b = u8::from_str_radix(&hex[4..6], 16).ok();
            let a = u8::from_str_radix(&hex[6..8], 16).ok();
            match (r, g, b, a) {
                (Some(r), Some(g), Some(b), Some(a)) => Some([r, g, b, a]),
                _ => None,
            }
        }
        _ => None,
    };
    let Some([r, g, b, a]) = bytes else {
        return Err(FrameScriptError::InvalidValue {
            key: String::from("color"),
            value: value.to_string(),
        });
    };
    Ok(RgbaColor { r, g, b, a })
}

pub fn parse_u32(key: &str, value: &str) -> Result<u32, FrameScriptError> {
    value
        .parse::<u32>()
        .map_err(|_| FrameScriptError::InvalidValue {
            key: key.to_string(),
            value: value.to_string(),
        })
}

pub fn parse_u8(key: &str, value: &str) -> Result<u8, FrameScriptError> {
    value
        .parse::<u8>()
        .map_err(|_| FrameScriptError::InvalidValue {
            key: key.to_string(),
            value: value.to_string(),
        })
}

pub fn parse_rect(value: &str) -> Result<DrawOp, FrameScriptError> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 5 {
        return Err(FrameScriptError::InvalidLine(format!("rect={value}")));
    }
    Ok(DrawOp::Rect {
        x: parse_u32("rect.x", parts[0])?,
        y: parse_u32("rect.y", parts[1])?,
        width: parse_u32("rect.width", parts[2])?,
        height: parse_u32("rect.height", parts[3])?,
        color: parse_color(parts[4])?,
    })
}

pub fn parse_gradient_rect(value: &str) -> Result<DrawOp, FrameScriptError> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 8 {
        return Err(FrameScriptError::InvalidLine(format!(
            "gradient-rect={value}"
        )));
    }
    Ok(DrawOp::GradientRect {
        x: parse_u32("gradient-rect.x", parts[0])?,
        y: parse_u32("gradient-rect.y", parts[1])?,
        width: parse_u32("gradient-rect.width", parts[2])?,
        height: parse_u32("gradient-rect.height", parts[3])?,
        top_left: parse_color(parts[4])?,
        top_right: parse_color(parts[5])?,
        bottom_left: parse_color(parts[6])?,
        bottom_right: parse_color(parts[7])?,
    })
}

pub fn parse_line(value: &str) -> Result<DrawOp, FrameScriptError> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 5 {
        return Err(FrameScriptError::InvalidLine(format!("line={value}")));
    }
    Ok(DrawOp::Line {
        x0: parse_u32("line.x0", parts[0])?,
        y0: parse_u32("line.y0", parts[1])?,
        x1: parse_u32("line.x1", parts[2])?,
        y1: parse_u32("line.y1", parts[3])?,
        color: parse_color(parts[4])?,
    })
}

pub fn parse_rounded_rect(value: &str) -> Result<DrawOp, FrameScriptError> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 6 {
        return Err(FrameScriptError::InvalidLine(format!(
            "rounded-rect={value}"
        )));
    }
    Ok(DrawOp::RoundedRect {
        x: parse_u32("rounded-rect.x", parts[0])?,
        y: parse_u32("rounded-rect.y", parts[1])?,
        width: parse_u32("rounded-rect.width", parts[2])?,
        height: parse_u32("rounded-rect.height", parts[3])?,
        radius: parse_u32("rounded-rect.radius", parts[4])?,
        color: parse_color(parts[5])?,
    })
}

pub fn parse_shadow_rect(value: &str) -> Result<DrawOp, FrameScriptError> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 6 {
        return Err(FrameScriptError::InvalidLine(format!(
            "shadow-rect={value}"
        )));
    }
    Ok(DrawOp::ShadowRect {
        x: parse_u32("shadow-rect.x", parts[0])?,
        y: parse_u32("shadow-rect.y", parts[1])?,
        width: parse_u32("shadow-rect.width", parts[2])?,
        height: parse_u32("shadow-rect.height", parts[3])?,
        blur: parse_u32("shadow-rect.blur", parts[4])?,
        color: parse_color(parts[5])?,
    })
}

pub fn parse_sprite(value: &str) -> Result<DrawOp, FrameScriptError> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 5 || parts[0].is_empty() {
        return Err(FrameScriptError::InvalidLine(format!("sprite={value}")));
    }
    Ok(DrawOp::Sprite {
        sprite: parts[0].to_string(),
        x: parse_u32("sprite.x", parts[1])?,
        y: parse_u32("sprite.y", parts[2])?,
        width: parse_u32("sprite.width", parts[3])?,
        height: parse_u32("sprite.height", parts[4])?,
    })
}

pub fn parse_blit(value: &str) -> Result<DrawOp, FrameScriptError> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 5 || parts[0].is_empty() {
        return Err(FrameScriptError::InvalidLine(format!("blit={value}")));
    }
    Ok(DrawOp::Blit {
        source: parts[0].to_string(),
        x: parse_u32("blit.x", parts[1])?,
        y: parse_u32("blit.y", parts[2])?,
        width: parse_u32("blit.width", parts[3])?,
        height: parse_u32("blit.height", parts[4])?,
    })
}

pub fn parse_triangle(value: &str) -> Result<DrawOp, FrameScriptError> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 7 {
        return Err(FrameScriptError::InvalidLine(format!("triangle={value}")));
    }
    Ok(DrawOp::Triangle {
        x0: parse_u32("triangle.x0", parts[0])?,
        y0: parse_u32("triangle.y0", parts[1])?,
        x1: parse_u32("triangle.x1", parts[2])?,
        y1: parse_u32("triangle.y1", parts[3])?,
        x2: parse_u32("triangle.x2", parts[4])?,
        y2: parse_u32("triangle.y2", parts[5])?,
        color: parse_color(parts[6])?,
    })
}

pub fn parse_ellipse(value: &str) -> Result<DrawOp, FrameScriptError> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 5 {
        return Err(FrameScriptError::InvalidLine(format!("ellipse={value}")));
    }
    let op = DrawOp::Ellipse {
        x: parse_u32("ellipse.x", parts[0])?,
        y: parse_u32("ellipse.y", parts[1])?,
        width: parse_u32("ellipse.width", parts[2])?,
        height: parse_u32("ellipse.height", parts[3])?,
        color: parse_color(parts[4])?,
    };
    op.validate_args()?;
    Ok(op)
}

pub fn parse_push_layer(value: &str) -> Result<DrawOp, FrameScriptError> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 1 || parts[0].is_empty() {
        return Err(FrameScriptError::InvalidLine(format!("push-layer={value}")));
    }
    Ok(DrawOp::PushLayer {
        opacity: parse_u8("push-layer.opacity", parts[0])?,
    })
}

pub fn parse_begin_pass(value: &str) -> Result<DrawOp, FrameScriptError> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(FrameScriptError::InvalidLine(format!("begin-pass={value}")));
    }
    let op = DrawOp::BeginPass {
        label: parts[0].to_string(),
        class: parse_render_pass_class(parts[1])?,
    };
    op.validate_args()?;
    Ok(op)
}

pub fn parse_set_blend_mode(value: &str) -> Result<DrawOp, FrameScriptError> {
    let mode = parse_blend_mode(value.trim())?;
    Ok(DrawOp::SetBlendMode { mode })
}

pub fn parse_set_clip(value: &str) -> Result<DrawOp, FrameScriptError> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 4 {
        return Err(FrameScriptError::InvalidLine(format!("set-clip={value}")));
    }
    let op = DrawOp::SetClip {
        x: parse_u32("set-clip.x", parts[0])?,
        y: parse_u32("set-clip.y", parts[1])?,
        width: parse_u32("set-clip.width", parts[2])?,
        height: parse_u32("set-clip.height", parts[3])?,
    };
    op.validate_args()?;
    Ok(op)
}

pub fn parse_gaussian_blur(value: &str) -> Result<DrawOp, FrameScriptError> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 5 {
        return Err(FrameScriptError::InvalidLine(format!(
            "gaussian-blur={value}"
        )));
    }
    let op = DrawOp::GaussianBlur {
        x: parse_u32("gaussian-blur.x", parts[0])?,
        y: parse_u32("gaussian-blur.y", parts[1])?,
        width: parse_u32("gaussian-blur.width", parts[2])?,
        height: parse_u32("gaussian-blur.height", parts[3])?,
        radius: parse_u32("gaussian-blur.radius", parts[4])?,
    };
    op.validate_args()?;
    Ok(op)
}

pub fn parse_backdrop(value: &str) -> Result<DrawOp, FrameScriptError> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 5 {
        return Err(FrameScriptError::InvalidLine(format!("backdrop={value}")));
    }
    let op = DrawOp::Backdrop {
        x: parse_u32("backdrop.x", parts[0])?,
        y: parse_u32("backdrop.y", parts[1])?,
        width: parse_u32("backdrop.width", parts[2])?,
        height: parse_u32("backdrop.height", parts[3])?,
        opacity: parse_u8("backdrop.opacity", parts[4])?,
    };
    op.validate_args()?;
    Ok(op)
}

pub fn parse_set_present_region(value: &str) -> Result<DrawOp, FrameScriptError> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 4 {
        return Err(FrameScriptError::InvalidLine(format!(
            "set-present-region={value}"
        )));
    }
    let op = DrawOp::SetPresentRegion {
        x: parse_u32("set-present-region.x", parts[0])?,
        y: parse_u32("set-present-region.y", parts[1])?,
        width: parse_u32("set-present-region.width", parts[2])?,
        height: parse_u32("set-present-region.height", parts[3])?,
    };
    op.validate_args()?;
    Ok(op)
}

pub fn parse_flip_region(value: &str) -> Result<DrawOp, FrameScriptError> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 4 {
        return Err(FrameScriptError::InvalidLine(format!(
            "flip-region={value}"
        )));
    }
    let op = DrawOp::FlipRegion {
        x: parse_u32("flip-region.x", parts[0])?,
        y: parse_u32("flip-region.y", parts[1])?,
        width: parse_u32("flip-region.width", parts[2])?,
        height: parse_u32("flip-region.height", parts[3])?,
    };
    op.validate_args()?;
    Ok(op)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn red() -> RgbaColor {
        RgbaColor {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        }
    }

    #[test]
    fn draw_op_class_geometry() {
        assert_eq!(
            DrawOp::Clear { color: red() }.class(),
            DrawOpClass::Geometry
        );
        assert_eq!(
            DrawOp::Triangle {
                x0: 0,
                y0: 0,
                x1: 10,
                y1: 0,
                x2: 5,
                y2: 10,
                color: red()
            }
            .class(),
            DrawOpClass::Geometry
        );
        assert_eq!(
            DrawOp::Ellipse {
                x: 0,
                y: 0,
                width: 10,
                height: 10,
                color: red()
            }
            .class(),
            DrawOpClass::Geometry
        );
    }

    #[test]
    fn draw_op_class_composition() {
        assert_eq!(
            DrawOp::PushLayer { opacity: 128 }.class(),
            DrawOpClass::Composition
        );
        assert_eq!(DrawOp::PopLayer.class(), DrawOpClass::Composition);
        assert_eq!(
            DrawOp::SetClip {
                x: 0,
                y: 0,
                width: 100,
                height: 100
            }
            .class(),
            DrawOpClass::Composition
        );
        assert_eq!(DrawOp::ClearClip.class(), DrawOpClass::Composition);
    }

    #[test]
    fn draw_op_class_effect() {
        assert_eq!(
            DrawOp::GaussianBlur {
                x: 0,
                y: 0,
                width: 100,
                height: 100,
                radius: 5
            }
            .class(),
            DrawOpClass::Effect
        );
        assert_eq!(
            DrawOp::Backdrop {
                x: 0,
                y: 0,
                width: 100,
                height: 100,
                opacity: 200
            }
            .class(),
            DrawOpClass::Effect
        );
        assert_eq!(
            DrawOp::ShadowRect {
                x: 0,
                y: 0,
                width: 100,
                height: 100,
                blur: 8,
                color: RgbaColor {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 128
                }
            }
            .class(),
            DrawOpClass::Effect
        );
    }

    #[test]
    fn draw_op_class_presentation() {
        assert_eq!(
            DrawOp::SetPresentRegion {
                x: 0,
                y: 0,
                width: 640,
                height: 480
            }
            .class(),
            DrawOpClass::Presentation
        );
        assert_eq!(
            DrawOp::FlipRegion {
                x: 0,
                y: 0,
                width: 640,
                height: 480
            }
            .class(),
            DrawOpClass::Presentation
        );
    }

    #[test]
    fn rejects_gaussian_blur_zero_radius() {
        let err = parse_gaussian_blur("0,0,100,100,0").unwrap_err();
        assert!(err.describe().contains("gaussian-blur"));
    }

    #[test]
    fn rejects_ellipse_zero_width() {
        let err = parse_ellipse("0,0,0,100,ff0000ff").unwrap_err();
        assert!(err.describe().contains("ellipse"));
    }

    #[test]
    fn rejects_set_clip_zero_height() {
        let err = parse_set_clip("0,0,100,0").unwrap_err();
        assert!(err.describe().contains("set-clip"));
    }

    #[test]
    fn rejects_flip_region_zero_dimensions() {
        let err = parse_flip_region("0,0,640,0").unwrap_err();
        assert!(err.describe().contains("flip-region"));
    }

    #[test]
    fn rejects_set_present_region_zero_dimensions() {
        let err = parse_set_present_region("0,0,0,480").unwrap_err();
        assert!(err.describe().contains("set-present-region"));
    }

    #[test]
    fn encodes_triangle() {
        let op = DrawOp::Triangle {
            x0: 0,
            y0: 0,
            x1: 10,
            y1: 0,
            x2: 5,
            y2: 10,
            color: red(),
        };
        let line = op.encode_line();
        assert!(line.contains("op=triangle"));
        assert!(line.contains("x0=0"));
        assert!(line.contains("x2=5"));
        assert!(line.contains("rgba=ff0000ff"));
    }

    #[test]
    fn encodes_gaussian_blur() {
        let op = DrawOp::GaussianBlur {
            x: 10,
            y: 20,
            width: 100,
            height: 80,
            radius: 4,
        };
        let line = op.encode_line();
        assert!(line.contains("op=gaussian-blur"));
        assert!(line.contains("radius=4"));
    }

    #[test]
    fn encodes_stateless_ops() {
        assert_eq!(DrawOp::PopLayer.encode_line(), "op=pop-layer");
        assert_eq!(DrawOp::ClearClip.encode_line(), "op=clear-clip");
    }
}
