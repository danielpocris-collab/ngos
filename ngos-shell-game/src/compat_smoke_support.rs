use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use ngos_game_compat_runtime::{
    CompatAbiSmokeStage, CompatLoaderProofFlavor, CompatLoaderSessionSnapshot, GameCompatManifest,
    compat_abi_boot_proof_line, compat_abi_cleanup_line, compat_abi_completion_line,
    compat_abi_failure_line, compat_abi_proc_environ_line, compat_abi_proc_recovery_line,
    compat_abi_proc_refusal_line, compat_abi_proc_step_line, compat_abi_proc_success_line,
    compat_abi_process_failure_line, compat_abi_required_dirs, compat_abi_scenario_manifests,
    compat_abi_smoke_scenarios, compat_abi_stage_line, compat_foreign_boot_proof_line,
    compat_foreign_completion_line, compat_foreign_reclaim_line,
    compat_loader_artifact_failure_line, compat_loader_cleanup_line, compat_loader_completion_line,
    compat_loader_foreign_recovery_line, compat_loader_foreign_success_line,
    compat_loader_matrix_line, compat_loader_plan_line, compat_loader_recovery_line,
    compat_loader_refusal_line, compat_loader_relaunch_stopped_line, compat_loader_required_dirs,
    compat_loader_scenario_manifests, compat_loader_success_line, foreign_loader_smoke_scenarios,
    native_loader_smoke_scenarios,
};
use ngos_shell_compat_abi::build_compat_abi_core_smoke_report;
use ngos_shell_proc::fixed_text_field;
use ngos_shell_vfs::shell_write_file;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatLaunchAbiObservationError, GameCompatLaunchLoaderObservationError,
    GameCompatLaunchedAbiObservation, GameCompatLaunchedLoaderObservation,
    GameCompatLoaderSessionObservationError, GameCompatSession, game_cleanup_sessions_and_paths,
    game_launch_and_observe_abi_session, game_launch_and_observe_loader_session,
    game_manifest_load, game_stop_session, game_stop_sessions, write_line,
};

type CompatLoaderSnapshotLine = fn(&str, &CompatLoaderSessionSnapshot) -> String;

struct CompatLoaderObservationCodes {
    render: ExitCode,
    env_read: ExitCode,
    loader_read: ExitCode,
    env_mismatch: ExitCode,
    loader_mismatch: ExitCode,
}

fn compat_loader_load_manifest<B: SyscallBackend>(
    runtime: &Runtime<B>,
    manifest_path: &str,
) -> Result<GameCompatManifest, ExitCode> {
    game_manifest_load(runtime, manifest_path)
}

fn compat_loader_launch_observation_exit_code<B: SyscallBackend>(
    runtime: &Runtime<B>,
    family: &str,
    session: &GameCompatSession,
    error: GameCompatLoaderSessionObservationError,
    codes: CompatLoaderObservationCodes,
) -> ExitCode {
    match error {
        GameCompatLoaderSessionObservationError::Render => codes.render,
        GameCompatLoaderSessionObservationError::ArtifactRead(code) => match code {
            237 | 238 => codes.env_read,
            240 => codes.loader_read,
            _ => code,
        },
        GameCompatLoaderSessionObservationError::ArtifactMismatch(mismatch) => {
            let _ = write_line(
                runtime,
                &compat_loader_artifact_failure_line(family, session.pid, &session.slug, mismatch),
            );
            match mismatch {
                ngos_game_compat_runtime::CompatLoaderArtifactMismatch::EnvPayload => {
                    codes.env_mismatch
                }
                ngos_game_compat_runtime::CompatLoaderArtifactMismatch::LoaderPayload => {
                    codes.loader_mismatch
                }
            }
        }
    }
}

fn compat_loader_launch_observed<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &mut String,
    family: &str,
    manifest: &GameCompatManifest,
    codes: CompatLoaderObservationCodes,
) -> Result<GameCompatLaunchedLoaderObservation, ExitCode> {
    match game_launch_and_observe_loader_session(runtime, current_cwd, manifest) {
        Ok(result) => Ok(result),
        Err(GameCompatLaunchLoaderObservationError::Launch(code)) => Err(code),
        Err(GameCompatLaunchLoaderObservationError::Observe(session, error)) => Err(
            compat_loader_launch_observation_exit_code(runtime, family, &session, error, codes),
        ),
    }
}

