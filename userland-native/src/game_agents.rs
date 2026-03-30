use super::*;
use ngos_gfx_translate::{DrawOp, RgbaColor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameQualityReport {
    pub title: String,
    pub slug: String,
    pub frames_submitted: usize,
    pub frames_presented: usize,
    pub max_latency: u64,
    pub avg_latency: u64,
    pub budget_hits: usize,
    pub backpressure_events: usize,
}

pub fn try_handle_game_agent_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &mut String,
    line: &str,
    game_sessions: &mut Vec<GameCompatSession>,
    jobs: &mut Vec<ShellJob>,
    last_spawned_pid: &mut Option<u64>,
    last_status: &mut ExitCode,
) -> Option<Result<(), ExitCode>> {
    if let Some(path) = line.strip_prefix("game-manifest ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_manifest(runtime, current_cwd, path.trim()),
        ));
    }
    if let Some(path) = line.strip_prefix("game-plan ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_plan(runtime, current_cwd, path.trim()),
        ));
    }
    if let Some(path) = line.strip_prefix("game-launch ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_launch(
                runtime,
                current_cwd,
                path.trim(),
                game_sessions,
                jobs,
                last_spawned_pid,
            ),
        ));
    }
    if line == "game-status" {
        return Some(settle_game_command_status(
            last_status,
            handle_game_status(runtime, game_sessions),
        ));
    }
    if line == "game-sessions" {
        return Some(settle_game_command_status(
            last_status,
            handle_game_sessions(runtime, game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-stop ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_stop(runtime, rest.trim(), game_sessions, jobs),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-gfx-plan ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_gfx_plan(runtime, current_cwd, rest, game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-gfx-submit ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_gfx_submit(runtime, current_cwd, rest, game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-next ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_next(runtime, rest.trim(), game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-gfx-status ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_gfx_status(runtime, rest.trim(), game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-gfx-next ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_gfx_next(runtime, rest.trim(), game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-audio-plan ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_audio_plan(runtime, current_cwd, rest, game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-audio-submit ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_audio_submit(runtime, current_cwd, rest, game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-audio-status ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_audio_status(runtime, rest.trim(), game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-audio-next ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_audio_next(runtime, rest.trim(), game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-input-plan ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_input_plan(runtime, current_cwd, rest, game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-input-submit ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_input_submit(runtime, current_cwd, rest, game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-input-status ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_input_status(runtime, rest.trim(), game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-input-next ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_input_next(runtime, rest.trim(), game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-watch-start ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_watch_start(runtime, rest, game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-watch-status ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_watch_status(runtime, rest, game_sessions),
        ));
    }
    if line == "game-watch-status-all" {
        return Some(settle_game_command_status(
            last_status,
            handle_game_watch_status_all(runtime, game_sessions),
        ));
    }
    if line == "game-watch-poll-all" {
        return Some(settle_game_command_status(
            last_status,
            handle_game_watch_poll_all(runtime, game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-watch-wait ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_watch_wait(runtime, rest, game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-watch-stop ") {
        return Some(settle_game_command_status(
            last_status,
            handle_game_watch_stop(runtime, rest, game_sessions),
        ));
    }
    if let Some(rest) = line.strip_prefix("game-simulate ") {
        let mut parts = rest.split_whitespace();
        let slug = parts.next()?.to_string();
        let frame_count = parts
            .next()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(60);
        return Some(settle_game_command_status(
            last_status,
            handle_game_simulate(runtime, current_cwd, game_sessions, &slug, frame_count),
        ));
    }
    None
}

fn settle_game_command_status(
    last_status: &mut ExitCode,
    result: Result<(), ExitCode>,
) -> Result<(), ExitCode> {
    *last_status = match result {
        Ok(()) => 0,
        Err(code) => code,
    };
    Ok(())
}

fn handle_game_manifest<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    path: &str,
) -> Result<(), ExitCode> {
    let resolved = resolve_shell_path(current_cwd, path);
    let manifest = game_manifest_load(runtime, &resolved)?;
    game_render_manifest(runtime, &resolved, &manifest)
}

fn handle_game_plan<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    path: &str,
) -> Result<(), ExitCode> {
    let resolved = resolve_shell_path(current_cwd, path);
    let manifest = game_manifest_load(runtime, &resolved)?;
    let plan = manifest.session_plan();
    game_render_plan(runtime, &plan)
}

fn handle_game_launch<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &mut String,
    path: &str,
    game_sessions: &mut Vec<GameCompatSession>,
    jobs: &mut Vec<ShellJob>,
    last_spawned_pid: &mut Option<u64>,
) -> Result<(), ExitCode> {
    let resolved = resolve_shell_path(current_cwd, path);
    let manifest = game_manifest_load(runtime, &resolved)?;
    let session = game_launch_session(runtime, current_cwd, &manifest)?;
    *last_spawned_pid = Some(session.pid);
    jobs.push(ShellJob {
        pid: session.pid,
        name: session.process_name.clone(),
        path: session.executable_path.clone(),
        reaped_exit: None,
        signal_count: 0,
    });
    write_line(
        runtime,
        &format!(
            "game.launched pid={} title={} slug={} cwd={} exec={}",
            session.pid, session.title, session.slug, session.working_dir, session.executable_path
        ),
    )?;
    game_sessions.push(*session);
    Ok(())
}

fn handle_game_status<B: SyscallBackend>(
    runtime: &Runtime<B>,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    if game_sessions.is_empty() {
        write_line(runtime, "game.sessions=0")?;
        return Ok(());
    }
    for session in game_sessions {
        game_render_session(runtime, session)?;
    }
    Ok(())
}

fn handle_game_sessions<B: SyscallBackend>(
    runtime: &Runtime<B>,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    if game_sessions.is_empty() {
        write_line(runtime, "game.sessions=0")?;
        return Ok(());
    }
    for session in game_sessions {
        game_render_session_summary(runtime, session)?;
    }
    Ok(())
}

fn handle_game_stop<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &mut [GameCompatSession],
    jobs: &mut [ShellJob],
) -> Result<(), ExitCode> {
    let pid = parse_game_pid_arg(runtime, rest, "usage: game-stop <pid>")?;
    let Some(session) = game_sessions.iter_mut().find(|session| session.pid == pid) else {
        return game_session_missing(runtime, pid);
    };
    game_stop_session(runtime, session)?;
    if let Some(job) = jobs.iter_mut().find(|job| job.pid == pid) {
        job.reaped_exit = session.exit_code;
    }
    game_render_session(runtime, session)
}

fn handle_game_gfx_plan<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    rest: &str,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    let (pid, resolved) = parse_game_pid_script_args(
        runtime,
        current_cwd,
        rest,
        "usage: game-gfx-plan <pid> <frame-script>",
    )?;
    let session = find_game_session(runtime, game_sessions, pid)?;
    let script = game_load_frame_script(runtime, &resolved)?;
    let encoded = game_encode_frame(session, &script)?;
    write_line(
        runtime,
        &format!(
            "game.gfx.plan pid={} frame={} ops={} bytes={} device={} profile={} queue={} present-mode={} completion={}",
            pid,
            encoded.frame_tag,
            encoded.op_count,
            encoded.payload.len(),
            session.graphics_device_path,
            session.graphics_profile,
            encoded.queue,
            encoded.present_mode,
            encoded.completion
        ),
    )
}

fn handle_game_gfx_submit<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    rest: &str,
    game_sessions: &mut [GameCompatSession],
) -> Result<(), ExitCode> {
    let (pid, resolved) = parse_game_pid_script_args(
        runtime,
        current_cwd,
        rest,
        "usage: game-gfx-submit <pid> <frame-script>",
    )?;
    let session = find_game_session_mut(runtime, game_sessions, pid)?;
    let script = game_load_frame_script(runtime, &resolved)?;
    let encoded = game_encode_frame(session, &script)?;
    let (presented, completion_observed) = game_submit_frame(runtime, session, &encoded)?;
    write_line(
        runtime,
        &format!(
            "game.gfx.submit pid={} frame={} ops={} bytes={} submitted={} presented={} present-ok={} queue={} present-mode={} completion={} completion-observed={}",
            pid,
            encoded.frame_tag,
            encoded.op_count,
            encoded.payload.len(),
            session.submitted_frames,
            session.presented_frames,
            presented,
            encoded.queue,
            encoded.present_mode,
            encoded.completion,
            completion_observed
        ),
    )
}

fn handle_game_next<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &mut [GameCompatSession],
) -> Result<(), ExitCode> {
    let pid = parse_game_pid_arg(runtime, rest, "usage: game-next <pid>")?;
    let session = find_game_session_mut(runtime, game_sessions, pid)?;
    match game_next_payload(runtime, session) {
        Ok(()) => Ok(()),
        Err(code) => {
            if code == 299 {
                write_line(
                    runtime,
                    &format!(
                        "game.next pid={} depth[gfx={};audio={};input={}]",
                        session.pid,
                        session.pending_graphics_frames.len(),
                        session.pending_audio_batches.len(),
                        session.pending_input_batches.len()
                    ),
                )?;
            }
            Err(code)
        }
    }
}

fn handle_game_gfx_status<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    let pid = parse_game_pid_arg(runtime, rest, "usage: game-gfx-status <pid>")?;
    let session = find_game_session(runtime, game_sessions, pid)?;
    write_line(
        runtime,
        &format!(
            "game.gfx.status pid={} device={} driver={} profile={} submitted={} frames={} presented={} last-frame={} queue={} present-mode={} completion={} completion-observed={} ops={} bytes={}",
            pid,
            session.graphics_device_path,
            session.graphics_driver_path,
            session.graphics_profile,
            session.submitted_frames,
            session.presented_frames,
            session.last_presented,
            session
                .last_frame_tag
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_graphics_queue
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_present_mode
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_completion_mode
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_completion_observed
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session.last_frame_op_count,
            session.last_frame_payload_bytes
        ),
    )
}

fn handle_game_gfx_next<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &mut [GameCompatSession],
) -> Result<(), ExitCode> {
    let pid = parse_game_pid_arg(runtime, rest, "usage: game-gfx-next <pid>")?;
    let session = find_game_session_mut(runtime, game_sessions, pid)?;
    if session.pending_graphics_frames.is_empty() {
        write_line(
            runtime,
            &format!("game.gfx.queue pid={} depth=0", session.pid),
        )?;
        return Err(299);
    }
    let encoded = session.pending_graphics_frames.remove(0);
    write_line(
        runtime,
        &format!(
            "game.gfx.next pid={} frame={} queue={} present-mode={} completion={} remaining={} payload={}",
            session.pid,
            encoded.frame_tag,
            encoded.queue,
            encoded.present_mode,
            encoded.completion,
            session.pending_graphics_frames.len(),
            encoded.payload
        ),
    )
}

fn handle_game_audio_plan<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    rest: &str,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    let (pid, resolved) = parse_game_pid_script_args(
        runtime,
        current_cwd,
        rest,
        "usage: game-audio-plan <pid> <mix-script>",
    )?;
    let session = find_game_session(runtime, game_sessions, pid)?;
    let script = game_load_mix_script(runtime, &resolved)?;
    let encoded = game_encode_mix(session, &script)?;
    write_line(
        runtime,
        &format!(
            "game.audio.plan pid={} stream={} ops={} bytes={} profile={} route={} latency-mode={} spatialization={}",
            pid,
            encoded.stream_tag,
            encoded.op_count,
            encoded.payload.len(),
            session.audio_profile,
            encoded.route,
            encoded.latency_mode,
            encoded.spatialization
        ),
    )
}

fn handle_game_audio_submit<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    rest: &str,
    game_sessions: &mut [GameCompatSession],
) -> Result<(), ExitCode> {
    let (pid, resolved) = parse_game_pid_script_args(
        runtime,
        current_cwd,
        rest,
        "usage: game-audio-submit <pid> <mix-script>",
    )?;
    let session = find_game_session_mut(runtime, game_sessions, pid)?;
    let script = game_load_mix_script(runtime, &resolved)?;
    let encoded = game_encode_mix(session, &script)?;
    let (token, completion_observed) = game_submit_mix(runtime, session, &encoded)?;
    write_line(
        runtime,
        &format!(
            "game.audio.submit pid={} stream={} ops={} bytes={} batches={} token={} route={} latency-mode={} spatialization={} completion={} completion-observed={}",
            pid,
            encoded.stream_tag,
            encoded.op_count,
            encoded.payload.len(),
            session.submitted_audio_batches,
            token,
            encoded.route,
            encoded.latency_mode,
            encoded.spatialization,
            encoded.completion,
            completion_observed
        ),
    )
}

fn handle_game_audio_status<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    let pid = parse_game_pid_arg(runtime, rest, "usage: game-audio-status <pid>")?;
    let session = find_game_session(runtime, game_sessions, pid)?;
    let lane = game_session_lane(session, CompatLaneKind::Audio)?;
    let device = runtime
        .inspect_device(&session.audio_device_path)
        .map_err(|_| 246)?;
    let driver = runtime
        .inspect_driver(&session.audio_driver_path)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "game.audio.status pid={} device={} driver={} profile={} claimed={} token={} batches={} stream={} route={} latency-mode={} spatialization={} completion={} completion-observed={} ops={} bytes={} device-queue={}/{} device-submitted={} device-completed={} driver-queued={} driver-inflight={} driver-completed={}",
            pid,
            session.audio_device_path,
            session.audio_driver_path,
            session.audio_profile,
            lane.claim_acquired,
            session
                .last_audio_invoke_token
                .map(|token| token.to_string())
                .unwrap_or_else(|| String::from("pending")),
            session.submitted_audio_batches,
            session
                .last_audio_stream_tag
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_audio_route
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_audio_latency_mode
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_audio_spatialization
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_audio_completion_mode
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_audio_completion_observed
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session.last_audio_op_count,
            session.last_audio_payload_bytes,
            device.queue_depth,
            device.queue_capacity,
            device.submitted_requests,
            device.completed_requests,
            driver.queued_requests,
            driver.in_flight_requests,
            driver.completed_requests
        ),
    )
}

fn handle_game_audio_next<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &mut [GameCompatSession],
) -> Result<(), ExitCode> {
    let pid = parse_game_pid_arg(runtime, rest, "usage: game-audio-next <pid>")?;
    let session = find_game_session_mut(runtime, game_sessions, pid)?;
    if session.pending_audio_batches.is_empty() {
        write_line(
            runtime,
            &format!("game.audio.queue pid={} depth=0", session.pid),
        )?;
        return Err(299);
    }
    let encoded = session.pending_audio_batches.remove(0);
    write_line(
        runtime,
        &format!(
            "game.audio.next pid={} stream={} route={} latency-mode={} spatialization={} completion={} remaining={} payload={}",
            session.pid,
            encoded.stream_tag,
            encoded.route,
            encoded.latency_mode,
            encoded.spatialization,
            encoded.completion,
            session.pending_audio_batches.len(),
            encoded.payload
        ),
    )
}

fn handle_game_input_plan<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    rest: &str,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    let (pid, resolved) = parse_game_pid_script_args(
        runtime,
        current_cwd,
        rest,
        "usage: game-input-plan <pid> <input-script>",
    )?;
    let session = find_game_session(runtime, game_sessions, pid)?;
    let script = game_load_input_script(runtime, &resolved)?;
    let encoded = game_encode_input(session, &script)?;
    write_line(
        runtime,
        &format!(
            "game.input.plan pid={} frame={} ops={} bytes={} profile={} family={} layout={} key-table={} pointer-capture={} delivery={}",
            pid,
            encoded.frame_tag,
            encoded.op_count,
            encoded.payload.len(),
            session.input_profile,
            encoded.device_family,
            encoded.layout,
            encoded.key_table,
            encoded.pointer_capture,
            encoded.delivery
        ),
    )
}

fn handle_game_input_submit<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    rest: &str,
    game_sessions: &mut [GameCompatSession],
) -> Result<(), ExitCode> {
    let (pid, resolved) = parse_game_pid_script_args(
        runtime,
        current_cwd,
        rest,
        "usage: game-input-submit <pid> <input-script>",
    )?;
    let session = find_game_session_mut(runtime, game_sessions, pid)?;
    let script = game_load_input_script(runtime, &resolved)?;
    let encoded = game_encode_input(session, &script)?;
    let (token, delivery_observed) = game_submit_input(runtime, session, &encoded)?;
    write_line(
        runtime,
        &format!(
            "game.input.submit pid={} frame={} ops={} bytes={} batches={} token={} delivery={} delivery-observed={}",
            pid,
            encoded.frame_tag,
            encoded.op_count,
            encoded.payload.len(),
            session.submitted_input_batches,
            token,
            encoded.delivery,
            delivery_observed
        ),
    )
}

fn handle_game_input_status<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    let pid = parse_game_pid_arg(runtime, rest, "usage: game-input-status <pid>")?;
    let session = find_game_session(runtime, game_sessions, pid)?;
    let lane = game_session_lane(session, CompatLaneKind::Input)?;
    let device = runtime
        .inspect_device(&session.input_device_path)
        .map_err(|_| 246)?;
    let driver = runtime
        .inspect_driver(&session.input_driver_path)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "game.input.status pid={} device={} driver={} profile={} claimed={} token={} batches={} frame={} family={} layout={} key-table={} pointer-capture={} delivery={} delivery-observed={} ops={} bytes={} device-queue={}/{} device-submitted={} device-completed={} driver-queued={} driver-inflight={} driver-completed={}",
            pid,
            session.input_device_path,
            session.input_driver_path,
            session.input_profile,
            lane.claim_acquired,
            session
                .last_input_invoke_token
                .map(|token| token.to_string())
                .unwrap_or_else(|| String::from("pending")),
            session.submitted_input_batches,
            session
                .last_input_frame_tag
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_input_family
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_input_layout
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_input_key_table
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_pointer_capture
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_input_delivery_mode
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session
                .last_input_delivery_observed
                .clone()
                .unwrap_or_else(|| String::from("-")),
            session.last_input_op_count,
            session.last_input_payload_bytes,
            device.queue_depth,
            device.queue_capacity,
            device.submitted_requests,
            device.completed_requests,
            driver.queued_requests,
            driver.in_flight_requests,
            driver.completed_requests
        ),
    )
}

