use alloc::vec::Vec;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::simulation_report_support::{GameQualityReport, render_quality_report};
use crate::{
    GameCompatSession, ensure_simulation_session, run_simulation_frames, write_simulation_start,
};

pub fn handle_game_simulate<B: SyscallBackend>(
    runtime: &Runtime<B>,
    current_cwd: &str,
    game_sessions: &mut Vec<GameCompatSession>,
    target: &str,
    frame_count: usize,
) -> Result<(), ExitCode> {
    let session_idx = ensure_simulation_session(runtime, current_cwd, game_sessions, target)?;

    let session = &mut game_sessions[session_idx];
    write_simulation_start(runtime, session, frame_count)?;
    let (total_latency, max_latency, budget_hits, backpressure_events) =
        run_simulation_frames(runtime, session, frame_count);

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

    render_quality_report(runtime, &report)
}
