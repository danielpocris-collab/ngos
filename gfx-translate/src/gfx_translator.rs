use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use crate::{
    frame_script_agent::FrameScript,
    render_command_agent::{DrawOp, FontFamily, RgbaColor, parse_color, parse_u8, parse_u32},
};

/// Source graphics API being translated from.
/// Excludes `Other` — unsupported APIs are refused before reaching the translator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceApi {
    Direct3D9,
    Direct3D10,
    DirectX11,
    DirectX12,
    OpenGL,
    OpenGLES,
    Metal,
    Vulkan,
    WebGPU,
    Wgpu,
}

impl SourceApi {
    pub fn name(self) -> &'static str {
        match self {
            Self::Direct3D9 => "direct3d9",
            Self::Direct3D10 => "direct3d10",
            Self::DirectX11 => "directx11",
            Self::DirectX12 => "directx12",
            Self::OpenGL => "opengl",
            Self::OpenGLES => "opengles",
            Self::Metal => "metal",
            Self::Vulkan => "vulkan",
            Self::WebGPU => "webgpu",
            Self::Wgpu => "wgpu",
        }
    }

    /// Translation label used in observability output.
    pub fn translation_label(self) -> &'static str {
        match self {
            Self::Vulkan => "native-vulkan",
            _ => "compat-to-vulkan",
        }
    }
}