fn handle_game_input_next<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &mut [GameCompatSession],
) -> Result<(), ExitCode> {
    let pid = parse_game_pid_arg(runtime, rest, "usage: game-input-next <pid>")?;
    let session = find_game_session_mut(runtime, game_sessions, pid)?;
    if session.pending_input_batches.is_empty() {
        write_line(runtime, &format!("game.input.queue pid={} depth=0", pid))?;
        return Err(299);
    }
    let encoded = session.pending_input_batches.remove(0);
    write_line(
        runtime,
        &format!(
            "game.input.next pid={} frame={} family={} layout={} delivery={} remaining={} payload={}",
            pid,
            encoded.frame_tag,
            encoded.device_family,
            encoded.layout,
            encoded.delivery,
            session.pending_input_batches.len(),
            encoded.payload
        ),
    )
}

fn handle_game_watch_start<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &mut [GameCompatSession],
) -> Result<(), ExitCode> {
    let (pid, kind) = parse_game_pid_lane_args(
        runtime,
        rest,
        "usage: game-watch-start <pid> <graphics|audio|input>",
    )?;
    let Some(session) = game_sessions.iter_mut().find(|session| session.pid == pid) else {
        return game_session_missing(runtime, pid);
    };
    let (queue_fd, token) = game_start_watch(runtime, session, kind)?;
    write_line(
        runtime,
        &format!(
            "game.watch.start pid={} kind={} queue={} token={}",
            pid,
            lane_name(kind),
            queue_fd,
            token
        ),
    )
}

