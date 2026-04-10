#![cfg_attr(not(test), no_std)]

//! Canonical subsystem role:
//! - subsystem: native control surface and userland orchestration
//! - owner layer: Layer 3
//! - semantic owner: `userland-native`
//! - truth path role: native consumer and operator on top of `user-runtime`
//!
//! Canonical contract families consumed or exposed here:
//! - shell and command contracts
//! - native control contracts
//! - repair / modernization control contracts
//! - proof and runtime demonstration contracts
//!
//! This crate may orchestrate, inspect, and operate real `ngos` behavior, but
//! it must not redefine kernel truth, ABI truth, or subsystem ownership that
//! belongs to lower layers.

extern crate alloc;

mod analysis_agents;
mod boot_desktop_agents;
mod boot_desktop_frame_agents;
mod boot_shell_agents;
mod bootstrap_contract_agents;
mod bus_smoke_agents;
mod compat_device_smoke_agents;
mod compat_foreign_smoke_agents;
mod compat_loader_smoke_agents;
mod compat_probe_agents;
mod device_runtime_smoke_agents;
mod fd_agents;
mod game_agents;
mod game_boot_agents;
mod gpu_agents;
mod intent_agents;
mod network_hardware_interface_smoke_agents;
mod network_hardware_rx_smoke_agents;
mod network_hardware_smoke_agents;
mod network_hardware_tx_smoke_agents;
mod network_hardware_udp_rx_smoke_agents;
mod network_hardware_udp_tx_smoke_agents;
mod network_smoke_agents;
mod nextmind_agents;
mod path_agents;
mod process_exec_smoke_agents;
mod proof_agents;
mod proof_support_agents;
mod render3d_smoke_agents;
mod resource_agents;
mod resource_smoke_agents;
mod runtime_entry_agents;
mod scheduler_smoke_agents;
mod semantic_agents;
mod session_agents;
mod shell_command_dispatch_agents;
mod shell_front_dispatch_agents;
mod shell_lang;
mod shell_loop_agents;
mod shell_semantic_dispatch_agents;
mod shell_state_agents;
mod shell_support_agents;
mod storage_block_agents;
mod storage_smoke_agents;
mod surface_agents;
mod surface_smoke_agents;
mod surface_smoke_command_agents;
mod vfs_agents;
mod vfs_smoke_agents;
mod vm_smoke_agents;
mod wasm_smoke_agents;
mod workflow_agents;