/// Normalized draw command from a foreign graphics API.
///
/// Each variant maps a canonical draw intent to the nearest NGOS DrawOp equivalent:
/// - DX ClearRenderTargetView / GL glClearColor / Metal clearColor → `Clear`
/// - DX DrawIndexed quad / GL glDrawArrays quad / Metal drawPrimitives → `FillRect`
/// - DX CopyResource / GL glBlitFramebuffer / Metal blit encoder → `Blit`
/// - DX Present / GL SwapBuffers / Metal presentDrawable → `Present`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForeignDrawCmd {
    Clear {
        color: RgbaColor,
    },
    FillRect {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        color: RgbaColor,
    },
    FillRoundedRect {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        radius: u32,
        color: RgbaColor,
    },
    DrawLine {
        x0: u32,
        y0: u32,
        x1: u32,
        y1: u32,
        color: RgbaColor,
    },
    DrawTriangle {
        x0: u32,
        y0: u32,
        x1: u32,
        y1: u32,
        x2: u32,
        y2: u32,
        color: RgbaColor,
    },
    DrawEllipse {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        color: RgbaColor,
    },
    DrawSprite {
        id: String,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
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
    ShadowRect {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        blur: u32,
        color: RgbaColor,
    },
    Backdrop {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        opacity: u8,
    },
    SetClip {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    ClearClip,
    Text {
        text: String,
        x: u32,
        y: u32,
        size: u32,
        color: RgbaColor,
        font: FontFamily,
    },
    Icon {
        icon: char,
        x: u32,
        y: u32,
        size: u32,
        color: RgbaColor,
    },
    Blit {
        source: String,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    PushLayer {
        opacity: u8,
    },
    PopLayer,
    GaussianBlur {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        radius: u32,
    },
    SetPresentRegion {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    /// Maps to FlipRegion — the final present/scanout call.
    Present {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
}

/// Errors from the gfx translation path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GfxTranslateError {
    /// Foreign frame file could not be parsed.
    ParseError(String),
    /// No draw operations remain after translation.
    EmptyFrame,
    /// A translated DrawOp failed validation.
    InvalidDrawOp(String),
    /// The resulting FrameScript is invalid.
    InvalidFrame(String),
}

impl GfxTranslateError {
    pub fn describe(&self) -> String {
        match self {
            Self::ParseError(msg) => format!("parse error: {msg}"),
            Self::EmptyFrame => String::from("translated frame has no draw ops"),
            Self::InvalidDrawOp(msg) => format!("invalid draw op: {msg}"),
            Self::InvalidFrame(msg) => format!("invalid frame: {msg}"),
        }
    }
}

/// A parsed foreign frame, ready for translation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignFrameScript {
    pub surface_width: u32,
    pub surface_height: u32,
    pub frame_tag: String,
    pub queue: String,
    pub present_mode: String,
    pub completion: String,
    pub cmds: Vec<ForeignDrawCmd>,
}

impl ForeignFrameScript {
    /// Parse a foreign frame from a text description.
    ///
    /// Format mirrors the NGOS FrameScript but uses foreign API semantics:
    /// ```text
    /// surface=1280x720
    /// frame=f001
    /// queue=graphics
    /// present-mode=mailbox
    /// completion=wait-present
    /// clear=ff0000ff
    /// fill-rect=0,0,1280,720,000000ff
    /// draw-sprite=ship,400,220,96,96
    /// present=0,0,1280,720
    /// ```
    pub fn parse(text: &str) -> Result<Self, GfxTranslateError> {
        Self::parse_for_api(None, text)
    }

    /// Parse a foreign frame using aliases specific to the source graphics API.
    ///
    /// This lets compat callers feed source-native command spellings such as:
    /// - `dx-clear-rtv`, `dx-present`
    /// - `gl-clear`, `gl-swap-buffers`
    /// - `metal-clear`, `metal-present-drawable`
    /// - `vk-cmd-clear-color`, `vk-queue-present`
    /// - `webgpu-clear-pass`, `webgpu-present`
    pub fn parse_for_api(
        source_api: Option<SourceApi>,
        text: &str,
    ) -> Result<Self, GfxTranslateError> {
        let mut surface_width = None::<u32>;
        let mut surface_height = None::<u32>;
        let mut frame_tag = None::<String>;
        let mut queue = None::<String>;
        let mut present_mode = None::<String>;
        let mut completion = None::<String>;
        let mut cmds = Vec::new();

        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(value) = line.strip_prefix("surface=") {
                let (w, h) = value
                    .split_once('x')
                    .ok_or_else(|| GfxTranslateError::ParseError(format!("surface={value}")))?;
                surface_width = Some(map_parse_u32(w, "surface.width")?);
                surface_height = Some(map_parse_u32(h, "surface.height")?);
                continue;
            }
            if let Some(value) = line.strip_prefix("frame=") {
                if value.is_empty() {
                    return Err(GfxTranslateError::ParseError(String::from(
                        "empty frame tag",
                    )));
                }
                frame_tag = Some(value.to_string());
                continue;
            }
            if let Some(value) = line.strip_prefix("queue=") {
                if value.is_empty() {
                    return Err(GfxTranslateError::ParseError(String::from("empty queue")));
                }
                queue = Some(value.to_string());
                continue;
            }
            if let Some(value) = line.strip_prefix("present-mode=") {
                if value.is_empty() {
                    return Err(GfxTranslateError::ParseError(String::from(
                        "empty present-mode",
                    )));
                }
                present_mode = Some(value.to_string());
                continue;
            }
            if let Some(value) = line.strip_prefix("completion=") {
                if value.is_empty() {
                    return Err(GfxTranslateError::ParseError(String::from(
                        "empty completion",
                    )));
                }
                completion = Some(value.to_string());
                continue;
            }
            if let Some(value) = line.strip_prefix("clear=") {
                cmds.push(ForeignDrawCmd::Clear {
                    color: map_parse_color(value)?,
                });
                continue;
            }
            if let Some(value) = line.strip_prefix("fill-rect=") {
                cmds.push(parse_fill_rect(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("fill-rounded-rect=") {
                cmds.push(parse_fill_rounded_rect(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("gradient-rect=") {
                cmds.push(parse_gradient_rect_cmd(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("draw-line=") {
                cmds.push(parse_draw_line(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("draw-triangle=") {
                cmds.push(parse_draw_triangle(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("draw-ellipse=") {
                cmds.push(parse_draw_ellipse(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("draw-sprite=") {
                cmds.push(parse_draw_sprite(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("shadow-rect=") {
                cmds.push(parse_shadow_rect_cmd(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("backdrop=") {
                cmds.push(parse_backdrop_cmd(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("set-clip=") {
                cmds.push(parse_set_clip_cmd(value)?);
                continue;
            }
            if line == "clear-clip" {
                cmds.push(ForeignDrawCmd::ClearClip);
                continue;
            }
            if let Some(value) = line.strip_prefix("text=") {
                cmds.push(parse_text_cmd(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("icon=") {
                cmds.push(parse_icon_cmd(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("blit=") {
                cmds.push(parse_blit_cmd(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("push-layer=") {
                cmds.push(ForeignDrawCmd::PushLayer {
                    opacity: map_parse_u8(value, "push-layer.opacity")?,
                });
                continue;
            }
            if line == "pop-layer" {
                cmds.push(ForeignDrawCmd::PopLayer);
                continue;
            }
            if let Some(value) = line.strip_prefix("gaussian-blur=") {
                cmds.push(parse_gaussian_blur_cmd(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("set-present-region=") {
                cmds.push(parse_set_present_region_cmd(value)?);
                continue;
            }
            if let Some(value) = line.strip_prefix("present=") {
                cmds.push(parse_present_cmd(value)?);
                continue;
            }
            if let Some(cmd) = parse_api_specific_cmd(source_api, line)? {
                cmds.push(cmd);
                continue;
            }
            return Err(GfxTranslateError::ParseError(format!(
                "unknown line: {line}"
            )));
        }

        Ok(Self {
            surface_width: surface_width
                .ok_or_else(|| GfxTranslateError::ParseError(String::from("missing surface")))?,
            surface_height: surface_height
                .ok_or_else(|| GfxTranslateError::ParseError(String::from("missing surface")))?,
            frame_tag: frame_tag
                .ok_or_else(|| GfxTranslateError::ParseError(String::from("missing frame")))?,
            queue: queue
                .ok_or_else(|| GfxTranslateError::ParseError(String::from("missing queue")))?,
            present_mode: present_mode.ok_or_else(|| {
                GfxTranslateError::ParseError(String::from("missing present-mode"))
            })?,
            completion: completion
                .ok_or_else(|| GfxTranslateError::ParseError(String::from("missing completion")))?,
            cmds,
        })
    }
}

fn parse_api_specific_cmd(
    source_api: Option<SourceApi>,
    line: &str,
) -> Result<Option<ForeignDrawCmd>, GfxTranslateError> {
    let Some(source_api) = source_api else {
        return Ok(None);
    };
    let parsed = match source_api {
        SourceApi::Direct3D9
        | SourceApi::Direct3D10
        | SourceApi::DirectX11
        | SourceApi::DirectX12 => parse_prefixed_cmd(
            line,
            &[
                ("dx-clear-rtv=", parse_clear_alias),
                ("dx-fill-rect=", parse_fill_rect),
                ("dx-fill-rounded-rect=", parse_fill_rounded_rect),
                ("dx-gradient-rect=", parse_gradient_rect_cmd),
                ("dx-draw-line=", parse_draw_line),
                ("dx-draw-triangle=", parse_draw_triangle),
                ("dx-draw-ellipse=", parse_draw_ellipse),
                ("dx-draw-sprite=", parse_draw_sprite),
                ("dx-draw-text=", parse_text_cmd),
                ("dx-draw-icon=", parse_icon_cmd),
                ("dx-copy-resource=", parse_blit_cmd),
                ("dx-shadow-rect=", parse_shadow_rect_cmd),
                ("dx-backdrop=", parse_backdrop_cmd),
                ("dx-push-layer=", parse_push_layer_alias),
                ("dx-pop-layer", parse_pop_layer_alias),
                ("dx-set-clip=", parse_set_clip_cmd),
                ("dx-clear-clip", parse_clear_clip_alias),
                ("dx-gaussian-blur=", parse_gaussian_blur_cmd),
                ("dx-set-present-region=", parse_set_present_region_cmd),
                ("dx-present=", parse_present_cmd),
            ],
        )?,
        SourceApi::OpenGL => parse_prefixed_cmd(
            line,
            &[
                ("gl-clear=", parse_clear_alias),
                ("gl-fill-rect=", parse_fill_rect),
                ("gl-fill-rounded-rect=", parse_fill_rounded_rect),
                ("gl-gradient-rect=", parse_gradient_rect_cmd),
                ("gl-draw-line=", parse_draw_line),
                ("gl-draw-triangle=", parse_draw_triangle),
                ("gl-draw-ellipse=", parse_draw_ellipse),
                ("gl-draw-sprite=", parse_draw_sprite),
                ("gl-draw-text=", parse_text_cmd),
                ("gl-draw-icon=", parse_icon_cmd),
                ("gl-blit=", parse_blit_cmd),
                ("gl-shadow-rect=", parse_shadow_rect_cmd),
                ("gl-backdrop=", parse_backdrop_cmd),
                ("gl-push-layer=", parse_push_layer_alias),
                ("gl-pop-layer", parse_pop_layer_alias),
                ("gl-set-clip=", parse_set_clip_cmd),
                ("gl-clear-clip", parse_clear_clip_alias),
                ("gl-gaussian-blur=", parse_gaussian_blur_cmd),
                ("gl-set-present-region=", parse_set_present_region_cmd),
                ("gl-swap-buffers=", parse_present_cmd),
            ],
        )?,
        SourceApi::OpenGLES => parse_prefixed_cmd(
            line,
            &[
                ("gles-clear=", parse_clear_alias),
                ("gles-fill-rect=", parse_fill_rect),
                ("gles-fill-rounded-rect=", parse_fill_rounded_rect),
                ("gles-gradient-rect=", parse_gradient_rect_cmd),
                ("gles-draw-line=", parse_draw_line),
                ("gles-draw-triangle=", parse_draw_triangle),
                ("gles-draw-ellipse=", parse_draw_ellipse),
                ("gles-draw-sprite=", parse_draw_sprite),
                ("gles-draw-text=", parse_text_cmd),
                ("gles-draw-icon=", parse_icon_cmd),
                ("gles-blit=", parse_blit_cmd),
                ("gles-shadow-rect=", parse_shadow_rect_cmd),
                ("gles-backdrop=", parse_backdrop_cmd),
                ("gles-push-layer=", parse_push_layer_alias),
                ("gles-pop-layer", parse_pop_layer_alias),
                ("gles-set-clip=", parse_set_clip_cmd),
                ("gles-clear-clip", parse_clear_clip_alias),
                ("gles-gaussian-blur=", parse_gaussian_blur_cmd),
                ("gles-set-present-region=", parse_set_present_region_cmd),
                ("gles-swap-buffers=", parse_present_cmd),
            ],
        )?,
        SourceApi::Metal => parse_prefixed_cmd(
            line,
            &[
                ("metal-clear=", parse_clear_alias),
                ("metal-fill-rect=", parse_fill_rect),
                ("metal-fill-rounded-rect=", parse_fill_rounded_rect),
                ("metal-gradient-rect=", parse_gradient_rect_cmd),
                ("metal-draw-line=", parse_draw_line),
                ("metal-draw-triangle=", parse_draw_triangle),
                ("metal-draw-ellipse=", parse_draw_ellipse),
                ("metal-draw-sprite=", parse_draw_sprite),
                ("metal-draw-text=", parse_text_cmd),
                ("metal-draw-icon=", parse_icon_cmd),
                ("metal-blit-texture=", parse_blit_cmd),
                ("metal-shadow-rect=", parse_shadow_rect_cmd),
                ("metal-backdrop=", parse_backdrop_cmd),
                ("metal-push-layer=", parse_push_layer_alias),
                ("metal-pop-layer", parse_pop_layer_alias),
                ("metal-set-clip=", parse_set_clip_cmd),
                ("metal-clear-clip", parse_clear_clip_alias),
                ("metal-gaussian-blur=", parse_gaussian_blur_cmd),
                ("metal-set-present-region=", parse_set_present_region_cmd),
                ("metal-present-drawable=", parse_present_cmd),
            ],
        )?,
        SourceApi::Vulkan => parse_prefixed_cmd(
            line,
            &[
                ("vk-cmd-clear-color=", parse_clear_alias),
                ("vk-cmd-fill-rect=", parse_fill_rect),
                ("vk-cmd-fill-rounded-rect=", parse_fill_rounded_rect),
                ("vk-cmd-gradient-rect=", parse_gradient_rect_cmd),
                ("vk-cmd-draw-line=", parse_draw_line),
                ("vk-cmd-draw-triangle=", parse_draw_triangle),
                ("vk-cmd-draw-ellipse=", parse_draw_ellipse),
                ("vk-cmd-draw-sprite=", parse_draw_sprite),
                ("vk-cmd-draw-text=", parse_text_cmd),
                ("vk-cmd-draw-icon=", parse_icon_cmd),
                ("vk-cmd-blit-image=", parse_blit_cmd),
                ("vk-cmd-shadow-rect=", parse_shadow_rect_cmd),
                ("vk-cmd-backdrop=", parse_backdrop_cmd),
                ("vk-cmd-push-layer=", parse_push_layer_alias),
                ("vk-cmd-pop-layer", parse_pop_layer_alias),
                ("vk-cmd-set-clip=", parse_set_clip_cmd),
                ("vk-cmd-clear-clip", parse_clear_clip_alias),
                ("vk-cmd-gaussian-blur=", parse_gaussian_blur_cmd),
                ("vk-cmd-set-present-region=", parse_set_present_region_cmd),
                ("vk-queue-present=", parse_present_cmd),
            ],
        )?,
        SourceApi::WebGPU => parse_prefixed_cmd(
            line,
            &[
                ("webgpu-clear-pass=", parse_clear_alias),
                ("webgpu-fill-rect=", parse_fill_rect),
                ("webgpu-fill-rounded-rect=", parse_fill_rounded_rect),
                ("webgpu-gradient-rect=", parse_gradient_rect_cmd),
                ("webgpu-draw-line=", parse_draw_line),
                ("webgpu-draw-triangle=", parse_draw_triangle),
                ("webgpu-draw-ellipse=", parse_draw_ellipse),
                ("webgpu-draw-sprite=", parse_draw_sprite),
                ("webgpu-draw-text=", parse_text_cmd),
                ("webgpu-draw-icon=", parse_icon_cmd),
                ("webgpu-copy-texture=", parse_blit_cmd),
                ("webgpu-shadow-rect=", parse_shadow_rect_cmd),
                ("webgpu-backdrop=", parse_backdrop_cmd),
                ("webgpu-push-layer=", parse_push_layer_alias),
                ("webgpu-pop-layer", parse_pop_layer_alias),
                ("webgpu-set-clip=", parse_set_clip_cmd),
                ("webgpu-clear-clip", parse_clear_clip_alias),
                ("webgpu-gaussian-blur=", parse_gaussian_blur_cmd),
                ("webgpu-set-present-region=", parse_set_present_region_cmd),
                ("webgpu-present=", parse_present_cmd),
            ],
        )?,
        SourceApi::Wgpu => parse_prefixed_cmd(
            line,
            &[
                ("wgpu-clear-pass=", parse_clear_alias),
                ("wgpu-fill-rect=", parse_fill_rect),
                ("wgpu-fill-rounded-rect=", parse_fill_rounded_rect),
                ("wgpu-gradient-rect=", parse_gradient_rect_cmd),
                ("wgpu-draw-line=", parse_draw_line),
                ("wgpu-draw-triangle=", parse_draw_triangle),
                ("wgpu-draw-ellipse=", parse_draw_ellipse),
                ("wgpu-draw-sprite=", parse_draw_sprite),
                ("wgpu-draw-text=", parse_text_cmd),
                ("wgpu-draw-icon=", parse_icon_cmd),
                ("wgpu-copy-texture=", parse_blit_cmd),
                ("wgpu-shadow-rect=", parse_shadow_rect_cmd),
                ("wgpu-backdrop=", parse_backdrop_cmd),
                ("wgpu-push-layer=", parse_push_layer_alias),
                ("wgpu-pop-layer", parse_pop_layer_alias),
                ("wgpu-set-clip=", parse_set_clip_cmd),
                ("wgpu-clear-clip", parse_clear_clip_alias),
                ("wgpu-gaussian-blur=", parse_gaussian_blur_cmd),
                ("wgpu-set-present-region=", parse_set_present_region_cmd),
                ("wgpu-present=", parse_present_cmd),
            ],
        )?,
    };
    Ok(parsed)
}

type AliasParser = fn(&str) -> Result<ForeignDrawCmd, GfxTranslateError>;

fn parse_prefixed_cmd(
    line: &str,
    aliases: &[(&str, AliasParser)],
) -> Result<Option<ForeignDrawCmd>, GfxTranslateError> {
    for (prefix, parser) in aliases {
        if let Some(value) = line.strip_prefix(prefix) {
            return parser(value).map(Some);
        }
        if line == *prefix {
            return parser("").map(Some);
        }
    }
    Ok(None)
}

fn parse_clear_alias(value: &str) -> Result<ForeignDrawCmd, GfxTranslateError> {
    Ok(ForeignDrawCmd::Clear {
        color: map_parse_color(value)?,
    })
}

fn parse_push_layer_alias(value: &str) -> Result<ForeignDrawCmd, GfxTranslateError> {
    Ok(ForeignDrawCmd::PushLayer {
        opacity: map_parse_u8(value, "push-layer.opacity")?,
    })
}

fn parse_pop_layer_alias(value: &str) -> Result<ForeignDrawCmd, GfxTranslateError> {
    if !value.is_empty() {
        return Err(GfxTranslateError::ParseError(format!("pop-layer={value}")));
    }
    Ok(ForeignDrawCmd::PopLayer)
}

/// Translates foreign API draw commands to NGOS FrameScript/DrawOp semantics.
pub struct GfxTranslator {
    pub source_api: SourceApi,
}

impl GfxTranslator {
    pub fn new(source_api: SourceApi) -> Self {
        Self { source_api }
    }

    /// Translate a `ForeignFrameScript` to a validated NGOS `FrameScript`.
    pub fn translate(
        &self,
        foreign: &ForeignFrameScript,
    ) -> Result<FrameScript, GfxTranslateError> {
        let mut ops = Vec::new();
        for cmd in &foreign.cmds {
            if let Some(op) = self.translate_cmd(cmd) {
                ops.push(op);
            }
        }
        if ops.is_empty() {
            return Err(GfxTranslateError::EmptyFrame);
        }
        for op in &ops {
            op.validate_args()
                .map_err(|e| GfxTranslateError::InvalidDrawOp(e.describe()))?;
        }
        let script = FrameScript {
            width: foreign.surface_width,
            height: foreign.surface_height,
            frame_tag: foreign.frame_tag.clone(),
            queue: foreign.queue.clone(),
            present_mode: foreign.present_mode.clone(),
            completion: foreign.completion.clone(),
            ops,
        };
        script
            .validate()
            .map_err(|e| GfxTranslateError::InvalidFrame(e.describe()))?;
        Ok(script)
    }

    fn translate_cmd(&self, cmd: &ForeignDrawCmd) -> Option<DrawOp> {
        Some(match cmd {
            ForeignDrawCmd::Clear { color } => DrawOp::Clear { color: *color },
            ForeignDrawCmd::FillRect {
                x,
                y,
                width,
                height,
                color,
            } => DrawOp::Rect {
                x: *x,
                y: *y,
                width: *width,
                height: *height,
                color: *color,
            },
            ForeignDrawCmd::FillRoundedRect {
                x,
                y,
                width,
                height,
                radius,
                color,
            } => DrawOp::RoundedRect {
                x: *x,
                y: *y,
                width: *width,
                height: *height,
                radius: *radius,
                color: *color,
            },
            ForeignDrawCmd::DrawLine {
                x0,
                y0,
                x1,
                y1,
                color,
            } => DrawOp::Line {
                x0: *x0,
                y0: *y0,
                x1: *x1,
                y1: *y1,
                color: *color,
            },
            ForeignDrawCmd::DrawTriangle {
                x0,
                y0,
                x1,
                y1,
                x2,
                y2,
                color,
            } => DrawOp::Triangle {
                x0: *x0,
                y0: *y0,
                x1: *x1,
                y1: *y1,
                x2: *x2,
                y2: *y2,
                color: *color,
            },
            ForeignDrawCmd::DrawEllipse {
                x,
                y,
                width,
                height,
                color,
            } => DrawOp::Ellipse {
                x: *x,
                y: *y,
                width: *width,
                height: *height,
                color: *color,
            },
            ForeignDrawCmd::DrawSprite {
                id,
                x,
                y,
                width,
                height,
            } => DrawOp::Sprite {
                sprite: id.clone(),
                x: *x,
                y: *y,
                width: *width,
                height: *height,
            },
            ForeignDrawCmd::GradientRect {
                x,
                y,
                width,
                height,
                top_left,
                top_right,
                bottom_left,
                bottom_right,
            } => DrawOp::GradientRect {
                x: *x,
                y: *y,
                width: *width,
                height: *height,
                top_left: *top_left,
                top_right: *top_right,
                bottom_left: *bottom_left,
                bottom_right: *bottom_right,
            },
            ForeignDrawCmd::ShadowRect {
                x,
                y,
                width,
                height,
                blur,
                color,
            } => DrawOp::ShadowRect {
                x: *x,
                y: *y,
                width: *width,
                height: *height,
                blur: *blur,
                color: *color,
            },
            ForeignDrawCmd::Backdrop {
                x,
                y,
                width,
                height,
                opacity,
            } => DrawOp::Backdrop {
                x: *x,
                y: *y,
                width: *width,
                height: *height,
                opacity: *opacity,
            },
            ForeignDrawCmd::SetClip {
                x,
                y,
                width,
                height,
            } => DrawOp::SetClip {
                x: *x,
                y: *y,
                width: *width,
                height: *height,
            },
            ForeignDrawCmd::ClearClip => DrawOp::ClearClip,
            ForeignDrawCmd::Text {
                text,
                x,
                y,
                size,
                color,
                font,
            } => DrawOp::Text {
                text: text.clone(),
                x: *x,
                y: *y,
                size: *size,
                color: *color,
                font: *font,
            },
            ForeignDrawCmd::Icon {
                icon,
                x,
                y,
                size,
                color,
            } => DrawOp::Icon {
                icon: *icon,
                x: *x,
                y: *y,
                size: *size,
                color: *color,
            },
            ForeignDrawCmd::Blit {
                source,
                x,
                y,
                width,
                height,
            } => DrawOp::Blit {
                source: source.clone(),
                x: *x,
                y: *y,
                width: *width,
                height: *height,
            },
            ForeignDrawCmd::PushLayer { opacity } => DrawOp::PushLayer { opacity: *opacity },
            ForeignDrawCmd::PopLayer => DrawOp::PopLayer,
            ForeignDrawCmd::GaussianBlur {
                x,
                y,
                width,
                height,
                radius,
            } => DrawOp::GaussianBlur {
                x: *x,
                y: *y,
                width: *width,
                height: *height,
                radius: *radius,
            },
            ForeignDrawCmd::SetPresentRegion {
                x,
                y,
                width,
                height,
            } => DrawOp::SetPresentRegion {
                x: *x,
                y: *y,
                width: *width,
                height: *height,
            },
            ForeignDrawCmd::Present {
                x,
                y,
                width,
                height,
            } => DrawOp::FlipRegion {
                x: *x,
                y: *y,
                width: *width,
                height: *height,
            },
        })
    }
}

// --- parse helpers ---

fn map_parse_color(value: &str) -> Result<RgbaColor, GfxTranslateError> {
    parse_color(value).map_err(|e| GfxTranslateError::ParseError(e.describe()))
}

fn map_parse_u32(value: &str, key: &'static str) -> Result<u32, GfxTranslateError> {
    parse_u32(key, value).map_err(|e| GfxTranslateError::ParseError(e.describe()))
}

fn map_parse_u8(value: &str, key: &'static str) -> Result<u8, GfxTranslateError> {
    parse_u8(key, value).map_err(|e| GfxTranslateError::ParseError(e.describe()))
}

fn parse_fill_rect(value: &str) -> Result<ForeignDrawCmd, GfxTranslateError> {
    let parts: Vec<&str> = value.split(',').map(str::trim).collect();
    if parts.len() != 5 {
        return Err(GfxTranslateError::ParseError(format!("fill-rect={value}")));
    }
    Ok(ForeignDrawCmd::FillRect {
        x: map_parse_u32(parts[0], "fill-rect.x")?,
        y: map_parse_u32(parts[1], "fill-rect.y")?,
        width: map_parse_u32(parts[2], "fill-rect.width")?,
        height: map_parse_u32(parts[3], "fill-rect.height")?,
        color: map_parse_color(parts[4])?,
    })
}

fn parse_fill_rounded_rect(value: &str) -> Result<ForeignDrawCmd, GfxTranslateError> {
    let parts: Vec<&str> = value.split(',').map(str::trim).collect();
    if parts.len() != 6 {
        return Err(GfxTranslateError::ParseError(format!(
            "fill-rounded-rect={value}"
        )));
    }
    Ok(ForeignDrawCmd::FillRoundedRect {
        x: map_parse_u32(parts[0], "fill-rounded-rect.x")?,
        y: map_parse_u32(parts[1], "fill-rounded-rect.y")?,
        width: map_parse_u32(parts[2], "fill-rounded-rect.width")?,
        height: map_parse_u32(parts[3], "fill-rounded-rect.height")?,
        radius: map_parse_u32(parts[4], "fill-rounded-rect.radius")?,
        color: map_parse_color(parts[5])?,
    })
}

fn parse_draw_line(value: &str) -> Result<ForeignDrawCmd, GfxTranslateError> {
    let parts: Vec<&str> = value.split(',').map(str::trim).collect();
    if parts.len() != 5 {
        return Err(GfxTranslateError::ParseError(format!("draw-line={value}")));
    }
    Ok(ForeignDrawCmd::DrawLine {
        x0: map_parse_u32(parts[0], "draw-line.x0")?,
        y0: map_parse_u32(parts[1], "draw-line.y0")?,
        x1: map_parse_u32(parts[2], "draw-line.x1")?,
        y1: map_parse_u32(parts[3], "draw-line.y1")?,
        color: map_parse_color(parts[4])?,
    })
}

fn parse_draw_triangle(value: &str) -> Result<ForeignDrawCmd, GfxTranslateError> {
    let parts: Vec<&str> = value.split(',').map(str::trim).collect();
    if parts.len() != 7 {
        return Err(GfxTranslateError::ParseError(format!(
            "draw-triangle={value}"
        )));
    }
    Ok(ForeignDrawCmd::DrawTriangle {
        x0: map_parse_u32(parts[0], "draw-triangle.x0")?,
        y0: map_parse_u32(parts[1], "draw-triangle.y0")?,
        x1: map_parse_u32(parts[2], "draw-triangle.x1")?,
        y1: map_parse_u32(parts[3], "draw-triangle.y1")?,
        x2: map_parse_u32(parts[4], "draw-triangle.x2")?,
        y2: map_parse_u32(parts[5], "draw-triangle.y2")?,
        color: map_parse_color(parts[6])?,
    })
}

fn parse_draw_ellipse(value: &str) -> Result<ForeignDrawCmd, GfxTranslateError> {
    let parts: Vec<&str> = value.split(',').map(str::trim).collect();
    if parts.len() != 5 {
        return Err(GfxTranslateError::ParseError(format!(
            "draw-ellipse={value}"
        )));
    }
    Ok(ForeignDrawCmd::DrawEllipse {
        x: map_parse_u32(parts[0], "draw-ellipse.x")?,
        y: map_parse_u32(parts[1], "draw-ellipse.y")?,
        width: map_parse_u32(parts[2], "draw-ellipse.width")?,
        height: map_parse_u32(parts[3], "draw-ellipse.height")?,
        color: map_parse_color(parts[4])?,
    })
}

fn parse_draw_sprite(value: &str) -> Result<ForeignDrawCmd, GfxTranslateError> {
    let parts: Vec<&str> = value.split(',').map(str::trim).collect();
    if parts.len() != 5 || parts[0].is_empty() {
        return Err(GfxTranslateError::ParseError(format!(
            "draw-sprite={value}"
        )));
    }
    Ok(ForeignDrawCmd::DrawSprite {
        id: parts[0].to_string(),
        x: map_parse_u32(parts[1], "draw-sprite.x")?,
        y: map_parse_u32(parts[2], "draw-sprite.y")?,
        width: map_parse_u32(parts[3], "draw-sprite.width")?,
        height: map_parse_u32(parts[4], "draw-sprite.height")?,
    })
}

fn parse_gradient_rect_cmd(value: &str) -> Result<ForeignDrawCmd, GfxTranslateError> {
    let parts: Vec<&str> = value.split(',').map(str::trim).collect();
    if parts.len() != 8 {
        return Err(GfxTranslateError::ParseError(format!(
            "gradient-rect={value}"
        )));
    }
    Ok(ForeignDrawCmd::GradientRect {
        x: map_parse_u32(parts[0], "gradient-rect.x")?,
        y: map_parse_u32(parts[1], "gradient-rect.y")?,
        width: map_parse_u32(parts[2], "gradient-rect.width")?,
        height: map_parse_u32(parts[3], "gradient-rect.height")?,
        top_left: map_parse_color(parts[4])?,
        top_right: map_parse_color(parts[5])?,
        bottom_left: map_parse_color(parts[6])?,
        bottom_right: map_parse_color(parts[7])?,
    })
}

fn parse_shadow_rect_cmd(value: &str) -> Result<ForeignDrawCmd, GfxTranslateError> {
    let parts: Vec<&str> = value.split(',').map(str::trim).collect();
    if parts.len() != 6 {
        return Err(GfxTranslateError::ParseError(format!(
            "shadow-rect={value}"
        )));
    }
    Ok(ForeignDrawCmd::ShadowRect {
        x: map_parse_u32(parts[0], "shadow-rect.x")?,
        y: map_parse_u32(parts[1], "shadow-rect.y")?,
        width: map_parse_u32(parts[2], "shadow-rect.width")?,
        height: map_parse_u32(parts[3], "shadow-rect.height")?,
        blur: map_parse_u32(parts[4], "shadow-rect.blur")?,
        color: map_parse_color(parts[5])?,
    })
}

fn parse_backdrop_cmd(value: &str) -> Result<ForeignDrawCmd, GfxTranslateError> {
    let parts: Vec<&str> = value.split(',').map(str::trim).collect();
    if parts.len() != 5 {
        return Err(GfxTranslateError::ParseError(format!("backdrop={value}")));
    }
    Ok(ForeignDrawCmd::Backdrop {
        x: map_parse_u32(parts[0], "backdrop.x")?,
        y: map_parse_u32(parts[1], "backdrop.y")?,
        width: map_parse_u32(parts[2], "backdrop.width")?,
        height: map_parse_u32(parts[3], "backdrop.height")?,
        opacity: map_parse_u8(parts[4], "backdrop.opacity")?,
    })
}

fn parse_set_clip_cmd(value: &str) -> Result<ForeignDrawCmd, GfxTranslateError> {
    let parts: Vec<&str> = value.split(',').map(str::trim).collect();
    if parts.len() != 4 {
        return Err(GfxTranslateError::ParseError(format!("set-clip={value}")));
    }
    Ok(ForeignDrawCmd::SetClip {
        x: map_parse_u32(parts[0], "set-clip.x")?,
        y: map_parse_u32(parts[1], "set-clip.y")?,
        width: map_parse_u32(parts[2], "set-clip.width")?,
        height: map_parse_u32(parts[3], "set-clip.height")?,
    })
}

fn parse_clear_clip_alias(value: &str) -> Result<ForeignDrawCmd, GfxTranslateError> {
    if !value.is_empty() {
        return Err(GfxTranslateError::ParseError(format!("clear-clip={value}")));
    }
    Ok(ForeignDrawCmd::ClearClip)
}

fn parse_text_cmd(value: &str) -> Result<ForeignDrawCmd, GfxTranslateError> {
    let parts: Vec<&str> = value.split(',').map(str::trim).collect();
    if parts.len() != 5 && parts.len() != 6 || parts[0].is_empty() {
        return Err(GfxTranslateError::ParseError(format!("text={value}")));
    }
    let font = if parts.len() == 6 {
        parse_font_family(parts[5])?
    } else {
        FontFamily::System
    };
    Ok(ForeignDrawCmd::Text {
        text: parts[0].to_string(),
        x: map_parse_u32(parts[1], "text.x")?,
        y: map_parse_u32(parts[2], "text.y")?,
        size: map_parse_u32(parts[3], "text.size")?,
        color: map_parse_color(parts[4])?,
        font,
    })
}

fn parse_icon_cmd(value: &str) -> Result<ForeignDrawCmd, GfxTranslateError> {
    let parts: Vec<&str> = value.split(',').map(str::trim).collect();
    if parts.len() != 5 || parts[0].is_empty() {
        return Err(GfxTranslateError::ParseError(format!("icon={value}")));
    }
    let mut chars = parts[0].chars();
    let Some(icon) = chars.next() else {
        return Err(GfxTranslateError::ParseError(format!("icon={value}")));
    };
    if chars.next().is_some() {
        return Err(GfxTranslateError::ParseError(format!("icon={value}")));
    }
    Ok(ForeignDrawCmd::Icon {
        icon,
        x: map_parse_u32(parts[1], "icon.x")?,
        y: map_parse_u32(parts[2], "icon.y")?,
        size: map_parse_u32(parts[3], "icon.size")?,
        color: map_parse_color(parts[4])?,
    })
}

fn parse_font_family(value: &str) -> Result<FontFamily, GfxTranslateError> {
    let font = match value.to_ascii_lowercase().as_str() {
        "system" => FontFamily::System,
        "monospace" | "mono" => FontFamily::Monospace,
        "sans" | "sans-serif" | "sansserif" => FontFamily::SansSerif,
        "serif" => FontFamily::Serif,
        _ => {
            return Err(GfxTranslateError::ParseError(format!("font={value}")));
        }
    };
    Ok(font)
}

fn parse_blit_cmd(value: &str) -> Result<ForeignDrawCmd, GfxTranslateError> {
    let parts: Vec<&str> = value.split(',').map(str::trim).collect();
    if parts.len() != 5 || parts[0].is_empty() {
        return Err(GfxTranslateError::ParseError(format!("blit={value}")));
    }
    Ok(ForeignDrawCmd::Blit {
        source: parts[0].to_string(),
        x: map_parse_u32(parts[1], "blit.x")?,
        y: map_parse_u32(parts[2], "blit.y")?,
        width: map_parse_u32(parts[3], "blit.width")?,
        height: map_parse_u32(parts[4], "blit.height")?,
    })
}

fn parse_gaussian_blur_cmd(value: &str) -> Result<ForeignDrawCmd, GfxTranslateError> {
    let parts: Vec<&str> = value.split(',').map(str::trim).collect();
    if parts.len() != 5 {
        return Err(GfxTranslateError::ParseError(format!(
            "gaussian-blur={value}"
        )));
    }
    Ok(ForeignDrawCmd::GaussianBlur {
        x: map_parse_u32(parts[0], "gaussian-blur.x")?,
        y: map_parse_u32(parts[1], "gaussian-blur.y")?,
        width: map_parse_u32(parts[2], "gaussian-blur.width")?,
        height: map_parse_u32(parts[3], "gaussian-blur.height")?,
        radius: map_parse_u32(parts[4], "gaussian-blur.radius")?,
    })
}

fn parse_set_present_region_cmd(value: &str) -> Result<ForeignDrawCmd, GfxTranslateError> {
    let parts: Vec<&str> = value.split(',').map(str::trim).collect();
    if parts.len() != 4 {
        return Err(GfxTranslateError::ParseError(format!(
            "set-present-region={value}"
        )));
    }
    Ok(ForeignDrawCmd::SetPresentRegion {
        x: map_parse_u32(parts[0], "set-present-region.x")?,
        y: map_parse_u32(parts[1], "set-present-region.y")?,
        width: map_parse_u32(parts[2], "set-present-region.width")?,
        height: map_parse_u32(parts[3], "set-present-region.height")?,
    })
}

fn parse_present_cmd(value: &str) -> Result<ForeignDrawCmd, GfxTranslateError> {
    let parts: Vec<&str> = value.split(',').map(str::trim).collect();
    if parts.len() != 4 {
        return Err(GfxTranslateError::ParseError(format!("present={value}")));
    }
    Ok(ForeignDrawCmd::Present {
        x: map_parse_u32(parts[0], "present.x")?,
        y: map_parse_u32(parts[1], "present.y")?,
        width: map_parse_u32(parts[2], "present.width")?,
        height: map_parse_u32(parts[3], "present.height")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dx12_translator() -> GfxTranslator {
        GfxTranslator::new(SourceApi::DirectX12)
    }

    fn metal_translator() -> GfxTranslator {
        GfxTranslator::new(SourceApi::Metal)
    }

    fn vulkan_translator() -> GfxTranslator {
        GfxTranslator::new(SourceApi::Vulkan)
    }

    #[test]
    fn source_api_names_and_translation_labels() {
        assert_eq!(SourceApi::DirectX12.name(), "directx12");
        assert_eq!(SourceApi::DirectX12.translation_label(), "compat-to-vulkan");
        assert_eq!(SourceApi::OpenGL.name(), "opengl");
        assert_eq!(SourceApi::OpenGL.translation_label(), "compat-to-vulkan");
        assert_eq!(SourceApi::Metal.name(), "metal");
        assert_eq!(SourceApi::Metal.translation_label(), "compat-to-vulkan");
        assert_eq!(SourceApi::Vulkan.name(), "vulkan");
        assert_eq!(SourceApi::Vulkan.translation_label(), "native-vulkan");
    }

    #[test]
    fn translates_dx12_frame_to_frame_script() {
        let foreign = ForeignFrameScript::parse(
            "surface=1280x720\nframe=dx12-001\nqueue=graphics\npresent-mode=mailbox\ncompletion=wait-present\nclear=111122ff\nfill-rect=0,0,1280,720,000000ff\ndraw-sprite=player,400,200,96,96\npresent=0,0,1280,720\n",
        )
        .unwrap();
        let script = dx12_translator().translate(&foreign).unwrap();
        assert_eq!(script.width, 1280);
        assert_eq!(script.height, 720);
        assert_eq!(script.frame_tag, "dx12-001");
        assert_eq!(script.ops.len(), 4);
        assert!(matches!(script.ops[0], DrawOp::Clear { .. }));
        assert!(matches!(script.ops[1], DrawOp::Rect { .. }));
        assert!(matches!(script.ops[2], DrawOp::Sprite { .. }));
        assert!(matches!(script.ops[3], DrawOp::FlipRegion { .. }));
    }

    #[test]
    fn translates_metal_frame_with_effects() {
        let foreign = ForeignFrameScript::parse(
            "surface=1920x1080\nframe=metal-001\nqueue=graphics\npresent-mode=fifo\ncompletion=fire-and-forget\nclear=000000ff\nfill-rounded-rect=10,10,400,300,16,003355ff\ngaussian-blur=10,10,400,300,8\nset-present-region=0,0,1920,1080\npresent=0,0,1920,1080\n",
        )
        .unwrap();
        let script = metal_translator().translate(&foreign).unwrap();
        assert_eq!(script.ops.len(), 5);
        assert!(matches!(script.ops[1], DrawOp::RoundedRect { .. }));
        assert!(matches!(script.ops[2], DrawOp::GaussianBlur { .. }));
        assert!(matches!(script.ops[3], DrawOp::SetPresentRegion { .. }));
        assert!(matches!(script.ops[4], DrawOp::FlipRegion { .. }));
    }

    #[test]
    fn translates_vulkan_native_passthrough() {
        let foreign = ForeignFrameScript::parse_for_api(
            Some(SourceApi::Vulkan),
            "surface=640x480\nframe=vk-001\nqueue=graphics\npresent-mode=fifo\ncompletion=wait-present\nvk-cmd-clear-color=002244ff\nvk-cmd-fill-rect=50,50,200,100,ff8800ff\nvk-queue-present=0,0,640,480\n",
        )
        .unwrap();
        let script = vulkan_translator().translate(&foreign).unwrap();
        assert_eq!(script.ops.len(), 3);
        // native vulkan — same translation path, different label
        assert_eq!(SourceApi::Vulkan.translation_label(), "native-vulkan");
    }

    #[test]
    fn translates_all_foreign_cmd_variants() {
        let foreign = ForeignFrameScript::parse(
            "surface=640x480\nframe=all-001\nqueue=graphics\npresent-mode=fifo\ncompletion=fire-and-forget\nclear=ffffffff\ndraw-line=0,0,640,480,ff0000ff\ndraw-triangle=0,0,100,0,50,100,00ff00ff\ndraw-ellipse=100,100,200,150,0000ffff\ndraw-sprite=icon,10,10,32,32\nblit=framebuffer,0,0,640,480\npush-layer=200\nfill-rect=0,0,100,100,ffffff88\npop-layer\npresent=0,0,640,480\n",
        )
        .unwrap();
        let script = dx12_translator().translate(&foreign).unwrap();
        assert_eq!(script.ops.len(), 10);
        assert!(matches!(script.ops[1], DrawOp::Line { .. }));
        assert!(matches!(script.ops[2], DrawOp::Triangle { .. }));
        assert!(matches!(script.ops[3], DrawOp::Ellipse { .. }));
        assert!(matches!(script.ops[4], DrawOp::Sprite { .. }));
        assert!(matches!(script.ops[5], DrawOp::Blit { .. }));
        assert!(matches!(script.ops[6], DrawOp::PushLayer { .. }));
        assert!(matches!(script.ops[8], DrawOp::PopLayer));
        assert!(matches!(script.ops[9], DrawOp::FlipRegion { .. }));
    }

    #[test]
    fn translates_deeper_foreign_ui_semantics() {
        let foreign = ForeignFrameScript::parse_for_api(
            Some(SourceApi::DirectX12),
            "surface=800x600\nframe=deep-001\nqueue=graphics\npresent-mode=mailbox\ncompletion=wait-present\ndx-clear-rtv=000000ff\ndx-gradient-rect=0,0,800,600,112233ff,223344ff,334455ff,445566ff\ndx-draw-text=Compat,24,24,18,ffddbbff,monospace\ndx-draw-icon=★,220,24,18,ffffffff\ndx-shadow-rect=20,60,240,120,12,00000080\ndx-backdrop=32,72,160,96,180\ndx-set-clip=0,0,320,240\ndx-clear-clip\ndx-present=0,0,800,600\n",
        )
        .unwrap();
        let script = dx12_translator().translate(&foreign).unwrap();
        assert_eq!(script.ops.len(), 9);
        assert!(matches!(script.ops[0], DrawOp::Clear { .. }));
        assert!(matches!(script.ops[1], DrawOp::GradientRect { .. }));
        assert!(matches!(script.ops[2], DrawOp::Text { .. }));
        assert!(matches!(script.ops[3], DrawOp::Icon { .. }));
        assert!(matches!(script.ops[4], DrawOp::ShadowRect { .. }));
        assert!(matches!(script.ops[5], DrawOp::Backdrop { .. }));
        assert!(matches!(script.ops[6], DrawOp::SetClip { .. }));
        assert!(matches!(script.ops[7], DrawOp::ClearClip));
        assert!(matches!(script.ops[8], DrawOp::FlipRegion { .. }));
    }

    #[test]
    fn parses_api_specific_deep_ui_aliases() {
        let foreign = ForeignFrameScript::parse_for_api(
            Some(SourceApi::OpenGL),
            "surface=640x360\nframe=deep-gl-001\nqueue=graphics\npresent-mode=fifo\ncompletion=fire-and-forget\ngl-clear=0f0f0fff\ngl-draw-text=Compat,12,16,14,ffffffff,sans\ngl-draw-icon=✓,120,16,14,00ff00ff\ngl-gradient-rect=0,0,640,180,0f0f0fff,1f1f1fff,2f2f2fff,3f3f3fff\ngl-shadow-rect=24,24,160,72,8,00000080\ngl-backdrop=40,40,128,64,160\ngl-set-clip=0,0,320,180\ngl-clear-clip\ngl-swap-buffers=0,0,640,360\n",
        )
        .unwrap();
        let script = GfxTranslator::new(SourceApi::OpenGL)
            .translate(&foreign)
            .unwrap();
        assert!(matches!(script.ops[0], DrawOp::Clear { .. }));
        assert!(matches!(script.ops[1], DrawOp::Text { .. }));
        assert!(matches!(script.ops[2], DrawOp::Icon { .. }));
        assert!(matches!(script.ops[3], DrawOp::GradientRect { .. }));
        assert!(matches!(script.ops[4], DrawOp::ShadowRect { .. }));
        assert!(matches!(script.ops[5], DrawOp::Backdrop { .. }));
        assert!(matches!(script.ops[6], DrawOp::SetClip { .. }));
        assert!(matches!(script.ops[7], DrawOp::ClearClip));
        assert!(matches!(script.ops[8], DrawOp::FlipRegion { .. }));
    }

    #[test]
    fn refuses_empty_frame() {
        let foreign = ForeignFrameScript {
            surface_width: 640,
            surface_height: 480,
            frame_tag: String::from("empty"),
            queue: String::from("graphics"),
            present_mode: String::from("fifo"),
            completion: String::from("fire-and-forget"),
            cmds: vec![],
        };
        let err = dx12_translator().translate(&foreign).unwrap_err();
        assert_eq!(err, GfxTranslateError::EmptyFrame);
        assert!(err.describe().contains("no draw ops"));
    }

    #[test]
    fn refuses_invalid_draw_op_zero_ellipse() {
        let foreign = ForeignFrameScript {
            surface_width: 640,
            surface_height: 480,
            frame_tag: String::from("bad"),
            queue: String::from("graphics"),
            present_mode: String::from("fifo"),
            completion: String::from("fire-and-forget"),
            cmds: vec![ForeignDrawCmd::DrawEllipse {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
                color: RgbaColor {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 255,
                },
            }],
        };
        let err = dx12_translator().translate(&foreign).unwrap_err();
        assert!(matches!(err, GfxTranslateError::InvalidDrawOp(_)));
    }

    #[test]
    fn parses_foreign_frame_rejects_unknown_line() {
        let err = ForeignFrameScript::parse(
            "surface=640x480\nframe=f\nqueue=graphics\npresent-mode=fifo\ncompletion=fire-and-forget\nunknown-cmd=foo\n",
        )
        .unwrap_err();
        assert!(matches!(err, GfxTranslateError::ParseError(_)));
        assert!(err.describe().contains("unknown line"));
    }

    #[test]
    fn parses_foreign_frame_rejects_missing_surface() {
        let err = ForeignFrameScript::parse(
            "frame=f\nqueue=graphics\npresent-mode=fifo\ncompletion=fire-and-forget\nclear=ffffffff\n",
        )
        .unwrap_err();
        assert!(err.describe().contains("surface"));
    }

    #[test]
    fn encodes_translated_frame_with_profile() {
        let foreign = ForeignFrameScript::parse(
            "surface=800x600\nframe=enc-001\nqueue=present\npresent-mode=immediate\ncompletion=wait-complete\nclear=000000ff\nfill-rect=0,0,800,600,223344ff\npresent=0,0,800,600\n",
        )
        .unwrap();
        let script = dx12_translator().translate(&foreign).unwrap();
        let encoded = script.encode_translated(
            dx12_translator().source_api.translation_label(),
            dx12_translator().source_api.name(),
            dx12_translator().source_api.translation_label(),
        );
        assert_eq!(encoded.frame_tag, "enc-001");
        assert_eq!(encoded.op_count, 3);
        assert!(encoded.payload.contains("profile=compat-to-vulkan"));
        assert!(encoded.payload.contains("source-api=directx12"));
        assert!(encoded.payload.contains("translation=compat-to-vulkan"));
        assert!(encoded.payload.contains("op=clear"));
        assert!(encoded.payload.contains("op=rect"));
        assert!(encoded.payload.contains("op=flip-region"));
    }

    #[test]
    fn parses_api_specific_directx_aliases() {
        let foreign = ForeignFrameScript::parse_for_api(
            Some(SourceApi::DirectX12),
            "surface=1280x720\nframe=dx12-alias\nqueue=graphics\npresent-mode=mailbox\ncompletion=wait-complete\ndx-clear-rtv=000000ff\ndx-fill-rect=0,0,1280,720,223344ff\ndx-copy-resource=hud,0,0,1280,64\ndx-present=0,0,1280,720\n",
        )
        .unwrap();
        let script = dx12_translator().translate(&foreign).unwrap();
        assert!(matches!(script.ops[0], DrawOp::Clear { .. }));
        assert!(matches!(script.ops[1], DrawOp::Rect { .. }));
        assert!(matches!(script.ops[2], DrawOp::Blit { .. }));
        assert!(matches!(script.ops[3], DrawOp::FlipRegion { .. }));
    }

    #[test]
    fn parses_api_specific_direct3d9_aliases() {
        let foreign = ForeignFrameScript::parse_for_api(
            Some(SourceApi::Direct3D9),
            "surface=640x480\nframe=d3d9-alias\nqueue=graphics\npresent-mode=fifo\ncompletion=fire-and-forget\ndx-clear-rtv=111111ff\ndx-draw-sprite=hero,32,32,64,64\ndx-present=0,0,640,480\n",
        )
        .unwrap();
        let script = GfxTranslator::new(SourceApi::Direct3D9)
            .translate(&foreign)
            .unwrap();
        assert!(matches!(script.ops[0], DrawOp::Clear { .. }));
        assert!(matches!(script.ops[1], DrawOp::Sprite { .. }));
        assert!(matches!(script.ops[2], DrawOp::FlipRegion { .. }));
    }

    #[test]
    fn parses_api_specific_direct3d10_aliases() {
        let foreign = ForeignFrameScript::parse_for_api(
            Some(SourceApi::Direct3D10),
            "surface=800x600\nframe=d3d10-alias\nqueue=graphics\npresent-mode=fifo\ncompletion=fire-and-forget\ndx-clear-rtv=222222ff\ndx-draw-triangle=0,0,100,0,50,80,00ff00ff\ndx-present=0,0,800,600\n",
        )
        .unwrap();
        let script = GfxTranslator::new(SourceApi::Direct3D10)
            .translate(&foreign)
            .unwrap();
        assert!(matches!(script.ops[0], DrawOp::Clear { .. }));
        assert!(matches!(script.ops[1], DrawOp::Triangle { .. }));
        assert!(matches!(script.ops[2], DrawOp::FlipRegion { .. }));
    }

    #[test]
    fn parses_api_specific_opengl_aliases() {
        let foreign = ForeignFrameScript::parse_for_api(
            Some(SourceApi::OpenGL),
            "surface=800x600\nframe=gl-alias\nqueue=graphics\npresent-mode=fifo\ncompletion=fire-and-forget\ngl-clear=101010ff\ngl-draw-line=0,0,799,599,ffffffff\ngl-swap-buffers=0,0,800,600\n",
        )
        .unwrap();
        let script = GfxTranslator::new(SourceApi::OpenGL)
            .translate(&foreign)
            .unwrap();
        assert!(matches!(script.ops[0], DrawOp::Clear { .. }));
        assert!(matches!(script.ops[1], DrawOp::Line { .. }));
        assert!(matches!(script.ops[2], DrawOp::FlipRegion { .. }));
    }

    #[test]
    fn parses_api_specific_opengles_aliases() {
        let foreign = ForeignFrameScript::parse_for_api(
            Some(SourceApi::OpenGLES),
            "surface=480x320\nframe=gles-alias\nqueue=graphics\npresent-mode=immediate\ncompletion=fire-and-forget\ngles-clear=333333ff\ngles-draw-ellipse=10,10,100,60,ff00ffff\ngles-swap-buffers=0,0,480,320\n",
        )
        .unwrap();
        let script = GfxTranslator::new(SourceApi::OpenGLES)
            .translate(&foreign)
            .unwrap();
        assert!(matches!(script.ops[0], DrawOp::Clear { .. }));
        assert!(matches!(script.ops[1], DrawOp::Ellipse { .. }));
        assert!(matches!(script.ops[2], DrawOp::FlipRegion { .. }));
    }

    #[test]
    fn parses_api_specific_metal_aliases() {
        let foreign = ForeignFrameScript::parse_for_api(
            Some(SourceApi::Metal),
            "surface=1024x768\nframe=metal-alias\nqueue=graphics\npresent-mode=fifo\ncompletion=wait-present\nmetal-clear=001122ff\nmetal-fill-rounded-rect=10,10,200,120,12,88aaffff\nmetal-present-drawable=0,0,1024,768\n",
        )
        .unwrap();
        let script = metal_translator().translate(&foreign).unwrap();
        assert!(matches!(script.ops[1], DrawOp::RoundedRect { .. }));
        assert!(matches!(script.ops[2], DrawOp::FlipRegion { .. }));
    }

    #[test]
    fn parses_api_specific_webgpu_aliases() {
        let foreign = ForeignFrameScript::parse_for_api(
            Some(SourceApi::WebGPU),
            "surface=640x360\nframe=webgpu-alias\nqueue=graphics\npresent-mode=immediate\ncompletion=fire-and-forget\nwebgpu-clear-pass=0f0f0fff\nwebgpu-copy-texture=backbuffer,0,0,640,360\nwebgpu-present=0,0,640,360\n",
        )
        .unwrap();
        let script = GfxTranslator::new(SourceApi::WebGPU)
            .translate(&foreign)
            .unwrap();
        assert!(matches!(script.ops[0], DrawOp::Clear { .. }));
        assert!(matches!(script.ops[1], DrawOp::Blit { .. }));
        assert!(matches!(script.ops[2], DrawOp::FlipRegion { .. }));
    }

    #[test]
    fn parses_api_specific_wgpu_aliases() {
        let foreign = ForeignFrameScript::parse_for_api(
            Some(SourceApi::Wgpu),
            "surface=960x540\nframe=wgpu-alias\nqueue=graphics\npresent-mode=fifo\ncompletion=wait-present\nwgpu-clear-pass=444444ff\nwgpu-fill-rect=0,0,960,540,123456ff\nwgpu-present=0,0,960,540\n",
        )
        .unwrap();
        let script = GfxTranslator::new(SourceApi::Wgpu)
            .translate(&foreign)
            .unwrap();
        assert!(matches!(script.ops[0], DrawOp::Clear { .. }));
        assert!(matches!(script.ops[1], DrawOp::Rect { .. }));
        assert!(matches!(script.ops[2], DrawOp::FlipRegion { .. }));
    }

    #[test]
    fn rejects_aliases_from_wrong_api_family() {
        let err = ForeignFrameScript::parse_for_api(
            Some(SourceApi::OpenGL),
            "surface=640x480\nframe=wrong-alias\nqueue=graphics\npresent-mode=fifo\ncompletion=fire-and-forget\ndx-clear-rtv=ff0000ff\n",
        )
        .unwrap_err();
        assert!(matches!(err, GfxTranslateError::ParseError(_)));
        assert!(err.describe().contains("unknown line"));
    }
}