fn handle_game_watch_status<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    let (pid, kind) = parse_game_pid_lane_args(
        runtime,
        rest,
        "usage: game-watch-status <pid> <graphics|audio|input>",
    )?;
    let Some(session) = game_sessions.iter().find(|session| session.pid == pid) else {
        return game_session_missing(runtime, pid);
    };
    let lane = game_session_lane(session, kind)?;
    write_line(
        runtime,
        &format!(
            "game.watch.status pid={} kind={} queue={} token={}",
            pid,
            lane_name(kind),
            lane.watch_queue_fd
                .map(|fd| fd.to_string())
                .unwrap_or_else(|| String::from("inactive")),
            lane.watch_token
                .map(|token| token.to_string())
                .unwrap_or_else(|| String::from("inactive"))
        ),
    )
}

fn handle_game_watch_status_all<B: SyscallBackend>(
    runtime: &Runtime<B>,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    if game_sessions.is_empty() {
        write_line(runtime, "game.watch.sessions=0")?;
        return Ok(());
    }
    for session in game_sessions {
        game_render_watch_summary(runtime, session)?;
    }
    Ok(())
}

fn handle_game_watch_poll_all<B: SyscallBackend>(
    runtime: &Runtime<B>,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    game_poll_all_watches(runtime, game_sessions).map(|_| ())
}

