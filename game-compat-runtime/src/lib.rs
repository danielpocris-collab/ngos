#![cfg_attr(not(test), no_std)]

//! Canonical subsystem role:
//! - subsystem: game compatibility runtime
//! - owner layer: Layer 2 to Layer 3 support surface
//! - semantic owner: `game-compat-runtime`
//! - truth path role: canonical compat-session planning and translation support
//!   for game-focused user/runtime flows
//!
//! Canonical contract families defined here:
//! - compat launch profile contracts
//! - graphics/audio/input translation planning contracts
//! - compat session planning contracts
//!
//! This crate may define compat-session planning truth for this vertical, but
//! it remains subordinate to the core `ngos` kernel and runtime ownership
//! model.

extern crate alloc;

pub mod abi_proof;
pub mod device_proof;
pub mod loader_proof;

use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use ngos_user_abi::{NativeContractKind, NativeResourceKind};
use ngos_user_runtime::Runtime;

pub use abi_proof::{
    CompatAbiProcProbeExpectation, CompatAbiProcProbeMismatch, CompatAbiProcProbeSnapshot,
    CompatAbiProcessMismatch, CompatAbiScenarioManifest, CompatAbiScenarioSet, CompatAbiSmokeStage,
    compat_abi_boot_proof_line, compat_abi_cleanup_line, compat_abi_completion_line,
    compat_abi_expected_payload_markers, compat_abi_failure_line, compat_abi_proc_environ_line,
    compat_abi_proc_failure_line, compat_abi_proc_recovery_line, compat_abi_proc_refusal_line,
    compat_abi_proc_step_line, compat_abi_proc_success_line, compat_abi_process_cmdline_matches,
    compat_abi_process_failure_line, compat_abi_process_image_matches, compat_abi_process_line,
    compat_abi_required_dirs, compat_abi_route_line, compat_abi_scenario_manifests,
    compat_abi_smoke_scenarios, compat_abi_stage_line, compat_abi_verify_payload,
    compat_abi_verify_proc_probe, compat_abi_verify_process_record,
};
pub use device_proof::{
    build_compat_gfx_translated_payload, compat_audio_roundtrip, compat_input_roundtrip,
    encode_compat_audio_payload, encode_compat_input_payload, parse_driver_request_id,
    resolve_device_request_id, run_native_compat_audio_boot_smoke,
    run_native_compat_graphics_boot_smoke, run_native_compat_input_boot_smoke,
};
pub use loader_proof::{
    CompatLoaderArtifactMismatch, CompatLoaderArtifactSnapshot, CompatLoaderProofFlavor,
    CompatLoaderScenarioManifest, CompatLoaderSessionSnapshot, CompatLoaderSmokeScenarioSet,
    compat_foreign_boot_proof_line, compat_foreign_completion_line, compat_foreign_reclaim_line,
    compat_loader_artifact_failure_line, compat_loader_cleanup_line, compat_loader_completion_line,
    compat_loader_expected_env_markers, compat_loader_expected_loader_markers,
    compat_loader_foreign_recovery_line, compat_loader_foreign_success_line,
    compat_loader_matrix_line, compat_loader_plan_line, compat_loader_recovery_line,
    compat_loader_refusal_line, compat_loader_relaunch_stopped_line, compat_loader_required_dirs,
    compat_loader_scenario_manifests, compat_loader_session_snapshot, compat_loader_success_line,
    compat_loader_verify_artifacts, foreign_loader_smoke_scenarios, native_loader_smoke_scenarios,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsApi {
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
    Other,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatTargetKind {
    Game,
    App,
    Tool,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphicsConfig {
    pub source_api: GraphicsApi,
    pub backend: GraphicsBackend,
    pub profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphicsTranslationPlan {
    pub source_api: GraphicsApi,
    pub backend: GraphicsBackend,
    pub translation: &'static str,
    pub source_api_name: &'static str,
    pub backend_name: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoaderRoutingPlan {
    pub route_class: &'static str,
    pub launch_mode: &'static str,
    pub entry_profile: &'static str,
    pub bootstrap_profile: &'static str,
    pub entrypoint: &'static str,
    pub requires_compat_shims: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbiRoutingPlan {
    pub route_class: &'static str,
    pub handle_profile: &'static str,
    pub path_profile: &'static str,
    pub scheduler_profile: &'static str,
    pub sync_profile: &'static str,
    pub timer_profile: &'static str,
    pub module_profile: &'static str,
    pub event_profile: &'static str,
    pub requires_kernel_abi_shims: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DllOverrideMode {
    Native,
    Builtin,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DllOverrideRule {
    pub library: String,
    pub mode: DllOverrideMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameCompatManifest {
    pub target: CompatTargetKind,
    pub title: String,
    pub slug: String,
    pub executable_path: String,
    pub working_dir: String,
    pub argv: Vec<String>,
    pub graphics: GraphicsConfig,
    pub audio: AudioConfig,
    pub input: InputConfig,
    pub shims: RuntimeShimPaths,
    pub shim_preloads: Vec<String>,
    pub dll_overrides: Vec<DllOverrideRule>,
    pub env_overrides: Vec<(String, String)>,
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
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameSessionPlan {
    pub target: CompatTargetKind,
    pub domain_name: String,
    pub process_name: String,
    pub executable_path: String,
    pub working_dir: String,
    pub argv: Vec<String>,
    pub lanes: [CompatLanePlan; 3],
    pub env_shims: Vec<EnvShim>,
    pub shim_preloads: Vec<String>,
    pub dll_overrides: Vec<DllOverrideRule>,
    pub loader_routing: LoaderRoutingPlan,
    pub abi_routing: AbiRoutingPlan,
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

impl GraphicsApi {
    /// Map to the gfx-translate `SourceApi`.
    /// Returns `None` for `Other` — unsupported APIs are refused at the caller.
    pub fn to_source_api(self) -> Option<ngos_gfx_translate::SourceApi> {
        match self {
            Self::Direct3D9 => Some(ngos_gfx_translate::SourceApi::Direct3D9),
            Self::Direct3D10 => Some(ngos_gfx_translate::SourceApi::Direct3D10),
            Self::DirectX11 => Some(ngos_gfx_translate::SourceApi::DirectX11),
            Self::DirectX12 => Some(ngos_gfx_translate::SourceApi::DirectX12),
            Self::OpenGL => Some(ngos_gfx_translate::SourceApi::OpenGL),
            Self::OpenGLES => Some(ngos_gfx_translate::SourceApi::OpenGLES),
            Self::Metal => Some(ngos_gfx_translate::SourceApi::Metal),
            Self::Vulkan => Some(ngos_gfx_translate::SourceApi::Vulkan),
            Self::WebGPU => Some(ngos_gfx_translate::SourceApi::WebGPU),
            Self::Wgpu => Some(ngos_gfx_translate::SourceApi::Wgpu),
            Self::Other => None,
        }
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
        let mut target = None::<CompatTargetKind>;
        let mut executable_path = None::<String>;
        let mut working_dir = None::<String>;
        let mut argv = Vec::<String>::new();
        let mut gfx_source_api = None::<GraphicsApi>;
        let mut gfx_backend = None::<GraphicsBackend>;
        let mut gfx_profile = None::<String>;
        let mut audio_backend = None::<AudioBackend>;
        let mut audio_profile = None::<String>;
        let mut input_backend = None::<InputBackend>;
        let mut input_profile = None::<String>;
        let mut shim_prefix = None::<String>;
        let mut shim_saves = None::<String>;
        let mut shim_cache = None::<String>;
        let mut shim_preloads = Vec::<String>::new();
        let mut dll_overrides = Vec::<DllOverrideRule>::new();
        let mut env_overrides = Vec::<(String, String)>::new();

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
                "target" => target = Some(parse_compat_target_kind(value)?),
                "exec" => executable_path = Some(value.to_string()),
                "cwd" => working_dir = Some(value.to_string()),
                "arg" => argv.push(value.to_string()),
                "gfx.api" => gfx_source_api = Some(parse_graphics_api(value)?),
                "gfx.backend" => gfx_backend = Some(parse_graphics_backend(value)?),
                "gfx.profile" => gfx_profile = Some(value.to_string()),
                "audio.backend" => audio_backend = Some(parse_audio_backend(value)?),
                "audio.profile" => audio_profile = Some(value.to_string()),
                "input.backend" => input_backend = Some(parse_input_backend(value)?),
                "input.profile" => input_profile = Some(value.to_string()),
                "shim.prefix" => shim_prefix = Some(value.to_string()),
                "shim.saves" => shim_saves = Some(value.to_string()),
                "shim.cache" => shim_cache = Some(value.to_string()),
                "shim.preload" => shim_preloads.push(value.to_string()),
                "shim.dll" => dll_overrides.push(parse_dll_override(value)?),
                "env.override" => env_overrides.push(parse_env_override(value)?),
                _ => {
                    return Err(ManifestError::InvalidField {
                        key: key.to_string(),
                        value: value.to_string(),
                    });
                }
            }
        }

        let manifest = Self {
            target: target.unwrap_or(CompatTargetKind::Game),
            title: title.ok_or(ManifestError::MissingField("title"))?,
            slug: slug.ok_or(ManifestError::MissingField("slug"))?,
            executable_path: executable_path.ok_or(ManifestError::MissingField("exec"))?,
            working_dir: working_dir.ok_or(ManifestError::MissingField("cwd"))?,
            argv,
            graphics: GraphicsConfig {
                source_api: gfx_source_api.unwrap_or(GraphicsApi::Vulkan),
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
            shim_preloads,
            dll_overrides,
            env_overrides,
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
        for preload in &self.shim_preloads {
            if !preload.starts_with('/') {
                return Err(invalid_field("shim.preload", preload));
            }
        }
        for (key, value) in &self.env_overrides {
            if key.is_empty()
                || value.is_empty()
                || key
                    .bytes()
                    .any(|byte| byte.is_ascii_whitespace() || byte == b'=')
            {
                return Err(invalid_field("env.override", &format!("{key}={value}")));
            }
        }
        Ok(())
    }

    pub fn session_plan(&self) -> GameSessionPlan {
        let process_name = format!("compat-{}", self.slug);
        let loader_routing = self.loader_routing_plan();
        let abi_routing = self.abi_routing_plan();
        GameSessionPlan {
            target: self.target,
            domain_name: format!("compat-{}", compat_target_name(self.target)),
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
                    key: String::from("NGOS_COMPAT_TARGET"),
                    value: compat_target_name(self.target).to_string(),
                },
                EnvShim {
                    key: String::from("NGOS_GAME_TITLE"),
                    value: self.title.clone(),
                },
                EnvShim {
                    key: String::from("NGOS_GAME_SLUG"),
                    value: self.slug.clone(),
                },
                EnvShim {
                    key: String::from("NGOS_GFX_API"),
                    value: graphics_api_name(self.graphics.source_api).to_string(),
                },
                EnvShim {
                    key: String::from("NGOS_GFX_BACKEND"),
                    value: graphics_backend_name(self.graphics.backend).to_string(),
                },
                EnvShim {
                    key: String::from("NGOS_GFX_TRANSLATION"),
                    value: graphics_translation_name(
                        self.graphics.source_api,
                        self.graphics.backend,
                    )
                    .to_string(),
                },
                EnvShim {
                    key: String::from("NGOS_AUDIO_BACKEND"),
                    value: audio_backend_name(self.audio.backend).to_string(),
                },
                EnvShim {
                    key: String::from("NGOS_INPUT_BACKEND"),
                    value: input_backend_name(self.input.backend).to_string(),
                },
                EnvShim {
                    key: String::from("NGOS_COMPAT_PREFIX"),
                    value: self.shims.prefix.clone(),
                },
                EnvShim {
                    key: String::from("NGOS_COMPAT_SAVES"),
                    value: self.shims.saves.clone(),
                },
                EnvShim {
                    key: String::from("NGOS_COMPAT_CACHE"),
                    value: self.shims.cache.clone(),
                },
                EnvShim {
                    key: String::from("NGOS_COMPAT_PRELOADS"),
                    value: join_string_list(&self.shim_preloads),
                },
                EnvShim {
                    key: String::from("NGOS_COMPAT_DLL_OVERRIDES"),
                    value: join_dll_override_rules(&self.dll_overrides),
                },
                EnvShim {
                    key: String::from("NGOS_COMPAT_ROUTE_CLASS"),
                    value: loader_routing.route_class.to_string(),
                },
                EnvShim {
                    key: String::from("NGOS_COMPAT_LAUNCH_MODE"),
                    value: loader_routing.launch_mode.to_string(),
                },
                EnvShim {
                    key: String::from("NGOS_COMPAT_ENTRY_PROFILE"),
                    value: loader_routing.entry_profile.to_string(),
                },
                EnvShim {
                    key: String::from("NGOS_COMPAT_BOOTSTRAP_PROFILE"),
                    value: loader_routing.bootstrap_profile.to_string(),
                },
                EnvShim {
                    key: String::from("NGOS_COMPAT_ENTRYPOINT"),
                    value: loader_routing.entrypoint.to_string(),
                },
                EnvShim {
                    key: String::from("NGOS_COMPAT_REQUIRES_SHIMS"),
                    value: if loader_routing.requires_compat_shims {
                        String::from("1")
                    } else {
                        String::from("0")
                    },
                },
                EnvShim {
                    key: String::from("NGOS_COMPAT_ABI_ROUTE_CLASS"),
                    value: abi_routing.route_class.to_string(),
                },
                EnvShim {
                    key: String::from("NGOS_COMPAT_ABI_HANDLE_PROFILE"),
                    value: abi_routing.handle_profile.to_string(),
                },
                EnvShim {
                    key: String::from("NGOS_COMPAT_ABI_PATH_PROFILE"),
                    value: abi_routing.path_profile.to_string(),
                },
                EnvShim {
                    key: String::from("NGOS_COMPAT_ABI_SCHEDULER_PROFILE"),
                    value: abi_routing.scheduler_profile.to_string(),
                },
                EnvShim {
                    key: String::from("NGOS_COMPAT_ABI_SYNC_PROFILE"),
                    value: abi_routing.sync_profile.to_string(),
                },
                EnvShim {
                    key: String::from("NGOS_COMPAT_ABI_TIMER_PROFILE"),
                    value: abi_routing.timer_profile.to_string(),
                },
                EnvShim {
                    key: String::from("NGOS_COMPAT_ABI_MODULE_PROFILE"),
                    value: abi_routing.module_profile.to_string(),
                },
                EnvShim {
                    key: String::from("NGOS_COMPAT_ABI_EVENT_PROFILE"),
                    value: abi_routing.event_profile.to_string(),
                },
                EnvShim {
                    key: String::from("NGOS_COMPAT_ABI_REQUIRES_SHIMS"),
                    value: if abi_routing.requires_kernel_abi_shims {
                        String::from("1")
                    } else {
                        String::from("0")
                    },
                },
            ],
            shim_preloads: self.shim_preloads.clone(),
            dll_overrides: self.dll_overrides.clone(),
            loader_routing,
            abi_routing,
        }
        .with_env_overrides(&self.env_overrides)
    }

    pub fn graphics_translation_plan(&self) -> GraphicsTranslationPlan {
        GraphicsTranslationPlan {
            source_api: self.graphics.source_api,
            backend: self.graphics.backend,
            translation: graphics_translation_name(self.graphics.source_api, self.graphics.backend),
            source_api_name: graphics_api_name(self.graphics.source_api),
            backend_name: graphics_backend_name(self.graphics.backend),
        }
    }

    pub fn loader_routing_plan(&self) -> LoaderRoutingPlan {
        let translation = self.graphics_translation_plan();
        let launch_mode = if translation.translation == "native-vulkan" {
            "native-direct"
        } else {
            "compat-shim"
        };
        let route_class = loader_route_class(self.target, launch_mode);
        let entry_profile =
            loader_entry_profile_name(self.graphics.source_api, self.graphics.backend);
        let bootstrap_profile = loader_bootstrap_profile_name(
            self.shim_preloads.len(),
            self.dll_overrides.len(),
            self.env_overrides.len(),
        );
        let entrypoint = loader_entrypoint_name(self.target);
        let requires_compat_shims = translation.translation != "native-vulkan"
            || !self.shim_preloads.is_empty()
            || !self.dll_overrides.is_empty();

        LoaderRoutingPlan {
            route_class,
            launch_mode,
            entry_profile,
            bootstrap_profile,
            entrypoint,
            requires_compat_shims,
        }
    }

    pub fn abi_routing_plan(&self) -> AbiRoutingPlan {
        AbiRoutingPlan {
            route_class: abi_route_class(self.target),
            handle_profile: abi_handle_profile_name(self.target),
            path_profile: abi_path_profile_name(self.target),
            scheduler_profile: abi_scheduler_profile_name(self.target),
            sync_profile: abi_sync_profile_name(self.target),
            timer_profile: abi_timer_profile_name(self.target),
            module_profile: abi_module_profile_name(self.target),
            event_profile: abi_event_profile_name(self.target),
            requires_kernel_abi_shims: true,
        }
    }
}

/// Sintetizează decizia de routing selectată la lansare per aplicație.
/// Include profilele active și calea de traducere efectivă.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatLaunchProfile {
    pub target: CompatTargetKind,
    pub gfx_api: GraphicsApi,
    pub gfx_profile: String,
    pub gfx_backend: GraphicsBackend,
    pub gfx_translation: &'static str,
    pub audio_profile: String,
    pub input_profile: String,
    pub env_prefix: String,
    pub env_saves: String,
    pub env_cache: String,
    pub preload_count: usize,
    pub dll_override_count: usize,
    pub env_override_count: usize,
    pub preload_spec: String,
    pub dll_override_spec: String,
    pub route_class: &'static str,
    pub launch_mode: &'static str,
    pub entry_profile: &'static str,
    pub bootstrap_profile: &'static str,
    pub entrypoint: &'static str,
    pub requires_compat_shims: bool,
    pub abi_route_class: &'static str,
    pub abi_handle_profile: &'static str,
    pub abi_path_profile: &'static str,
    pub abi_scheduler_profile: &'static str,
    pub abi_sync_profile: &'static str,
    pub abi_timer_profile: &'static str,
    pub abi_module_profile: &'static str,
    pub abi_event_profile: &'static str,
    pub abi_requires_kernel_shims: bool,
    pub executable_path: String,
    pub working_dir: String,
    pub argc: usize,
}

impl CompatLaunchProfile {
    pub fn from_manifest(manifest: &GameCompatManifest) -> Self {
        let routing = manifest.loader_routing_plan();
        let abi_routing = manifest.abi_routing_plan();
        Self {
            target: manifest.target,
            gfx_api: manifest.graphics.source_api,
            gfx_profile: manifest.graphics.profile.clone(),
            gfx_backend: manifest.graphics.backend,
            gfx_translation: graphics_translation_name(
                manifest.graphics.source_api,
                manifest.graphics.backend,
            ),
            audio_profile: manifest.audio.profile.clone(),
            input_profile: manifest.input.profile.clone(),
            env_prefix: manifest.shims.prefix.clone(),
            env_saves: manifest.shims.saves.clone(),
            env_cache: manifest.shims.cache.clone(),
            preload_count: manifest.shim_preloads.len(),
            dll_override_count: manifest.dll_overrides.len(),
            env_override_count: manifest.env_overrides.len(),
            preload_spec: join_string_list(&manifest.shim_preloads),
            dll_override_spec: join_dll_override_rules(&manifest.dll_overrides),
            route_class: routing.route_class,
            launch_mode: routing.launch_mode,
            entry_profile: routing.entry_profile,
            bootstrap_profile: routing.bootstrap_profile,
            entrypoint: routing.entrypoint,
            requires_compat_shims: routing.requires_compat_shims,
            abi_route_class: abi_routing.route_class,
            abi_handle_profile: abi_routing.handle_profile,
            abi_path_profile: abi_routing.path_profile,
            abi_scheduler_profile: abi_routing.scheduler_profile,
            abi_sync_profile: abi_routing.sync_profile,
            abi_timer_profile: abi_routing.timer_profile,
            abi_module_profile: abi_routing.module_profile,
            abi_event_profile: abi_routing.event_profile,
            abi_requires_kernel_shims: abi_routing.requires_kernel_abi_shims,
            executable_path: manifest.executable_path.clone(),
            working_dir: manifest.working_dir.clone(),
            argc: manifest.argv.len(),
        }
    }

    pub fn describe(&self) -> String {
        format!(
            "target={} route={} mode={} entry={} bootstrap={} entrypoint={} requires-shims={} abi-route={} abi-handles={} abi-paths={} abi-scheduler={} abi-sync={} abi-timer={} abi-module={} abi-event={} abi-requires-shims={} gfx-api={} gfx-profile={} gfx-backend={} translation={} audio-profile={} input-profile={} exec={} cwd={} argc={} prefix={} preloads={} dll-overrides={} env-overrides={}",
            compat_target_name(self.target),
            self.route_class,
            self.launch_mode,
            self.entry_profile,
            self.bootstrap_profile,
            self.entrypoint,
            if self.requires_compat_shims { 1 } else { 0 },
            self.abi_route_class,
            self.abi_handle_profile,
            self.abi_path_profile,
            self.abi_scheduler_profile,
            self.abi_sync_profile,
            self.abi_timer_profile,
            self.abi_module_profile,
            self.abi_event_profile,
            if self.abi_requires_kernel_shims { 1 } else { 0 },
            graphics_api_name(self.gfx_api),
            self.gfx_profile,
            graphics_backend_name(self.gfx_backend),
            self.gfx_translation,
            self.audio_profile,
            self.input_profile,
            self.executable_path,
            self.working_dir,
            self.argc,
            self.env_prefix,
            self.preload_count,
            self.dll_override_count,
            self.env_override_count,
        )
    }
}

impl GameSessionPlan {
    fn with_env_overrides(mut self, env_overrides: &[(String, String)]) -> Self {
        for (key, value) in env_overrides {
            self.env_shims.push(EnvShim {
                key: key.clone(),
                value: value.clone(),
            });
        }
        self
    }
}

pub const fn compat_target_name(target: CompatTargetKind) -> &'static str {
    match target {
        CompatTargetKind::Game => "game",
        CompatTargetKind::App => "app",
        CompatTargetKind::Tool => "tool",
        CompatTargetKind::Other => "other",
    }
}

pub const fn graphics_api_name(api: GraphicsApi) -> &'static str {
    match api {
        GraphicsApi::Direct3D9 => "direct3d9",
        GraphicsApi::Direct3D10 => "direct3d10",
        GraphicsApi::DirectX11 => "directx11",
        GraphicsApi::DirectX12 => "directx12",
        GraphicsApi::OpenGL => "opengl",
        GraphicsApi::OpenGLES => "opengles",
        GraphicsApi::Metal => "metal",
        GraphicsApi::Vulkan => "vulkan",
        GraphicsApi::WebGPU => "webgpu",
        GraphicsApi::Wgpu => "wgpu",
        GraphicsApi::Other => "other",
    }
}

pub const fn graphics_backend_name(backend: GraphicsBackend) -> &'static str {
    match backend {
        GraphicsBackend::Vulkan => "vulkan",
    }
}

pub const fn graphics_translation_name(api: GraphicsApi, backend: GraphicsBackend) -> &'static str {
    match (api, backend) {
        (GraphicsApi::Vulkan, GraphicsBackend::Vulkan) => "native-vulkan",
        (GraphicsApi::DirectX11, GraphicsBackend::Vulkan)
        | (GraphicsApi::DirectX12, GraphicsBackend::Vulkan)
        | (GraphicsApi::Direct3D9, GraphicsBackend::Vulkan)
        | (GraphicsApi::Direct3D10, GraphicsBackend::Vulkan)
        | (GraphicsApi::OpenGL, GraphicsBackend::Vulkan)
        | (GraphicsApi::OpenGLES, GraphicsBackend::Vulkan)
        | (GraphicsApi::Metal, GraphicsBackend::Vulkan)
        | (GraphicsApi::WebGPU, GraphicsBackend::Vulkan)
        | (GraphicsApi::Wgpu, GraphicsBackend::Vulkan)
        | (GraphicsApi::Other, GraphicsBackend::Vulkan) => "compat-to-vulkan",
    }
}

pub const fn loader_entrypoint_name(target: CompatTargetKind) -> &'static str {
    match target {
        CompatTargetKind::Game => "/compat/bin/game-entry",
        CompatTargetKind::App => "/compat/bin/app-entry",
        CompatTargetKind::Tool => "/compat/bin/tool-entry",
        CompatTargetKind::Other => "/compat/bin/other-entry",
    }
}

pub fn loader_route_class(target: CompatTargetKind, launch_mode: &str) -> &'static str {
    match (target, launch_mode) {
        (CompatTargetKind::Game, "native-direct") => "native-game-runtime",
        (CompatTargetKind::App, "native-direct") => "native-app-runtime",
        (CompatTargetKind::Tool, "native-direct") => "native-tool-runtime",
        (CompatTargetKind::Other, "native-direct") => "native-other-runtime",
        (CompatTargetKind::Game, _) => "compat-game-runtime",
        (CompatTargetKind::App, _) => "compat-app-runtime",
        (CompatTargetKind::Tool, _) => "compat-tool-runtime",
        (CompatTargetKind::Other, _) => "compat-other-runtime",
    }
}

pub const fn loader_entry_profile_name(api: GraphicsApi, backend: GraphicsBackend) -> &'static str {
    match (api, backend) {
        (GraphicsApi::Vulkan, GraphicsBackend::Vulkan) => "native-vulkan-entry",
        (GraphicsApi::Direct3D9, GraphicsBackend::Vulkan)
        | (GraphicsApi::Direct3D10, GraphicsBackend::Vulkan)
        | (GraphicsApi::DirectX11, GraphicsBackend::Vulkan)
        | (GraphicsApi::DirectX12, GraphicsBackend::Vulkan) => "dx-to-vulkan-entry",
        (GraphicsApi::OpenGL, GraphicsBackend::Vulkan)
        | (GraphicsApi::OpenGLES, GraphicsBackend::Vulkan) => "gl-to-vulkan-entry",
        (GraphicsApi::Metal, GraphicsBackend::Vulkan) => "metal-to-vulkan-entry",
        (GraphicsApi::WebGPU, GraphicsBackend::Vulkan)
        | (GraphicsApi::Wgpu, GraphicsBackend::Vulkan) => "webgpu-to-vulkan-entry",
        (GraphicsApi::Other, GraphicsBackend::Vulkan) => "generic-compat-entry",
    }
}

pub const fn abi_route_class(target: CompatTargetKind) -> &'static str {
    match target {
        CompatTargetKind::Game => "compat-game-abi",
        CompatTargetKind::App => "compat-app-abi",
        CompatTargetKind::Tool => "compat-tool-abi",
        CompatTargetKind::Other => "compat-other-abi",
    }
}

