#![cfg_attr(not(test), no_std)]

//! Canonical subsystem role:
//! - subsystem: audio translation support
//! - owner layer: support translation layer
//! - semantic owner: `audio-translate`
//! - truth path role: translation and scripting support for audio-oriented
//!   runtime/userland flows
//!
//! Canonical contract families defined here:
//! - audio mix script contracts
//! - waveform and encoding contracts
//! - foreign-audio translation contracts
//!
//! This crate may define translation support for audio flows, but it must not
//! redefine kernel, device-runtime, or product-level audio truth.

extern crate alloc;

use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waveform {
    Sine,
    Square,
    Triangle,
    Noise,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MixOp {
    Tone {
        bus: String,
        hz: u32,
        millis: u32,
        gain_milli: u16,
        pan_milli: i16,
        waveform: Waveform,
    },
    Clip {
        bus: String,
        clip: String,
        loops: u32,
        gain_milli: u16,
        pan_milli: i16,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixScript {
    pub sample_rate: u32,
    pub channels: u8,
    pub stream_tag: String,
    pub route: String,
    pub latency_mode: String,
    pub spatialization: String,
    pub completion: String,
    pub ops: Vec<MixOp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedMix {
    pub stream_tag: String,
    pub route: String,
    pub latency_mode: String,
    pub spatialization: String,
    pub completion: String,
    pub op_count: usize,
    pub payload: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MixScriptError {
    MissingField(&'static str),
    InvalidLine(String),
    InvalidValue { key: String, value: String },
}

impl MixScriptError {
    pub fn describe(&self) -> String {
        match self {
            Self::MissingField(field) => format!("missing field {field}"),
            Self::InvalidLine(line) => format!("invalid line {line}"),
            Self::InvalidValue { key, value } => format!("invalid value {key}={value}"),
        }
    }
}

impl MixScript {
    pub fn parse(text: &str) -> Result<Self, MixScriptError> {
        let mut sample_rate = None::<u32>;
        let mut channels = None::<u8>;
        let mut stream_tag = None::<String>;
        let mut route = None::<String>;
        let mut latency_mode = None::<String>;
        let mut spatialization = None::<String>;
        let mut completion = None::<String>;
        let mut ops = Vec::new();

        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(value) = line.strip_prefix("rate=") {
                sample_rate = Some(parse_u32("rate", value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("channels=") {
                channels = Some(parse_u8("channels", value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("stream=") {
                if value.is_empty() {
                    return Err(MixScriptError::InvalidValue {
                        key: String::from("stream"),
                        value: value.to_string(),
                    });
                }
                stream_tag = Some(value.to_string());
                continue;
            }
            if let Some(value) = line.strip_prefix("route=") {
                route = Some(parse_named_value("route", value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("latency-mode=") {
                latency_mode = Some(parse_named_value("latency-mode", value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("spatialization=") {
                spatialization = Some(parse_named_value("spatialization", value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("completion=") {
                completion = Some(parse_named_value("completion", value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("tone=") {
                ops.push(parse_tone(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("clip=") {
                ops.push(parse_clip(value)?);
                continue;
            }
            return Err(MixScriptError::InvalidLine(line.to_string()));
        }

        let script = Self {
            sample_rate: sample_rate.ok_or(MixScriptError::MissingField("rate"))?,
            channels: channels.ok_or(MixScriptError::MissingField("channels"))?,
            stream_tag: stream_tag.ok_or(MixScriptError::MissingField("stream"))?,
            route: route.ok_or(MixScriptError::MissingField("route"))?,
            latency_mode: latency_mode.ok_or(MixScriptError::MissingField("latency-mode"))?,
            spatialization: spatialization.ok_or(MixScriptError::MissingField("spatialization"))?,
            completion: completion.ok_or(MixScriptError::MissingField("completion"))?,
            ops,
        };
        script.validate()?;
        Ok(script)
    }

    pub fn validate(&self) -> Result<(), MixScriptError> {
        if self.sample_rate < 8_000 {
            return Err(MixScriptError::InvalidValue {
                key: String::from("rate"),
                value: self.sample_rate.to_string(),
            });
        }
        if !matches!(self.channels, 1 | 2) {
            return Err(MixScriptError::InvalidValue {
                key: String::from("channels"),
                value: self.channels.to_string(),
            });
        }
        if self.stream_tag.is_empty() {
            return Err(MixScriptError::InvalidValue {
                key: String::from("stream"),
                value: self.stream_tag.clone(),
            });
        }
        if !matches!(
            self.route.as_str(),
            "master" | "music" | "voice" | "effects"
        ) {
            return Err(MixScriptError::InvalidValue {
                key: String::from("route"),
                value: self.route.clone(),
            });
        }
        if !matches!(
            self.latency_mode.as_str(),
            "interactive" | "balanced" | "buffered"
        ) {
            return Err(MixScriptError::InvalidValue {
                key: String::from("latency-mode"),
                value: self.latency_mode.clone(),
            });
        }
        if !matches!(
            self.spatialization.as_str(),
            "stereo" | "headlocked" | "world-3d"
        ) {
            return Err(MixScriptError::InvalidValue {
                key: String::from("spatialization"),
                value: self.spatialization.clone(),
            });
        }
        if !matches!(
            self.completion.as_str(),
            "fire-and-forget" | "wait-batch" | "wait-drain"
        ) {
            return Err(MixScriptError::InvalidValue {
                key: String::from("completion"),
                value: self.completion.clone(),
            });
        }
        if self.ops.is_empty() {
            return Err(MixScriptError::MissingField("mix-op"));
        }
        Ok(())
    }

    pub fn encode(&self, profile: &str) -> EncodedMix {
        let mut lines = vec![
            String::from("ngos-audio-translate/v1"),
            format!("profile={profile}"),
            format!("rate={}", self.sample_rate),
            format!("channels={}", self.channels),
            format!("stream={}", self.stream_tag),
            format!("route={}", self.route),
            format!("latency-mode={}", self.latency_mode),
            format!("spatialization={}", self.spatialization),
            format!("completion={}", self.completion),
        ];
        for op in &self.ops {
            match op {
                MixOp::Tone {
                    bus,
                    hz,
                    millis,
                    gain_milli,
                    pan_milli,
                    waveform,
                } => lines.push(format!(
                    "op=tone bus={} hz={} ms={} gain={} pan={} wave={}",
                    bus,
                    hz,
                    millis,
                    gain_milli,
                    pan_milli,
                    waveform_name(*waveform)
                )),
                MixOp::Clip {
                    bus,
                    clip,
                    loops,
                    gain_milli,
                    pan_milli,
                } => lines.push(format!(
                    "op=clip bus={} clip={} loops={} gain={} pan={}",
                    bus, clip, loops, gain_milli, pan_milli
                )),
            }
        }
        EncodedMix {
            stream_tag: self.stream_tag.clone(),
            route: self.route.clone(),
            latency_mode: self.latency_mode.clone(),
            spatialization: self.spatialization.clone(),
            completion: self.completion.clone(),
            op_count: self.ops.len(),
            payload: lines.join("\n"),
        }
    }
}

/// API-uri audio externe pe care le suportăm ca sursă de traducere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForeignAudioApi {
    DirectSound,
    XAudio2,
    CoreAudio,
    ALSA,
    OpenAL,
    PulseAudio,
    WebAudio,
    /// API necunoscut / neacoperit — refuse
    Other,
}

impl ForeignAudioApi {
    pub fn name(self) -> &'static str {
        match self {
            Self::DirectSound => "directsound",
            Self::XAudio2 => "xaudio2",
            Self::CoreAudio => "coreaudio",
            Self::ALSA => "alsa",
            Self::OpenAL => "openal",
            Self::PulseAudio => "pulseaudio",
            Self::WebAudio => "webaudio",
            Self::Other => "other",
        }
    }

    pub fn translation_label(self) -> &'static str {
        match self {
            Self::ALSA | Self::PulseAudio | Self::WebAudio => "native-mixer",
            Self::Other => "unsupported",
            _ => "compat-to-mixer",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "directsound" => Some(Self::DirectSound),
            "xaudio2" => Some(Self::XAudio2),
            "coreaudio" => Some(Self::CoreAudio),
            "alsa" => Some(Self::ALSA),
            "openal" => Some(Self::OpenAL),
            "pulseaudio" => Some(Self::PulseAudio),
            "webaudio" => Some(Self::WebAudio),
            "other" => Some(Self::Other),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioTranslateError {
    UnsupportedApi(String),
    EmptyScript,
}

impl AudioTranslateError {
    pub fn describe(&self) -> String {
        match self {
            Self::UnsupportedApi(api) => format!("unsupported audio api={api}"),
            Self::EmptyScript => String::from("empty mix script"),
        }
    }
}

pub struct AudioTranslator {
    source_api: ForeignAudioApi,
}

impl AudioTranslator {
    pub fn new(source_api: ForeignAudioApi) -> Self {
        Self { source_api }
    }

    pub fn translate(&self, script: &MixScript) -> Result<EncodedMix, AudioTranslateError> {
        if self.source_api == ForeignAudioApi::Other {
            return Err(AudioTranslateError::UnsupportedApi(String::from(
                self.source_api.name(),
            )));
        }
        if script.ops.is_empty() {
            return Err(AudioTranslateError::EmptyScript);
        }
        let profile = format!("{}-{}", self.source_api.name(), script.latency_mode);
        Ok(script.encode(&profile))
    }
}

fn parse_named_value(key: &str, value: &str) -> Result<String, MixScriptError> {
    if value.is_empty() {
        return Err(MixScriptError::InvalidValue {
            key: key.to_string(),
            value: value.to_string(),
        });
    }
    Ok(value.to_string())
}

fn parse_tone(value: &str) -> Result<MixOp, MixScriptError> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 6 || parts[0].is_empty() {
        return Err(MixScriptError::InvalidLine(format!("tone={value}")));
    }
    Ok(MixOp::Tone {
        bus: parts[0].to_string(),
        hz: parse_u32("tone.hz", parts[1])?,
        millis: parse_u32("tone.ms", parts[2])?,
        gain_milli: parse_gain(parts[3])?,
        pan_milli: parse_pan(parts[4])?,
        waveform: parse_waveform(parts[5])?,
    })
}

fn parse_clip(value: &str) -> Result<MixOp, MixScriptError> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 5 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(MixScriptError::InvalidLine(format!("clip={value}")));
    }
    Ok(MixOp::Clip {
        bus: parts[0].to_string(),
        clip: parts[1].to_string(),
        loops: parse_u32("clip.loops", parts[2])?,
        gain_milli: parse_gain(parts[3])?,
        pan_milli: parse_pan(parts[4])?,
    })
}

fn parse_u32(key: &str, value: &str) -> Result<u32, MixScriptError> {
    value
        .parse::<u32>()
        .map_err(|_| MixScriptError::InvalidValue {
            key: key.to_string(),
            value: value.to_string(),
        })
}

fn parse_u8(key: &str, value: &str) -> Result<u8, MixScriptError> {
    value
        .parse::<u8>()
        .map_err(|_| MixScriptError::InvalidValue {
            key: key.to_string(),
            value: value.to_string(),
        })
}

fn parse_waveform(value: &str) -> Result<Waveform, MixScriptError> {
    match value {
        "sine" => Ok(Waveform::Sine),
        "square" => Ok(Waveform::Square),
        "triangle" => Ok(Waveform::Triangle),
        "noise" => Ok(Waveform::Noise),
        _ => Err(MixScriptError::InvalidValue {
            key: String::from("tone.wave"),
            value: value.to_string(),
        }),
    }
}

fn parse_gain(value: &str) -> Result<u16, MixScriptError> {
    parse_milli("gain", value, 0, 1000).map(|value| value as u16)
}

fn parse_pan(value: &str) -> Result<i16, MixScriptError> {
    parse_signed_milli("pan", value, -1000, 1000).map(|value| value as i16)
}

fn parse_milli(key: &str, value: &str, min: i32, max: i32) -> Result<i32, MixScriptError> {
    let parsed = parse_decimal_milli(key, value)?;
    if parsed < min || parsed > max {
        return Err(MixScriptError::InvalidValue {
            key: key.to_string(),
            value: value.to_string(),
        });
    }
    Ok(parsed)
}

fn parse_signed_milli(key: &str, value: &str, min: i32, max: i32) -> Result<i32, MixScriptError> {
    let parsed = parse_decimal_milli(key, value)?;
    if parsed < min || parsed > max {
        return Err(MixScriptError::InvalidValue {
            key: key.to_string(),
            value: value.to_string(),
        });
    }
    Ok(parsed)
}

fn parse_decimal_milli(key: &str, value: &str) -> Result<i32, MixScriptError> {
    let negative = value.starts_with('-');
    let digits = if negative { &value[1..] } else { value };
    let (whole_text, frac_text) = match digits.split_once('.') {
        Some(parts) => parts,
        None => (digits, ""),
    };
    if whole_text.is_empty() || frac_text.len() > 3 {
        return Err(MixScriptError::InvalidValue {
            key: key.to_string(),
            value: value.to_string(),
        });
    }
    let whole = whole_text
        .parse::<i32>()
        .map_err(|_| MixScriptError::InvalidValue {
            key: key.to_string(),
            value: value.to_string(),
        })?;
    let frac = if frac_text.is_empty() {
        0
    } else {
        let frac_digits = frac_text
            .parse::<i32>()
            .map_err(|_| MixScriptError::InvalidValue {
                key: key.to_string(),
                value: value.to_string(),
            })?;
        let scale = match frac_text.len() {
            1 => 100,
            2 => 10,
            _ => 1,
        };
        frac_digits * scale
    };
    let total = whole
        .checked_mul(1000)
        .and_then(|base| base.checked_add(frac))
        .ok_or_else(|| MixScriptError::InvalidValue {
            key: key.to_string(),
            value: value.to_string(),
        })?;
    Ok(if negative { -total } else { total })
}

fn waveform_name(waveform: Waveform) -> &'static str {
    match waveform {
        Waveform::Sine => "sine",
        Waveform::Square => "square",
        Waveform::Triangle => "triangle",
        Waveform::Noise => "noise",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_encodes_mix_script() {
        let script = MixScript::parse(
            "rate=48000\nchannels=2\nstream=orbit-intro\nroute=music\nlatency-mode=interactive\nspatialization=world-3d\ncompletion=wait-drain\ntone=lead,440,120,0.800,-0.250,sine\nclip=ambience,hangar-loop,2,0.650,0.100\n",
        )
        .unwrap();
        let encoded = script.encode("spatial-mix");
        assert_eq!(encoded.stream_tag, "orbit-intro");
        assert_eq!(encoded.route, "music");
        assert_eq!(encoded.latency_mode, "interactive");
        assert_eq!(encoded.spatialization, "world-3d");
        assert_eq!(encoded.completion, "wait-drain");
        assert_eq!(encoded.op_count, 2);
        assert!(encoded.payload.contains("profile=spatial-mix"));
        assert!(encoded.payload.contains("route=music"));
        assert!(encoded.payload.contains("latency-mode=interactive"));
        assert!(encoded.payload.contains("spatialization=world-3d"));
        assert!(encoded.payload.contains("completion=wait-drain"));
        assert!(
            encoded
                .payload
                .contains("op=tone bus=lead hz=440 ms=120 gain=800 pan=-250 wave=sine")
        );
        assert!(
            encoded
                .payload
                .contains("op=clip bus=ambience clip=hangar-loop loops=2 gain=650 pan=100")
        );
    }

    #[test]
    fn audio_translator_encodes_xaudio2_to_native_mixer() {
        use super::{AudioTranslator, ForeignAudioApi};
        let script = MixScript::parse(
            "rate=48000\nchannels=2\nstream=test-stream\nroute=music\nlatency-mode=interactive\nspatialization=stereo\ncompletion=fire-and-forget\ntone=main,440,100,0.800,0.000,sine\n",
        )
        .unwrap();
        let translator = AudioTranslator::new(ForeignAudioApi::XAudio2);
        let encoded = translator.translate(&script).unwrap();
        assert_eq!(encoded.stream_tag, "test-stream");
        assert!(encoded.payload.contains("ngos-audio-translate/v1"));
        assert!(encoded.payload.contains("profile=xaudio2-interactive"));
    }

    #[test]
    fn audio_translator_refuses_other_api() {
        use super::{AudioTranslateError, AudioTranslator, ForeignAudioApi};
        let script = MixScript::parse(
            "rate=48000\nchannels=2\nstream=s\nroute=music\nlatency-mode=interactive\nspatialization=stereo\ncompletion=fire-and-forget\ntone=b,440,100,0.800,0.000,sine\n",
        )
        .unwrap();
        let translator = AudioTranslator::new(ForeignAudioApi::Other);
        assert!(matches!(
            translator.translate(&script),
            Err(AudioTranslateError::UnsupportedApi(_))
        ));
    }

    #[test]
    fn rejects_missing_ops_and_invalid_ranges() {
        let error = MixScript::parse(
            "rate=4000\nchannels=3\nstream=\nroute=unknown\nlatency-mode=interactive\nspatialization=stereo\ncompletion=fire-and-forget\n",
        )
        .unwrap_err();
        assert!(!error.describe().is_empty());
    }
}