fn handle_game_watch_wait<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &[GameCompatSession],
) -> Result<(), ExitCode> {
    let (pid, kind) = parse_game_pid_lane_args(
        runtime,
        rest,
        "usage: game-watch-wait <pid> <graphics|audio|input>",
    )?;
    let Some(session) = game_sessions.iter().find(|session| session.pid == pid) else {
        return game_session_missing(runtime, pid);
    };
    game_wait_watch(runtime, session, kind)
}

fn handle_game_watch_stop<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    game_sessions: &mut [GameCompatSession],
) -> Result<(), ExitCode> {
    let (pid, kind) = parse_game_pid_lane_args(
        runtime,
        rest,
        "usage: game-watch-stop <pid> <graphics|audio|input>",
    )?;
    let Some(session) = game_sessions.iter_mut().find(|session| session.pid == pid) else {
        return game_session_missing(runtime, pid);
    };
    game_stop_watch(runtime, session, kind)?;
    write_line(
        runtime,
        &format!("game.watch.stop pid={} kind={}", pid, lane_name(kind)),
    )
}

fn handle_game_simulate<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    game_sessions: &mut Vec<GameCompatSession>,
    target: &str,
    frame_count: usize,
) -> Result<(), ExitCode> {
    let session_key = game_simulation_key(target);
    let manifest_path = game_simulation_manifest_path(current_cwd, target);

    // 1. Find or Launch Session
    let session_idx = if let Some(idx) = game_sessions.iter().position(|s| s.slug == session_key) {
        idx
    } else {
        let manifest = game_manifest_load(runtime, &manifest_path)?;
        let session = game_launch_session(runtime, &mut current_cwd.to_string(), &manifest)?;
        game_sessions.push(*session);
        game_sessions.len() - 1
    };

    let session = &mut game_sessions[session_idx];
    let pid = session.pid;

    write_line(
        runtime,
        &format!(
            "game-simulate starting slug={} frames={frame_count} pid={pid}",
            session.slug
        ),
    )?;

    let mut total_latency = 0u64;
    let mut max_latency = 0u64;
    let mut budget_hits = 0usize;
    let mut backpressure_events = 0usize;

    // 2. Simulation Loop
    for i in 0..frame_count {
        let frame_tag = format!("{}-frame-{:03}", session.slug, i);

        // Simulate a frame script
        let script = FrameScript {
            width: 1280,
            height: 720,
            frame_tag: frame_tag.clone(),
            queue: String::from("graphics"),
            present_mode: String::from("mailbox"),
            completion: String::from("wait-complete"),
            ops: vec![DrawOp::Clear {
                color: RgbaColor {
                    r: (i % 255) as u8,
                    g: 0,
                    b: 0,
                    a: 255,
                },
            }],
        };

        let encoded = game_encode_frame(session, &script)?;

        let start_tick = runtime
            .inspect_system_snapshot()
            .map(|s| s.current_tick)
            .unwrap_or(0);

        match game_submit_frame(runtime, session, &encoded) {
            Ok(_) => {
                let end_tick = runtime
                    .inspect_system_snapshot()
                    .map(|s| s.current_tick)
                    .unwrap_or(0);
                let latency = end_tick.saturating_sub(start_tick);
                total_latency += latency;
                max_latency = max_latency.max(latency);

                // Inspect backpressure
                if let Ok(record) = runtime.inspect_device(&session.graphics_device_path) {
                    if record.queue_depth >= record.queue_capacity {
                        backpressure_events += 1;
                    }
                }
            }
            Err(_) => {
                budget_hits += 1;
            }
        }
    }

    let report = GameQualityReport {
        title: session.title.clone(),
        slug: session.slug.clone(),
        frames_submitted: frame_count,
        frames_presented: session.presented_frames as usize,
        max_latency,
        avg_latency: if frame_count > 0 {
            total_latency / frame_count as u64
        } else {
            0
        },
        budget_hits,
        backpressure_events,
    };

    render_quality_report(runtime, &report)?;

    Ok(())
}

