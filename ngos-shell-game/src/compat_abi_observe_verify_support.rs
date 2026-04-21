use alloc::string::String;

use ngos_game_compat_runtime::{
    CompatAbiProcessMismatch, GameCompatManifest, compat_abi_process_line, compat_abi_route_line,
    compat_abi_verify_payload, compat_abi_verify_process_record,
};
use ngos_user_abi::{NativeProcessCompatRecord, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::{GameCompatSession, game_compat_abi_payload};

pub struct GameCompatAbiSessionObservation {
    pub compat: NativeProcessCompatRecord,
    pub route_line: String,
    pub process_line: String,
}

pub enum GameCompatAbiSessionObservationError {
    ReadAbi,
    InvalidAbiPayload,
    InspectCompat,
    ProcessRecord(CompatAbiProcessMismatch),
}

pub fn game_compat_observe_abi_session<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
    manifest: &GameCompatManifest,
) -> Result<GameCompatAbiSessionObservation, GameCompatAbiSessionObservationError> {
    let abi_payload = game_compat_abi_payload(runtime, session)
        .map_err(|_| GameCompatAbiSessionObservationError::ReadAbi)?;
    if !compat_abi_verify_payload(manifest, &abi_payload) {
        return Err(GameCompatAbiSessionObservationError::InvalidAbiPayload);
    }
    let compat = runtime
        .inspect_process_compat(session.pid)
        .map_err(|_| GameCompatAbiSessionObservationError::InspectCompat)?;
    if let Err(mismatch) = compat_abi_verify_process_record(manifest, &compat) {
        return Err(GameCompatAbiSessionObservationError::ProcessRecord(
            mismatch,
        ));
    }
    Ok(GameCompatAbiSessionObservation {
        route_line: compat_abi_route_line(session.pid, manifest, &session.runtime_abi_path),
        process_line: compat_abi_process_line(session.pid, &compat),
        compat,
    })
}
