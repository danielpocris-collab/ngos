#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbaColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawOp {
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
    ShadowRect {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        blur: u32,
        color: RgbaColor,
    },
    Sprite {
        sprite: String,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    Blit {
        source: String,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameScript {
    pub width: u32,
    pub height: u32,
    pub frame_tag: String,
    pub queue: String,
    pub present_mode: String,
    pub completion: String,
    pub ops: Vec<DrawOp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedFrame {
    pub frame_tag: String,
    pub queue: String,
    pub present_mode: String,
    pub completion: String,
    pub op_count: usize,
    pub payload: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameScriptError {
    MissingField(&'static str),
    InvalidLine(String),
    InvalidValue { key: String, value: String },
}

impl FrameScriptError {
    pub fn describe(&self) -> String {
        match self {
            Self::MissingField(field) => format!("missing field {field}"),
            Self::InvalidLine(line) => format!("invalid line {line}"),
            Self::InvalidValue { key, value } => format!("invalid value {key}={value}"),
        }
    }
}

impl FrameScript {
    pub fn parse(text: &str) -> Result<Self, FrameScriptError> {
        let mut width = None::<u32>;
        let mut height = None::<u32>;
        let mut frame_tag = None::<String>;
        let mut queue = None::<String>;
        let mut present_mode = None::<String>;
        let mut completion = None::<String>;
        let mut ops = Vec::new();

        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(value) = line.strip_prefix("surface=") {
                let Some((w, h)) = value.split_once('x') else {
                    return Err(FrameScriptError::InvalidLine(line.to_string()));
                };
                width = Some(parse_u32("surface.width", w)?);
                height = Some(parse_u32("surface.height", h)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("frame=") {
                if value.is_empty() {
                    return Err(FrameScriptError::InvalidValue {
                        key: String::from("frame"),
                        value: value.to_string(),
                    });
                }
                frame_tag = Some(value.to_string());
                continue;
            }
            if let Some(value) = line.strip_prefix("queue=") {
                queue = Some(parse_named_value("queue", value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("present-mode=") {
                present_mode = Some(parse_named_value("present-mode", value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("completion=") {
                completion = Some(parse_named_value("completion", value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("clear=") {
                ops.push(DrawOp::Clear {
                    color: parse_color(value)?,
                });
                continue;
            }
            if let Some(value) = line.strip_prefix("line=") {
                ops.push(parse_line(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("gradient-rect=") {
                ops.push(parse_gradient_rect(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("rect=") {
                ops.push(parse_rect(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("rounded-rect=") {
                ops.push(parse_rounded_rect(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("shadow-rect=") {
                ops.push(parse_shadow_rect(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("sprite=") {
                ops.push(parse_sprite(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("blit=") {
                ops.push(parse_blit(value)?);
                continue;
            }
            return Err(FrameScriptError::InvalidLine(line.to_string()));
        }

        let script = Self {
            width: width.ok_or(FrameScriptError::MissingField("surface"))?,
            height: height.ok_or(FrameScriptError::MissingField("surface"))?,
            frame_tag: frame_tag.ok_or(FrameScriptError::MissingField("frame"))?,
            queue: queue.ok_or(FrameScriptError::MissingField("queue"))?,
            present_mode: present_mode.ok_or(FrameScriptError::MissingField("present-mode"))?,
            completion: completion.ok_or(FrameScriptError::MissingField("completion"))?,
            ops,
        };
        script.validate()?;
        Ok(script)
    }

    pub fn validate(&self) -> Result<(), FrameScriptError> {
        if self.width == 0 {
            return Err(FrameScriptError::InvalidValue {
                key: String::from("surface.width"),
                value: self.width.to_string(),
            });
        }
        if self.height == 0 {
            return Err(FrameScriptError::InvalidValue {
                key: String::from("surface.height"),
                value: self.height.to_string(),
            });
        }
        if self.frame_tag.is_empty() {
            return Err(FrameScriptError::InvalidValue {
                key: String::from("frame"),
                value: self.frame_tag.clone(),
            });
        }
        if !matches!(self.queue.as_str(), "graphics" | "present" | "transfer") {
            return Err(FrameScriptError::InvalidValue {
                key: String::from("queue"),
                value: self.queue.clone(),
            });
        }
        if !matches!(self.present_mode.as_str(), "fifo" | "mailbox" | "immediate") {
            return Err(FrameScriptError::InvalidValue {
                key: String::from("present-mode"),
                value: self.present_mode.clone(),
            });
        }
        if !matches!(
            self.completion.as_str(),
            "fire-and-forget" | "wait-present" | "wait-complete"
        ) {
            return Err(FrameScriptError::InvalidValue {
                key: String::from("completion"),
                value: self.completion.clone(),
            });
        }
        if self.ops.is_empty() {
            return Err(FrameScriptError::MissingField("draw-op"));
        }
        Ok(())
    }

    pub fn encode(&self, profile: &str) -> EncodedFrame {
        let mut lines = vec![
            String::from("ngos-gfx-translate/v1"),
            format!("profile={profile}"),
            format!("surface={}x{}", self.width, self.height),
            format!("frame={}", self.frame_tag),
            format!("queue={}", self.queue),
            format!("present-mode={}", self.present_mode),
            format!("completion={}", self.completion),
        ];
        for op in &self.ops {
            match op {
                DrawOp::Clear { color } => lines.push(format!(
                    "op=clear rgba={:02x}{:02x}{:02x}{:02x}",
                    color.r, color.g, color.b, color.a
                )),
                DrawOp::GradientRect {
                    x,
                    y,
                    width,
                    height,
                    top_left,
                    top_right,
                    bottom_left,
                    bottom_right,
                } => lines.push(format!(
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
                    bottom_right.a
                )),
                DrawOp::Line {
                    x0,
                    y0,
                    x1,
                    y1,
                    color,
                } => lines.push(format!(
                    "op=line x0={} y0={} x1={} y1={} rgba={:02x}{:02x}{:02x}{:02x}",
                    x0, y0, x1, y1, color.r, color.g, color.b, color.a
                )),
                DrawOp::Rect {
                    x,
                    y,
                    width,
                    height,
                    color,
                } => lines.push(format!(
                    "op=rect x={} y={} w={} h={} rgba={:02x}{:02x}{:02x}{:02x}",
                    x, y, width, height, color.r, color.g, color.b, color.a
                )),
                DrawOp::RoundedRect {
                    x,
                    y,
                    width,
                    height,
                    radius,
                    color,
                } => lines.push(format!(
                    "op=rounded-rect x={} y={} w={} h={} radius={} rgba={:02x}{:02x}{:02x}{:02x}",
                    x,
                    y,
                    width,
                    height,
                    radius,
                    color.r,
                    color.g,
                    color.b,
                    color.a
                )),
                DrawOp::ShadowRect {
                    x,
                    y,
                    width,
                    height,
                    blur,
                    color,
                } => lines.push(format!(
                    "op=shadow-rect x={} y={} w={} h={} blur={} rgba={:02x}{:02x}{:02x}{:02x}",
                    x,
                    y,
                    width,
                    height,
                    blur,
                    color.r,
                    color.g,
                    color.b,
                    color.a
                )),
                DrawOp::Sprite {
                    sprite,
                    x,
                    y,
                    width,
                    height,
                } => lines.push(format!(
                    "op=sprite id={} x={} y={} w={} h={}",
                    sprite, x, y, width, height
                )),
                DrawOp::Blit {
                    source,
                    x,
                    y,
                    width,
                    height,
                } => lines.push(format!(
                    "op=blit source={} x={} y={} w={} h={}",
                    source, x, y, width, height
                )),
            }
        }
        EncodedFrame {
            frame_tag: self.frame_tag.clone(),
            queue: self.queue.clone(),
            present_mode: self.present_mode.clone(),
            completion: self.completion.clone(),
            op_count: self.ops.len(),
            payload: lines.join("\n"),
        }
    }
}

fn parse_named_value(key: &str, value: &str) -> Result<String, FrameScriptError> {
    if value.is_empty() {
        return Err(FrameScriptError::InvalidValue {
            key: key.to_string(),
            value: value.to_string(),
        });
    }
    Ok(value.to_string())
}

fn parse_u32(key: &str, value: &str) -> Result<u32, FrameScriptError> {
    value
        .parse::<u32>()
        .map_err(|_| FrameScriptError::InvalidValue {
            key: key.to_string(),
            value: value.to_string(),
        })
}

fn parse_color(value: &str) -> Result<RgbaColor, FrameScriptError> {
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

fn parse_rect(value: &str) -> Result<DrawOp, FrameScriptError> {
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

fn parse_gradient_rect(value: &str) -> Result<DrawOp, FrameScriptError> {
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

fn parse_line(value: &str) -> Result<DrawOp, FrameScriptError> {
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

fn parse_rounded_rect(value: &str) -> Result<DrawOp, FrameScriptError> {
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

fn parse_shadow_rect(value: &str) -> Result<DrawOp, FrameScriptError> {
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

fn parse_sprite(value: &str) -> Result<DrawOp, FrameScriptError> {
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

fn parse_blit(value: &str) -> Result<DrawOp, FrameScriptError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_encodes_frame_script() {
        let script = FrameScript::parse(
            "surface=1280x720\nframe=orbit-001\nqueue=graphics\npresent-mode=mailbox\ncompletion=wait-present\nclear=#112233\ngradient-rect=0,0,1280,720,#112233ff,#223344ff,#334455ff,#445566ff\nline=0,0,1279,719,#44ccffff\nrect=10,20,200,100,#ff8800ff\nrounded-rect=30,40,220,120,18,#7788ccdd\nshadow-rect=24,32,236,136,24,#00000044\nsprite=ship-main,400,220,96,96\nblit=hud-overlay,0,0,1280,64\n",
        )
        .unwrap();
        let encoded = script.encode("frame-pace");
        assert_eq!(encoded.frame_tag, "orbit-001");
        assert_eq!(encoded.queue, "graphics");
        assert_eq!(encoded.present_mode, "mailbox");
        assert_eq!(encoded.completion, "wait-present");
        assert_eq!(encoded.op_count, 8);
        assert!(encoded.payload.contains("profile=frame-pace"));
        assert!(encoded.payload.contains("queue=graphics"));
        assert!(encoded.payload.contains("present-mode=mailbox"));
        assert!(encoded.payload.contains("completion=wait-present"));
        assert!(
            encoded
                .payload
                .contains("op=gradient-rect x=0 y=0 w=1280 h=720")
        );
        assert!(
            encoded
                .payload
                .contains("op=line x0=0 y0=0 x1=1279 y1=719 rgba=44ccffff")
        );
        assert!(
            encoded
                .payload
                .contains("op=rect x=10 y=20 w=200 h=100 rgba=ff8800ff")
        );
        assert!(
            encoded
                .payload
                .contains("op=rounded-rect x=30 y=40 w=220 h=120 radius=18 rgba=7788ccdd")
        );
        assert!(
            encoded
                .payload
                .contains("op=shadow-rect x=24 y=32 w=236 h=136 blur=24 rgba=00000044")
        );
        assert!(encoded.payload.contains("op=sprite id=ship-main"));
        assert!(
            encoded
                .payload
                .contains("op=blit source=hud-overlay x=0 y=0 w=1280 h=64")
        );
    }

    #[test]
    fn rejects_invalid_extended_shape_arguments() {
        let error =
            FrameScript::parse("surface=64x64\nframe=x\nqueue=graphics\npresent-mode=fifo\ncompletion=wait-present\nrounded-rect=1,2,3,#ff00ff\n")
                .unwrap_err();
        assert!(!error.describe().is_empty());

        let error =
            FrameScript::parse("surface=64x64\nframe=x\nqueue=graphics\npresent-mode=fifo\ncompletion=wait-present\ngradient-rect=0,0,64,64,#111111ff,#222222ff,#333333ff\n")
                .unwrap_err();
        assert!(!error.describe().is_empty());
    }

    #[test]
    fn rejects_missing_surface_and_invalid_color() {
        let error = FrameScript::parse(
            "surface=1x1\nframe=x\nqueue=unknown\npresent-mode=fifo\ncompletion=wait-present\nclear=#11zz33\n",
        )
        .unwrap_err();
        assert!(!error.describe().is_empty());
    }
}
