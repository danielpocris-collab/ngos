#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    Press,
    Release,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputOp {
    Key {
        code: String,
        action: InputAction,
    },
    Button {
        code: String,
        action: InputAction,
    },
    Axis {
        axis: String,
        value_milli: i16,
    },
    Pointer {
        dx: i32,
        dy: i32,
        wheel_x: i16,
        wheel_y: i16,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputScript {
    pub device_class: String,
    pub device_family: String,
    pub frame_tag: String,
    pub layout: String,
    pub key_table: String,
    pub pointer_capture: String,
    pub delivery: String,
    pub ops: Vec<InputOp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedInput {
    pub frame_tag: String,
    pub device_family: String,
    pub layout: String,
    pub key_table: String,
    pub pointer_capture: String,
    pub delivery: String,
    pub op_count: usize,
    pub payload: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputScriptError {
    MissingField(&'static str),
    InvalidLine(String),
    InvalidValue { key: String, value: String },
}

impl InputScriptError {
    pub fn describe(&self) -> String {
        match self {
            Self::MissingField(field) => format!("missing field {field}"),
            Self::InvalidLine(line) => format!("invalid line {line}"),
            Self::InvalidValue { key, value } => format!("invalid value {key}={value}"),
        }
    }
}

impl InputScript {
    pub fn parse(text: &str) -> Result<Self, InputScriptError> {
        let mut device_class = None::<String>;
        let mut device_family = None::<String>;
        let mut frame_tag = None::<String>;
        let mut layout = None::<String>;
        let mut key_table = None::<String>;
        let mut pointer_capture = None::<String>;
        let mut delivery = None::<String>;
        let mut ops = Vec::new();

        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(value) = line.strip_prefix("device=") {
                if value.is_empty() {
                    return Err(InputScriptError::InvalidValue {
                        key: String::from("device"),
                        value: value.to_string(),
                    });
                }
                device_class = Some(value.to_string());
                continue;
            }
            if let Some(value) = line.strip_prefix("family=") {
                device_family = Some(parse_named_value("family", value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("frame=") {
                if value.is_empty() {
                    return Err(InputScriptError::InvalidValue {
                        key: String::from("frame"),
                        value: value.to_string(),
                    });
                }
                frame_tag = Some(value.to_string());
                continue;
            }
            if let Some(value) = line.strip_prefix("layout=") {
                layout = Some(parse_named_value("layout", value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("key-table=") {
                key_table = Some(parse_named_value("key-table", value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("pointer-capture=") {
                pointer_capture = Some(parse_named_value("pointer-capture", value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("delivery=") {
                delivery = Some(parse_named_value("delivery", value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("key=") {
                ops.push(parse_key(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("button=") {
                ops.push(parse_button(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("axis=") {
                ops.push(parse_axis(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("pointer=") {
                ops.push(parse_pointer(value)?);
                continue;
            }
            return Err(InputScriptError::InvalidLine(line.to_string()));
        }

        let script = Self {
            device_class: device_class.ok_or(InputScriptError::MissingField("device"))?,
            device_family: device_family.ok_or(InputScriptError::MissingField("family"))?,
            frame_tag: frame_tag.ok_or(InputScriptError::MissingField("frame"))?,
            layout: layout.ok_or(InputScriptError::MissingField("layout"))?,
            key_table: key_table.ok_or(InputScriptError::MissingField("key-table"))?,
            pointer_capture: pointer_capture
                .ok_or(InputScriptError::MissingField("pointer-capture"))?,
            delivery: delivery.ok_or(InputScriptError::MissingField("delivery"))?,
            ops,
        };
        script.validate()?;
        Ok(script)
    }

    pub fn validate(&self) -> Result<(), InputScriptError> {
        if self.device_class.is_empty() {
            return Err(InputScriptError::InvalidValue {
                key: String::from("device"),
                value: self.device_class.clone(),
            });
        }
        if self.frame_tag.is_empty() {
            return Err(InputScriptError::InvalidValue {
                key: String::from("frame"),
                value: self.frame_tag.clone(),
            });
        }
        if !matches!(
            self.device_family.as_str(),
            "xinput" | "dualshock" | "switch-pro" | "keyboard-mouse"
        ) {
            return Err(InputScriptError::InvalidValue {
                key: String::from("family"),
                value: self.device_family.clone(),
            });
        }
        if self.device_class == "gamepad" && self.device_family == "keyboard-mouse" {
            return Err(InputScriptError::InvalidValue {
                key: String::from("family"),
                value: self.device_family.clone(),
            });
        }
        if self.device_class == "keyboard-mouse" && self.device_family != "keyboard-mouse" {
            return Err(InputScriptError::InvalidValue {
                key: String::from("family"),
                value: self.device_family.clone(),
            });
        }
        if !matches!(
            self.layout.as_str(),
            "gamepad-standard" | "gamepad-southpaw" | "keyboard-mouse"
        ) {
            return Err(InputScriptError::InvalidValue {
                key: String::from("layout"),
                value: self.layout.clone(),
            });
        }
        if !matches!(
            self.key_table.as_str(),
            "us-pc" | "us-game" | "scancode-set2"
        ) {
            return Err(InputScriptError::InvalidValue {
                key: String::from("key-table"),
                value: self.key_table.clone(),
            });
        }
        if !matches!(
            self.pointer_capture.as_str(),
            "relative-lock" | "absolute-free" | "relative-edge"
        ) {
            return Err(InputScriptError::InvalidValue {
                key: String::from("pointer-capture"),
                value: self.pointer_capture.clone(),
            });
        }
        if !matches!(
            self.delivery.as_str(),
            "immediate" | "wait-batch" | "wait-frame"
        ) {
            return Err(InputScriptError::InvalidValue {
                key: String::from("delivery"),
                value: self.delivery.clone(),
            });
        }
        if self.ops.is_empty() {
            return Err(InputScriptError::MissingField("input-op"));
        }
        Ok(())
    }

    pub fn encode(&self, profile: &str) -> EncodedInput {
        let mut lines = vec![
            String::from("ngos-input-translate/v1"),
            format!("profile={profile}"),
            format!("device={}", self.device_class),
            format!("family={}", self.device_family),
            format!("frame={}", self.frame_tag),
            format!("layout={}", self.layout),
            format!("key-table={}", self.key_table),
            format!("pointer-capture={}", self.pointer_capture),
            format!("delivery={}", self.delivery),
        ];
        for op in &self.ops {
            match op {
                InputOp::Key { code, action } => lines.push(format!(
                    "op=key code={} action={}",
                    code,
                    action_name(*action)
                )),
                InputOp::Button { code, action } => lines.push(format!(
                    "op=button code={} action={}",
                    canonicalize_button(&self.device_family, code),
                    action_name(*action)
                )),
                InputOp::Axis { axis, value_milli } => {
                    lines.push(format!("op=axis axis={} value={}", axis, value_milli))
                }
                InputOp::Pointer {
                    dx,
                    dy,
                    wheel_x,
                    wheel_y,
                } => lines.push(format!(
                    "op=pointer dx={} dy={} wheel-x={} wheel-y={}",
                    dx, dy, wheel_x, wheel_y
                )),
            }
        }
        EncodedInput {
            frame_tag: self.frame_tag.clone(),
            device_family: self.device_family.clone(),
            layout: self.layout.clone(),
            key_table: self.key_table.clone(),
            pointer_capture: self.pointer_capture.clone(),
            delivery: self.delivery.clone(),
            op_count: self.ops.len(),
            payload: lines.join("\n"),
        }
    }
}

fn canonicalize_button<'a>(family: &str, code: &'a str) -> &'a str {
    match (family, code) {
        ("dualshock", "cross") => "south",
        ("dualshock", "circle") => "east",
        ("dualshock", "square") => "west",
        ("dualshock", "triangle") => "north",
        ("switch-pro", "b") => "south",
        ("switch-pro", "a") => "east",
        ("switch-pro", "y") => "west",
        ("switch-pro", "x") => "north",
        ("xinput", "a") => "south",
        ("xinput", "b") => "east",
        ("xinput", "x") => "west",
        ("xinput", "y") => "north",
        _ => code,
    }
}

fn parse_named_value(key: &str, value: &str) -> Result<String, InputScriptError> {
    if value.is_empty() {
        return Err(InputScriptError::InvalidValue {
            key: key.to_string(),
            value: value.to_string(),
        });
    }
    Ok(value.to_string())
}

fn parse_key(value: &str) -> Result<InputOp, InputScriptError> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 2 || parts[0].is_empty() {
        return Err(InputScriptError::InvalidLine(format!("key={value}")));
    }
    Ok(InputOp::Key {
        code: parts[0].to_string(),
        action: parse_action(parts[1])?,
    })
}

fn parse_button(value: &str) -> Result<InputOp, InputScriptError> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 2 || parts[0].is_empty() {
        return Err(InputScriptError::InvalidLine(format!("button={value}")));
    }
    Ok(InputOp::Button {
        code: parts[0].to_string(),
        action: parse_action(parts[1])?,
    })
}

fn parse_axis(value: &str) -> Result<InputOp, InputScriptError> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 2 || parts[0].is_empty() {
        return Err(InputScriptError::InvalidLine(format!("axis={value}")));
    }
    Ok(InputOp::Axis {
        axis: parts[0].to_string(),
        value_milli: parse_milli("axis.value", parts[1])?,
    })
}

fn parse_pointer(value: &str) -> Result<InputOp, InputScriptError> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 4 {
        return Err(InputScriptError::InvalidLine(format!("pointer={value}")));
    }
    Ok(InputOp::Pointer {
        dx: parse_i32("pointer.dx", parts[0])?,
        dy: parse_i32("pointer.dy", parts[1])?,
        wheel_x: parse_i16("pointer.wheel_x", parts[2])?,
        wheel_y: parse_i16("pointer.wheel_y", parts[3])?,
    })
}

fn parse_action(value: &str) -> Result<InputAction, InputScriptError> {
    match value {
        "press" => Ok(InputAction::Press),
        "release" => Ok(InputAction::Release),
        _ => Err(InputScriptError::InvalidValue {
            key: String::from("action"),
            value: value.to_string(),
        }),
    }
}

fn parse_i32(key: &str, value: &str) -> Result<i32, InputScriptError> {
    value
        .parse::<i32>()
        .map_err(|_| InputScriptError::InvalidValue {
            key: key.to_string(),
            value: value.to_string(),
        })
}

fn parse_i16(key: &str, value: &str) -> Result<i16, InputScriptError> {
    value
        .parse::<i16>()
        .map_err(|_| InputScriptError::InvalidValue {
            key: key.to_string(),
            value: value.to_string(),
        })
}

fn parse_milli(key: &str, value: &str) -> Result<i16, InputScriptError> {
    let negative = value.starts_with('-');
    let digits = if negative { &value[1..] } else { value };
    let (whole_text, frac_text) = match digits.split_once('.') {
        Some(parts) => parts,
        None => (digits, ""),
    };
    if whole_text.is_empty() || frac_text.len() > 3 {
        return Err(InputScriptError::InvalidValue {
            key: key.to_string(),
            value: value.to_string(),
        });
    }
    let whole = whole_text
        .parse::<i32>()
        .map_err(|_| InputScriptError::InvalidValue {
            key: key.to_string(),
            value: value.to_string(),
        })?;
    let frac = if frac_text.is_empty() {
        0
    } else {
        let raw = frac_text
            .parse::<i32>()
            .map_err(|_| InputScriptError::InvalidValue {
                key: key.to_string(),
                value: value.to_string(),
            })?;
        raw * match frac_text.len() {
            1 => 100,
            2 => 10,
            _ => 1,
        }
    };
    let total = whole * 1000 + frac;
    let signed = if negative { -total } else { total };
    if !(-1000..=1000).contains(&signed) {
        return Err(InputScriptError::InvalidValue {
            key: key.to_string(),
            value: value.to_string(),
        });
    }
    Ok(signed as i16)
}

fn action_name(action: InputAction) -> &'static str {
    match action {
        InputAction::Press => "press",
        InputAction::Release => "release",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_encodes_input_script() {
        let script = InputScript::parse(
            "device=gamepad\nfamily=dualshock\nframe=input-001\nlayout=gamepad-standard\nkey-table=us-game\npointer-capture=relative-lock\ndelivery=wait-frame\nbutton=cross,press\naxis=left-x,0.750\npointer=4,-2,0,1\n",
        )
        .unwrap();
        let encoded = script.encode("gamepad-first");
        assert_eq!(encoded.frame_tag, "input-001");
        assert_eq!(encoded.device_family, "dualshock");
        assert_eq!(encoded.layout, "gamepad-standard");
        assert_eq!(encoded.key_table, "us-game");
        assert_eq!(encoded.pointer_capture, "relative-lock");
        assert_eq!(encoded.delivery, "wait-frame");
        assert_eq!(encoded.op_count, 3);
        assert!(encoded.payload.contains("profile=gamepad-first"));
        assert!(encoded.payload.contains("layout=gamepad-standard"));
        assert!(encoded.payload.contains("key-table=us-game"));
        assert!(encoded.payload.contains("pointer-capture=relative-lock"));
        assert!(encoded.payload.contains("delivery=wait-frame"));
        assert!(
            encoded
                .payload
                .contains("op=button code=south action=press")
        );
        assert!(encoded.payload.contains("op=axis axis=left-x value=750"));
    }

    #[test]
    fn rejects_invalid_action_and_missing_ops() {
        let error = InputScript::parse(
            "device=pad\nfamily=unknown\nframe=x\nlayout=unknown\nkey-table=us-game\npointer-capture=relative-lock\ndelivery=immediate\nbutton=a,hold\n",
        )
        .unwrap_err();
        assert!(!error.describe().is_empty());
    }
}