fn compat_loader_launch_and_write<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &mut String,
    family: &str,
    manifest: &GameCompatManifest,
    codes: CompatLoaderObservationCodes,
    line_writer: CompatLoaderSnapshotLine,
    write_code: ExitCode,
) -> Result<Box<GameCompatSession>, ExitCode> {
    let GameCompatLaunchedLoaderObservation {
        session,
        observation,
    } = compat_loader_launch_observed(runtime, current_cwd, family, manifest, codes)?;
    if write_line(runtime, &line_writer(family, &observation.snapshot)).is_err() {
        return Err(write_code);
    }
    Ok(session)
}

fn compat_loader_write_refusal<B: SyscallBackend>(
    runtime: &Runtime<B>,
    family: &str,
    manifest_path: &str,
    success_is_error: ExitCode,
    write_error: ExitCode,
) -> Result<(), ExitCode> {
    match compat_loader_load_manifest(runtime, manifest_path) {
        Ok(_) => Err(success_is_error),
        Err(code) => {
            if write_line(
                runtime,
                &compat_loader_refusal_line(family, manifest_path, code),
            )
            .is_err()
            {
                return Err(write_error);
            }
            Ok(())
        }
    }
}

fn compat_loader_stop_and_report_relaunch<B: SyscallBackend>(
    runtime: &Runtime<B>,
    family: &str,
    session: &mut GameCompatSession,
    stop_code: ExitCode,
    write_code: ExitCode,
) -> Result<(), ExitCode> {
    if game_stop_session(runtime, session).is_err() {
        return Err(stop_code);
    }
    if write_line(
        runtime,
        &compat_loader_relaunch_stopped_line(
            family,
            session.pid,
            &session.slug,
            session.exit_code.unwrap_or(-1),
        ),
    )
    .is_err()
    {
        return Err(write_code);
    }
    Ok(())
}

fn compat_loader_prepare_native_scenarios<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> Result<ngos_game_compat_runtime::CompatLoaderSmokeScenarioSet, ExitCode> {
    for (index, path) in compat_loader_required_dirs().iter().enumerate() {
        if runtime.mkdir_path(path).is_err() {
            return Err(399 + index as i32);
        }
    }

    let scenarios = native_loader_smoke_scenarios();
    let manifest_specs = compat_loader_scenario_manifests(&scenarios);
    for (index, manifest_spec) in manifest_specs.iter().enumerate() {
        if shell_write_file(runtime, manifest_spec.path, manifest_spec.text).is_err() {
            return Err(401 + index as i32);
        }
    }
    Ok(scenarios)
}

fn compat_loader_prepare_foreign_scenarios<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> Result<ngos_game_compat_runtime::CompatLoaderSmokeScenarioSet, ExitCode> {
    for (index, path) in compat_loader_required_dirs().iter().enumerate() {
        if runtime.mkdir_path(path).is_err() {
            return Err(438 + index as i32);
        }
    }

    let scenarios = foreign_loader_smoke_scenarios();
    let manifest_specs = compat_loader_scenario_manifests(&scenarios);
    for (index, manifest_spec) in manifest_specs.iter().enumerate() {
        if shell_write_file(runtime, manifest_spec.path, manifest_spec.text).is_err() {
            return Err(440 + index as i32);
        }
    }
    Ok(scenarios)
}

