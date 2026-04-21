use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use crate::{GameCompatManifest, compat_target_name};
use ngos_user_abi::NativeProcessCompatRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatAbiScenarioManifest {
    pub path: &'static str,
    pub text: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatAbiScenarioSet {
    pub game: CompatAbiScenarioManifest,
    pub app: CompatAbiScenarioManifest,
    pub tool: CompatAbiScenarioManifest,
    pub other: CompatAbiScenarioManifest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatAbiSmokeStage {
    ManifestLoad,
    Launch,
    ReadAbi,
    InspectCompat,
    ProcfsProbe,
    Accepted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatAbiProcessMismatch {
    pub target: String,
    pub observed_target: String,
    pub route: String,
    pub observed_route: String,
    pub handles: String,
    pub observed_handles: String,
    pub paths: String,
    pub observed_paths: String,
    pub scheduler: String,
    pub observed_scheduler: String,
    pub sync: String,
    pub observed_sync: String,
    pub timer: String,
    pub observed_timer: String,
    pub module: String,
    pub observed_module: String,
    pub event: String,
    pub observed_event: String,
    pub requires_shims: u32,
    pub observed_requires_shims: u32,
    pub prefix: String,
    pub executable_path: String,
    pub working_dir: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatAbiProcProbeSnapshot {
    pub pid: u64,
    pub fd_count: usize,
    pub has_fd_0: bool,
    pub has_fd_1: bool,
    pub has_fd_2: bool,
    pub cwd: String,
    pub executable_path: String,
    pub cmdline: String,
    pub environ: Option<String>,
    pub invalid_fd_opened: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatAbiProcProbeExpectation {
    pub cwd: String,
    pub executable_path: String,
    pub require_environ: bool,
    pub environ_marker: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompatAbiProcProbeMismatch {
    DescriptorSet,
    Cmdline,
    Environ,
    InvalidFdRefusal,
    Identity,
}

fn fixed_text_field(bytes: &[u8]) -> &str {
    let len = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    core::str::from_utf8(&bytes[..len]).unwrap_or("")
}

pub fn compat_abi_smoke_scenarios() -> CompatAbiScenarioSet {
    CompatAbiScenarioSet {
        game: CompatAbiScenarioManifest {
            path: "/abi/nova/game.manifest",
            text: "target=game\ntitle=Nova Arena\nslug=nova-arena\nexec=/bin/worker\ncwd=/abi/nova\ngfx.api=directx12\ngfx.backend=vulkan\ngfx.profile=latency\naudio.backend=native-mixer\naudio.profile=stereo\ninput.backend=native-input\ninput.profile=gamepad\nshim.prefix=/compat/abi-game\nshim.saves=/saves/abi-game\nshim.cache=/cache/abi-game\n",
        },
        app: CompatAbiScenarioManifest {
            path: "/abi/nova/app.manifest",
            text: "target=app\ntitle=Nova Desktop\nslug=nova-desktop\nexec=/bin/worker\ncwd=/abi/nova\ngfx.api=vulkan\ngfx.backend=vulkan\ngfx.profile=desktop\naudio.backend=native-mixer\naudio.profile=stereo\ninput.backend=native-input\ninput.profile=kbm\nshim.prefix=/compat/abi-app\nshim.saves=/saves/abi-app\nshim.cache=/cache/abi-app\n",
        },
        tool: CompatAbiScenarioManifest {
            path: "/abi/nova/tool.manifest",
            text: "target=tool\ntitle=Nova Inspector\nslug=nova-inspector\nexec=/bin/worker\ncwd=/abi/nova\ngfx.api=webgpu\ngfx.backend=vulkan\ngfx.profile=inspect\naudio.backend=native-mixer\naudio.profile=mono\ninput.backend=native-input\ninput.profile=kbm\nshim.prefix=/compat/abi-tool\nshim.saves=/saves/abi-tool\nshim.cache=/cache/abi-tool\n",
        },
        other: CompatAbiScenarioManifest {
            path: "/abi/nova/other.manifest",
            text: "target=other\ntitle=Nova Service\nslug=nova-service\nexec=/bin/worker\ncwd=/abi/nova\ngfx.api=vulkan\ngfx.backend=vulkan\ngfx.profile=service\naudio.backend=native-mixer\naudio.profile=mono\ninput.backend=native-input\ninput.profile=kbm\nshim.prefix=/compat/abi-service\nshim.saves=/saves/abi-service\nshim.cache=/cache/abi-service\n",
        },
    }
}

pub fn compat_abi_scenario_manifests(
    scenarios: &CompatAbiScenarioSet,
) -> [&CompatAbiScenarioManifest; 4] {
    [
        &scenarios.game,
        &scenarios.app,
        &scenarios.tool,
        &scenarios.other,
    ]
}

pub fn compat_abi_required_dirs() -> [&'static str; 2] {
    ["/abi", "/abi/nova"]
}

pub fn compat_abi_expected_payload_markers(manifest: &GameCompatManifest) -> Vec<String> {
    let abi = manifest.abi_routing_plan();
    vec![
        format!("route-class={}", abi.route_class),
        format!("handle-profile={}", abi.handle_profile),
        format!("path-profile={}", abi.path_profile),
        format!("scheduler-profile={}", abi.scheduler_profile),
        format!("sync-profile={}", abi.sync_profile),
        format!("timer-profile={}", abi.timer_profile),
        format!("module-profile={}", abi.module_profile),
        format!("event-profile={}", abi.event_profile),
        format!(
            "requires-kernel-abi-shims={}",
            if abi.requires_kernel_abi_shims { 1 } else { 0 }
        ),
        format!("target={}", compat_target_name(manifest.target)),
    ]
}

pub fn compat_abi_verify_payload(manifest: &GameCompatManifest, payload: &[u8]) -> bool {
    let markers = compat_abi_expected_payload_markers(manifest);
    markers.iter().all(|marker| {
        payload
            .windows(marker.len())
            .any(|window| window == marker.as_bytes())
    })
}

pub fn compat_abi_verify_process_record(
    manifest: &GameCompatManifest,
    compat: &NativeProcessCompatRecord,
) -> Result<(), CompatAbiProcessMismatch> {
    let abi = manifest.abi_routing_plan();
    let expected_target = compat_target_name(manifest.target).to_string();
    let observed_target = fixed_text_field(&compat.target).to_string();
    let route = abi.route_class.to_string();
    let observed_route = fixed_text_field(&compat.route_class).to_string();
    let handles = abi.handle_profile.to_string();
    let observed_handles = fixed_text_field(&compat.handle_profile).to_string();
    let paths = abi.path_profile.to_string();
    let observed_paths = fixed_text_field(&compat.path_profile).to_string();
    let scheduler = abi.scheduler_profile.to_string();
    let observed_scheduler = fixed_text_field(&compat.scheduler_profile).to_string();
    let sync = abi.sync_profile.to_string();
    let observed_sync = fixed_text_field(&compat.sync_profile).to_string();
    let timer = abi.timer_profile.to_string();
    let observed_timer = fixed_text_field(&compat.timer_profile).to_string();
    let module = abi.module_profile.to_string();
    let observed_module = fixed_text_field(&compat.module_profile).to_string();
    let event = abi.event_profile.to_string();
    let observed_event = fixed_text_field(&compat.event_profile).to_string();
    let requires_shims = if abi.requires_kernel_abi_shims { 1 } else { 0 };
    let observed_requires_shims = compat.requires_kernel_abi_shims;

    let ok = observed_target == expected_target
        && observed_route == route
        && observed_handles == handles
        && observed_paths == paths
        && observed_scheduler == scheduler
        && observed_sync == sync
        && observed_timer == timer
        && observed_module == module
        && observed_event == event
        && observed_requires_shims == requires_shims;

    if ok {
        return Ok(());
    }

    Err(CompatAbiProcessMismatch {
        target: expected_target,
        observed_target,
        route,
        observed_route,
        handles,
        observed_handles,
        paths,
        observed_paths,
        scheduler,
        observed_scheduler,
        sync,
        observed_sync,
        timer,
        observed_timer,
        module,
        observed_module,
        event,
        observed_event,
        requires_shims,
        observed_requires_shims,
        prefix: fixed_text_field(&compat.prefix).to_string(),
        executable_path: fixed_text_field(&compat.executable_path).to_string(),
        working_dir: fixed_text_field(&compat.working_dir).to_string(),
    })
}

pub fn compat_abi_process_failure_line(pid: u64, mismatch: &CompatAbiProcessMismatch) -> String {
    format!(
        "compat.abi.smoke.process-failure pid={} target={} observed-target={} route={} observed-route={} handles={} observed-handles={} paths={} observed-paths={} scheduler={} observed-scheduler={} sync={} observed-sync={} timer={} observed-timer={} module={} observed-module={} event={} observed-event={} requires-shims={} observed-requires-shims={} prefix={} exec={} cwd={}",
        pid,
        mismatch.target,
        mismatch.observed_target,
        mismatch.route,
        mismatch.observed_route,
        mismatch.handles,
        mismatch.observed_handles,
        mismatch.paths,
        mismatch.observed_paths,
        mismatch.scheduler,
        mismatch.observed_scheduler,
        mismatch.sync,
        mismatch.observed_sync,
        mismatch.timer,
        mismatch.observed_timer,
        mismatch.module,
        mismatch.observed_module,
        mismatch.event,
        mismatch.observed_event,
        mismatch.requires_shims,
        mismatch.observed_requires_shims,
        mismatch.prefix,
        mismatch.executable_path,
        mismatch.working_dir,
    )
}

pub fn compat_abi_route_line(pid: u64, manifest: &GameCompatManifest, abi_file: &str) -> String {
    let abi = manifest.abi_routing_plan();
    format!(
        "compat.abi.smoke.route pid={} target={} route={} handles={} paths={} scheduler={} sync={} timer={} module={} event={} requires-shims={} file={}",
        pid,
        compat_target_name(manifest.target),
        abi.route_class,
        abi.handle_profile,
        abi.path_profile,
        abi.scheduler_profile,
        abi.sync_profile,
        abi.timer_profile,
        abi.module_profile,
        abi.event_profile,
        if abi.requires_kernel_abi_shims { 1 } else { 0 },
        abi_file,
    )
}

pub fn compat_abi_process_line(pid: u64, compat: &NativeProcessCompatRecord) -> String {
    format!(
        "compat.abi.smoke.process pid={} target={} route={} handles={} paths={} scheduler={} sync={} timer={} module={} event={} requires-shims={} prefix={} exec={} cwd={}",
        pid,
        fixed_text_field(&compat.target),
        fixed_text_field(&compat.route_class),
        fixed_text_field(&compat.handle_profile),
        fixed_text_field(&compat.path_profile),
        fixed_text_field(&compat.scheduler_profile),
        fixed_text_field(&compat.sync_profile),
        fixed_text_field(&compat.timer_profile),
        fixed_text_field(&compat.module_profile),
        fixed_text_field(&compat.event_profile),
        compat.requires_kernel_abi_shims,
        fixed_text_field(&compat.prefix),
        fixed_text_field(&compat.executable_path),
        fixed_text_field(&compat.working_dir),
    )
}

pub fn compat_abi_cleanup_line(stopped_count: usize) -> String {
    format!(
        "compat.abi.smoke.cleanup running=0 stopped={} outcome=ok",
        stopped_count
    )
}

pub fn compat_abi_completion_line() -> &'static str {
    "compat-abi-smoke-ok"
}

pub fn compat_abi_boot_proof_line() -> &'static str {
    "boot.proof=compat-abi"
}

pub fn compat_abi_stage_line(
    stage: CompatAbiSmokeStage,
    target: &str,
    pid: Option<u64>,
    path: Option<&str>,
    cwd: Option<&str>,
    executable_path: Option<&str>,
) -> String {
    match stage {
        CompatAbiSmokeStage::ManifestLoad => format!(
            "compat.abi.smoke.stage target={} stage=manifest-load path={}",
            target,
            path.unwrap_or("")
        ),
        CompatAbiSmokeStage::Launch => format!(
            "compat.abi.smoke.stage target={} stage=launch cwd={} exec={}",
            target,
            cwd.unwrap_or(""),
            executable_path.unwrap_or("")
        ),
        CompatAbiSmokeStage::ReadAbi => format!(
            "compat.abi.smoke.stage target={} stage=read-abi pid={} file={}",
            target,
            pid.unwrap_or(0),
            path.unwrap_or("")
        ),
        CompatAbiSmokeStage::InspectCompat => format!(
            "compat.abi.smoke.stage target={} stage=inspect-compat pid={}",
            target,
            pid.unwrap_or(0)
        ),
        CompatAbiSmokeStage::ProcfsProbe => format!(
            "compat.abi.smoke.stage target={} stage=procfs-probe pid={}",
            target,
            pid.unwrap_or(0)
        ),
        CompatAbiSmokeStage::Accepted => format!(
            "compat.abi.smoke.stage target={} stage=accepted pid={}",
            target,
            pid.unwrap_or(0)
        ),
    }
}

pub fn compat_abi_failure_line(
    stage: CompatAbiSmokeStage,
    target: &str,
    pid: Option<u64>,
    path: Option<&str>,
    code: i32,
) -> String {
    match stage {
        CompatAbiSmokeStage::ManifestLoad => format!(
            "compat.abi.smoke.failure target={} stage=manifest-load path={} code={}",
            target,
            path.unwrap_or(""),
            code
        ),
        CompatAbiSmokeStage::Launch => format!(
            "compat.abi.smoke.failure target={} stage=launch code={}",
            target, code
        ),
        CompatAbiSmokeStage::ReadAbi => format!(
            "compat.abi.smoke.failure target={} stage=read-abi pid={} file={} code={}",
            target,
            pid.unwrap_or(0),
            path.unwrap_or(""),
            code
        ),
        CompatAbiSmokeStage::InspectCompat => format!(
            "compat.abi.smoke.failure target={} stage=inspect-compat pid={} code={}",
            target,
            pid.unwrap_or(0),
            code
        ),
        CompatAbiSmokeStage::ProcfsProbe => format!(
            "compat.abi.smoke.failure target={} stage=procfs-probe pid={} code={}",
            target,
            pid.unwrap_or(0),
            code
        ),
        CompatAbiSmokeStage::Accepted => format!(
            "compat.abi.smoke.failure target={} stage=accepted pid={} code={}",
            target,
            pid.unwrap_or(0),
            code
        ),
    }
}

pub fn compat_abi_process_image_matches(actual: &str, expected: &str) -> bool {
    actual == expected
        || compat_abi_process_image_name(actual) == compat_abi_process_image_name(expected)
}

pub fn compat_abi_process_cmdline_matches(cmdline: &str, expected: &str) -> bool {
    cmdline
        .split(['\0', '\n'])
        .filter(|segment| !segment.is_empty())
        .any(|segment| compat_abi_process_image_matches(segment, expected))
}

pub fn compat_abi_verify_proc_probe(
    snapshot: &CompatAbiProcProbeSnapshot,
    expectation: &CompatAbiProcProbeExpectation,
) -> Result<(), CompatAbiProcProbeMismatch> {
    if !(snapshot.has_fd_0 && snapshot.has_fd_1 && snapshot.has_fd_2 && snapshot.fd_count >= 3) {
        return Err(CompatAbiProcProbeMismatch::DescriptorSet);
    }
    if !compat_abi_process_cmdline_matches(&snapshot.cmdline, &expectation.executable_path) {
        return Err(CompatAbiProcProbeMismatch::Cmdline);
    }
    if expectation.require_environ {
        let Some(environ) = snapshot.environ.as_deref() else {
            return Err(CompatAbiProcProbeMismatch::Environ);
        };
        if environ.is_empty() {
            return Err(CompatAbiProcProbeMismatch::Environ);
        }
        if expectation.environ_marker.is_some()
            && !environ.contains(expectation.environ_marker.as_deref().unwrap_or(""))
        {
            return Err(CompatAbiProcProbeMismatch::Environ);
        }
    }
    if snapshot.invalid_fd_opened {
        return Err(CompatAbiProcProbeMismatch::InvalidFdRefusal);
    }
    if snapshot.cwd != expectation.cwd
        || !compat_abi_process_image_matches(
            &snapshot.executable_path,
            &expectation.executable_path,
        )
    {
        return Err(CompatAbiProcProbeMismatch::Identity);
    }
    Ok(())
}

pub fn compat_abi_proc_failure_line(
    snapshot: &CompatAbiProcProbeSnapshot,
    expectation: &CompatAbiProcProbeExpectation,
    mismatch: &CompatAbiProcProbeMismatch,
) -> String {
    match mismatch {
        CompatAbiProcProbeMismatch::DescriptorSet => format!(
            "compat.abi.smoke.proc-failure pid={} fd-count={} has-0={} has-1={} has-2={} cwd={} exe={}",
            snapshot.pid,
            snapshot.fd_count,
            snapshot.has_fd_0,
            snapshot.has_fd_1,
            snapshot.has_fd_2,
            snapshot.cwd,
            snapshot.executable_path
        ),
        CompatAbiProcProbeMismatch::Cmdline => format!(
            "compat.abi.smoke.proc-failure pid={} cmdline={}",
            snapshot.pid, snapshot.cmdline
        ),
        CompatAbiProcProbeMismatch::Environ => format!(
            "compat.abi.smoke.proc-failure pid={} environ={}",
            snapshot.pid,
            snapshot.environ.as_deref().unwrap_or("")
        ),
        CompatAbiProcProbeMismatch::InvalidFdRefusal => format!(
            "compat.abi.smoke.proc-failure pid={} path=/proc/{}/fd/9999 reason=unexpected-success",
            snapshot.pid, snapshot.pid
        ),
        CompatAbiProcProbeMismatch::Identity => format!(
            "compat.abi.smoke.proc-failure pid={} cwd={} observed-cwd={} exe={} observed-exe={}",
            snapshot.pid,
            expectation.cwd,
            snapshot.cwd,
            expectation.executable_path,
            snapshot.executable_path
        ),
    }
}

pub fn compat_abi_proc_success_line(snapshot: &CompatAbiProcProbeSnapshot) -> String {
    format!(
        "compat.abi.smoke.proc.success pid={} fd-count={} fd0=present fd1=present fd2=present cwd={} exe={} cmdline=present",
        snapshot.pid, snapshot.fd_count, snapshot.cwd, snapshot.executable_path
    )
}

pub fn compat_abi_proc_step_line(pid: u64, path: &str) -> String {
    format!("compat.abi.smoke.proc.step pid={} path={}", pid, path)
}

pub fn compat_abi_proc_environ_line(pid: u64, marker: &str) -> String {
    format!(
        "compat.abi.smoke.proc.environ pid={} outcome=ok marker={}",
        pid, marker
    )
}

pub fn compat_abi_proc_refusal_line(pid: u64) -> String {
    format!(
        "compat.abi.smoke.proc.refusal pid={} path=/proc/{}/fd/9999 outcome=expected",
        pid, pid
    )
}

pub fn compat_abi_proc_recovery_line(pid: u64) -> String {
    format!(
        "compat.abi.smoke.proc.recovery pid={} fd-list=ok outcome=ok",
        pid
    )
}

fn compat_abi_process_image_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::{
        CompatAbiProcProbeExpectation, CompatAbiProcProbeMismatch, CompatAbiProcProbeSnapshot,
        compat_abi_process_cmdline_matches, compat_abi_verify_proc_probe,
    };
    use alloc::string::String;

    #[test]
    fn process_cmdline_matches_nul_delimited_payloads() {
        assert!(compat_abi_process_cmdline_matches(
            "/kernel/ngos-userland-native\0--compat-proc-probe\0",
            "/kernel/ngos-userland-native",
        ));
    }

    #[test]
    fn process_cmdline_matches_newline_delimited_payloads() {
        assert!(compat_abi_process_cmdline_matches(
            "/kernel/ngos-userland-native\n--compat-proc-probe",
            "/kernel/ngos-userland-native",
        ));
    }

    #[test]
    fn verify_proc_probe_reports_identity_mismatch() {
        let snapshot = CompatAbiProcProbeSnapshot {
            pid: 7,
            fd_count: 3,
            has_fd_0: true,
            has_fd_1: true,
            has_fd_2: true,
            cwd: String::from("/wrong"),
            executable_path: String::from("/kernel/ngos-userland-native"),
            cmdline: String::from("/kernel/ngos-userland-native"),
            environ: Some(String::from("NGOS_COMPAT_TARGET=game")),
            invalid_fd_opened: false,
        };
        let expectation = CompatAbiProcProbeExpectation {
            cwd: String::from("/expected"),
            executable_path: String::from("/kernel/ngos-userland-native"),
            require_environ: true,
            environ_marker: Some(String::from("NGOS_COMPAT_TARGET=game")),
        };
        assert_eq!(
            compat_abi_verify_proc_probe(&snapshot, &expectation),
            Err(CompatAbiProcProbeMismatch::Identity)
        );
    }
}