pub const fn abi_handle_profile_name(target: CompatTargetKind) -> &'static str {
    match target {
        CompatTargetKind::Game => "win32-game-handles",
        CompatTargetKind::App => "win32-app-handles",
        CompatTargetKind::Tool => "utility-handles",
        CompatTargetKind::Other => "service-handles",
    }
}

pub const fn abi_path_profile_name(target: CompatTargetKind) -> &'static str {
    match target {
        CompatTargetKind::Game | CompatTargetKind::App | CompatTargetKind::Other => {
            "prefix-overlay-paths"
        }
        CompatTargetKind::Tool => "workspace-overlay-paths",
    }
}

pub const fn abi_scheduler_profile_name(target: CompatTargetKind) -> &'static str {
    match target {
        CompatTargetKind::Game => "latency-game-scheduler",
        CompatTargetKind::App => "interactive-app-scheduler",
        CompatTargetKind::Tool => "batch-tool-scheduler",
        CompatTargetKind::Other => "background-service-scheduler",
    }
}

pub const fn abi_sync_profile_name(target: CompatTargetKind) -> &'static str {
    match target {
        CompatTargetKind::Game => "event-heavy-sync",
        CompatTargetKind::App => "desktop-sync",
        CompatTargetKind::Tool => "utility-sync",
        CompatTargetKind::Other => "service-sync",
    }
}