fn compat_loader_finish_native_sessions<B: SyscallBackend>(
    runtime: &Runtime<B>,
    scenarios: &ngos_game_compat_runtime::CompatLoaderSmokeScenarioSet,
    first_session: &GameCompatSession,
    second_session: &mut GameCompatSession,
    tool_session: &mut GameCompatSession,
    other_session: &mut GameCompatSession,
) -> ExitCode {
    let mut running_sessions = [
        &mut *second_session,
        &mut *tool_session,
        &mut *other_session,
    ];
    if game_stop_sessions(runtime, &mut running_sessions).is_err() {
        return 433;
    }
    if write_line(
        runtime,
        &compat_loader_cleanup_line(
            "compat.loader.smoke",
            second_session.pid,
            &second_session.slug,
            second_session.exit_code.unwrap_or(-1),
            4,
        ),
    )
    .is_err()
    {
        return 436;
    }
    let cleanup_paths = compat_loader_scenario_manifests(scenarios)
        .into_iter()
        .map(|manifest| manifest.path)
        .collect::<Vec<_>>();
    let cleanup_sessions = [
        first_session,
        &*second_session,
        &*tool_session,
        &*other_session,
    ];
    game_cleanup_sessions_and_paths(runtime, &cleanup_sessions, &cleanup_paths);
    if write_line(
        runtime,
        compat_loader_completion_line(CompatLoaderProofFlavor::Native),
    )
    .is_err()
    {
        return 437;
    }
    0
}

fn compat_loader_finish_foreign_sessions<B: SyscallBackend>(
    runtime: &Runtime<B>,
    scenarios: &ngos_game_compat_runtime::CompatLoaderSmokeScenarioSet,
    first_session: &GameCompatSession,
    second_session: &mut GameCompatSession,
) -> ExitCode {
    let mut recovery_sessions = [&mut *second_session];
    if game_stop_sessions(runtime, &mut recovery_sessions).is_err() {
        return 460;
    }
    if write_line(
        runtime,
        &compat_loader_cleanup_line(
            "compat.loader.foreign",
            second_session.pid,
            &second_session.slug,
            second_session.exit_code.unwrap_or(-1),
            2,
        ),
    )
    .is_err()
    {
        return 461;
    }
    let cleanup_paths = compat_loader_scenario_manifests(scenarios)
        .into_iter()
        .map(|manifest| manifest.path)
        .collect::<Vec<_>>();
    let cleanup_sessions = [first_session, &*second_session];
    game_cleanup_sessions_and_paths(runtime, &cleanup_sessions, &cleanup_paths);
    if write_line(
        runtime,
        compat_loader_completion_line(CompatLoaderProofFlavor::Foreign),
    )
    .is_err()
    {
        return 462;
    }
    0
}

