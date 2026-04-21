use alloc::format;
use alloc::string::String;
use alloc::vec;

use ngos_gfx_translate::{DrawOp, FrameScript, RgbaColor};
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, write_line};

pub fn write_simulation_start<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
    frame_count: usize,
) -> Result<(), ExitCode> {
    write_line(
        runtime,
        &format!(
            "game-simulate starting slug={} frames={frame_count} pid={}",
            session.slug, session.pid
        ),
    )
}

pub fn simulation_frame_script(session: &GameCompatSession, index: usize) -> FrameScript {
    FrameScript {
        width: 1280,
        height: 720,
        frame_tag: format!("{}-frame-{:03}", session.slug, index),
        queue: String::from("graphics"),
        present_mode: String::from("mailbox"),
        completion: String::from("wait-complete"),
        ops: vec![DrawOp::Clear {
            color: RgbaColor {
                r: (index % 255) as u8,
                g: 0,
                b: 0,
                a: 255,
            },
        }],
    }
}
