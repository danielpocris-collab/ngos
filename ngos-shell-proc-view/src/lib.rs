//! Canonical subsystem role:
//! - subsystem: shell process control and inspection crate
//! - owner layer: Layer 3
//! - semantic owner: `ngos-shell-proc-view`
//! - truth path role: process and job-control command surfaces for the ngos native shell

#![no_std]
extern crate alloc;

use alloc::format;
use alloc::vec::Vec;

use ngos_shell_game::GameCompatSession;
use ngos_shell_proc::{read_process_text, shell_render_procfs_path};
use ngos_shell_types::{ShellJob, parse_u64_arg};
use ngos_shell_vfs::{shell_emit_lines, shell_read_file_text};
use ngos_user_abi::bootstrap::SessionContext;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

const PROC_VIEW_USAGE: &str = "usage: proc <pid> <status|stat|cmdline|cwd|environ|exe|auxv|maps|vmobjects|vmdecisions|vmepisodes|fd|caps|queues>";

fn write_line<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 196)?;
    Ok(())
}

fn game_overlay_path<'a>(session: &'a GameCompatSession, section: &str) -> Option<&'a str> {
    match section {
        "cmdline" => Some(session.runtime_argv_path.as_str()),
        "environ" => Some(session.runtime_env_path.as_str()),
        _ => None,
    }
}

fn is_supported_proc_section(section: &str) -> bool {
    matches!(
        section,
        "status"
            | "stat"
            | "cmdline"
            | "cwd"
            | "environ"
            | "exe"
            | "auxv"
            | "maps"
            | "vmobjects"
            | "vmdecisions"
            | "vmepisodes"
            | "fd"
            | "caps"
            | "queues"
            | "mounts"
    )
}