pub fn run_native_compat_loader_boot_smoke<B: SyscallBackend>(runtime: &Runtime<B>) -> ExitCode {
    let scenarios = match compat_loader_prepare_native_scenarios(runtime) {
        Ok(scenarios) => scenarios,
        Err(code) => return code,
    };

    let bad_manifest_path = scenarios.invalid.path;
    let recovery_manifest_path = scenarios.recovery.path;
    let Some(tool_manifest) = scenarios.tool.as_ref() else {
        return 422;
    };
    let tool_manifest_path = tool_manifest.path;
    let Some(other_manifest) = scenarios.other.as_ref() else {
        return 423;
    };
    let other_manifest_path = other_manifest.path;

    let manifest = match GameCompatManifest::parse(scenarios.valid.text) {
        Ok(manifest) => manifest,
        Err(_) => return 283,
    };
    if write_line(
        runtime,
        &compat_loader_plan_line("compat.loader.smoke", &manifest),
    )
    .is_err()
    {
        return 404;
    }

    let mut cwd = String::from("/");
    let mut first_session = match compat_loader_launch_and_write(
        runtime,
        &mut cwd,
        "compat.loader.smoke",
        &manifest,
        CompatLoaderObservationCodes {
            render: 405,
            env_read: 406,
            loader_read: 407,
            env_mismatch: 408,
            loader_mismatch: 409,
        },
        compat_loader_success_line,
        410,
    ) {
        Ok(session) => session,
        Err(code) => return code,
    };

    if let Err(code) =
        compat_loader_write_refusal(runtime, "compat.loader.smoke", bad_manifest_path, 411, 412)
    {
        return code;
    }

    if let Err(code) = compat_loader_stop_and_report_relaunch(
        runtime,
        "compat.loader.smoke",
        &mut first_session,
        413,
        414,
    ) {
        return code;
    }

    let recovery_loaded = match compat_loader_load_manifest(runtime, recovery_manifest_path) {
        Ok(manifest) => manifest,
        Err(code) => return code,
    };
    let mut second_session = match compat_loader_launch_and_write(
        runtime,
        &mut cwd,
        "compat.loader.smoke",
        &recovery_loaded,
        CompatLoaderObservationCodes {
            render: 415,
            env_read: 416,
            loader_read: 417,
            env_mismatch: 418,
            loader_mismatch: 419,
        },
        compat_loader_recovery_line,
        420,
    ) {
        Ok(session) => session,
        Err(code) => return code,
    };

    let tool_loaded = match compat_loader_load_manifest(runtime, tool_manifest_path) {
        Ok(manifest) => manifest,
        Err(code) => return code,
    };
    let mut tool_session = match compat_loader_launch_and_write(
        runtime,
        &mut cwd,
        "compat.loader.smoke",
        &tool_loaded,
        CompatLoaderObservationCodes {
            render: 421,
            env_read: 422,
            loader_read: 423,
            env_mismatch: 424,
            loader_mismatch: 425,
        },
        compat_loader_matrix_line,
        426,
    ) {
        Ok(session) => session,
        Err(code) => return code,
    };

    let other_loaded = match compat_loader_load_manifest(runtime, other_manifest_path) {
        Ok(manifest) => manifest,
        Err(code) => return code,
    };
    let mut other_session = match compat_loader_launch_and_write(
        runtime,
        &mut cwd,
        "compat.loader.smoke",
        &other_loaded,
        CompatLoaderObservationCodes {
            render: 427,
            env_read: 428,
            loader_read: 429,
            env_mismatch: 430,
            loader_mismatch: 431,
        },
        compat_loader_matrix_line,
        432,
    ) {
        Ok(session) => session,
        Err(code) => return code,
    };

    compat_loader_finish_native_sessions(
        runtime,
        &scenarios,
        &first_session,
        &mut second_session,
        &mut tool_session,
        &mut other_session,
    )
}

pub fn run_native_compat_foreign_loader_boot_smoke<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> ExitCode {
    let scenarios = match compat_loader_prepare_foreign_scenarios(runtime) {
        Ok(scenarios) => scenarios,
        Err(code) => return code,
    };
    let bad_manifest_path = scenarios.invalid.path;
    let recovery_manifest_path = scenarios.recovery.path;

    let manifest = match GameCompatManifest::parse(scenarios.valid.text) {
        Ok(manifest) => manifest,
        Err(_) => return 283,
    };
    if write_line(
        runtime,
        &compat_loader_plan_line("compat.loader.foreign", &manifest),
    )
    .is_err()
    {
        return 443;
    }

    let mut cwd = String::from("/");
    let mut first_session = match compat_loader_launch_and_write(
        runtime,
        &mut cwd,
        "compat.loader.foreign",
        &manifest,
        CompatLoaderObservationCodes {
            render: 444,
            env_read: 445,
            loader_read: 446,
            env_mismatch: 447,
            loader_mismatch: 448,
        },
        compat_loader_foreign_success_line,
        449,
    ) {
        Ok(session) => session,
        Err(code) => return code,
    };

    if let Err(code) = compat_loader_write_refusal(
        runtime,
        "compat.loader.foreign",
        bad_manifest_path,
        450,
        451,
    ) {
        return code;
    }

    if let Err(code) = compat_loader_stop_and_report_relaunch(
        runtime,
        "compat.loader.foreign",
        &mut first_session,
        452,
        453,
    ) {
        return code;
    }

    let recovery_loaded = match compat_loader_load_manifest(runtime, recovery_manifest_path) {
        Ok(manifest) => manifest,
        Err(code) => return code,
    };
    let mut second_session = match compat_loader_launch_and_write(
        runtime,
        &mut cwd,
        "compat.loader.foreign",
        &recovery_loaded,
        CompatLoaderObservationCodes {
            render: 454,
            env_read: 455,
            loader_read: 456,
            env_mismatch: 457,
            loader_mismatch: 458,
        },
        compat_loader_foreign_recovery_line,
        459,
    ) {
        Ok(session) => session,
        Err(code) => return code,
    };

    compat_loader_finish_foreign_sessions(runtime, &scenarios, &first_session, &mut second_session)
}

