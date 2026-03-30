#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use ngos_user_abi::{NativeContractKind, NativeResourceKind};
use ngos_user_runtime::Runtime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsBackend {
    Vulkan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioBackend {
    NativeMixer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputBackend {
    NativeInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphicsConfig {
    pub backend: GraphicsBackend,
    pub profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioConfig {
    pub backend: AudioBackend,
    pub profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputConfig {
    pub backend: InputBackend,
    pub profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeShimPaths {
    pub prefix: String,
    pub saves: String,
    pub cache: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameCompatManifest {
    pub title: String,
    pub slug: String,
    pub executable_path: String,
    pub working_dir: String,
    pub argv: Vec<String>,
    pub graphics: GraphicsConfig,
    pub audio: AudioConfig,
    pub input: InputConfig,
    pub shims: RuntimeShimPaths,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatLaneKind {
    Graphics,
    Audio,
    Input,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatLanePlan {
    pub kind: CompatLaneKind,
    pub resource_name: String,
    pub contract_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvShim {
    pub key: &'static str,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameSessionPlan {
    pub domain_name: String,
    pub process_name: String,
    pub executable_path: String,
    pub working_dir: String,
    pub argv: Vec<String>,
    pub lanes: [CompatLanePlan; 3],
    pub env_shims: Vec<EnvShim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    MissingField(&'static str),
    InvalidField { key: String, value: String },
}

pub struct GameSession {
    pub plan: GameSessionPlan,
    pub domain_id: u64,
    pub process_id: u64,
}

impl GameSession {
    pub fn launch<B: ngos_user_abi::SyscallBackend>(
        runtime: &Runtime<B>,
        manifest: &GameCompatManifest,
    ) -> Result<Self, ngos_user_abi::Errno> {
        let plan = manifest.session_plan();

        // 1. Create Domain
        let domain_id = runtime.create_domain(None, &plan.domain_name)? as u64;

        // 2. Create Resources and Contracts for each lane
        for lane in &plan.lanes {
            let resource_kind = match lane.kind {
                CompatLaneKind::Graphics => NativeResourceKind::Surface,
                CompatLaneKind::Audio => NativeResourceKind::Channel,
                CompatLaneKind::Input => NativeResourceKind::Device,
            };

            let resource_id =
                runtime.create_resource(domain_id as usize, resource_kind, &lane.resource_name)?
                    as u64;

            let contract_kind = match lane.kind {
                CompatLaneKind::Graphics => NativeContractKind::Display,
                CompatLaneKind::Audio => NativeContractKind::Io,
                CompatLaneKind::Input => NativeContractKind::Observe,
            };

            let _contract_id = runtime.create_contract(
                domain_id as usize,
                resource_id as usize,
                contract_kind,
                &lane.contract_label,
            )?;
        }

        // 3. Spawn Process
        let pid = runtime.spawn_path_process(&plan.process_name, &plan.executable_path)?;

        let mut process_argv = vec![plan.executable_path.as_str()];
        for arg in &plan.argv {
            process_argv.push(arg.as_str());
        }

        let mut env_shims = Vec::new();
        for shim in &plan.env_shims {
            env_shims.push(format!("{}={}", shim.key, shim.value));
        }
        let env_refs: Vec<&str> = env_shims.iter().map(|s| s.as_str()).collect();

        runtime.set_process_cwd(pid, &plan.working_dir)?;
        runtime.set_process_args(pid, &process_argv)?;
        runtime.set_process_env(pid, &env_refs)?;

        Ok(Self {
            plan,
            domain_id,
            process_id: pid,
        })
    }
}

impl ManifestError {
    pub fn describe(&self) -> String {
        match self {
            Self::MissingField(field) => format!("missing field {field}"),
            Self::InvalidField { key, value } => {
                format!("invalid field {key} value={value}")
            }
        }
    }
}

impl GameCompatManifest {
    pub fn parse(text: &str) -> Result<Self, ManifestError> {
        let mut title = None::<String>;
        let mut slug = None::<String>;
        let mut executable_path = None::<String>;
        let mut working_dir = None::<String>;
        let mut argv = Vec::<String>::new();
        let mut gfx_backend = None::<GraphicsBackend>;
        let mut gfx_profile = None::<String>;
        let mut audio_backend = None::<AudioBackend>;
        let mut audio_profile = None::<String>;
        let mut input_backend = None::<InputBackend>;
        let mut input_profile = None::<String>;
        let mut shim_prefix = None::<String>;
        let mut shim_saves = None::<String>;
        let mut shim_cache = None::<String>;

        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((raw_key, raw_value)) = line.split_once('=') else {
                return Err(ManifestError::InvalidField {
                    key: line.to_string(),
                    value: String::new(),
                });
            };
            let key = raw_key.trim();
            let value = raw_value.trim();
            if value.is_empty() {
                return Err(ManifestError::InvalidField {
                    key: key.to_string(),
                    value: value.to_string(),
                });
            }
            match key {
                "title" => title = Some(value.to_string()),
                "slug" => slug = Some(value.to_string()),
                "exec" => executable_path = Some(value.to_string()),
                "cwd" => working_dir = Some(value.to_string()),
                "arg" => argv.push(value.to_string()),
                "gfx.backend" => gfx_backend = Some(parse_graphics_backend(value)?),
                "gfx.profile" => gfx_profile = Some(value.to_string()),
                "audio.backend" => audio_backend = Some(parse_audio_backend(value)?),
                "audio.profile" => audio_profile = Some(value.to_string()),
                "input.backend" => input_backend = Some(parse_input_backend(value)?),
                "input.profile" => input_profile = Some(value.to_string()),
                "shim.prefix" => shim_prefix = Some(value.to_string()),
                "shim.saves" => shim_saves = Some(value.to_string()),
                "shim.cache" => shim_cache = Some(value.to_string()),
                _ => {
                    return Err(ManifestError::InvalidField {
                        key: key.to_string(),
                        value: value.to_string(),
                    });
                }
            }
        }

        let manifest = Self {
            title: title.ok_or(ManifestError::MissingField("title"))?,
            slug: slug.ok_or(ManifestError::MissingField("slug"))?,
            executable_path: executable_path.ok_or(ManifestError::MissingField("exec"))?,
            working_dir: working_dir.ok_or(ManifestError::MissingField("cwd"))?,
            argv,
            graphics: GraphicsConfig {
                backend: gfx_backend.ok_or(ManifestError::MissingField("gfx.backend"))?,
                profile: gfx_profile.ok_or(ManifestError::MissingField("gfx.profile"))?,
            },
            audio: AudioConfig {
                backend: audio_backend.ok_or(ManifestError::MissingField("audio.backend"))?,
                profile: audio_profile.ok_or(ManifestError::MissingField("audio.profile"))?,
            },
            input: InputConfig {
                backend: input_backend.ok_or(ManifestError::MissingField("input.backend"))?,
                profile: input_profile.ok_or(ManifestError::MissingField("input.profile"))?,
            },
            shims: RuntimeShimPaths {
                prefix: shim_prefix.ok_or(ManifestError::MissingField("shim.prefix"))?,
                saves: shim_saves.ok_or(ManifestError::MissingField("shim.saves"))?,
                cache: shim_cache.ok_or(ManifestError::MissingField("shim.cache"))?,
            },
        };
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), ManifestError> {
        if !valid_slug(&self.slug) {
            return Err(invalid_field("slug", &self.slug));
        }
        if !self.executable_path.starts_with('/') {
            return Err(invalid_field("exec", &self.executable_path));
        }
        if !self.working_dir.starts_with('/') {
            return Err(invalid_field("cwd", &self.working_dir));
        }
        if !self.shims.prefix.starts_with('/') {
            return Err(invalid_field("shim.prefix", &self.shims.prefix));
        }
        if !self.shims.saves.starts_with('/') {
            return Err(invalid_field("shim.saves", &self.shims.saves));
        }
        if !self.shims.cache.starts_with('/') {
            return Err(invalid_field("shim.cache", &self.shims.cache));
        }
        Ok(())
    }

    pub fn session_plan(&self) -> GameSessionPlan {
        let process_name = format!("game-{}", self.slug);
        GameSessionPlan {
            domain_name: format!("compat-game-{}", self.slug),
            process_name,
            executable_path: self.executable_path.clone(),
            working_dir: self.working_dir.clone(),
            argv: self.argv.clone(),
            lanes: [
                CompatLanePlan {
                    kind: CompatLaneKind::Graphics,
                    resource_name: format!("{}-gfx", self.slug),
                    contract_label: format!("{}-display", self.graphics.profile),
                },
                CompatLanePlan {
                    kind: CompatLaneKind::Audio,
                    resource_name: format!("{}-audio", self.slug),
                    contract_label: format!("{}-mix", self.audio.profile),
                },
                CompatLanePlan {
                    kind: CompatLaneKind::Input,
                    resource_name: format!("{}-input", self.slug),
                    contract_label: format!("{}-capture", self.input.profile),
                },
            ],
            env_shims: vec![
                EnvShim {
                    key: "NGOS_GAME_TITLE",
                    value: self.title.clone(),
                },
                EnvShim {
                    key: "NGOS_GAME_SLUG",
                    value: self.slug.clone(),
                },
                EnvShim {
                    key: "NGOS_GFX_BACKEND",
                    value: graphics_backend_name(self.graphics.backend).to_string(),
                },
                EnvShim {
                    key: "NGOS_AUDIO_BACKEND",
                    value: audio_backend_name(self.audio.backend).to_string(),
                },
                EnvShim {
                    key: "NGOS_INPUT_BACKEND",
                    value: input_backend_name(self.input.backend).to_string(),
                },
                EnvShim {
                    key: "NGOS_COMPAT_PREFIX",
                    value: self.shims.prefix.clone(),
                },
                EnvShim {
                    key: "NGOS_COMPAT_SAVES",
                    value: self.shims.saves.clone(),
                },
                EnvShim {
                    key: "NGOS_COMPAT_CACHE",
                    value: self.shims.cache.clone(),
                },
            ],
        }
    }
}

pub const fn graphics_backend_name(backend: GraphicsBackend) -> &'static str {
    match backend {
        GraphicsBackend::Vulkan => "vulkan",
    }
}

pub const fn audio_backend_name(backend: AudioBackend) -> &'static str {
    match backend {
        AudioBackend::NativeMixer => "native-mixer",
    }
}

pub const fn input_backend_name(backend: InputBackend) -> &'static str {
    match backend {
        InputBackend::NativeInput => "native-input",
    }
}

pub const fn lane_name(kind: CompatLaneKind) -> &'static str {
    match kind {
        CompatLaneKind::Graphics => "graphics",
        CompatLaneKind::Audio => "audio",
        CompatLaneKind::Input => "input",
    }
}

fn invalid_field(key: &str, value: &str) -> ManifestError {
    ManifestError::InvalidField {
        key: key.to_string(),
        value: value.to_string(),
    }
}

fn parse_graphics_backend(value: &str) -> Result<GraphicsBackend, ManifestError> {
    match value {
        "vulkan" => Ok(GraphicsBackend::Vulkan),
        _ => Err(invalid_field("gfx.backend", value)),
    }
}

fn parse_audio_backend(value: &str) -> Result<AudioBackend, ManifestError> {
    match value {
        "native-mixer" => Ok(AudioBackend::NativeMixer),
        _ => Err(invalid_field("audio.backend", value)),
    }
}

fn parse_input_backend(value: &str) -> Result<InputBackend, ManifestError> {
    match value {
        "native-input" => Ok(InputBackend::NativeInput),
        _ => Err(invalid_field("input.backend", value)),
    }
}

fn valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_manifest_and_builds_session_plan() {
        let manifest = GameCompatManifest::parse(
            "title=Orbit Runner\nslug=orbit-runner\nexec=/games/orbit/run\ncwd=/games/orbit\narg=--fullscreen\ngfx.backend=vulkan\ngfx.profile=frame-pace\naudio.backend=native-mixer\naudio.profile=spatial-mix\ninput.backend=native-input\ninput.profile=gamepad-first\nshim.prefix=/compat/orbit\nshim.saves=/saves/orbit\nshim.cache=/cache/orbit\n",
        )
        .unwrap();
        let plan = manifest.session_plan();

        assert_eq!(manifest.title, "Orbit Runner");
        assert_eq!(plan.domain_name, "compat-game-orbit-runner");
        assert_eq!(plan.process_name, "game-orbit-runner");
        assert_eq!(plan.lanes[0].resource_name, "orbit-runner-gfx");
        assert_eq!(plan.lanes[1].contract_label, "spatial-mix-mix");
        assert_eq!(plan.env_shims[0].key, "NGOS_GAME_TITLE");
        assert_eq!(plan.argv, ["--fullscreen"]);
    }

    #[test]
    fn rejects_invalid_backends_and_relative_paths() {
        let error = GameCompatManifest::parse(
            "title=Bad\nslug=bad\nexec=games/bad\ncwd=/games/bad\ngfx.backend=gl\ngfx.profile=fast\naudio.backend=native-mixer\naudio.profile=a\ninput.backend=native-input\ninput.profile=i\nshim.prefix=/compat/bad\nshim.saves=/saves/bad\nshim.cache=/cache/bad\n",
        )
        .unwrap_err();
        assert!(error.describe().contains("gfx.backend"));
    }
}