fn shell_render_game_process_view<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
    section: &str,
) -> Result<bool, ExitCode> {
    match section {
        "cmdline" | "environ" => {
            let procfs_path = format!("/proc/{}/{}", session.pid, section);
            if shell_render_procfs_path(runtime, &procfs_path).is_ok() {
                return Ok(true);
            }
            let Some(overlay_path) = game_overlay_path(session, section) else {
                return Ok(false);
            };
            let text = shell_read_file_text(runtime, overlay_path)?;
            shell_emit_lines(runtime, &text)?;
            Ok(true)
        }
        "cwd" => {
            let cwd = read_process_text(runtime, session.pid, Runtime::get_process_cwd)?;
            shell_emit_lines(runtime, &cwd)?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn shell_render_process_view<B: SyscallBackend>(
    runtime: &Runtime<B>,
    game_sessions: &[GameCompatSession],
    pid: u64,
    section: &str,
) -> Result<(), ExitCode> {
    if let Some(session) = game_sessions.iter().find(|session| session.pid == pid)
        && shell_render_game_process_view(runtime, session, section)?
    {
        return Ok(());
    }
    if is_supported_proc_section(section) {
        shell_render_procfs_path(runtime, &format!("/proc/{pid}/{section}"))
    } else {
        Err(230)
    }
}

pub fn try_handle_proc_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    context: &SessionContext,
    cwd: &str,
    line: &str,
    jobs: &mut Vec<ShellJob>,
    game_sessions: &[GameCompatSession],
    last_spawned_pid: &mut Option<u64>,
) -> Option<Result<(), ExitCode>> {
    if let Some(rest) = line.strip_prefix("proc ") {
        let mut parts = rest.split_whitespace();
        let pid = match parse_u64_arg(parts.next()) {
            Some(pid) => pid,
            None => {
                let _ = write_line(runtime, PROC_VIEW_USAGE);
                return Some(Err(2));
            }
        };
        let section = match parts.next() {
            Some(section) => section,
            None => {
                let _ = write_line(runtime, PROC_VIEW_USAGE);
                return Some(Err(2));
            }
        };
        return Some(
            shell_render_process_view(runtime, game_sessions, pid, section).map_err(|_| 205),
        );
    }
    ngos_shell_proc::try_handle_proc_agent_command(
        runtime,
        context,
        cwd,
        line,
        jobs,
        last_spawned_pid,
    )
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec::Vec;

    use super::{PROC_VIEW_USAGE, game_overlay_path, is_supported_proc_section};
    use ngos_game_compat_runtime::{
        AbiRoutingPlan, CompatTargetKind, GraphicsApi, GraphicsBackend, GraphicsTranslationPlan,
        LoaderRoutingPlan,
    };
    use ngos_shell_game::GameCompatSession;

    fn fake_session() -> GameCompatSession {
        GameCompatSession {
            target: CompatTargetKind::Game,
            title: String::from("Orbit Runner"),
            slug: String::from("orbit-runner"),
            pid: 77,
            domain_id: 41,
            process_name: String::from("compat-orbit-runner"),
            executable_path: String::from("/bin/worker"),
            working_dir: String::from("/games/orbit"),
            prefix_path: String::from("/compat/orbit"),
            saves_path: String::from("/saves/orbit"),
            cache_path: String::from("/cache/orbit"),
            runtime_env_path: String::from("/runtime/orbit.env"),
            runtime_argv_path: String::from("/runtime/orbit.argv"),
            runtime_channel_path: String::from("/runtime/orbit.channel"),
            runtime_loader_path: String::from("/runtime/orbit.loader"),
            runtime_abi_path: String::from("/runtime/orbit.abi"),
            loader_preloads: Vec::new(),
            loader_dll_overrides: Vec::new(),
            loader_env_overrides: Vec::new(),
            loader_routing: LoaderRoutingPlan {
                route_class: "compat-game-runtime",
                launch_mode: "compat-shim",
                entry_profile: "dx-to-vulkan-entry",
                bootstrap_profile: "shim-heavy",
                entrypoint: "/compat/bin/game-entry",
                requires_compat_shims: true,
            },
            abi_routing: AbiRoutingPlan {
                route_class: "compat-game-abi",
                handle_profile: "win32-game-handles",
                path_profile: "prefix-overlay-paths",
                scheduler_profile: "latency-game-scheduler",
                sync_profile: "event-heavy-sync",
                timer_profile: "frame-budget-timers",
                module_profile: "game-module-registry",
                event_profile: "game-window-events",
                requires_kernel_abi_shims: true,
            },
            graphics_device_path: String::from("/dev/gpu0"),
            graphics_driver_path: String::from("/drv/gpu"),
            graphics_source_api: GraphicsApi::DirectX12,
            graphics_translation: GraphicsTranslationPlan {
                source_api: GraphicsApi::DirectX12,
                backend: GraphicsBackend::Vulkan,
                translation: "compat-to-vulkan",
                source_api_name: "directx12",
                backend_name: "vulkan",
            },
            graphics_profile: String::from("frame-pace"),
            audio_device_path: String::from("/dev/audio0"),
            audio_driver_path: String::from("/drv/audio"),
            audio_profile: String::from("spatial-mix"),
            input_device_path: String::from("/dev/input0"),
            input_driver_path: String::from("/drv/input"),
            input_profile: String::from("gamepad-first"),
            last_frame_tag: None,
            last_graphics_queue: None,
            last_present_mode: None,
            last_completion_mode: None,
            last_completion_observed: None,
            last_frame_op_count: 0,
            last_frame_payload_bytes: 0,
            last_graphics_deep_ops: None,
            submitted_frames: 0,
            presented_frames: 0,
            last_presented: false,
            pending_graphics_frames: Vec::new(),
            last_audio_stream_tag: None,
            last_audio_route: None,
            last_audio_latency_mode: None,
            last_audio_spatialization: None,
            last_audio_completion_mode: None,
            last_audio_completion_observed: None,
            last_audio_op_count: 0,
            last_audio_payload_bytes: 0,
            submitted_audio_batches: 0,
            last_audio_invoke_token: None,
            pending_audio_batches: Vec::new(),
            last_input_frame_tag: None,
            last_input_family: None,
            last_input_layout: None,
            last_input_key_table: None,
            last_pointer_capture: None,
            last_input_delivery_mode: None,
            last_input_delivery_observed: None,
            last_input_op_count: 0,
            last_input_payload_bytes: 0,
            submitted_input_batches: 0,
            last_input_invoke_token: None,
            pending_input_batches: Vec::new(),
            lanes: Vec::new(),
            stopped: false,
            exit_code: None,
        }
    }

    #[test]
    fn supported_proc_sections_include_mounts_and_vm_views() {
        assert!(is_supported_proc_section("mounts"));
        assert!(is_supported_proc_section("vmobjects"));
        assert!(is_supported_proc_section("vmdecisions"));
        assert!(!is_supported_proc_section("unknown"));
    }

    #[test]
    fn game_overlay_paths_route_cmdline_and_environ_only() {
        let session = fake_session();
        assert_eq!(
            game_overlay_path(&session, "cmdline"),
            Some("/runtime/orbit.argv")
        );
        assert_eq!(
            game_overlay_path(&session, "environ"),
            Some("/runtime/orbit.env")
        );
        assert_eq!(game_overlay_path(&session, "cwd"), None);
    }

    #[test]
    fn proc_view_usage_is_stable() {
        assert!(PROC_VIEW_USAGE.starts_with("usage: proc <pid> <status|stat|cmdline|cwd"));
        assert!(PROC_VIEW_USAGE.contains("vmepisodes"));
        assert!(PROC_VIEW_USAGE.contains("queues>"));
    }
}