fn compat_abi_write_stage<B: SyscallBackend>(
    runtime: &Runtime<B>,
    stage: CompatAbiSmokeStage,
    target: &str,
    pid: Option<u64>,
    path: Option<&str>,
    cwd: Option<&str>,
    image: Option<&str>,
) {
    let _ = write_line(
        runtime,
        &compat_abi_stage_line(stage, target, pid, path, cwd, image),
    );
}

fn compat_abi_write_failure<B: SyscallBackend>(
    runtime: &Runtime<B>,
    stage: CompatAbiSmokeStage,
    target: &str,
    pid: Option<u64>,
    path: Option<&str>,
    code: ExitCode,
) {
    let _ = write_line(
        runtime,
        &compat_abi_failure_line(stage, target, pid, path, code),
    );
}

fn compat_abi_launch_observed<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &mut String,
    manifest: &GameCompatManifest,
    target_name: &str,
) -> Result<GameCompatLaunchedAbiObservation, ExitCode> {
    compat_abi_write_stage(
        runtime,
        CompatAbiSmokeStage::Launch,
        target_name,
        None,
        None,
        Some(&manifest.working_dir),
        Some(&manifest.executable_path),
    );
    match game_launch_and_observe_abi_session(runtime, current_cwd, manifest) {
        Ok(launched) => {
            compat_abi_write_stage(
                runtime,
                CompatAbiSmokeStage::ReadAbi,
                target_name,
                Some(launched.session.pid),
                Some(&launched.session.runtime_abi_path),
                None,
                None,
            );
            compat_abi_write_stage(
                runtime,
                CompatAbiSmokeStage::InspectCompat,
                target_name,
                Some(launched.session.pid),
                None,
                None,
                None,
            );
            Ok(launched)
        }
        Err(GameCompatLaunchAbiObservationError::Launch(code)) => {
            compat_abi_write_failure(
                runtime,
                CompatAbiSmokeStage::Launch,
                target_name,
                None,
                None,
                code,
            );
            Err(code)
        }
        Err(GameCompatLaunchAbiObservationError::Observe(session, error)) => {
            compat_abi_write_stage(
                runtime,
                CompatAbiSmokeStage::ReadAbi,
                target_name,
                Some(session.pid),
                Some(&session.runtime_abi_path),
                None,
                None,
            );
            compat_abi_write_stage(
                runtime,
                CompatAbiSmokeStage::InspectCompat,
                target_name,
                Some(session.pid),
                None,
                None,
                None,
            );
            match error {
                crate::GameCompatAbiSessionObservationError::ReadAbi => {
                    compat_abi_write_failure(
                        runtime,
                        CompatAbiSmokeStage::ReadAbi,
                        target_name,
                        Some(session.pid),
                        Some(&session.runtime_abi_path),
                        458,
                    );
                    Err(458)
                }
                crate::GameCompatAbiSessionObservationError::InvalidAbiPayload => {
                    compat_abi_write_failure(
                        runtime,
                        CompatAbiSmokeStage::ReadAbi,
                        target_name,
                        Some(session.pid),
                        Some(&session.runtime_abi_path),
                        459,
                    );
                    Err(459)
                }
                crate::GameCompatAbiSessionObservationError::InspectCompat => {
                    compat_abi_write_failure(
                        runtime,
                        CompatAbiSmokeStage::InspectCompat,
                        target_name,
                        Some(session.pid),
                        None,
                        459,
                    );
                    Err(459)
                }
                crate::GameCompatAbiSessionObservationError::ProcessRecord(mismatch) => {
                    let _ = write_line(
                        runtime,
                        &compat_abi_process_failure_line(session.pid, &mismatch),
                    );
                    Err(459)
                }
            }
        }
    }
}

