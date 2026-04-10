use ngos_gfx_translate::EncodedFrame;
use ngos_user_abi::{ExitCode, NativeEventQueueMode, POLLPRI, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{
    GameCompatSession, drain_graphics_driver_requests, shell_gpu_present_encoded,
    shell_wait_event_queue,
};

fn game_wait_graphics_completion<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
    encoded: &EncodedFrame,
    watch_token_suffix: u64,
) -> Result<bool, ExitCode> {
    let queue_fd = runtime
        .create_event_queue(NativeEventQueueMode::Kqueue)
        .map_err(|_| 298)?;
    let watch_token = ((session.pid & 0xffff_ffff) << 32) | watch_token_suffix;
    runtime
        .watch_graphics_events(
            queue_fd,
            &session.graphics_device_path,
            watch_token,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            POLLPRI,
        )
        .map_err(|_| 298)?;
    let presented =
        shell_gpu_present_encoded(runtime, &session.graphics_device_path, encoded).is_ok();
    shell_wait_event_queue(runtime, queue_fd)?;
    runtime
        .remove_graphics_events(queue_fd, &session.graphics_device_path, watch_token)
        .map_err(|_| 299)?;
    Ok(presented)
}

pub fn game_submit_frame_completion<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
    encoded: &EncodedFrame,
) -> Result<(bool, &'static str), ExitCode> {
    match encoded.completion.as_str() {
        "fire-and-forget" => Ok((false, "submitted")),
        "wait-present" => Ok((
            game_wait_graphics_completion(runtime, session, encoded, 0x4758_0001u64)?,
            "graphics-event-present",
        )),
        "wait-complete" => {
            let presented =
                game_wait_graphics_completion(runtime, session, encoded, 0x4758_0002u64)?;
            drain_graphics_driver_requests(
                runtime,
                &session.graphics_driver_path,
                &encoded.frame_tag,
                &encoded.payload,
            )?;
            Ok((presented, "graphics-event-complete"))
        }
        _ => Err(291),
    }
}