fn parse_game_pid_arg<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    usage: &str,
) -> Result<u64, ExitCode> {
    match rest.trim().parse::<u64>().ok() {
        Some(pid) => Ok(pid),
        None => {
            let _ = write_line(runtime, usage);
            Err(2)
        }
    }
}

fn parse_game_pid_lane_args<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    usage: &str,
) -> Result<(u64, CompatLaneKind), ExitCode> {
    let mut parts = rest.split_whitespace();
    let pid = match parts.next().and_then(|value| value.parse::<u64>().ok()) {
        Some(pid) => pid,
        None => {
            let _ = write_line(runtime, usage);
            return Err(2);
        }
    };
    let Some(kind) = parts.next().and_then(parse_game_lane_kind) else {
        let _ = write_line(runtime, usage);
        return Err(2);
    };
    Ok((pid, kind))
}

fn parse_game_pid_script_args<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    rest: &str,
    usage: &str,
) -> Result<(u64, String), ExitCode> {
    let mut parts = rest.split_whitespace();
    let pid = match parts.next().and_then(|value| value.parse::<u64>().ok()) {
        Some(pid) => pid,
        None => {
            let _ = write_line(runtime, usage);
            return Err(2);
        }
    };
    let Some(script_path) = parts.next() else {
        let _ = write_line(runtime, usage);
        return Err(2);
    };
    Ok((pid, resolve_shell_path(current_cwd, script_path)))
}