fn compat_abi_process_fd_probe<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
    compat: &ngos_user_abi::NativeProcessCompatRecord,
) -> Result<(), ExitCode> {
    let pid = session.pid;
    let fd_paths = ["/proc/{pid}/fd/0", "/proc/{pid}/fd/1", "/proc/{pid}/fd/2"];
    for suffix in ["fd/0", "fd/1", "fd/2"] {
        if write_line(
            runtime,
            &compat_abi_proc_step_line(pid, &format!("/proc/{pid}/{suffix}")),
        )
        .is_err()
        {
            return Err(464);
        }
    }
    let snapshot = crate::game_compat_proc_probe_snapshot(runtime, pid, true).map_err(|_| 464)?;
    if write_line(runtime, &compat_abi_proc_success_line(&snapshot)).is_err() {
        return Err(464);
    }
    if write_line(
        runtime,
        &compat_abi_proc_environ_line(pid, "NGOS_COMPAT_ABI_ROUTE_CLASS"),
    )
    .is_err()
    {
        return Err(464);
    }
    let mut missing = [0u8; 64];
    match runtime.read_procfs(&format!("/proc/{pid}/fd/9999"), &mut missing) {
        Err(ngos_user_abi::Errno::NoEnt) => {}
        _ => return Err(464),
    }
    if write_line(runtime, &compat_abi_proc_refusal_line(pid)).is_err() {
        return Err(464);
    }
    if write_line(runtime, &compat_abi_proc_recovery_line(pid)).is_err() {
        return Err(464);
    }
    if fd_paths.iter().any(|path| path.is_empty()) || fixed_text_field(&compat.target).is_empty() {
        return Err(464);
    }
    Ok(())
}

fn compat_abi_process_manifest<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &mut String,
    manifest_path: &str,
) -> Result<Box<GameCompatSession>, ExitCode> {
    let manifest_name = manifest_path.rsplit('/').next().unwrap_or(manifest_path);
    compat_abi_write_stage(
        runtime,
        CompatAbiSmokeStage::ManifestLoad,
        manifest_name,
        None,
        Some(manifest_path),
        None,
        None,
    );
    let manifest = match game_manifest_load(runtime, manifest_path) {
        Ok(manifest) => manifest,
        Err(code) => {
            compat_abi_write_failure(
                runtime,
                CompatAbiSmokeStage::ManifestLoad,
                manifest_name,
                None,
                Some(manifest_path),
                code,
            );
            return Err(code);
        }
    };
    let target_name = ngos_game_compat_runtime::compat_target_name(manifest.target);
    let GameCompatLaunchedAbiObservation {
        session,
        observation,
    } = compat_abi_launch_observed(runtime, current_cwd, &manifest, target_name)?;
    if write_line(runtime, &observation.route_line).is_err() {
        return Err(460);
    }
    if write_line(runtime, &observation.process_line).is_err() {
        return Err(460);
    }
    compat_abi_write_stage(
        runtime,
        CompatAbiSmokeStage::ProcfsProbe,
        target_name,
        Some(session.pid),
        None,
        None,
        None,
    );
    if compat_abi_process_fd_probe(runtime, &session, &observation.compat).is_err() {
        compat_abi_write_failure(
            runtime,
            CompatAbiSmokeStage::ProcfsProbe,
            target_name,
            Some(session.pid),
            None,
            464,
        );
        return Err(464);
    }
    compat_abi_write_stage(
        runtime,
        CompatAbiSmokeStage::Accepted,
        target_name,
        Some(session.pid),
        None,
        None,
        None,
    );
    Ok(session)
}

fn compat_abi_prepare_scenarios<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> Result<ngos_game_compat_runtime::CompatAbiScenarioSet, ExitCode> {
    for (index, path) in compat_abi_required_dirs().iter().enumerate() {
        if runtime.mkdir_path(path).is_err() {
            return Err(452 + index as i32);
        }
    }
    let scenarios = compat_abi_smoke_scenarios();
    let manifests = compat_abi_scenario_manifests(&scenarios);
    for (index, manifest_spec) in manifests.iter().enumerate() {
        if shell_write_file(runtime, manifest_spec.path, manifest_spec.text).is_err() {
            return Err(454 + index as i32);
        }
    }
    Ok(scenarios)
}

