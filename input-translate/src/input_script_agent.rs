use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputOp {
    Button { name: String, action: String },
    Axis { name: String, value: i32 },
    Pointer { x: i32, y: i32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputScript {
    pub device: String,
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
        let mut device = None::<String>;
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
                device = Some(parse_named_value("device", value)?);
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
            device: device.ok_or(InputScriptError::MissingField("device"))?,
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
        if self.frame_tag.is_empty() {
            return Err(InputScriptError::InvalidValue {
                key: String::from("frame"),
                value: self.frame_tag.clone(),
            });
        }
        if !matches!(
            self.device.as_str(),
            "gamepad" | "keyboard" | "mouse" | "touch" | "stylus"
        ) {
            return Err(InputScriptError::InvalidValue {
                key: String::from("device"),
                value: self.device.clone(),
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
        if !matches!(
            self.pointer_capture.as_str(),
            "relative-lock" | "absolute" | "none"
        ) {
            return Err(InputScriptError::InvalidValue {
                key: String::from("pointer-capture"),
                value: self.pointer_capture.clone(),
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
            format!("device={}", self.device),
            format!("family={}", self.device_family),
            format!("frame={}", self.frame_tag),
            format!("layout={}", self.layout),
            format!("key-table={}", self.key_table),
            format!("pointer-capture={}", self.pointer_capture),
            format!("delivery={}", self.delivery),
        ];
        for op in &self.ops {
            match op {
                InputOp::Button { name, action } => {
                    lines.push(format!("op=button name={name} action={action}"))
                }
                InputOp::Axis { name, value } => {
                    lines.push(format!("op=axis name={name} value={value}"))
                }
                InputOp::Pointer { x, y } => lines.push(format!("op=pointer x={x} y={y}")),
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

fn parse_named_value(key: &str, value: &str) -> Result<String, InputScriptError> {
    if value.is_empty() {
        return Err(InputScriptError::InvalidValue {
            key: key.to_string(),
            value: value.to_string(),
        });
    }
    Ok(value.to_string())
}

fn parse_button(value: &str) -> Result<InputOp, InputScriptError> {
    let Some((name, action)) = value.split_once(',') else {
        return Err(InputScriptError::InvalidLine(format!("button={value}")));
    };
    if name.is_empty() || action.is_empty() {
        return Err(InputScriptError::InvalidLine(format!("button={value}")));
    }
    if !matches!(action, "press" | "release" | "hold") {
        return Err(InputScriptError::InvalidValue {
            key: String::from("button.action"),
            value: action.to_string(),
        });
    }
    Ok(InputOp::Button {
        name: name.to_string(),
        action: action.to_string(),
    })
}

fn parse_axis(value: &str) -> Result<InputOp, InputScriptError> {
    let Some((name, val)) = value.split_once(',') else {
        return Err(InputScriptError::InvalidLine(format!("axis={value}")));
    };
    if name.is_empty() {
        return Err(InputScriptError::InvalidLine(format!("axis={value}")));
    }
    let parsed = val
        .parse::<i32>()
        .map_err(|_| InputScriptError::InvalidValue {
            key: String::from("axis.value"),
            value: val.to_string(),
        })?;
    Ok(InputOp::Axis {
        name: name.to_string(),
        value: parsed,
    })
}

fn parse_pointer(value: &str) -> Result<InputOp, InputScriptError> {
    let parts: Vec<&str> = value.split(',').collect();
    if parts.len() != 2 {
        return Err(InputScriptError::InvalidLine(format!("pointer={value}")));
    }
    let x = parts[0]
        .parse::<i32>()
        .map_err(|_| InputScriptError::InvalidValue {
            key: String::from("pointer.x"),
            value: parts[0].to_string(),
        })?;
    let y = parts[1]
        .parse::<i32>()
        .map_err(|_| InputScriptError::InvalidValue {
            key: String::from("pointer.y"),
            value: parts[1].to_string(),
        })?;
    Ok(InputOp::Pointer { x, y })
}

/// API-uri de input externe pe care le suportăm ca sursă de traducere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForeignInputApi {
    DirectInput,
    XInput,
    HidApi,
    IOKit,
    Evdev,
    SDL,
    /// API necunoscut / neacoperit — refuse
    Other,
}

impl ForeignInputApi {
    pub fn name(self) -> &'static str {
        match self {
            Self::DirectInput => "directinput",
            Self::XInput => "xinput",
            Self::HidApi => "hidapi",
            Self::IOKit => "iokit",
            Self::Evdev => "evdev",
            Self::SDL => "sdl",
            Self::Other => "other",
        }
    }

    pub fn translation_label(self) -> &'static str {
        match self {
            Self::Evdev | Self::HidApi => "native-input",
            Self::Other => "unsupported",
            _ => "compat-to-input",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "directinput" => Some(Self::DirectInput),
            "xinput" => Some(Self::XInput),
            "hidapi" => Some(Self::HidApi),
            "iokit" => Some(Self::IOKit),
            "evdev" => Some(Self::Evdev),
            "sdl" => Some(Self::SDL),
            "other" => Some(Self::Other),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputTranslateError {
    UnsupportedApi(String),
    EmptyScript,
}

impl InputTranslateError {
    pub fn describe(&self) -> String {
        match self {
            Self::UnsupportedApi(api) => format!("unsupported input api={api}"),
            Self::EmptyScript => String::from("empty input script"),
        }
    }
}

pub struct InputTranslator {
    source_api: ForeignInputApi,
}

impl InputTranslator {
    pub fn new(source_api: ForeignInputApi) -> Self {
        Self { source_api }
    }

    pub fn translate(&self, script: &InputScript) -> Result<EncodedInput, InputTranslateError> {
        if self.source_api == ForeignInputApi::Other {
            return Err(InputTranslateError::UnsupportedApi(String::from(
                self.source_api.name(),
            )));
        }
        if script.ops.is_empty() {
            return Err(InputTranslateError::EmptyScript);
        }
        let profile = format!("{}-{}", self.source_api.name(), script.device_family);
        Ok(script.encode(&profile))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_encodes_input_script() {
        let script = InputScript::parse(
            "device=gamepad\nfamily=dualshock\nframe=input-001\nlayout=gamepad-standard\nkey-table=us-game\npointer-capture=relative-lock\ndelivery=immediate\nbutton=cross,press\n",
        )
        .unwrap();
        let encoded = script.encode("gamepad-hid");
        assert_eq!(encoded.frame_tag, "input-001");
        assert_eq!(encoded.device_family, "dualshock");
        assert_eq!(encoded.layout, "gamepad-standard");
        assert_eq!(encoded.key_table, "us-game");
        assert_eq!(encoded.pointer_capture, "relative-lock");
        assert_eq!(encoded.delivery, "immediate");
        assert_eq!(encoded.op_count, 1);
        assert!(encoded.payload.contains("profile=gamepad-hid"));
        assert!(encoded.payload.contains("device=gamepad"));
        assert!(encoded.payload.contains("family=dualshock"));
        assert!(
            encoded
                .payload
                .contains("op=button name=cross action=press")
        );
    }

    #[test]
    fn rejects_invalid_device_and_delivery() {
        let err = InputScript::parse(
            "device=joystick\nfamily=generic\nframe=f1\nlayout=gamepad-standard\nkey-table=us\npointer-capture=none\ndelivery=immediate\nbutton=a,press\n",
        )
        .unwrap_err();
        assert!(!err.describe().is_empty());

        let err = InputScript::parse(
            "device=gamepad\nfamily=generic\nframe=f1\nlayout=gamepad-standard\nkey-table=us\npointer-capture=none\ndelivery=unknown\nbutton=a,press\n",
        )
        .unwrap_err();
        assert!(!err.describe().is_empty());
    }

    #[test]
    fn input_translator_encodes_xinput_to_native() {
        use super::{ForeignInputApi, InputTranslator};
        let script = InputScript::parse(
            "device=gamepad\nfamily=xbox\nframe=f1\nlayout=gamepad-standard\nkey-table=us-game\npointer-capture=none\ndelivery=immediate\nbutton=a,press\n",
        )
        .unwrap();
        let translator = InputTranslator::new(ForeignInputApi::XInput);
        let encoded = translator.translate(&script).unwrap();
        assert_eq!(encoded.frame_tag, "f1");
        assert!(encoded.payload.contains("profile=xinput-xbox"));
        assert!(encoded.payload.contains("ngos-input-translate/v1"));
    }

    #[test]
    fn input_translator_refuses_other_api() {
        use super::{ForeignInputApi, InputTranslateError, InputTranslator};
        let script = InputScript::parse(
            "device=gamepad\nfamily=generic\nframe=f1\nlayout=gamepad-standard\nkey-table=us-game\npointer-capture=none\ndelivery=immediate\nbutton=a,press\n",
        )
        .unwrap();
        let translator = InputTranslator::new(ForeignInputApi::Other);
        assert!(matches!(
            translator.translate(&script),
            Err(InputTranslateError::UnsupportedApi(_))
        ));
    }

    #[test]
    fn rejects_missing_ops() {
        let err = InputScript::parse(
            "device=keyboard\nfamily=generic\nframe=f2\nlayout=us-104\nkey-table=us\npointer-capture=none\ndelivery=immediate\n",
        )
        .unwrap_err();
        assert!(!err.describe().is_empty());
    }
}