fn find_game_session<'a, B: SyscallBackend>(
    runtime: &Runtime<B>,
    game_sessions: &'a [GameCompatSession],
    pid: u64,
) -> Result<&'a GameCompatSession, ExitCode> {
    game_sessions
        .iter()
        .find(|session| session.pid == pid)
        .ok_or_else(|| {
            let _ = write_line(runtime, &format!("game.session-missing pid={pid}"));
            2
        })
}

fn find_game_session_mut<'a, B: SyscallBackend>(
    runtime: &Runtime<B>,
    game_sessions: &'a mut [GameCompatSession],
    pid: u64,
) -> Result<&'a mut GameCompatSession, ExitCode> {
    game_sessions
        .iter_mut()
        .find(|session| session.pid == pid)
        .ok_or_else(|| {
            let _ = write_line(runtime, &format!("game.session-missing pid={pid}"));
            2
        })
}

fn game_session_missing<B: SyscallBackend>(runtime: &Runtime<B>, pid: u64) -> Result<(), ExitCode> {
    let _ = write_line(runtime, &format!("game.session-missing pid={pid}"));
    Err(2)
}

fn game_simulation_key(target: &str) -> &str {
    let without_manifest = target.strip_suffix(".manifest").unwrap_or(target);
    without_manifest
        .rsplit('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .unwrap_or(without_manifest)
}

fn game_simulation_manifest_path(current_cwd: &str, target: &str) -> String {
    if target.ends_with(".manifest") {
        return resolve_shell_path(current_cwd, target);
    }
    let without_trailing = target.trim_end_matches('/');
    let with_manifest = format!("{without_trailing}.manifest");
    resolve_shell_path(current_cwd, &with_manifest)
}

fn render_quality_report<B: SyscallBackend>(
    runtime: &Runtime<B>,
    report: &GameQualityReport,
) -> Result<(), ExitCode> {
    write_line(runtime, "== GAME QUALITY REPORT ==")?;
    write_line(runtime, &format!("title: {}", report.title))?;
    write_line(runtime, &format!("slug:  {}", report.slug))?;
    write_line(
        runtime,
        &format!("frames_submitted: {}", report.frames_submitted),
    )?;
    write_line(
        runtime,
        &format!("frames_presented: {}", report.frames_presented),
    )?;
    write_line(
        runtime,
        &format!("max_latency_ticks: {}", report.max_latency),
    )?;
    write_line(
        runtime,
        &format!("avg_latency_ticks: {}", report.avg_latency),
    )?;
    write_line(runtime, &format!("budget_hits: {}", report.budget_hits))?;
    write_line(
        runtime,
        &format!("backpressure_events: {}", report.backpressure_events),
    )?;

    let quality_score = if report.frames_submitted > 0 {
        let base = (report.frames_presented as f32 / report.frames_submitted as f32) * 100.0;
        let penalty = (report.budget_hits as f32 * 5.0) + (report.backpressure_events as f32 * 2.0);
        (base - penalty).max(0.0)
    } else {
        0.0
    };

    write_line(runtime, &format!("quality_score: {:.2}", quality_score))?;
    write_line(runtime, "== END REPORT ==")?;
    Ok(())
}