pub const fn abi_timer_profile_name(target: CompatTargetKind) -> &'static str {
    match target {
        CompatTargetKind::Game => "frame-budget-timers",
        CompatTargetKind::App => "app-deadline-timers",
        CompatTargetKind::Tool | CompatTargetKind::Other => "service-poll-timers",
    }
}

pub const fn abi_module_profile_name(target: CompatTargetKind) -> &'static str {
    match target {
        CompatTargetKind::Game => "game-module-registry",
        CompatTargetKind::App => "app-module-registry",
        CompatTargetKind::Tool => "tool-module-registry",
        CompatTargetKind::Other => "service-module-registry",
    }
}

pub const fn abi_event_profile_name(target: CompatTargetKind) -> &'static str {
    match target {
        CompatTargetKind::Game => "game-window-events",
        CompatTargetKind::App => "desktop-window-events",
        CompatTargetKind::Tool => "tool-event-stream",
        CompatTargetKind::Other => "service-event-stream",
    }
}

pub const fn loader_bootstrap_profile_name(
    preload_count: usize,
    dll_override_count: usize,
    env_override_count: usize,
) -> &'static str {
    if preload_count > 0 || dll_override_count > 0 {
        "shim-heavy"
    } else if env_override_count > 0 {
        "env-overlay"
    } else {
        "bootstrap-light"
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

fn parse_compat_target_kind(value: &str) -> Result<CompatTargetKind, ManifestError> {
    match value {
        "game" => Ok(CompatTargetKind::Game),
        "app" => Ok(CompatTargetKind::App),
        "tool" => Ok(CompatTargetKind::Tool),
        "other" => Ok(CompatTargetKind::Other),
        _ => Err(invalid_field("target", value)),
    }
}

fn parse_graphics_api(value: &str) -> Result<GraphicsApi, ManifestError> {
    match value {
        "direct3d9" | "d3d9" | "dx9" => Ok(GraphicsApi::Direct3D9),
        "direct3d10" | "d3d10" | "dx10" => Ok(GraphicsApi::Direct3D10),
        "directx11" | "dx11" => Ok(GraphicsApi::DirectX11),
        "directx12" | "dx12" => Ok(GraphicsApi::DirectX12),
        "opengl" | "gl" => Ok(GraphicsApi::OpenGL),
        "opengles" | "gles" => Ok(GraphicsApi::OpenGLES),
        "metal" => Ok(GraphicsApi::Metal),
        "vulkan" => Ok(GraphicsApi::Vulkan),
        "webgpu" => Ok(GraphicsApi::WebGPU),
        "wgpu" => Ok(GraphicsApi::Wgpu),
        "other" => Ok(GraphicsApi::Other),
        _ => Err(invalid_field("gfx.api", value)),
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

fn parse_dll_override(value: &str) -> Result<DllOverrideRule, ManifestError> {
    let Some((library, mode)) = value.split_once('=') else {
        return Err(invalid_field("shim.dll", value));
    };
    if library.is_empty() {
        return Err(invalid_field("shim.dll", value));
    }
    let mode = match mode {
        "native" => DllOverrideMode::Native,
        "builtin" => DllOverrideMode::Builtin,
        "disabled" => DllOverrideMode::Disabled,
        _ => return Err(invalid_field("shim.dll", value)),
    };
    Ok(DllOverrideRule {
        library: library.to_string(),
        mode,
    })
}

fn parse_env_override(value: &str) -> Result<(String, String), ManifestError> {
    let Some((key, env_value)) = value.split_once('=') else {
        return Err(invalid_field("env.override", value));
    };
    if key.is_empty() || env_value.is_empty() {
        return Err(invalid_field("env.override", value));
    }
    Ok((key.to_string(), env_value.to_string()))
}

fn join_string_list(values: &[String]) -> String {
    if values.is_empty() {
        String::from("-")
    } else {
        values.join(";")
    }
}

fn join_dll_override_rules(values: &[DllOverrideRule]) -> String {
    if values.is_empty() {
        String::from("-")
    } else {
        values
            .iter()
            .map(|rule| format!("{}={}", rule.library, dll_override_mode_name(rule.mode)))
            .collect::<Vec<_>>()
            .join(";")
    }
}

pub const fn dll_override_mode_name(mode: DllOverrideMode) -> &'static str {
    match mode {
        DllOverrideMode::Native => "native",
        DllOverrideMode::Builtin => "builtin",
        DllOverrideMode::Disabled => "disabled",
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
            "target=game\ntitle=Orbit Runner\nslug=orbit-runner\nexec=/games/orbit/run\ncwd=/games/orbit\narg=--fullscreen\ngfx.api=directx12\ngfx.backend=vulkan\ngfx.profile=frame-pace\naudio.backend=native-mixer\naudio.profile=spatial-mix\ninput.backend=native-input\ninput.profile=gamepad-first\nshim.prefix=/compat/orbit\nshim.saves=/saves/orbit\nshim.cache=/cache/orbit\nshim.preload=/compat/orbit/preload/d3d12.ngm\nshim.dll=d3d12=builtin\nenv.override=DXVK_HUD=1\n",
        )
        .unwrap();
        let plan = manifest.session_plan();

        assert_eq!(manifest.target, CompatTargetKind::Game);
        assert_eq!(manifest.title, "Orbit Runner");
        assert_eq!(plan.domain_name, "compat-game");
        assert_eq!(plan.process_name, "compat-orbit-runner");
        assert_eq!(plan.lanes[0].resource_name, "orbit-runner-gfx");
        assert_eq!(plan.lanes[1].contract_label, "spatial-mix-mix");
        assert_eq!(graphics_api_name(manifest.graphics.source_api), "directx12");
        assert_eq!(graphics_backend_name(manifest.graphics.backend), "vulkan");
        assert_eq!(
            graphics_translation_name(manifest.graphics.source_api, manifest.graphics.backend),
            "compat-to-vulkan"
        );
        assert_eq!(plan.env_shims[0].key, "NGOS_COMPAT_TARGET");
        assert_eq!(plan.env_shims[1].key, "NGOS_GAME_TITLE");
        assert_eq!(manifest.shim_preloads, ["/compat/orbit/preload/d3d12.ngm"]);
        assert_eq!(manifest.dll_overrides[0].library, "d3d12");
        assert_eq!(
            dll_override_mode_name(manifest.dll_overrides[0].mode),
            "builtin"
        );
        assert!(
            plan.env_shims
                .iter()
                .any(|shim| shim.key == "DXVK_HUD" && shim.value == "1")
        );
        assert_eq!(plan.argv, ["--fullscreen"]);
    }

    #[test]
    fn rejects_invalid_backends_and_relative_paths() {
        let error = GameCompatManifest::parse(
            "target=game\ntitle=Bad\nslug=bad\nexec=games/bad\ncwd=/games/bad\ngfx.api=gl\ngfx.backend=gl\ngfx.profile=fast\naudio.backend=native-mixer\naudio.profile=a\ninput.backend=native-input\ninput.profile=i\nshim.prefix=/compat/bad\nshim.saves=/saves/bad\nshim.cache=/cache/bad\n",
        )
        .unwrap_err();
        assert!(error.describe().contains("gfx.api") || error.describe().contains("gfx.backend"));
    }

    #[test]
    fn parses_directx_opengl_and_metal_source_apis() {
        let directx = GameCompatManifest::parse(
            "target=app\ntitle=Dx Game\nslug=dx-game\nexec=/games/dx/run\ncwd=/games/dx\ngfx.api=directx12\ngfx.backend=vulkan\ngfx.profile=compat\naudio.backend=native-mixer\naudio.profile=mx\ninput.backend=native-input\ninput.profile=kbm\nshim.prefix=/compat/dx\nshim.saves=/saves/dx\nshim.cache=/cache/dx\n",
        )
        .unwrap();
        let opengl = GameCompatManifest::parse(
            "target=tool\ntitle=Gl Game\nslug=gl-game\nexec=/games/gl/run\ncwd=/games/gl\ngfx.api=opengl\ngfx.backend=vulkan\ngfx.profile=compat\naudio.backend=native-mixer\naudio.profile=mx\ninput.backend=native-input\ninput.profile=kbm\nshim.prefix=/compat/gl\nshim.saves=/saves/gl\nshim.cache=/cache/gl\n",
        )
        .unwrap();
        let metal = GameCompatManifest::parse(
            "target=other\ntitle=Metal Game\nslug=metal-game\nexec=/games/metal/run\ncwd=/games/metal\ngfx.api=metal\ngfx.backend=vulkan\ngfx.profile=compat\naudio.backend=native-mixer\naudio.profile=mx\ninput.backend=native-input\ninput.profile=kbm\nshim.prefix=/compat/metal\nshim.saves=/saves/metal\nshim.cache=/cache/metal\n",
        )
        .unwrap();

        assert_eq!(graphics_api_name(directx.graphics.source_api), "directx12");
        assert_eq!(graphics_api_name(opengl.graphics.source_api), "opengl");
        assert_eq!(graphics_api_name(metal.graphics.source_api), "metal");
        assert_eq!(
            graphics_translation_name(directx.graphics.source_api, directx.graphics.backend),
            "compat-to-vulkan"
        );
        assert_eq!(
            graphics_translation_name(opengl.graphics.source_api, opengl.graphics.backend),
            "compat-to-vulkan"
        );
        assert_eq!(
            graphics_translation_name(metal.graphics.source_api, metal.graphics.backend),
            "compat-to-vulkan"
        );
        assert_eq!(
            directx.graphics_translation_plan().translation,
            "compat-to-vulkan"
        );
        assert_eq!(opengl.graphics_translation_plan().source_api_name, "opengl");
        assert_eq!(metal.graphics_translation_plan().backend_name, "vulkan");
    }

    #[test]
    fn parses_more_graphics_api_families() {
        for (value, expected) in [
            ("dx9", GraphicsApi::Direct3D9),
            ("d3d10", GraphicsApi::Direct3D10),
            ("gles", GraphicsApi::OpenGLES),
            ("webgpu", GraphicsApi::WebGPU),
            ("wgpu", GraphicsApi::Wgpu),
            ("other", GraphicsApi::Other),
        ] {
            let manifest = GameCompatManifest::parse(&format!(
                "title=X\nslug=x\nexec=/games/x/run\ncwd=/games/x\ngfx.api={value}\ngfx.backend=vulkan\ngfx.profile=compat\naudio.backend=native-mixer\naudio.profile=mx\ninput.backend=native-input\ninput.profile=kbm\nshim.prefix=/compat/x\nshim.saves=/saves/x\nshim.cache=/cache/x\n"
            ))
            .unwrap();
            assert_eq!(manifest.graphics.source_api, expected);
        }
    }

    #[test]
    fn parses_loader_override_fields_and_describes_profile() {
        let manifest = GameCompatManifest::parse(
            "title=Nova\nslug=nova\nexec=/games/nova/run\ncwd=/games/nova\ngfx.api=directx11\ngfx.backend=vulkan\ngfx.profile=latency\naudio.backend=native-mixer\naudio.profile=stereo\ninput.backend=native-input\ninput.profile=kbm\nshim.prefix=/compat/nova\nshim.saves=/saves/nova\nshim.cache=/cache/nova\nshim.preload=/compat/nova/preload/d3d11.ngm\nshim.preload=/compat/nova/preload/xaudio2.ngm\nshim.dll=d3d11=builtin\nshim.dll=xaudio2=native\nenv.override=DXVK_HUD=1\nenv.override=WINEDEBUG=-all\n",
        )
        .unwrap();
        let profile = CompatLaunchProfile::from_manifest(&manifest);
        let described = profile.describe();

        assert_eq!(profile.preload_count, 2);
        assert_eq!(profile.dll_override_count, 2);
        assert_eq!(profile.env_override_count, 2);
        assert_eq!(
            profile.preload_spec,
            "/compat/nova/preload/d3d11.ngm;/compat/nova/preload/xaudio2.ngm"
        );
        assert_eq!(profile.dll_override_spec, "d3d11=builtin;xaudio2=native");
        assert_eq!(profile.route_class, "compat-game-runtime");
        assert_eq!(profile.launch_mode, "compat-shim");
        assert_eq!(profile.entry_profile, "dx-to-vulkan-entry");
        assert_eq!(profile.bootstrap_profile, "shim-heavy");
        assert_eq!(profile.entrypoint, "/compat/bin/game-entry");
        assert!(profile.requires_compat_shims);
        assert_eq!(profile.abi_route_class, "compat-game-abi");
        assert_eq!(profile.abi_handle_profile, "win32-game-handles");
        assert_eq!(profile.abi_path_profile, "prefix-overlay-paths");
        assert_eq!(profile.abi_scheduler_profile, "latency-game-scheduler");
        assert_eq!(profile.abi_sync_profile, "event-heavy-sync");
        assert_eq!(profile.abi_timer_profile, "frame-budget-timers");
        assert_eq!(profile.abi_module_profile, "game-module-registry");
        assert_eq!(profile.abi_event_profile, "game-window-events");
        assert!(profile.abi_requires_kernel_shims);
        assert!(described.contains("preloads=2"));
        assert!(described.contains("dll-overrides=2"));
        assert!(described.contains("env-overrides=2"));
        assert!(described.contains("route=compat-game-runtime"));
        assert!(described.contains("mode=compat-shim"));
        assert!(described.contains("entry=dx-to-vulkan-entry"));
        assert!(described.contains("bootstrap=shim-heavy"));
        assert!(described.contains("abi-route=compat-game-abi"));
        assert!(described.contains("abi-scheduler=latency-game-scheduler"));
    }

    #[test]
    fn derives_native_loader_route_for_vulkan_app_without_shims() {
        let manifest = GameCompatManifest::parse(
            "target=app\ntitle=Nova Native\nslug=nova-native\nexec=/games/nova/run\ncwd=/games/nova\ngfx.api=vulkan\ngfx.backend=vulkan\ngfx.profile=native\naudio.backend=native-mixer\naudio.profile=stereo\ninput.backend=native-input\ninput.profile=kbm\nshim.prefix=/compat/nova\nshim.saves=/saves/nova\nshim.cache=/cache/nova\n",
        )
        .unwrap();
        let routing = manifest.loader_routing_plan();

        assert_eq!(routing.route_class, "native-app-runtime");
        assert_eq!(routing.launch_mode, "native-direct");
        assert_eq!(routing.entry_profile, "native-vulkan-entry");
        assert_eq!(routing.bootstrap_profile, "bootstrap-light");
        assert_eq!(routing.entrypoint, "/compat/bin/app-entry");
        assert!(!routing.requires_compat_shims);
        let abi_routing = manifest.abi_routing_plan();
        assert_eq!(abi_routing.route_class, "compat-app-abi");
        assert_eq!(abi_routing.handle_profile, "win32-app-handles");
        assert_eq!(abi_routing.path_profile, "prefix-overlay-paths");
        assert_eq!(abi_routing.scheduler_profile, "interactive-app-scheduler");
        assert_eq!(abi_routing.sync_profile, "desktop-sync");
        assert_eq!(abi_routing.timer_profile, "app-deadline-timers");
        assert_eq!(abi_routing.module_profile, "app-module-registry");
        assert_eq!(abi_routing.event_profile, "desktop-window-events");
        assert!(abi_routing.requires_kernel_abi_shims);
    }

    #[test]
    fn derives_target_specific_abi_routes_for_tool_and_other() {
        let tool = GameCompatManifest::parse(
            "target=tool\ntitle=Nova Tool\nslug=nova-tool\nexec=/games/nova/run\ncwd=/games/nova\ngfx.api=webgpu\ngfx.backend=vulkan\ngfx.profile=tool\naudio.backend=native-mixer\naudio.profile=mono\ninput.backend=native-input\ninput.profile=kbm\nshim.prefix=/compat/nova-tool\nshim.saves=/saves/nova-tool\nshim.cache=/cache/nova-tool\n",
        )
        .unwrap();
        let other = GameCompatManifest::parse(
            "target=other\ntitle=Nova Service\nslug=nova-service\nexec=/games/nova/run\ncwd=/games/nova\ngfx.api=vulkan\ngfx.backend=vulkan\ngfx.profile=service\naudio.backend=native-mixer\naudio.profile=mono\ninput.backend=native-input\ninput.profile=kbm\nshim.prefix=/compat/nova-service\nshim.saves=/saves/nova-service\nshim.cache=/cache/nova-service\n",
        )
        .unwrap();

        let tool_abi = tool.abi_routing_plan();
        let other_abi = other.abi_routing_plan();

        assert_eq!(tool_abi.route_class, "compat-tool-abi");
        assert_eq!(tool_abi.handle_profile, "utility-handles");
        assert_eq!(tool_abi.path_profile, "workspace-overlay-paths");
        assert_eq!(tool_abi.scheduler_profile, "batch-tool-scheduler");
        assert_eq!(tool_abi.sync_profile, "utility-sync");
        assert_eq!(tool_abi.timer_profile, "service-poll-timers");
        assert_eq!(tool_abi.module_profile, "tool-module-registry");
        assert_eq!(tool_abi.event_profile, "tool-event-stream");

        assert_eq!(other_abi.route_class, "compat-other-abi");
        assert_eq!(other_abi.handle_profile, "service-handles");
        assert_eq!(other_abi.path_profile, "prefix-overlay-paths");
        assert_eq!(other_abi.scheduler_profile, "background-service-scheduler");
        assert_eq!(other_abi.sync_profile, "service-sync");
        assert_eq!(other_abi.timer_profile, "service-poll-timers");
        assert_eq!(other_abi.module_profile, "service-module-registry");
        assert_eq!(other_abi.event_profile, "service-event-stream");
    }

    #[test]
    fn rejects_invalid_loader_override_fields() {
        let dll_error = GameCompatManifest::parse(
            "title=Bad\nslug=bad\nexec=/games/bad/run\ncwd=/games/bad\ngfx.backend=vulkan\ngfx.profile=compat\naudio.backend=native-mixer\naudio.profile=mx\ninput.backend=native-input\ninput.profile=kbm\nshim.prefix=/compat/bad\nshim.saves=/saves/bad\nshim.cache=/cache/bad\nshim.dll=d3d11=forbidden\n",
        )
        .unwrap_err();
        assert!(dll_error.describe().contains("shim.dll"));

        let preload_error = GameCompatManifest::parse(
            "title=Bad\nslug=bad\nexec=/games/bad/run\ncwd=/games/bad\ngfx.backend=vulkan\ngfx.profile=compat\naudio.backend=native-mixer\naudio.profile=mx\ninput.backend=native-input\ninput.profile=kbm\nshim.prefix=/compat/bad\nshim.saves=/saves/bad\nshim.cache=/cache/bad\nshim.preload=compat/bad/preload.ngm\n",
        )
        .unwrap_err();
        assert!(preload_error.describe().contains("shim.preload"));
    }
}
