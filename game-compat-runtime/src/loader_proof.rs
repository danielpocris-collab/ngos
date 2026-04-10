use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use crate::{
    CompatLaunchProfile, CompatTargetKind, GameCompatManifest, GraphicsApi, compat_target_name,
    graphics_api_name,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatLoaderProofFlavor {
    Native,
    Foreign,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatLoaderScenarioManifest {
    pub path: &'static str,
    pub text: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatLoaderSmokeScenarioSet {
    pub valid: CompatLoaderScenarioManifest,
    pub invalid: CompatLoaderScenarioManifest,
    pub recovery: CompatLoaderScenarioManifest,
    pub tool: Option<CompatLoaderScenarioManifest>,
    pub other: Option<CompatLoaderScenarioManifest>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatLoaderSessionSnapshot {
    pub pid: u64,
    pub target: CompatTargetKind,
    pub slug: String,
    pub graphics_api: GraphicsApi,
    pub translation: String,
    pub route_class: String,
    pub launch_mode: String,
    pub entry_profile: String,
    pub bootstrap_profile: String,
    pub entrypoint: String,
    pub requires_compat_shims: bool,
    pub working_dir: String,
    pub executable_path: String,
    pub prefix_path: String,
    pub preload_spec: String,
    pub dll_override_spec: String,
    pub env_override_spec: String,
    pub preload_count: usize,
    pub dll_override_count: usize,
    pub env_override_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatLoaderArtifactSnapshot {
    pub env_payload: Vec<u8>,
    pub loader_payload: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatLoaderArtifactMismatch {
    EnvPayload,
    LoaderPayload,
}

fn joined_or_dash(values: &[String]) -> String {
    if values.is_empty() {
        String::from("-")
    } else {
        values.join(";")
    }
}

pub fn native_loader_smoke_scenarios() -> CompatLoaderSmokeScenarioSet {
    CompatLoaderSmokeScenarioSet {
        valid: CompatLoaderScenarioManifest {
            path: "/games/nova.manifest",
            text: "target=game\ntitle=Nova Strike\nslug=nova-strike\nexec=/bin/worker\ncwd=/games/nova\narg=--fullscreen\ngfx.api=directx11\ngfx.backend=vulkan\ngfx.profile=latency-opt\naudio.backend=native-mixer\naudio.profile=stereo-hifi\ninput.backend=native-input\ninput.profile=kbm-first\nshim.prefix=/compat/nova\nshim.saves=/saves/nova\nshim.cache=/cache/nova\nshim.preload=/compat/nova/preload/d3d11.ngm\nshim.preload=/compat/nova/preload/xaudio2.ngm\nshim.dll=d3d11=builtin\nshim.dll=xaudio2=native\nenv.override=DXVK_HUD=1\nenv.override=WINEDEBUG=-all\n",
        },
        invalid: CompatLoaderScenarioManifest {
            path: "/games/bad.manifest",
            text: "title=Bad Loader\nslug=bad-loader\nexec=/bin/worker\ncwd=/games\ngfx.backend=vulkan\ngfx.profile=compat\naudio.backend=native-mixer\naudio.profile=mono\ninput.backend=native-input\ninput.profile=kbm\nshim.prefix=/compat/bad\nshim.saves=/saves/bad\nshim.cache=/cache/bad\nshim.dll=d3d12=forbidden\n",
        },
        recovery: CompatLoaderScenarioManifest {
            path: "/games/nova-recovery.manifest",
            text: "target=app\ntitle=Nova Strike Recovery\nslug=nova-strike\nexec=/bin/worker\ncwd=/games/nova\narg=--safe-mode\ngfx.api=vulkan\ngfx.backend=vulkan\ngfx.profile=recovery-lowlat\naudio.backend=native-mixer\naudio.profile=stereo-safe\ninput.backend=native-input\ninput.profile=kbm-safe\nshim.prefix=/compat/nova\nshim.saves=/saves/nova\nshim.cache=/cache/nova\nenv.override=NGOS_COMPAT_RECOVERY=1\n",
        },
        tool: Some(CompatLoaderScenarioManifest {
            path: "/games/nova-tool.manifest",
            text: "target=tool\ntitle=Nova Tooling\nslug=nova-tool\nexec=/bin/worker\ncwd=/games/nova\narg=--diagnostics\ngfx.api=webgpu\ngfx.backend=vulkan\ngfx.profile=tool-inspect\naudio.backend=native-mixer\naudio.profile=mono-safe\ninput.backend=native-input\ninput.profile=kbm-tool\nshim.prefix=/compat/nova-tool\nshim.saves=/saves/nova-tool\nshim.cache=/cache/nova-tool\n",
        }),
        other: Some(CompatLoaderScenarioManifest {
            path: "/games/nova-other.manifest",
            text: "target=other\ntitle=Nova Service\nslug=nova-service\nexec=/bin/worker\ncwd=/games/nova\narg=--background\ngfx.api=vulkan\ngfx.backend=vulkan\ngfx.profile=service-overlay\naudio.backend=native-mixer\naudio.profile=mono-service\ninput.backend=native-input\ninput.profile=kbm-service\nshim.prefix=/compat/nova-service\nshim.saves=/saves/nova-service\nshim.cache=/cache/nova-service\nshim.preload=/compat/nova-service/preload/telemetry.ngm\nshim.dll=telemetry=builtin\n",
        }),
    }
}

pub fn foreign_loader_smoke_scenarios() -> CompatLoaderSmokeScenarioSet {
    CompatLoaderSmokeScenarioSet {
        valid: CompatLoaderScenarioManifest {
            path: "/games/nova.manifest",
            text: "target=game\ntitle=Nova Strike\nslug=nova-strike\nexec=/bin/worker\ncwd=/games/nova\narg=--fullscreen\ngfx.api=directx11\ngfx.backend=vulkan\ngfx.profile=latency-opt\naudio.backend=native-mixer\naudio.profile=stereo-hifi\ninput.backend=native-input\ninput.profile=kbm-first\nshim.prefix=/compat/nova\nshim.saves=/saves/nova\nshim.cache=/cache/nova\nshim.preload=/compat/nova/preload/d3d11.ngm\nshim.dll=d3d11=builtin\nenv.override=DXVK_HUD=1\n",
        },
        invalid: CompatLoaderScenarioManifest {
            path: "/games/bad.manifest",
            text: "title=Bad Loader\nslug=bad-loader\nexec=/bin/worker\ncwd=/games\ngfx.backend=vulkan\ngfx.profile=compat\naudio.backend=native-mixer\naudio.profile=mono\ninput.backend=native-input\ninput.profile=kbm\nshim.prefix=/compat/bad\nshim.saves=/saves/bad\nshim.cache=/cache/bad\nshim.dll=d3d12=forbidden\n",
        },
        recovery: CompatLoaderScenarioManifest {
            path: "/games/nova-recovery.manifest",
            text: "target=app\ntitle=Nova Strike Recovery\nslug=nova-strike\nexec=/bin/worker\ncwd=/games/nova\narg=--safe-mode\ngfx.api=vulkan\ngfx.backend=vulkan\ngfx.profile=recovery-lowlat\naudio.backend=native-mixer\naudio.profile=stereo-safe\ninput.backend=native-input\ninput.profile=kbm-safe\nshim.prefix=/compat/nova\nshim.saves=/saves/nova\nshim.cache=/cache/nova\nenv.override=NGOS_COMPAT_RECOVERY=1\n",
        },
        tool: None,
        other: None,
    }
}

pub fn compat_loader_scenario_manifests(
    scenarios: &CompatLoaderSmokeScenarioSet,
) -> Vec<&CompatLoaderScenarioManifest> {
    let mut manifests = vec![&scenarios.valid, &scenarios.invalid, &scenarios.recovery];
    if let Some(tool) = scenarios.tool.as_ref() {
        manifests.push(tool);
    }
    if let Some(other) = scenarios.other.as_ref() {
        manifests.push(other);
    }
    manifests
}

pub fn compat_loader_required_dirs() -> [&'static str; 2] {
    ["/games", "/games/nova"]
}

pub fn compat_loader_session_snapshot(
    pid: u64,
    manifest: &GameCompatManifest,
    preload_values: &[String],
    dll_override_values: &[String],
    env_override_values: &[String],
) -> CompatLoaderSessionSnapshot {
    let routing = manifest.loader_routing_plan();
    let translation = manifest.graphics_translation_plan();
    CompatLoaderSessionSnapshot {
        pid,
        target: manifest.target,
        slug: manifest.slug.clone(),
        graphics_api: manifest.graphics.source_api,
        translation: translation.translation.to_string(),
        route_class: routing.route_class.to_string(),
        launch_mode: routing.launch_mode.to_string(),
        entry_profile: routing.entry_profile.to_string(),
        bootstrap_profile: routing.bootstrap_profile.to_string(),
        entrypoint: routing.entrypoint.to_string(),
        requires_compat_shims: routing.requires_compat_shims,
        working_dir: manifest.working_dir.clone(),
        executable_path: manifest.executable_path.clone(),
        prefix_path: manifest.shims.prefix.clone(),
        preload_spec: joined_or_dash(preload_values),
        dll_override_spec: joined_or_dash(dll_override_values),
        env_override_spec: joined_or_dash(env_override_values),
        preload_count: preload_values.len(),
        dll_override_count: dll_override_values.len(),
        env_override_count: env_override_values.len(),
    }
}

pub fn compat_loader_plan_line(prefix: &str, manifest: &GameCompatManifest) -> String {
    let profile = CompatLaunchProfile::from_manifest(manifest);
    let graphics_plan = manifest.graphics_translation_plan();
    format!(
        "{prefix}.plan slug={} api={} backend={} translation={} {}",
        manifest.slug,
        graphics_api_name(manifest.graphics.source_api),
        graphics_plan.backend_name,
        graphics_plan.translation,
        profile.describe()
    )
}

pub fn compat_loader_expected_env_markers(manifest: &GameCompatManifest) -> Vec<String> {
    let routing = manifest.loader_routing_plan();
    let translation = manifest.graphics_translation_plan();
    let mut markers = vec![
        format!("NGOS_COMPAT_TARGET={}", compat_target_name(manifest.target)),
        format!(
            "NGOS_GFX_API={}",
            graphics_api_name(manifest.graphics.source_api)
        ),
        format!("NGOS_GFX_TRANSLATION={}", translation.translation),
        format!("NGOS_COMPAT_ROUTE_CLASS={}", routing.route_class),
        format!("NGOS_COMPAT_LAUNCH_MODE={}", routing.launch_mode),
        format!("NGOS_COMPAT_ENTRY_PROFILE={}", routing.entry_profile),
        format!(
            "NGOS_COMPAT_BOOTSTRAP_PROFILE={}",
            routing.bootstrap_profile
        ),
        format!("NGOS_COMPAT_ENTRYPOINT={}", routing.entrypoint),
        format!(
            "NGOS_COMPAT_REQUIRES_SHIMS={}",
            if routing.requires_compat_shims { 1 } else { 0 }
        ),
    ];
    if !manifest.shim_preloads.is_empty() {
        markers.push(format!(
            "NGOS_COMPAT_PRELOADS={}",
            manifest.shim_preloads.join(";")
        ));
    }
    if !manifest.dll_overrides.is_empty() {
        markers.push(format!(
            "NGOS_COMPAT_DLL_OVERRIDES={}",
            manifest
                .dll_overrides
                .iter()
                .map(|rule| format!(
                    "{}={}",
                    rule.library,
                    crate::dll_override_mode_name(rule.mode)
                ))
                .collect::<Vec<_>>()
                .join(";")
        ));
    }
    for (key, value) in &manifest.env_overrides {
        markers.push(format!("{key}={value}"));
    }
    markers
}

pub fn compat_loader_expected_loader_markers(manifest: &GameCompatManifest) -> Vec<String> {
    let routing = manifest.loader_routing_plan();
    let translation = manifest.graphics_translation_plan();
    let mut markers = vec![
        format!("target={}", compat_target_name(manifest.target)),
        format!("gfx-translation={}", translation.translation),
        format!("route-class={}", routing.route_class),
        format!("launch-mode={}", routing.launch_mode),
        format!("entry-profile={}", routing.entry_profile),
        format!("bootstrap-profile={}", routing.bootstrap_profile),
        format!("entrypoint={}", routing.entrypoint),
        format!(
            "requires-compat-shims={}",
            if routing.requires_compat_shims { 1 } else { 0 }
        ),
    ];
    if !manifest.shim_preloads.is_empty() {
        markers.push(format!("preloads={}", manifest.shim_preloads.join(";")));
    }
    if !manifest.dll_overrides.is_empty() {
        markers.push(format!(
            "dll-overrides={}",
            manifest
                .dll_overrides
                .iter()
                .map(|rule| format!(
                    "{}={}",
                    rule.library,
                    crate::dll_override_mode_name(rule.mode)
                ))
                .collect::<Vec<_>>()
                .join(";")
        ));
    }
    markers
}

pub fn compat_loader_verify_artifacts(
    manifest: &GameCompatManifest,
    snapshot: &CompatLoaderArtifactSnapshot,
) -> Result<(), CompatLoaderArtifactMismatch> {
    let env_markers = compat_loader_expected_env_markers(manifest);
    if !payload_contains_all_strings(&snapshot.env_payload, &env_markers) {
        return Err(CompatLoaderArtifactMismatch::EnvPayload);
    }
    let loader_markers = compat_loader_expected_loader_markers(manifest);
    if !payload_contains_all_strings(&snapshot.loader_payload, &loader_markers) {
        return Err(CompatLoaderArtifactMismatch::LoaderPayload);
    }
    Ok(())
}

pub fn compat_loader_artifact_failure_line(
    prefix: &str,
    pid: u64,
    slug: &str,
    mismatch: CompatLoaderArtifactMismatch,
) -> String {
    let reason = match mismatch {
        CompatLoaderArtifactMismatch::EnvPayload => "env-markers-missing",
        CompatLoaderArtifactMismatch::LoaderPayload => "loader-markers-missing",
    };
    format!("{prefix}.artifact-failure pid={pid} slug={slug} reason={reason} outcome=unexpected")
}

pub fn compat_loader_success_line(prefix: &str, snapshot: &CompatLoaderSessionSnapshot) -> String {
    format!(
        "{prefix}.success pid={} slug={} api={} translation={} route={} mode={} entry={} bootstrap={} entrypoint={} requires-shims={} cwd={} exec={} prefix={} preloads={} dll-overrides={} env-overrides={}",
        snapshot.pid,
        snapshot.slug,
        graphics_api_name(snapshot.graphics_api),
        snapshot.translation,
        snapshot.route_class,
        snapshot.launch_mode,
        snapshot.entry_profile,
        snapshot.bootstrap_profile,
        snapshot.entrypoint,
        if snapshot.requires_compat_shims { 1 } else { 0 },
        snapshot.working_dir,
        snapshot.executable_path,
        snapshot.prefix_path,
        snapshot.preload_spec,
        snapshot.dll_override_spec,
        snapshot.env_override_spec,
    )
}

pub fn compat_loader_foreign_success_line(
    prefix: &str,
    snapshot: &CompatLoaderSessionSnapshot,
) -> String {
    format!(
        "{prefix}.success pid={} slug={} api={} translation={} route={} mode={} entry={} bootstrap={} entrypoint={} requires-shims={}",
        snapshot.pid,
        snapshot.slug,
        graphics_api_name(snapshot.graphics_api),
        snapshot.translation,
        snapshot.route_class,
        snapshot.launch_mode,
        snapshot.entry_profile,
        snapshot.bootstrap_profile,
        snapshot.entrypoint,
        if snapshot.requires_compat_shims { 1 } else { 0 },
    )
}

pub fn compat_loader_refusal_line(prefix: &str, path: &str, code: ExitCode) -> String {
    format!(
        "{prefix}.refusal path={path} code={code} outcome=expected reason=loader-overrides-invalid"
    )
}

pub type ExitCode = i32;

pub fn compat_loader_relaunch_stopped_line(
    prefix: &str,
    pid: u64,
    slug: &str,
    exit_code: i32,
) -> String {
    format!("{prefix}.relaunch.stopped pid={pid} slug={slug} exit={exit_code}")
}

pub fn compat_loader_recovery_line(prefix: &str, snapshot: &CompatLoaderSessionSnapshot) -> String {
    format!(
        "{prefix}.recovery pid={} slug={} running=1 stopped=1 api={} translation={} route={} mode={} entry={} bootstrap={} entrypoint={} requires-shims={} preloads={} dll-overrides={} env-overrides={}",
        snapshot.pid,
        snapshot.slug,
        graphics_api_name(snapshot.graphics_api),
        snapshot.translation,
        snapshot.route_class,
        snapshot.launch_mode,
        snapshot.entry_profile,
        snapshot.bootstrap_profile,
        snapshot.entrypoint,
        if snapshot.requires_compat_shims { 1 } else { 0 },
        snapshot.preload_spec,
        snapshot.dll_override_spec,
        snapshot.env_override_spec,
    )
}

pub fn compat_loader_foreign_recovery_line(
    prefix: &str,
    snapshot: &CompatLoaderSessionSnapshot,
) -> String {
    format!(
        "{prefix}.recovery pid={} slug={} running=1 stopped=1 api={} translation={} route={} mode={} entry={} bootstrap={} entrypoint={} requires-shims={}",
        snapshot.pid,
        snapshot.slug,
        graphics_api_name(snapshot.graphics_api),
        snapshot.translation,
        snapshot.route_class,
        snapshot.launch_mode,
        snapshot.entry_profile,
        snapshot.bootstrap_profile,
        snapshot.entrypoint,
        if snapshot.requires_compat_shims { 1 } else { 0 },
    )
}

pub fn compat_loader_matrix_line(prefix: &str, snapshot: &CompatLoaderSessionSnapshot) -> String {
    format!(
        "{prefix}.matrix pid={} target={} slug={} api={} translation={} route={} mode={} entry={} bootstrap={} entrypoint={} requires-shims={} preloads={} dll-overrides={} env-overrides={}",
        snapshot.pid,
        compat_target_name(snapshot.target),
        snapshot.slug,
        graphics_api_name(snapshot.graphics_api),
        snapshot.translation,
        snapshot.route_class,
        snapshot.launch_mode,
        snapshot.entry_profile,
        snapshot.bootstrap_profile,
        snapshot.entrypoint,
        if snapshot.requires_compat_shims { 1 } else { 0 },
        snapshot.preload_count,
        snapshot.dll_override_count,
        snapshot.env_override_count,
    )
}

pub fn compat_loader_cleanup_line(
    prefix: &str,
    pid: u64,
    slug: &str,
    exit_code: i32,
    stopped_count: usize,
) -> String {
    format!(
        "{prefix}.cleanup pid={pid} slug={slug} exit={exit_code} running=0 stopped={stopped_count}"
    )
}

pub fn compat_loader_completion_line(flavor: CompatLoaderProofFlavor) -> &'static str {
    match flavor {
        CompatLoaderProofFlavor::Native => "compat-loader-smoke-ok",
        CompatLoaderProofFlavor::Foreign => "compat-loader-foreign-smoke-ok",
    }
}

pub fn compat_foreign_boot_proof_line() -> &'static str {
    "boot.proof=compat-foreign"
}

pub fn compat_foreign_reclaim_line(reclaimed: u64) -> String {
    format!(
        "compat.foreign.reclaim amount=3 reclaimed={} outcome=ok",
        reclaimed
    )
}

pub fn compat_foreign_completion_line() -> &'static str {
    "compat-foreign-smoke-ok"
}

fn payload_contains_all_strings(payload: &[u8], markers: &[String]) -> bool {
    markers.iter().all(|marker| {
        payload
            .windows(marker.len())
            .any(|window| window == marker.as_bytes())
    })
}
