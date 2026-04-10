use ngos_game_compat_runtime::CompatLaneKind;
use ngos_shell_types::ShellJob;
use ngos_user_abi::{ExitCode, NativeContractState, NativeResourceState, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, game_render_session, game_session_lane, game_stop_watch};

pub fn game_stop_session<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &mut GameCompatSession,
) -> Result<(), ExitCode> {
    if session.stopped {
        return Err(295);
    }
    runtime.send_signal(session.pid, 15).map_err(|_| 288)?;
    let exit_code = runtime.reap_process(session.pid).map_err(|_| 288)?;
    session.exit_code = Some(exit_code);
    session.stopped = true;
    for kind in [
        CompatLaneKind::Graphics,
        CompatLaneKind::Audio,
        CompatLaneKind::Input,
    ] {
        let watch_active = game_session_lane(session, kind)?.watch_queue_fd.is_some();
        if watch_active {
            game_stop_watch(runtime, session, kind)?;
        }
    }
    session.pending_graphics_frames.clear();
    session.pending_audio_batches.clear();
    session.pending_input_batches.clear();
    for lane in &session.lanes {
        if lane.claim_acquired {
            let _ = runtime.release_resource(lane.contract_id);
        }
        let _ = runtime.set_contract_state(lane.contract_id, NativeContractState::Suspended);
        let _ = runtime.set_resource_state(lane.resource_id, NativeResourceState::Suspended);
    }
    Ok(())
}

pub fn shell_cleanup_game_sessions<B: SyscallBackend>(
    runtime: &Runtime<B>,
    game_sessions: &mut [GameCompatSession],
    jobs: &mut [ShellJob],
) {
    for session in game_sessions {
        if session.stopped {
            continue;
        }
        if game_stop_session(runtime, session).is_ok() {
            if let Some(job) = jobs.iter_mut().find(|job| job.pid == session.pid) {
                job.reaped_exit = session.exit_code;
            }
            let _ = game_render_session(runtime, session);
        }
    }
}
