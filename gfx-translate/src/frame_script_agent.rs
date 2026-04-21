use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use crate::{
    FrameScriptError,
    frame_profile_agent::FrameProfile,
    render_command_agent::{
        DrawOp, parse_backdrop, parse_begin_pass, parse_blit, parse_color, parse_ellipse,
        parse_flip_region, parse_gaussian_blur, parse_gradient_rect, parse_line, parse_push_layer,
        parse_rect, parse_rounded_rect, parse_set_blend_mode, parse_set_clip,
        parse_set_present_region, parse_shadow_rect, parse_sprite, parse_triangle, parse_u32,
    },
};

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
    pub profile: FrameProfile,
    pub source_api: Option<String>,
    pub translation_label: Option<String>,
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
            // geometry ops
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
            if let Some(value) = line.strip_prefix("sprite=") {
                ops.push(parse_sprite(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("triangle=") {
                ops.push(parse_triangle(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("ellipse=") {
                ops.push(parse_ellipse(value)?);
                continue;
            }
            // effect ops
            if let Some(value) = line.strip_prefix("shadow-rect=") {
                ops.push(parse_shadow_rect(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("gaussian-blur=") {
                ops.push(parse_gaussian_blur(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("backdrop=") {
                ops.push(parse_backdrop(value)?);
                continue;
            }
            // composition ops
            if let Some(value) = line.strip_prefix("blit=") {
                ops.push(parse_blit(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("begin-pass=") {
                ops.push(parse_begin_pass(value)?);
                continue;
            }
            if line == "end-pass" {
                ops.push(DrawOp::EndPass);
                continue;
            }
            if let Some(value) = line.strip_prefix("set-blend-mode=") {
                ops.push(parse_set_blend_mode(value)?);
                continue;
            }
            if line == "clear-blend-mode" {
                ops.push(DrawOp::ClearBlendMode);
                continue;
            }
            if let Some(value) = line.strip_prefix("push-layer=") {
                ops.push(parse_push_layer(value)?);
                continue;
            }
            if line == "pop-layer" {
                ops.push(DrawOp::PopLayer);
                continue;
            }
            if let Some(value) = line.strip_prefix("set-clip=") {
                ops.push(parse_set_clip(value)?);
                continue;
            }
            if line == "clear-clip" {
                ops.push(DrawOp::ClearClip);
                continue;
            }
            // presentation ops
            if let Some(value) = line.strip_prefix("set-present-region=") {
                ops.push(parse_set_present_region(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("flip-region=") {
                ops.push(parse_flip_region(value)?);
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
        for op in &self.ops {
            op.validate_args()?;
        }
        Ok(())
    }

    pub fn profile(&self) -> FrameProfile {
        FrameProfile::from_ops(&self.ops)
    }

    pub fn encode(&self, profile_label: &str) -> EncodedFrame {
        self.encode_with_metadata(profile_label, None, None)
    }

    pub fn encode_translated(
        &self,
        profile_label: &str,
        source_api_label: &str,
        translation_label: &str,
    ) -> EncodedFrame {
        self.encode_with_metadata(
            profile_label,
            Some(source_api_label),
            Some(translation_label),
        )
    }

    fn encode_with_metadata(
        &self,
        profile_label: &str,
        source_api_label: Option<&str>,
        translation_label: Option<&str>,
    ) -> EncodedFrame {
        let profile = self.profile();
        let mut lines = vec![
            String::from("ngos-gfx-translate/v1"),
            format!("profile={profile_label}"),
            format!("profile-geometry={}", profile.geometry_ops),
            format!("profile-composition={}", profile.composition_ops),
            format!("profile-effect={}", profile.effect_ops),
            format!("profile-presentation={}", profile.presentation_ops),
            format!("profile-total={}", profile.total_ops),
            format!("profile-passes={}", profile.pass_ops),
            format!("profile-blend={}", profile.blend_ops),
            format!("surface={}x{}", self.width, self.height),
            format!("frame={}", self.frame_tag),
            format!("queue={}", self.queue),
            format!("present-mode={}", self.present_mode),
            format!("completion={}", self.completion),
        ];
        if let Some(source_api_label) = source_api_label {
            lines.push(format!("source-api={source_api_label}"));
        }
        if let Some(translation_label) = translation_label {
            lines.push(format!("translation={translation_label}"));
        }
        for op in &self.ops {
            lines.push(op.encode_line());
        }
        EncodedFrame {
            frame_tag: self.frame_tag.clone(),
            queue: self.queue.clone(),
            present_mode: self.present_mode.clone(),
            completion: self.completion.clone(),
            op_count: self.ops.len(),
            payload: lines.join("\n"),
            profile,
            source_api: source_api_label.map(ToString::to_string),
            translation_label: translation_label.map(ToString::to_string),
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
        assert_eq!(encoded.source_api, None);
        assert_eq!(encoded.translation_label, None);
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
        let error = FrameScript::parse(
            "surface=64x64\nframe=x\nqueue=graphics\npresent-mode=fifo\ncompletion=wait-present\nrounded-rect=1,2,3,#ff00ff\n",
        )
        .unwrap_err();
        assert!(!error.describe().is_empty());

        let error = FrameScript::parse(
            "surface=64x64\nframe=x\nqueue=graphics\npresent-mode=fifo\ncompletion=wait-present\ngradient-rect=0,0,64,64,#111111ff,#222222ff,#333333ff\n",
        )
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

    #[test]
    fn parses_new_geometry_ops() {
        let script = FrameScript::parse(
            "surface=640x480\nframe=t1\nqueue=graphics\npresent-mode=fifo\ncompletion=wait-present\ntriangle=0,0,100,0,50,100,ff0000ff\nellipse=10,10,80,60,00ff00ff\n",
        )
        .unwrap();
        assert_eq!(script.ops.len(), 2);
    }

    #[test]
    fn parses_composition_ops() {
        let script = FrameScript::parse(
            "surface=640x480\nframe=t2\nqueue=graphics\npresent-mode=fifo\ncompletion=wait-present\npush-layer=200\nrect=0,0,100,100,ffffffff\npop-layer\n",
        )
        .unwrap();
        assert_eq!(script.ops.len(), 3);
    }

    #[test]
    fn parses_effect_ops() {
        let script = FrameScript::parse(
            "surface=640x480\nframe=t3\nqueue=graphics\npresent-mode=fifo\ncompletion=wait-present\ngaussian-blur=0,0,200,100,8\nbackdrop=0,0,200,100,180\n",
        )
        .unwrap();
        assert_eq!(script.ops.len(), 2);
    }

    #[test]
    fn parses_presentation_ops() {
        let script = FrameScript::parse(
            "surface=640x480\nframe=t4\nqueue=graphics\npresent-mode=fifo\ncompletion=wait-present\nrect=0,0,640,480,000000ff\nset-present-region=0,0,640,480\nflip-region=0,0,640,480\n",
        )
        .unwrap();
        assert_eq!(script.ops.len(), 3);
    }

    #[test]
    fn profile_in_encoded_frame() {
        let script = FrameScript::parse(
            "surface=640x480\nframe=t5\nqueue=graphics\npresent-mode=fifo\ncompletion=wait-present\nrect=0,0,640,480,000000ff\ngaussian-blur=0,0,100,100,4\npush-layer=255\npop-layer\nset-present-region=0,0,640,480\n",
        )
        .unwrap();
        let encoded = script.encode("test");
        assert_eq!(encoded.profile.geometry_ops, 1);
        assert_eq!(encoded.profile.effect_ops, 1);
        assert_eq!(encoded.profile.composition_ops, 2);
        assert_eq!(encoded.profile.presentation_ops, 1);
        assert_eq!(encoded.profile.total_ops, 5);
        assert!(encoded.payload.contains("profile-geometry=1"));
        assert!(encoded.payload.contains("profile-effect=1"));
        assert!(encoded.payload.contains("profile-composition=2"));
        assert!(encoded.payload.contains("profile-presentation=1"));
        assert!(encoded.payload.contains("profile-total=5"));
    }

    #[test]
    fn encode_translated_adds_source_api_and_translation_metadata() {
        let script = FrameScript::parse(
            "surface=640x480\nframe=t8\nqueue=graphics\npresent-mode=fifo\ncompletion=fire-and-forget\nrect=0,0,640,480,000000ff\nflip-region=0,0,640,480\n",
        )
        .unwrap();
        let encoded = script.encode_translated("compat-to-vulkan", "directx12", "compat-to-vulkan");
        assert_eq!(encoded.source_api.as_deref(), Some("directx12"));
        assert_eq!(
            encoded.translation_label.as_deref(),
            Some("compat-to-vulkan")
        );
        assert!(encoded.payload.contains("source-api=directx12"));
        assert!(encoded.payload.contains("translation=compat-to-vulkan"));
    }

    #[test]
    fn rejects_gaussian_blur_zero_radius_in_frame() {
        let err = FrameScript::parse(
            "surface=640x480\nframe=t6\nqueue=graphics\npresent-mode=fifo\ncompletion=wait-present\ngaussian-blur=0,0,100,100,0\n",
        )
        .unwrap_err();
        assert!(err.describe().contains("gaussian-blur"));
    }

    #[test]
    fn parses_clip_and_clear_clip() {
        let script = FrameScript::parse(
            "surface=640x480\nframe=t7\nqueue=graphics\npresent-mode=fifo\ncompletion=wait-present\nset-clip=10,10,200,200\nrect=10,10,200,200,ff0000ff\nclear-clip\n",
        )
        .unwrap();
        assert_eq!(script.ops.len(), 3);
        assert!(matches!(script.ops[0], DrawOp::SetClip { .. }));
        assert!(matches!(script.ops[2], DrawOp::ClearClip));
    }
}