fn compat_abi_finish_sessions<B: SyscallBackend>(
    runtime: &Runtime<B>,
    manifest_paths: &[&str],
    mut sessions: Vec<Box<GameCompatSession>>,
) -> ExitCode {
    let mut session_refs = sessions
        .iter_mut()
        .map(|session| &mut **session)
        .collect::<Vec<_>>();
    if game_stop_sessions(runtime, &mut session_refs).is_err() {
        return 461;
    }
    if write_line(runtime, &compat_abi_cleanup_line(sessions.len())).is_err() {
        return 462;
    }
    let cleanup_sessions = sessions
        .iter()
        .map(|session| &**session)
        .collect::<Vec<_>>();
    game_cleanup_sessions_and_paths(runtime, &cleanup_sessions, manifest_paths);
    if write_line(runtime, compat_abi_completion_line()).is_err() {
        return 463;
    }
    0
}

fn compat_abi_write_core_report<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    let core_report = build_compat_abi_core_smoke_report()?;
    for line in [
        core_report.handle_line.as_str(),
        core_report.path_line.as_str(),
        core_report.sched_line.as_str(),
        core_report.sync_line.as_str(),
        core_report.timer_line.as_str(),
        core_report.module_line.as_str(),
        core_report.refusal_line.as_str(),
        core_report.recovery_line.as_str(),
    ] {
        if write_line(runtime, line).is_err() {
            return Err(451);
        }
    }
    Ok(())
}

fn compat_abi_run_manifest_scenarios<B: SyscallBackend>(
    runtime: &Runtime<B>,
    abi_scenarios: &ngos_game_compat_runtime::CompatAbiScenarioSet,
) -> ExitCode {
    let abi_manifests = compat_abi_scenario_manifests(abi_scenarios);
    let mut cwd = String::from("/");
    let mut sessions = Vec::new();
    for manifest_spec in abi_manifests {
        match compat_abi_process_manifest(runtime, &mut cwd, manifest_spec.path) {
            Ok(session) => sessions.push(session),
            Err(code) => return code,
        }
    }
    let cleanup_paths = abi_manifests.map(|manifest| manifest.path);
    compat_abi_finish_sessions(runtime, &cleanup_paths, sessions)
}

fn compat_foreign_write_reclaim<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    let reclaimed = runtime.reclaim_memory_pressure_global(3).map_err(|_| 465)?;
    if write_line(runtime, &compat_foreign_reclaim_line(reclaimed)).is_err() {
        return Err(467);
    }
    Ok(())
}

fn compat_foreign_run_step<B: SyscallBackend>(
    runtime: &Runtime<B>,
    step: fn(&Runtime<B>) -> ExitCode,
    reclaim_after: bool,
) -> ExitCode {
    let code = step(runtime);
    if code != 0 {
        return code;
    }
    if reclaim_after {
        if let Err(code) = compat_foreign_write_reclaim(runtime) {
            return code;
        }
    }
    0
}

pub fn run_native_compat_abi_boot_smoke<B: SyscallBackend>(runtime: &Runtime<B>) -> ExitCode {
    if write_line(runtime, compat_abi_boot_proof_line()).is_err() {
        return 410;
    }
    if let Err(code) = compat_abi_write_core_report(runtime) {
        return code;
    }

    let abi_scenarios = match compat_abi_prepare_scenarios(runtime) {
        Ok(scenarios) => scenarios,
        Err(code) => return code,
    };
    compat_abi_run_manifest_scenarios(runtime, &abi_scenarios)
}

pub fn run_native_compat_foreign_boot_smoke<B: SyscallBackend>(runtime: &Runtime<B>) -> ExitCode {
    if write_line(runtime, compat_foreign_boot_proof_line()).is_err() {
        return 468;
    }
    let steps: [fn(&Runtime<B>) -> ExitCode; 2] = [
        run_native_compat_abi_boot_smoke,
        run_native_compat_foreign_loader_boot_smoke,
    ];
    for (index, step) in steps.iter().enumerate() {
        let code = compat_foreign_run_step(runtime, *step, index == 0);
        if code != 0 {
            return code;
        }
    }
    if write_line(runtime, compat_foreign_completion_line()).is_err() {
        return 468;
    }
    0
}