use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
use analysis_agents::*;
use boot_desktop_agents::*;
use boot_desktop_frame_agents::*;
use boot_shell_agents::*;
use bootstrap_contract_agents::*;
use bus_smoke_agents::*;
use compat_device_smoke_agents::*;
use compat_probe_agents::*;
use device_runtime_smoke_agents::*;
use fd_agents::*;
use game_agents::*;
use game_boot_agents::*;
use network_hardware_interface_smoke_agents::*;
use network_hardware_rx_smoke_agents::*;
use network_hardware_smoke_agents::*;
use network_hardware_tx_smoke_agents::*;
use network_hardware_udp_rx_smoke_agents::*;
use network_hardware_udp_tx_smoke_agents::*;
use network_smoke_agents::*;
use nextmind_agents::{NextMindAutoState, NextMindDecisionReport, nextmind_drain_auto_events};
#[cfg(test)]
use nextmind_agents::{
    nextmind_auto_summary, nextmind_auto_triggered, nextmind_channel_for_metrics,
    nextmind_explain_last, nextmind_metrics_score, nextmind_subscribe_auto_streams,
    test_nextmind_metrics,
};
use ngos_audio_translate::MixScript;
use ngos_game_compat_runtime::{CompatLaneKind, GameCompatManifest};
use ngos_gfx_translate::{DrawOp, FrameScript, RgbaColor};
use ngos_input_translate::InputScript;
#[cfg(test)]
use ngos_shell_network::shell_wait_event_queue;
use ngos_shell_network::wait_for_network_event;
use ngos_shell_proc::{
    fixed_text_field, read_procfs_all, shell_render_procfs_path, shell_resolve_self_pid,
};
use ngos_shell_proof::build_device_runtime_smoke_report;
use ngos_shell_types::*;
use ngos_shell_ux::shell_render_unknown_command_feedback;
use ngos_shell_vfs::{
    shell_append_file, shell_copy_file, shell_read_file_text, shell_write_all, shell_write_file,
};
use ngos_user_abi::bootstrap::{
    BootOutcomePolicy, SessionContext, parse_boot_context, parse_session_context,
};
use ngos_user_abi::{
    BOOT_ARG_FLAG, BOOT_ENV_PROOF_PREFIX, BootSessionStage, BootSessionStatus, BootstrapArgs,
    Errno, ExitCode, FcntlCmd, NATIVE_BLOCK_IO_MAGIC, NATIVE_BLOCK_IO_OP_READ,
    NATIVE_BLOCK_IO_VERSION, NativeBlockIoRequest, NativeContractKind, NativeContractState,
    NativeEventQueueMode, NativeEventRecord, NativeEventSourceKind, NativeMountPropagationMode,
    NativeNetworkEventKind, NativeObjectKind, NativeProcessCompatRecord,
    NativeResourceArbitrationPolicy, NativeResourceContractPolicy, NativeResourceGovernanceMode,
    NativeResourceIssuerPolicy, NativeResourceKind, NativeResourceState, POLLIN, POLLOUT, POLLPRI,
    SeekWhence, SyscallBackend,
};
#[cfg(test)]
use ngos_user_runtime::system_control::{
    CapabilityToken, EventFilter, PressureState, SemanticVerdict, SystemController,
};
use ngos_user_runtime::{
    ResourceCancelOutcome, ResourceClaimOutcome, ResourceReleaseOutcome, Runtime,
    WASM_BOOT_PROOF_COMPONENT, WASM_PROCESS_IDENTITY_COMPONENT, WasmCapability, WasmExecutionError,
    execute_wasm_component,
    system_control::{AdaptiveState, SemanticContext, SemanticFeedbackStore},
};
use process_exec_smoke_agents::*;
use proof_agents::*;
use proof_support_agents::*;
use render3d_smoke_agents::*;
use resource_smoke_agents::*;
use runtime_entry_agents::*;
use scheduler_smoke_agents::*;
use semantic_agents::SemanticEntityEpoch;
use shell_command_dispatch_agents::*;
use shell_front_dispatch_agents::*;
use shell_lang::*;
use shell_loop_agents::*;
use shell_semantic_dispatch_agents::*;
use shell_support_agents::*;
use storage_block_agents::*;
use storage_smoke_agents::*;
use surface_smoke_agents::*;
use surface_smoke_command_agents::*;
use vfs_smoke_agents::*;
use vm_smoke_agents::*;
use wasm_smoke_agents::*;
use workflow_agents::*;
pub const PROGRAM_NAME: &str = "ngos-userland-native";
const LEGACY_PROGRAM_NAME: &str = "userland-native";
const COMPAT_WORKER_ARG: &str = "--compat-worker";
const COMPAT_PROC_PROBE_ARG: &str = "--compat-proc-probe";

pub fn main<B: SyscallBackend>(runtime: &Runtime<B>, bootstrap: &BootstrapArgs<'_>) -> ExitCode {
    let session_context = if !bootstrap.is_boot_mode() {
        parse_session_context(bootstrap).ok()
    } else {
        None
    };
    if let Some(context) = &session_context {
        if context.protocol != "kernel-launch" {
            return 127;
        }
        if context.page_size != 4096 || context.entry == 0 {
            return 128;
        }
        if context.process_name != PROGRAM_NAME {
            return 129;
        }
        if context.root_mount_path != "/" || context.root_mount_name != "rootfs" {
            return 130;
        }
        let _ = runtime.report_boot_session(
            BootSessionStatus::Success,
            BootSessionStage::Bootstrap,
            0,
            context.image_base,
        );
        let _ = runtime.report_boot_session(
            BootSessionStatus::Success,
            BootSessionStage::NativeRuntime,
            0,
            context.entry,
        );
    }
    let code = if let Some(context) = &session_context {
        run_session_shell(runtime, context)
    } else {
        run_program(runtime, bootstrap)
    };
    if let Some(context) = &session_context {
        let status = if code == 0 {
            BootSessionStatus::Success
        } else {
            BootSessionStatus::Failure
        };
        let _ =
            runtime.report_boot_session(status, BootSessionStage::Complete, code, context.phnum);
    }
    code
}

#[cfg(test)]
mod tests;
