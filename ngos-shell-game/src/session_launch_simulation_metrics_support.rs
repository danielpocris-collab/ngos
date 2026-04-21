use ngos_user_abi::SyscallBackend;
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatSession, game_encode_frame, game_submit_frame,
    session_launch_simulation_script_support::simulation_frame_script,
};

pub fn run_simulation_frames<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &mut GameCompatSession,
    frame_count: usize,
) -> (u64, u64, usize, usize) {
    let mut total_latency = 0u64;
    let mut max_latency = 0u64;
    let mut budget_hits = 0usize;
    let mut backpressure_events = 0usize;

    for i in 0..frame_count {
        let script = simulation_frame_script(session, i);
        let Ok(encoded) = game_encode_frame(session, &script) else {
            budget_hits += 1;
            continue;
        };
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

    (total_latency, max_latency, budget_hits, backpressure_events)
}
