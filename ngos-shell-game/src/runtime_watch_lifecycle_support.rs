use ngos_game_compat_runtime::CompatLaneKind;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::GameCompatSession;
use crate::runtime_watch_graphics_support::{
    game_start_graphics_watch, game_stop_graphics_watch, game_wait_graphics_watch,
};
use crate::runtime_watch_resource_support::{
    game_start_resource_watch, game_stop_resource_watch, game_wait_resource_watch,
};

pub fn game_start_watch<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &mut GameCompatSession,
    kind: CompatLaneKind,
) -> Result<(usize, u64), ExitCode> {
    match kind {
        CompatLaneKind::Graphics => game_start_graphics_watch(runtime, session),
        CompatLaneKind::Audio | CompatLaneKind::Input => {
            game_start_resource_watch(runtime, session, kind)
        }
    }
}

pub fn game_stop_watch<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &mut GameCompatSession,
    kind: CompatLaneKind,
) -> Result<(), ExitCode> {
    match kind {
        CompatLaneKind::Graphics => game_stop_graphics_watch(runtime, session),
        CompatLaneKind::Audio | CompatLaneKind::Input => {
            game_stop_resource_watch(runtime, session, kind)
        }
    }
}

pub fn game_wait_watch<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
    kind: CompatLaneKind,
) -> Result<(), ExitCode> {
    match kind {
        CompatLaneKind::Graphics => game_wait_graphics_watch(runtime, session),
        CompatLaneKind::Audio | CompatLaneKind::Input => {
            game_wait_resource_watch(runtime, session, kind)
        }
    }
}
