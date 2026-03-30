#![cfg_attr(not(test), no_std)]

extern crate alloc;

mod analysis_agents;
mod code_agents;
mod dev_agents;
mod edit_agents;
mod fd_agents;
mod game_agents;
mod gpu_agents;
mod intent_agents;
mod network_agents;
mod nextmind_agents;
mod path_agents;
mod proc_agents;
mod project_agents;
mod resource_agents;
mod rust_agents;
mod semantic_agents;
mod session_agents;
mod shell_lang;
mod shell_state_agents;
mod surface_agents;
mod vfs_agents;
mod vm_agents;
mod workflow_agents;

use alloc::boxed::Box;
use alloc::fmt::Write;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
use analysis_agents::*;
use code_agents::*;
use dev_agents::*;
use edit_agents::*;
use fd_agents::*;
use game_agents::*;
use ngos_audio_translate::{EncodedMix, MixScript};
use ngos_game_compat_runtime::{
    CompatLaneKind, GameCompatManifest, GameSessionPlan, audio_backend_name, graphics_backend_name,
    input_backend_name, lane_name,
};
use ngos_gfx_translate::{DrawOp, EncodedFrame, FrameScript, RgbaColor};
use ngos_input_translate::{EncodedInput, InputScript};
use ngos_user_abi::bootstrap::{
    BootOutcomePolicy, SessionContext, parse_boot_context, parse_session_context,
};
use ngos_user_abi::{
    BOOT_ARG_FLAG, BOOT_ENV_PROOF_PREFIX, BootSessionStage, BootSessionStatus, BootstrapArgs,
    Errno, ExitCode, FcntlCmd, NATIVE_BLOCK_IO_MAGIC, NATIVE_BLOCK_IO_OP_READ,
    NATIVE_BLOCK_IO_VERSION, NativeBlockIoRequest, NativeContractKind, NativeContractState,
    NativeDeviceRecord, NativeDeviceRequestRecord, NativeDriverRecord, NativeEventQueueMode,
    NativeEventRecord, NativeEventSourceKind, NativeGpuScanoutRecord, NativeGraphicsEventKind,
    NativeNetworkEventKind, NativeNetworkInterfaceRecord, NativeNetworkSocketRecord,
    NativeObjectKind, NativeProcessRecord, NativeReadinessRecord, NativeResourceArbitrationPolicy,
    NativeResourceContractPolicy, NativeResourceGovernanceMode, NativeResourceIssuerPolicy,
    NativeResourceKind, NativeResourceState, NativeSchedulerClass, POLLIN, POLLOUT, POLLPRI,
    SyscallBackend,
};
use ngos_user_runtime::{
    ResourceCancelOutcome, ResourceClaimOutcome, ResourceReleaseOutcome, Runtime,
    WASM_BOOT_PROOF_COMPONENT, WASM_PROCESS_IDENTITY_COMPONENT, WasmCapability, WasmExecutionError,
    execute_wasm_component,
    system_control::{
        AdaptiveState, AdaptiveStateSnapshot, CapabilityToken, DeviceHandle, EventFilter,
        EventSemantic, EventStream, PressureState, ProcessAction, ProcessEntity, ProcessHandle,
        ResourceContract, ResourceUpdate, SemanticActionRecord, SemanticContext, SemanticEntity,
        SemanticFeedbackStore, SemanticVerdict, SystemController, SystemFact,
        SystemPressureMetrics, cpu_mask_for, event_source_name, load_percent,
        pressure_channel_name, select_cpu, semantic_capabilities_csv, semantic_class_name,
        semantic_entity_kind_name, semantic_verdict_name,
    },
};
use project_agents::*;
use rust_agents::*;
use shell_lang::*;
use vm_agents::*;
use workflow_agents::*;
pub const PROGRAM_NAME: &str = "ngos-userland-native";
const LEGACY_PROGRAM_NAME: &str = "userland-native";
const COMPAT_WORKER_ARG: &str = "--compat-worker";

fn image_path_matches_program(image_path: &str) -> bool {
    matches!(
        image_path,
        PROGRAM_NAME | LEGACY_PROGRAM_NAME | "/bin/ngos-userland-native" | "/bin/userland-native"
    ) || image_path
        .rsplit('/')
        .next()
        .is_some_and(|name| matches!(name, PROGRAM_NAME | LEGACY_PROGRAM_NAME))
}

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

fn write_line<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 190)?;
    Ok(())
}

struct StackLineBuffer<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> StackLineBuffer<N> {
    fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }

    fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    fn push_byte(&mut self, byte: u8) -> Result<(), ExitCode> {
        if self.len == N {
            return Err(190);
        }
        self.bytes[self.len] = byte;
        self.len += 1;
        Ok(())
    }

    fn push_bytes(&mut self, bytes: &[u8]) -> Result<(), ExitCode> {
        if self.len + bytes.len() > N {
            return Err(190);
        }
        self.bytes[self.len..self.len + bytes.len()].copy_from_slice(bytes);
        self.len += bytes.len();
        Ok(())
    }

    fn push_str(&mut self, text: &str) -> Result<(), ExitCode> {
        self.push_bytes(text.as_bytes())
    }

    fn push_bool(&mut self, value: bool) -> Result<(), ExitCode> {
        if value {
            self.push_bytes(b"true")
        } else {
            self.push_bytes(b"false")
        }
    }

    fn push_u64(&mut self, value: u64) -> Result<(), ExitCode> {
        self.push_usize(value as usize)
    }

    fn push_i32(&mut self, value: i32) -> Result<(), ExitCode> {
        if value < 0 {
            self.push_byte(b'-')?;
            self.push_u64(value.unsigned_abs() as u64)
        } else {
            self.push_u64(value as u64)
        }
    }

    fn push_usize(&mut self, mut value: usize) -> Result<(), ExitCode> {
        if value == 0 {
            return self.push_byte(b'0');
        }
        let mut digits = [0u8; 20];
        let mut count = 0usize;
        while value != 0 {
            digits[count] = b'0' + (value % 10) as u8;
            count += 1;
            value /= 10;
        }
        while count != 0 {
            count -= 1;
            self.push_byte(digits[count])?;
        }
        Ok(())
    }
}

impl<const N: usize> Write for StackLineBuffer<N> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        if self.len + bytes.len() > N {
            return Err(core::fmt::Error);
        }
        self.bytes[self.len..self.len + bytes.len()].copy_from_slice(bytes);
        self.len += bytes.len();
        Ok(())
    }
}

fn path_contains_all_markers<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    markers: &[&str],
) -> Result<bool, ExitCode> {
    const MAX_MARKERS: usize = 8;
    let fd = runtime.open_path(path).map_err(|_| 237)?;
    if markers.len() > MAX_MARKERS {
        let _ = runtime.close(fd);
        return Err(241);
    }
    let mut marker_bytes = [&[][..]; MAX_MARKERS];
    let mut max_marker_len = 0usize;
    for (index, marker) in markers.iter().enumerate() {
        let bytes = marker.as_bytes();
        marker_bytes[index] = bytes;
        max_marker_len = max_marker_len.max(bytes.len());
    }
    if max_marker_len > 128 {
        let _ = runtime.close(fd);
        return Err(241);
    }
    let tail_len = max_marker_len.saturating_sub(1);
    let mut seen = [false; MAX_MARKERS];
    let mut tail = [0u8; 128];
    let mut tail_count = 0usize;
    let mut buffer = [0u8; 256];
    let mut window = [0u8; 384];
    loop {
        let count = runtime.read(fd, &mut buffer).map_err(|_| 238)?;
        if count == 0 {
            break;
        }
        let window_len = tail_count + count;
        window[..tail_count].copy_from_slice(&tail[..tail_count]);
        window[tail_count..window_len].copy_from_slice(&buffer[..count]);
        let haystack = &window[..window_len];
        for (index, marker) in marker_bytes[..markers.len()].iter().enumerate() {
            if !seen[index]
                && haystack
                    .windows(marker.len())
                    .any(|candidate| candidate == *marker)
            {
                seen[index] = true;
            }
        }
        if seen[..markers.len()].iter().all(|seen| *seen) {
            break;
        }
        if tail_len != 0 {
            let keep = window_len.min(tail_len);
            tail[..keep].copy_from_slice(&window[window_len - keep..window_len]);
            tail_count = keep;
        }
    }
    runtime.close(fd).map_err(|_| 240)?;
    Ok(seen[..markers.len()].iter().all(|seen| *seen))
}

fn ensure_vm_smoke_backing_file<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    byte_len: usize,
    fill: u8,
    open_error: ExitCode,
    write_error: ExitCode,
    close_error: ExitCode,
) -> Result<(), ExitCode> {
    if runtime.mkdir_path("/lib").is_err() && runtime.stat_path("/lib").is_err() {
        return Err(open_error);
    }
    if runtime.mkfile_path(path).is_err() && runtime.stat_path(path).is_err() {
        return Err(open_error);
    }
    let fd = runtime.open_path(path).map_err(|_| open_error)?;
    let chunk = [fill; 256];
    let mut written = 0usize;
    while written < byte_len {
        if runtime.write(fd, &chunk).is_err() {
            let _ = runtime.close(fd);
            return Err(write_error);
        }
        written += chunk.len();
    }
    runtime.close(fd).map_err(|_| close_error)?;
    Ok(())
}

fn run_vm_stress_hardening<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
) -> Result<(), ExitCode> {
    let mut cycles = 0u64;
    let mut refusals = 0u64;
    while cycles < 12 {
        let label = format!("boot-vm-stress-{cycles}");
        let start = runtime
            .map_anonymous_memory(pid, 0x2000, true, true, false, &label)
            .map_err(|_| 206)?;
        runtime
            .store_memory_word(pid, start, (cycles + 1) as u32)
            .map_err(|_| 207)?;
        runtime
            .protect_memory_range(pid, start + 0x1000, 0x1000, true, false, false)
            .map_err(|_| 208)?;
        if runtime.store_memory_word(pid, start + 0x1000, 0x55) != Err(Errno::Fault) {
            return Err(209);
        }
        refusals += 1;
        runtime
            .unmap_memory_range(pid, start + 0x1000, 0x1000)
            .map_err(|_| 210)?;
        runtime
            .unmap_memory_range(pid, start, 0x1000)
            .map_err(|_| 211)?;
        cycles += 1;
    }

    write_line(
        runtime,
        &format!("vm.smoke.stress pid={pid} cycles={cycles} refusals={refusals} outcome=ok"),
    )
}

fn run_vm_pressure_hardening<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
) -> Result<(), ExitCode> {
    ensure_vm_smoke_backing_file(
        runtime,
        "/lib/libvm-pressure.so",
        0x4000,
        0x37,
        212,
        213,
        214,
    )?;
    let mapped = runtime
        .map_file_memory(
            pid,
            "/lib/libvm-pressure.so",
            0x3000,
            0x0,
            true,
            false,
            true,
            true,
        )
        .map_err(|_| 215)?;
    runtime
        .protect_memory_range(pid, mapped, 0x3000, true, true, false)
        .map_err(|_| 216)?;
    for (page, value) in [(0u64, 31u32), (0x1000, 32u32), (0x2000, 33u32)] {
        runtime
            .store_memory_word(pid, mapped + page, value)
            .map_err(|_| 217)?;
    }

    let target_pages = 3u64;
    let reclaimed = runtime
        .reclaim_memory_pressure(pid, target_pages)
        .map_err(|_| 218)?;
    if reclaimed < 3 {
        return Err(219);
    }

    let vmdecisions = path_contains_all_markers(
        runtime,
        "/proc/1/vmdecisions",
        &["agent=pressure-trigger", "agent=pressure-victim"],
    )?;
    if !vmdecisions {
        return Err(220);
    }

    let vmobjects = path_contains_all_markers(
        runtime,
        "/proc/1/vmobjects",
        &["/lib/libvm-pressure.so", "resident=0", "dirty=0"],
    )?;
    if !vmobjects {
        return Err(221);
    }

    for page in [0u64, 0x1000, 0x2000] {
        runtime
            .load_memory_word(pid, mapped + page)
            .map_err(|_| 222)?;
    }

    write_line(
        runtime,
        &format!(
            "vm.smoke.pressure pid={pid} target-pages={target_pages} reclaimed-pages={reclaimed} restored=yes outcome=ok"
        ),
    )
}

fn run_vm_global_pressure_hardening<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
) -> Result<(), ExitCode> {
    ensure_vm_smoke_backing_file(
        runtime,
        "/lib/libvm-global-a.so",
        0x4000,
        0x41,
        224,
        225,
        226,
    )?;
    ensure_vm_smoke_backing_file(
        runtime,
        "/lib/libvm-global-b.so",
        0x2000,
        0x52,
        227,
        228,
        229,
    )?;

    let mapped_a = runtime
        .map_file_memory(
            pid,
            "/lib/libvm-global-a.so",
            0x3000,
            0x0,
            true,
            false,
            true,
            true,
        )
        .map_err(|_| 230)?;
    runtime
        .protect_memory_range(pid, mapped_a, 0x3000, true, true, false)
        .map_err(|_| 231)?;
    for (page, value) in [(0u64, 41u32), (0x1000, 42u32), (0x2000, 43u32)] {
        runtime
            .store_memory_word(pid, mapped_a + page, value)
            .map_err(|_| 232)?;
    }

    let child = runtime
        .spawn_process_copy_vm("g", "/g", pid)
        .map_err(|_| 233)?;
    let mapped_b = runtime
        .map_file_memory(
            child,
            "/lib/libvm-global-b.so",
            0x1000,
            0x0,
            true,
            false,
            true,
            true,
        )
        .map_err(|_| 234)?;
    runtime
        .protect_memory_range(child, mapped_b, 0x1000, true, true, false)
        .map_err(|_| 235)?;
    runtime
        .store_memory_word(child, mapped_b, 51)
        .map_err(|_| 236)?;

    let reclaimed = runtime.reclaim_memory_pressure_global(3).map_err(|_| 237)?;
    if reclaimed < 3 {
        return Err(238);
    }

    let parent_vmobjects = path_contains_all_markers(
        runtime,
        "/proc/1/vmobjects",
        &["/lib/libvm-global-a.so", "resident=0", "dirty=0"],
    )?;
    if !parent_vmobjects {
        return Err(239);
    }

    let child_decisions = path_contains_all_markers(
        runtime,
        &format!("/proc/{child}/vmdecisions"),
        &["/lib/libvm-global-b.so", "agent=map-file", "agent=protect"],
    )?;
    if !child_decisions {
        return Err(242);
    }

    let parent_decisions = path_contains_all_markers(
        runtime,
        "/proc/1/vmdecisions",
        &[
            "/lib/libvm-global-a.so",
            "agent=pressure-trigger",
            "agent=pressure-victim",
        ],
    )?;
    if !parent_decisions {
        return Err(243);
    }

    let child_vmobjects = path_contains_all_markers(
        runtime,
        &format!("/proc/{child}/vmobjects"),
        &["/lib/libvm-global-b.so", "resident=1", "dirty=1"],
    )?;
    if !child_vmobjects {
        return Err(245);
    }

    write_line(
        runtime,
        &format!(
            "vm.smoke.pressure.global pid={pid} child={child} target-pages=3 reclaimed-pages={reclaimed} victim=libvm-global-a survivor=libvm-global-b outcome=ok"
        ),
    )
}

fn bytes_contain_all_markers(bytes: &[u8], markers: &[&str]) -> bool {
    markers.iter().all(|marker| {
        let marker = marker.as_bytes();
        bytes
            .windows(marker.len())
            .any(|candidate| candidate == marker)
    })
}

fn parse_exit_code(token: Option<&str>) -> ExitCode {
    token
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(0)
}

fn shell_errno_status(errno: Errno) -> ExitCode {
    257 - i32::from(errno.code())
}

fn shell_report_resource_errno<B: SyscallBackend>(
    runtime: &Runtime<B>,
    operation: &str,
    contract: usize,
    errno: Errno,
) -> Result<ExitCode, ExitCode> {
    write_line(
        runtime,
        &format!(
            "{operation}-refused contract={contract} errno={} code={}",
            errno,
            errno.code()
        ),
    )?;
    Ok(shell_errno_status(errno))
}

fn shell_report_contract_target_errno<B: SyscallBackend>(
    runtime: &Runtime<B>,
    operation: &str,
    contract: usize,
    errno: Errno,
) -> Result<ExitCode, ExitCode> {
    write_line(
        runtime,
        &format!(
            "{operation}-refused contract={contract} errno={} code={}",
            errno,
            errno.code()
        ),
    )?;
    Ok(shell_errno_status(errno))
}

fn shell_report_resource_target_errno<B: SyscallBackend>(
    runtime: &Runtime<B>,
    operation: &str,
    resource: usize,
    errno: Errno,
) -> Result<ExitCode, ExitCode> {
    write_line(
        runtime,
        &format!(
            "{operation}-refused resource={resource} errno={} code={}",
            errno,
            errno.code()
        ),
    )?;
    Ok(shell_errno_status(errno))
}

fn shell_report_transfer_errno<B: SyscallBackend>(
    runtime: &Runtime<B>,
    source: usize,
    target: usize,
    errno: Errno,
) -> Result<ExitCode, ExitCode> {
    write_line(
        runtime,
        &format!(
            "transfer-refused source={source} target={target} errno={} code={}",
            errno,
            errno.code()
        ),
    )?;
    Ok(shell_errno_status(errno))
}

fn bootstrap_env_value<'a>(bootstrap: &'a BootstrapArgs<'_>, key: &str) -> Option<&'a str> {
    bootstrap.envp.iter().find_map(|entry| {
        let (entry_key, entry_value) = entry.split_once('=')?;
        (entry_key == key).then_some(entry_value)
    })
}

fn bootstrap_has_arg(bootstrap: &BootstrapArgs<'_>, needle: &str) -> bool {
    bootstrap.argv.iter().any(|arg| *arg == needle)
}

#[inline(never)]
fn native_game_smoke_image_path<B: SyscallBackend>(runtime: &Runtime<B>) -> String {
    if runtime.stat_path("/bin/ngos-userland-native").is_ok() {
        return String::from("/bin/ngos-userland-native");
    }
    if runtime.stat_path("/bin/userland-native").is_ok() {
        return String::from("/bin/userland-native");
    }
    String::from("/kernel/ngos-userland-native")
}

#[inline(never)]
fn run_native_game_compat_worker<B: SyscallBackend>(
    runtime: &Runtime<B>,
    bootstrap: &BootstrapArgs<'_>,
) -> ExitCode {
    let channel_path = match bootstrap_env_value(bootstrap, "NGOS_GAME_CHANNEL") {
        Some(path) if !path.is_empty() => path,
        _ => return 341,
    };
    if write_line(
        runtime,
        &format!("game.worker.ready channel={channel_path}"),
    )
    .is_err()
    {
        return 342;
    }
    let fd = match runtime.open_path(channel_path) {
        Ok(fd) => fd,
        Err(_) => return 343,
    };
    let mut payload_count = 0usize;
    let mut buffer = [0u8; 512];
    loop {
        let events = match runtime.poll(fd, POLLIN) {
            Ok(events) => events,
            Err(_) => {
                let _ = runtime.close(fd);
                return 344;
            }
        };
        if events & POLLIN == 0 {
            continue;
        }
        let count = match runtime.read(fd, &mut buffer) {
            Ok(count) => count,
            Err(_) => {
                let _ = runtime.close(fd);
                return 345;
            }
        };
        if count == 0 {
            continue;
        }
        payload_count = payload_count.saturating_add(1);
        if write_line(
            runtime,
            &format!("game.worker.payload count={payload_count} bytes={count}"),
        )
        .is_err()
        {
            let _ = runtime.close(fd);
            return 346;
        }
    }
}

#[inline(never)]
fn run_native_game_stack_smoke<B: SyscallBackend>(runtime: &Runtime<B>) -> ExitCode {
    debug_break(0x4e47_4f53_4753_3130, 0);
    let executable_path = native_game_smoke_image_path(runtime);
    debug_break(0x4e47_4f53_4753_3131, 0);
    let manifest_text = format!(
        "title=Orbit Runner\nslug=orbit-runner\nexec={executable_path}\ncwd=/\narg={COMPAT_WORKER_ARG}\ngfx.backend=vulkan\ngfx.profile=frame-pace\naudio.backend=native-mixer\naudio.profile=spatial-mix\ninput.backend=native-input\ninput.profile=gamepad-first\nshim.prefix=/\nshim.saves=/\nshim.cache=/\n"
    );
    let manifest = match GameCompatManifest::parse(&manifest_text) {
        Ok(manifest) => manifest,
        Err(_) => return 348,
    };
    if game_render_manifest(runtime, "/orbit.manifest", &manifest).is_err() {
        return 349;
    }
    if game_render_plan(runtime, &manifest.session_plan()).is_err() {
        return 350;
    }
    debug_break(0x4e47_4f53_4753_3132, 0);
    let mut cwd = String::from("/");
    let mut session = match game_launch_session(runtime, &mut cwd, &manifest) {
        Ok(session) => session,
        Err(code) => return code,
    };
    debug_break(0x4e47_4f53_4753_3133, session.pid);
    if game_render_session(runtime, &session).is_err() {
        return 351;
    }
    let env_text = match shell_read_file_text(runtime, &session.runtime_env_path) {
        Ok(text) => text,
        Err(code) => return code,
    };
    debug_break(0x4e47_4f53_4753_3134, env_text.len() as u64);
    if !bytes_contain_all_markers(env_text.as_bytes(), &["NGOS_GAME_CHANNEL=/session.chan"]) {
        return 354;
    }
    debug_break(0x4e47_4f53_4753_3135, 0);
    let argv_text = match shell_read_file_text(runtime, &session.runtime_argv_path) {
        Ok(text) => text,
        Err(code) => return code,
    };
    debug_break(0x4e47_4f53_4753_3136, argv_text.len() as u64);
    if !bytes_contain_all_markers(
        argv_text.as_bytes(),
        &[executable_path.as_str(), COMPAT_WORKER_ARG],
    ) {
        return 352;
    }
    let frame_script = match FrameScript::parse(
        "surface=1280x720\nframe=orbit-001\nqueue=graphics\npresent-mode=mailbox\ncompletion=fire-and-forget\nclear=#112233\nrect=10,20,200,100,#ff8800ff\n",
    ) {
        Ok(script) => script,
        Err(_) => return 353,
    };
    let encoded_frame = match game_encode_frame(&session, &frame_script) {
        Ok(encoded) => encoded,
        Err(code) => return code,
    };
    debug_break(0x4e47_4f53_4753_3137, encoded_frame.payload.len() as u64);
    if game_submit_frame(runtime, &mut session, &encoded_frame).is_err() {
        return 354;
    }
    let mix_script = match MixScript::parse(
        "rate=48000\nchannels=2\nstream=orbit-intro\nroute=music\nlatency-mode=interactive\nspatialization=world-3d\ncompletion=fire-and-forget\ntone=lead,440,120,0.800,-0.250,sine\n",
    ) {
        Ok(script) => script,
        Err(_) => return 355,
    };
    let encoded_mix = match game_encode_mix(&session, &mix_script) {
        Ok(encoded) => encoded,
        Err(code) => return code,
    };
    debug_break(0x4e47_4f53_4753_3138, encoded_mix.payload.len() as u64);
    if game_submit_mix(runtime, &mut session, &encoded_mix).is_err() {
        return 356;
    }
    let input_script = match InputScript::parse(
        "device=gamepad\nfamily=dualshock\nframe=input-001\nlayout=gamepad-standard\nkey-table=us-game\npointer-capture=relative-lock\ndelivery=immediate\nbutton=cross,press\n",
    ) {
        Ok(script) => script,
        Err(_) => return 357,
    };
    let encoded_input = match game_encode_input(&session, &input_script) {
        Ok(encoded) => encoded,
        Err(code) => return code,
    };
    debug_break(0x4e47_4f53_4753_3139, encoded_input.payload.len() as u64);
    if game_submit_input(runtime, &mut session, &encoded_input).is_err() {
        return 358;
    }
    debug_break(0x4e47_4f53_4753_3140, 0);
    if game_render_session_summary(runtime, &session).is_err() {
        return 359;
    }
    debug_break(0x4e47_4f53_4753_3141, 0);
    let channel_text = match shell_read_file_text(runtime, &session.runtime_channel_path) {
        Ok(text) => text,
        Err(code) => return code,
    };
    debug_break(0x4e47_4f53_4753_3142, channel_text.len() as u64);
    if !bytes_contain_all_markers(
        channel_text.as_bytes(),
        &[
            "kind=graphics tag=orbit-001",
            "kind=audio tag=orbit-intro",
            "kind=input tag=input-001",
        ],
    ) {
        return 360;
    }
    debug_break(0x4e47_4f53_4753_3143, 0);
    if game_stop_session(runtime, &mut session).is_err() {
        return 361;
    }
    debug_break(
        0x4e47_4f53_4753_3144,
        session.exit_code.unwrap_or_default() as u64,
    );
    match game_submit_frame(runtime, &mut session, &encoded_frame) {
        Err(295) => {}
        _ => return 362,
    }
    if game_render_session(runtime, &session).is_err()
        || game_render_session_summary(runtime, &session).is_err()
    {
        return 363;
    }
    if !session.stopped || session.exit_code.is_none() {
        return 364;
    }
    0
}

fn list_process_ids<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<Vec<u64>, ExitCode> {
    let mut ids = vec![0u64; 16];
    loop {
        let count = runtime.list_processes(&mut ids).map_err(|_| 200)?;
        if count <= ids.len() {
            ids.truncate(count);
            return Ok(ids);
        }
        ids.resize(count, 0);
    }
}

fn read_procfs_all<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<Vec<u8>, ExitCode> {
    let mut buffer = vec![0u8; 512];
    loop {
        let count = runtime.read_procfs(path, &mut buffer).map_err(|_| 201)?;
        if count < buffer.len() {
            buffer.truncate(count);
            return Ok(buffer);
        }
        buffer.resize(buffer.len() * 2, 0);
    }
}

fn read_process_text<B: SyscallBackend, F>(
    runtime: &Runtime<B>,
    pid: u64,
    loader: F,
) -> Result<String, ExitCode>
where
    F: Fn(&Runtime<B>, u64, &mut [u8]) -> Result<usize, ngos_user_abi::Errno>,
{
    let mut buffer = vec![0u8; 256];
    loop {
        let count = loader(runtime, pid, &mut buffer).map_err(|_| 202)?;
        if count < buffer.len() {
            buffer.truncate(count);
            return String::from_utf8(buffer).map_err(|_| 202);
        }
        buffer.resize(buffer.len() * 2, 0);
    }
}

fn parse_u64_arg(token: Option<&str>) -> Option<u64> {
    token.and_then(|value| value.parse::<u64>().ok())
}

fn parse_usize_arg(token: Option<&str>) -> Option<usize> {
    token.and_then(|value| value.parse::<usize>().ok())
}

fn parse_u16_arg(token: Option<&str>) -> Option<u16> {
    token.and_then(|value| value.parse::<u16>().ok())
}

fn parse_ipv4(text: &str) -> Option<[u8; 4]> {
    let mut octets = [0u8; 4];
    let mut parts = text.split('.');
    for octet in &mut octets {
        *octet = parts.next()?.parse::<u8>().ok()?;
    }
    if parts.next().is_some() {
        return None;
    }
    Some(octets)
}

fn render_ipv4(addr: [u8; 4]) -> String {
    format!("{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3])
}

fn checksum16(bytes: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut chunks = bytes.chunks_exact(2);
    for chunk in &mut chunks {
        sum = sum.wrapping_add(u16::from_be_bytes([chunk[0], chunk[1]]) as u32);
    }
    if let Some(byte) = chunks.remainder().first() {
        sum = sum.wrapping_add((*byte as u32) << 8);
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

fn build_udp_ipv4_frame(
    src_mac: [u8; 6],
    dst_mac: [u8; 6],
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    src_port: u16,
    dst_port: u16,
    payload: &[u8],
) -> Vec<u8> {
    let udp_len = 8 + payload.len();
    let ip_len = 20 + udp_len;
    let mut frame = Vec::with_capacity(14 + ip_len);
    frame.extend_from_slice(&dst_mac);
    frame.extend_from_slice(&src_mac);
    frame.extend_from_slice(&0x0800u16.to_be_bytes());

    let mut ip_header = [0u8; 20];
    ip_header[0] = 0x45;
    ip_header[2..4].copy_from_slice(&(ip_len as u16).to_be_bytes());
    ip_header[8] = 64;
    ip_header[9] = 17;
    ip_header[12..16].copy_from_slice(&src_ip);
    ip_header[16..20].copy_from_slice(&dst_ip);
    let ip_checksum = checksum16(&ip_header);
    ip_header[10..12].copy_from_slice(&ip_checksum.to_be_bytes());
    frame.extend_from_slice(&ip_header);

    let mut udp = Vec::with_capacity(udp_len);
    udp.extend_from_slice(&src_port.to_be_bytes());
    udp.extend_from_slice(&dst_port.to_be_bytes());
    udp.extend_from_slice(&(udp_len as u16).to_be_bytes());
    udp.extend_from_slice(&0u16.to_be_bytes());
    udp.extend_from_slice(payload);

    let mut pseudo = Vec::with_capacity(12 + udp.len());
    pseudo.extend_from_slice(&src_ip);
    pseudo.extend_from_slice(&dst_ip);
    pseudo.push(0);
    pseudo.push(17);
    pseudo.extend_from_slice(&(udp_len as u16).to_be_bytes());
    pseudo.extend_from_slice(&udp);
    let udp_checksum = checksum16(&pseudo);
    udp[6..8].copy_from_slice(&udp_checksum.to_be_bytes());
    frame.extend_from_slice(&udp);
    frame
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ShellJob {
    pid: u64,
    name: String,
    path: String,
    reaped_exit: Option<i32>,
    signal_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ShellAlias {
    name: String,
    value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ShellVariable {
    name: String,
    value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellMode {
    Direct,
    Semantic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NextMindDecisionReport {
    trigger: PressureState,
    before: SystemPressureMetrics,
    after: SystemPressureMetrics,
    semantic: EventSemantic,
    observation: ngos_user_runtime::system_control::SemanticObservation,
    adaptive: AdaptiveStateSnapshot,
    actions: Vec<SemanticActionRecord>,
    verdict: SemanticVerdict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NextMindAutoState {
    enabled: bool,
    streams: Vec<EventStream>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GameCompatLaneRuntime {
    kind: CompatLaneKind,
    resource_id: usize,
    resource_name: String,
    contract_id: usize,
    contract_label: String,
    claim_acquired: bool,
    invoke_token: Option<usize>,
    watch_queue_fd: Option<usize>,
    watch_token: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GameCompatSession {
    title: String,
    slug: String,
    pid: u64,
    domain_id: usize,
    process_name: String,
    executable_path: String,
    working_dir: String,
    prefix_path: String,
    saves_path: String,
    cache_path: String,
    runtime_env_path: String,
    runtime_argv_path: String,
    runtime_channel_path: String,
    graphics_device_path: String,
    graphics_driver_path: String,
    graphics_profile: String,
    audio_device_path: String,
    audio_driver_path: String,
    audio_profile: String,
    input_device_path: String,
    input_driver_path: String,
    input_profile: String,
    last_frame_tag: Option<String>,
    last_graphics_queue: Option<String>,
    last_present_mode: Option<String>,
    last_completion_mode: Option<String>,
    last_completion_observed: Option<String>,
    last_frame_op_count: usize,
    last_frame_payload_bytes: usize,
    submitted_frames: u64,
    presented_frames: u64,
    last_presented: bool,
    pending_graphics_frames: Vec<EncodedFrame>,
    last_audio_stream_tag: Option<String>,
    last_audio_route: Option<String>,
    last_audio_latency_mode: Option<String>,
    last_audio_spatialization: Option<String>,
    last_audio_completion_mode: Option<String>,
    last_audio_completion_observed: Option<String>,
    last_audio_op_count: usize,
    last_audio_payload_bytes: usize,
    submitted_audio_batches: u64,
    last_audio_invoke_token: Option<usize>,
    pending_audio_batches: Vec<EncodedMix>,
    last_input_frame_tag: Option<String>,
    last_input_family: Option<String>,
    last_input_layout: Option<String>,
    last_input_key_table: Option<String>,
    last_pointer_capture: Option<String>,
    last_input_delivery_mode: Option<String>,
    last_input_delivery_observed: Option<String>,
    last_input_op_count: usize,
    last_input_payload_bytes: usize,
    submitted_input_batches: u64,
    last_input_invoke_token: Option<usize>,
    pending_input_batches: Vec<EncodedInput>,
    lanes: Vec<GameCompatLaneRuntime>,
    stopped: bool,
    exit_code: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SemanticEntityEpoch {
    subject: String,
    policy_fingerprint: u64,
    policy_epoch: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellCommandGuard {
    Always,
    OnSuccess,
    OnFailure,
}

fn normalize_shell_path(path: &str) -> String {
    let mut parts = Vec::<&str>::new();
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            let _ = parts.pop();
            continue;
        }
        parts.push(segment);
    }
    if parts.is_empty() {
        return String::from("/");
    }
    format!("/{}", parts.join("/"))
}

fn resolve_shell_path(cwd: &str, path: &str) -> String {
    if path.is_empty() {
        return normalize_shell_path(cwd);
    }
    if path.starts_with('/') {
        return normalize_shell_path(path);
    }
    if cwd == "/" {
        return normalize_shell_path(&format!("/{}", path));
    }
    normalize_shell_path(&format!("{cwd}/{path}"))
}

fn shell_lookup_variable<'a>(variables: &'a [ShellVariable], name: &str) -> Option<&'a str> {
    variables
        .iter()
        .rev()
        .find(|variable| variable.name == name)
        .map(|variable| variable.value.as_str())
}

fn shell_expand_variables(text: &str, variables: &[ShellVariable]) -> String {
    let bytes = text.as_bytes();
    let mut expanded = String::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'$' {
            let start = index + 1;
            let mut end = start;
            while end < bytes.len()
                && ((bytes[end] as char).is_ascii_alphanumeric() || bytes[end] == b'_')
            {
                end += 1;
            }
            if end > start {
                let name = &text[start..end];
                if let Some(value) = shell_lookup_variable(variables, name) {
                    expanded.push_str(value);
                }
                index = end;
                continue;
            }
        }
        expanded.push(bytes[index] as char);
        index += 1;
    }
    expanded
}

fn shell_expand_aliases(line: &str, aliases: &[ShellAlias]) -> String {
    let mut parts = line.splitn(2, char::is_whitespace);
    let command = match parts.next() {
        Some(command) => command,
        None => return String::new(),
    };
    let rest = parts.next().unwrap_or("").trim_start();
    if let Some(alias) = aliases.iter().rev().find(|alias| alias.name == command) {
        if rest.is_empty() {
            alias.value.clone()
        } else {
            format!("{} {}", alias.value, rest)
        }
    } else {
        line.to_string()
    }
}

fn shell_render_aliases<B: SyscallBackend>(
    runtime: &Runtime<B>,
    aliases: &[ShellAlias],
) -> Result<(), ExitCode> {
    if aliases.is_empty() {
        return write_line(runtime, "aliases=0");
    }
    for alias in aliases {
        write_line(runtime, &format!("alias {}='{}'", alias.name, alias.value))?;
    }
    Ok(())
}

fn shell_render_variables<B: SyscallBackend>(
    runtime: &Runtime<B>,
    variables: &[ShellVariable],
) -> Result<(), ExitCode> {
    if variables.is_empty() {
        return write_line(runtime, "vars=0");
    }
    for variable in variables {
        write_line(
            runtime,
            &format!("var {}={}", variable.name, variable.value),
        )?;
    }
    Ok(())
}

fn shell_render_history<B: SyscallBackend>(
    runtime: &Runtime<B>,
    history: &[String],
) -> Result<(), ExitCode> {
    if history.is_empty() {
        return write_line(runtime, "history=0");
    }
    for (index, entry) in history.iter().enumerate() {
        write_line(runtime, &format!("history {} {}", index + 1, entry))?;
    }
    Ok(())
}

fn shell_set_variable(variables: &mut Vec<ShellVariable>, name: &str, value: String) {
    if let Some(variable) = variables.iter_mut().find(|variable| variable.name == name) {
        variable.value = value;
    } else {
        variables.push(ShellVariable {
            name: name.to_string(),
            value,
        });
    }
}

fn shell_sync_runtime_variables(
    variables: &mut Vec<ShellVariable>,
    last_status: i32,
    cwd: &str,
    last_pid: Option<u64>,
) {
    shell_set_variable(variables, "STATUS", last_status.to_string());
    shell_set_variable(variables, "CWD", cwd.to_string());
    shell_set_variable(
        variables,
        "LAST_PID",
        last_pid.map(|pid| pid.to_string()).unwrap_or_default(),
    );
}

fn shell_parse_guarded_commands(line: &str) -> Vec<(ShellCommandGuard, String)> {
    let bytes = line.as_bytes();
    let mut commands = Vec::new();
    let mut start = 0usize;
    let mut guard = ShellCommandGuard::Always;
    let mut index = 0usize;
    while index < bytes.len() {
        if index + 1 < bytes.len() && bytes[index] == b'&' && bytes[index + 1] == b'&' {
            let command = line[start..index].trim();
            if !command.is_empty() {
                commands.push((guard, command.to_string()));
            }
            guard = ShellCommandGuard::OnSuccess;
            index += 2;
            start = index;
            continue;
        }
        if index + 1 < bytes.len() && bytes[index] == b'|' && bytes[index + 1] == b'|' {
            let command = line[start..index].trim();
            if !command.is_empty() {
                commands.push((guard, command.to_string()));
            }
            guard = ShellCommandGuard::OnFailure;
            index += 2;
            start = index;
            continue;
        }
        if bytes[index] == b';' {
            let command = line[start..index].trim();
            if !command.is_empty() {
                commands.push((guard, command.to_string()));
            }
            guard = ShellCommandGuard::Always;
            index += 1;
            start = index;
            continue;
        }
        index += 1;
    }
    let command = line[start..].trim();
    if !command.is_empty() {
        commands.push((guard, command.to_string()));
    }
    commands
}

fn parse_resource_kind(token: &str) -> Option<NativeResourceKind> {
    match token {
        "memory" => Some(NativeResourceKind::Memory),
        "storage" => Some(NativeResourceKind::Storage),
        "channel" => Some(NativeResourceKind::Channel),
        "device" => Some(NativeResourceKind::Device),
        "namespace" => Some(NativeResourceKind::Namespace),
        "surface" => Some(NativeResourceKind::Surface),
        _ => None,
    }
}

fn parse_contract_kind(token: &str) -> Option<NativeContractKind> {
    match token {
        "execution" => Some(NativeContractKind::Execution),
        "memory" => Some(NativeContractKind::Memory),
        "io" => Some(NativeContractKind::Io),
        "device" => Some(NativeContractKind::Device),
        "display" => Some(NativeContractKind::Display),
        "observe" => Some(NativeContractKind::Observe),
        _ => None,
    }
}

fn parse_contract_state(token: &str) -> Option<NativeContractState> {
    match token {
        "active" => Some(NativeContractState::Active),
        "suspended" => Some(NativeContractState::Suspended),
        "revoked" => Some(NativeContractState::Revoked),
        _ => None,
    }
}

fn parse_resource_arbitration(token: &str) -> Option<NativeResourceArbitrationPolicy> {
    match token {
        "fifo" => Some(NativeResourceArbitrationPolicy::Fifo),
        "lifo" => Some(NativeResourceArbitrationPolicy::Lifo),
        _ => None,
    }
}

fn parse_resource_governance(token: &str) -> Option<NativeResourceGovernanceMode> {
    match token {
        "queueing" => Some(NativeResourceGovernanceMode::Queueing),
        "exclusive-lease" => Some(NativeResourceGovernanceMode::ExclusiveLease),
        _ => None,
    }
}

fn parse_resource_contract_policy(token: &str) -> Option<NativeResourceContractPolicy> {
    match token {
        "any" => Some(NativeResourceContractPolicy::Any),
        "execution" => Some(NativeResourceContractPolicy::Execution),
        "memory" => Some(NativeResourceContractPolicy::Memory),
        "io" => Some(NativeResourceContractPolicy::Io),
        "device" => Some(NativeResourceContractPolicy::Device),
        "display" => Some(NativeResourceContractPolicy::Display),
        "observe" => Some(NativeResourceContractPolicy::Observe),
        _ => None,
    }
}

fn parse_resource_issuer_policy(token: &str) -> Option<NativeResourceIssuerPolicy> {
    match token {
        "any-issuer" => Some(NativeResourceIssuerPolicy::AnyIssuer),
        "creator-only" => Some(NativeResourceIssuerPolicy::CreatorOnly),
        "domain-owner-only" => Some(NativeResourceIssuerPolicy::DomainOwnerOnly),
        _ => None,
    }
}

fn shell_resolve_self_pid<B: SyscallBackend>(
    runtime: &Runtime<B>,
    context: &SessionContext,
    cwd: &str,
) -> Result<u64, ExitCode> {
    let mut matches = Vec::new();
    for pid in list_process_ids(runtime)? {
        let name = read_process_text(runtime, pid, Runtime::get_process_name).map_err(|_| 218)?;
        let process_cwd =
            read_process_text(runtime, pid, Runtime::get_process_cwd).map_err(|_| 218)?;
        let image =
            read_process_text(runtime, pid, Runtime::get_process_image_path).map_err(|_| 218)?;
        if name == context.process_name && process_cwd == cwd && image == context.image_path {
            matches.push(pid);
        }
    }
    matches.into_iter().max().ok_or(219)
}

fn shell_emit_lines<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    if text.is_empty() {
        return Ok(());
    }
    for line in text.lines() {
        write_line(runtime, line)?;
    }
    if text.as_bytes().last().is_some_and(|byte| *byte != b'\n') {
        write_line(runtime, "")?;
    }
    Ok(())
}

fn shell_render_ps<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    for pid in list_process_ids(runtime)? {
        let process = runtime.inspect_process(pid).map_err(|_| 221)?;
        let name = read_process_text(runtime, pid, Runtime::get_process_name)
            .unwrap_or_else(|_| String::from("unknown"));
        let cwd = read_process_text(runtime, pid, Runtime::get_process_cwd)
            .unwrap_or_else(|_| String::from("?"));
        write_line(
            runtime,
            &format!(
                "pid={pid} name={name} state={} cwd={cwd}",
                native_process_state_label(process.state)
            ),
        )?;
    }
    Ok(())
}

fn native_process_state_label(state: u32) -> &'static str {
    match state {
        0 => "Created",
        1 => "Ready",
        2 => "Running",
        3 => "Blocked",
        4 => "Exited",
        _ => "Unknown",
    }
}

fn scheduler_class_label(raw: u32) -> &'static str {
    match NativeSchedulerClass::from_raw(raw) {
        Some(NativeSchedulerClass::LatencyCritical) => "latency-critical",
        Some(NativeSchedulerClass::Interactive) => "interactive",
        Some(NativeSchedulerClass::BestEffort) => "best-effort",
        Some(NativeSchedulerClass::Background) => "background",
        None => "unknown",
    }
}

fn parse_scheduler_class(token: &str) -> Option<NativeSchedulerClass> {
    match token {
        "latency-critical" | "critical" => Some(NativeSchedulerClass::LatencyCritical),
        "interactive" => Some(NativeSchedulerClass::Interactive),
        "best-effort" | "besteffort" => Some(NativeSchedulerClass::BestEffort),
        "background" => Some(NativeSchedulerClass::Background),
        _ => None,
    }
}

fn nextmind_pressure_state_label(state: PressureState) -> &'static str {
    match state {
        PressureState::Stable => "stable",
        PressureState::HighSchedulerPressure => "high-scheduler-pressure",
        PressureState::NetworkBackpressure => "network-backpressure",
        PressureState::MixedPressure => "mixed-pressure",
    }
}

fn nextmind_metrics_score(metrics: &SystemPressureMetrics) -> u64 {
    metrics.run_queue_total.saturating_mul(100)
        + metrics.snapshot.saturated_socket_count.saturating_mul(100)
        + u64::from(metrics.cpu_utilization_pct)
        + u64::from(metrics.socket_pressure_pct)
        + u64::from(metrics.event_queue_pressure_pct)
        + metrics.snapshot.blocked_process_count.saturating_mul(5)
}

fn nextmind_render_metrics<B: SyscallBackend>(
    runtime: &Runtime<B>,
    label: &str,
    metrics: &SystemPressureMetrics,
    state: PressureState,
) -> Result<(), ExitCode> {
    write_line(
        runtime,
        &format!(
            "nextmind.metrics label={label} state={} runq={} classes={}/{}/{}/{} cpu={} active={} blocked={} sockets={}/{} socket-pressure={} event-pressure={} drops={}/{} drop-delta={}/{} busy={}/{}",
            nextmind_pressure_state_label(state),
            metrics.run_queue_total,
            metrics.run_queue_latency_critical,
            metrics.run_queue_interactive,
            metrics.run_queue_normal,
            metrics.run_queue_background,
            metrics.cpu_utilization_pct,
            metrics.snapshot.active_process_count,
            metrics.snapshot.blocked_process_count,
            metrics.snapshot.total_socket_rx_depth,
            metrics.snapshot.total_socket_rx_limit,
            metrics.socket_pressure_pct,
            metrics.event_queue_pressure_pct,
            metrics.snapshot.total_network_tx_dropped,
            metrics.snapshot.total_network_rx_dropped,
            metrics.tx_drop_delta,
            metrics.rx_drop_delta,
            metrics.snapshot.busy_ticks,
            metrics.snapshot.current_tick,
        ),
    )
}

fn nextmind_collect_process_entities(facts: &[SystemFact]) -> Vec<ProcessEntity> {
    facts
        .iter()
        .filter_map(|fact| match fact {
            SystemFact::Process(process) => Some(process.clone()),
            _ => None,
        })
        .collect()
}

fn nextmind_collect_device_entities(
    facts: &[SystemFact],
) -> Vec<(DeviceHandle, NativeNetworkInterfaceRecord)> {
    facts
        .iter()
        .filter_map(|fact| match fact {
            SystemFact::Device(device) => {
                device.record.map(|record| (device.handle.clone(), record))
            }
            _ => None,
        })
        .collect()
}

fn nextmind_protected_process(process: &ProcessEntity) -> bool {
    matches!(
        NativeSchedulerClass::from_raw(process.record.scheduler_class),
        Some(NativeSchedulerClass::LatencyCritical | NativeSchedulerClass::Interactive)
    ) || process.handle.pid == 1
}

fn nextmind_candidate_processes(processes: &[ProcessEntity]) -> Vec<ProcessEntity> {
    let mut candidates = processes
        .iter()
        .filter(|process| {
            !nextmind_protected_process(process) && matches!(process.record.state, 1 | 2)
        })
        .cloned()
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .record
            .cpu_runtime_ticks
            .cmp(&left.record.cpu_runtime_ticks)
            .then(
                right
                    .record
                    .scheduler_budget
                    .cmp(&left.record.scheduler_budget),
            )
            .then(left.handle.pid.cmp(&right.handle.pid))
    });
    candidates
}

fn nextmind_subscribe_auto_streams<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> Result<Vec<EventStream>, ExitCode> {
    let controller = SystemController::new(runtime);
    let facts = controller.collect_facts().map_err(|_| 265)?;
    let mut streams = Vec::new();
    for process in nextmind_collect_process_entities(&facts) {
        streams.push(
            controller
                .subscribe(EventFilter::Process {
                    pid: process.handle.pid,
                    token: CapabilityToken {
                        value: process.handle.pid,
                    },
                    exited: true,
                    reaped: true,
                    poll_events: POLLPRI,
                })
                .map_err(|_| 265)?,
        );
    }
    for (handle, _) in nextmind_collect_device_entities(&facts) {
        streams.push(
            controller
                .subscribe(EventFilter::Network {
                    interface_path: handle.path,
                    socket_path: None,
                    token: CapabilityToken { value: 1 },
                    link_changed: true,
                    rx_ready: true,
                    tx_drained: true,
                    poll_events: POLLPRI,
                })
                .map_err(|_| 265)?,
        );
    }
    Ok(streams)
}

fn nextmind_explain_last<B: SyscallBackend>(
    runtime: &Runtime<B>,
    adaptive_state: &AdaptiveState,
    context: &SemanticContext,
    last_report: &Option<NextMindDecisionReport>,
) -> Result<(), ExitCode> {
    let Some(report) = last_report else {
        return write_line(runtime, "nextmind.explain last=none");
    };
    let diagnostics = SystemController::new(runtime).semantic_diagnostics(adaptive_state, context);
    write_line(
        runtime,
        &format!(
            "nextmind.explain trigger={} verdict={} thresholds=runq>3,cpu>=75,socket>=80,event>=75 channel={} class={} caps={} tier={:?} mode={:?} budget={}",
            nextmind_pressure_state_label(report.trigger),
            semantic_verdict_name(report.verdict),
            pressure_channel_name(report.trigger),
            semantic_class_name(report.semantic.class),
            semantic_capabilities_csv(&report.semantic),
            report.adaptive.tier,
            report.adaptive.compute_mode,
            report.adaptive.budget_points,
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "nextmind.observation cpu={} mem={} anomaly={} thermal={} stress={} focus={}",
            report.observation.cpu_load,
            report.observation.mem_pressure,
            report.observation.anomaly_score,
            report.observation.thermal_c,
            report.adaptive.stress,
            report.adaptive.focus
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "nextmind.diagnostics stress={} focus={} tier={:?} mode={:?} budget={} events={}",
            diagnostics.stress,
            diagnostics.focus,
            diagnostics.tier,
            diagnostics.compute_mode,
            diagnostics.budget_points,
            diagnostics.event_count
        ),
    )?;
    if !diagnostics.context_tail.is_empty() {
        for line in diagnostics.context_tail.lines() {
            write_line(runtime, &format!("nextmind.context {line}"))?;
        }
    }
    nextmind_render_metrics(runtime, "before", &report.before, report.trigger)?;
    nextmind_render_metrics(
        runtime,
        "after",
        &report.after,
        SystemController::new(runtime).classify_pressure(&report.after),
    )?;
    if report.actions.is_empty() {
        write_line(
            runtime,
            "nextmind.action reason=none detail=no-direct-adjustment-required",
        )?;
    } else {
        for action in &report.actions {
            write_line(
                runtime,
                &format!(
                    "nextmind.action reason={} detail={}",
                    action.reason, action.detail
                ),
            )?;
        }
    }
    Ok(())
}

fn nextmind_update_entity_epochs(
    epochs: &mut Vec<SemanticEntityEpoch>,
    entities: &[SemanticEntity],
) -> Vec<(SemanticEntity, u32)> {
    let mut resolved = Vec::new();
    for entity in entities {
        if let Some(entry) = epochs
            .iter_mut()
            .find(|entry| entry.subject == entity.subject)
        {
            if entry.policy_fingerprint != entity.policy.policy_fingerprint {
                entry.policy_fingerprint = entity.policy.policy_fingerprint;
                let next = entry.policy_epoch.wrapping_add(1);
                entry.policy_epoch = if next == 0 { 1 } else { next };
            }
            resolved.push((entity.clone(), entry.policy_epoch));
            continue;
        }
        epochs.push(SemanticEntityEpoch {
            subject: entity.subject.clone(),
            policy_fingerprint: entity.policy.policy_fingerprint,
            policy_epoch: 1,
        });
        resolved.push((entity.clone(), 1));
    }
    resolved
}

fn nextmind_optimize_system<B: SyscallBackend>(
    runtime: &Runtime<B>,
    last_snapshot: &mut Option<ngos_user_abi::NativeSystemSnapshotRecord>,
    adaptive_state: &mut AdaptiveState,
) -> Result<NextMindDecisionReport, ExitCode> {
    let controller = SystemController::new(runtime);
    let plan = controller
        .plan_pressure_response(last_snapshot.as_ref(), adaptive_state)
        .map_err(|_| 266)?;
    let before = plan.before.clone();
    let trigger = plan.trigger;
    let facts = controller.collect_facts().map_err(|_| 266)?;
    let processes = nextmind_collect_process_entities(&facts);
    let devices = nextmind_collect_device_entities(&facts);
    let mut actions = plan.actions.clone();
    let mut original_net_admin = Vec::<(DeviceHandle, NativeNetworkInterfaceRecord)>::new();

    if matches!(
        trigger,
        PressureState::HighSchedulerPressure | PressureState::MixedPressure
    ) {
        for process in nextmind_candidate_processes(&processes).into_iter().take(2) {
            if process.record.scheduler_class == NativeSchedulerClass::Background as u32
                && process.record.scheduler_budget <= 1
            {
                continue;
            }
            let reason = String::from("scheduler-pressure");
            if !actions.iter().any(|action| {
                action.reason == reason
                    && action
                        .detail
                        .contains(&format!("pid={}", process.handle.pid))
            }) {
                continue;
            }
            controller
                .act_on_process(
                    process.handle,
                    ProcessAction::Renice {
                        class: NativeSchedulerClass::Background,
                        budget: 1,
                    },
                )
                .map_err(|_| 266)?;
        }
    }

    if matches!(
        trigger,
        PressureState::NetworkBackpressure | PressureState::MixedPressure
    ) {
        for (handle, record) in devices {
            let socket_pressure = if before.snapshot.total_socket_rx_limit == 0 {
                0
            } else {
                before.socket_pressure_pct
            };
            if record.rx_dropped == 0
                && record.tx_dropped == 0
                && socket_pressure < 80
                && record.tx_inflight_depth < record.tx_inflight_limit
            {
                continue;
            }
            if !actions.iter().any(|action| {
                action.reason == "network-backpressure"
                    && action.detail.contains(&format!("iface={}", handle.path))
            }) {
                continue;
            }
            original_net_admin.push((handle.clone(), record));
            let new_tx_capacity = (record.tx_capacity as usize)
                .saturating_add((record.tx_capacity as usize / 2).max(1));
            let new_rx_capacity = (record.rx_capacity as usize)
                .saturating_add((record.rx_capacity as usize / 2).max(1));
            let new_tx_inflight_limit = (record.tx_inflight_limit as usize)
                .saturating_add((record.tx_inflight_limit as usize / 2).max(1))
                .min(new_tx_capacity.max(1));
            controller
                .configure_interface_admin(
                    &handle,
                    record.mtu as usize,
                    new_tx_capacity,
                    new_rx_capacity,
                    new_tx_inflight_limit,
                    record.admin_up != 0,
                    false,
                )
                .map_err(|_| 266)?;
        }
    }

    let mut after = controller
        .observe_pressure(Some(&before.snapshot))
        .map_err(|_| 266)?;
    let mut verdict = match nextmind_metrics_score(&after).cmp(&nextmind_metrics_score(&before)) {
        core::cmp::Ordering::Less => SemanticVerdict::Improved,
        core::cmp::Ordering::Equal => SemanticVerdict::NoChange,
        core::cmp::Ordering::Greater => SemanticVerdict::Worse,
    };

    if !matches!(verdict, SemanticVerdict::Improved)
        && matches!(
            trigger,
            PressureState::MixedPressure | PressureState::HighSchedulerPressure
        )
        && let Some(process) = nextmind_candidate_processes(&processes).into_iter().next()
    {
        if !actions
            .iter()
            .any(|action| action.reason == "fallback-throttle")
        {
            actions.push(SemanticActionRecord {
                reason: String::from("fallback-throttle"),
                detail: format!(
                    "pause pid={} name={} cpu_ticks={}",
                    process.handle.pid, process.name, process.record.cpu_runtime_ticks
                ),
            });
        }
        controller
            .act_on_process(process.handle, ProcessAction::Pause)
            .map_err(|_| 266)?;
        after = controller
            .observe_pressure(Some(&before.snapshot))
            .map_err(|_| 266)?;
        verdict = match nextmind_metrics_score(&after).cmp(&nextmind_metrics_score(&before)) {
            core::cmp::Ordering::Less => SemanticVerdict::Improved,
            core::cmp::Ordering::Equal => SemanticVerdict::NoChange,
            core::cmp::Ordering::Greater => SemanticVerdict::Worse,
        };
        if matches!(verdict, SemanticVerdict::Worse) {
            controller
                .act_on_process(process.handle, ProcessAction::Resume)
                .map_err(|_| 266)?;
            actions.push(SemanticActionRecord {
                reason: String::from("rollback"),
                detail: format!("resume pid={} after worse outcome", process.handle.pid),
            });
            after = controller
                .observe_pressure(Some(&before.snapshot))
                .map_err(|_| 266)?;
            verdict = match nextmind_metrics_score(&after).cmp(&nextmind_metrics_score(&before)) {
                core::cmp::Ordering::Less => SemanticVerdict::Improved,
                core::cmp::Ordering::Equal => SemanticVerdict::NoChange,
                core::cmp::Ordering::Greater => SemanticVerdict::Worse,
            };
        }
    }

    if matches!(verdict, SemanticVerdict::Worse) {
        for (handle, record) in original_net_admin {
            controller
                .configure_interface_admin(
                    &handle,
                    record.mtu as usize,
                    record.tx_capacity as usize,
                    record.rx_capacity as usize,
                    record.tx_inflight_limit as usize,
                    record.admin_up != 0,
                    record.promiscuous != 0,
                )
                .map_err(|_| 266)?;
            actions.push(SemanticActionRecord {
                reason: String::from("rollback"),
                detail: format!("restore iface={} admin profile", handle.path),
            });
        }
        after = controller
            .observe_pressure(Some(&before.snapshot))
            .map_err(|_| 266)?;
        verdict = match nextmind_metrics_score(&after).cmp(&nextmind_metrics_score(&before)) {
            core::cmp::Ordering::Less => SemanticVerdict::Improved,
            core::cmp::Ordering::Equal => SemanticVerdict::NoChange,
            core::cmp::Ordering::Greater => SemanticVerdict::Worse,
        };
    }

    *last_snapshot = Some(after.snapshot);
    Ok(NextMindDecisionReport {
        trigger,
        before,
        after,
        semantic: plan.semantic,
        observation: plan.observation,
        adaptive: plan.adaptive,
        actions,
        verdict,
    })
}

fn nextmind_drain_auto_events<B: SyscallBackend>(
    runtime: &Runtime<B>,
    auto_state: &NextMindAutoState,
    last_snapshot: &mut Option<ngos_user_abi::NativeSystemSnapshotRecord>,
    adaptive_state: &mut AdaptiveState,
    last_report: &mut Option<NextMindDecisionReport>,
) -> Result<(), ExitCode> {
    if !auto_state.enabled {
        return Ok(());
    }
    let mut triggered = false;
    for stream in &auto_state.streams {
        let ready = runtime
            .poll(stream.queue_fd, POLLIN | POLLPRI)
            .map_err(|_| 267)?;
        if ready == 0 {
            continue;
        }
        let mut events = [NativeEventRecord {
            token: 0,
            events: 0,
            source_kind: 0,
            source_arg0: 0,
            source_arg1: 0,
            source_arg2: 0,
            detail0: 0,
            detail1: 0,
        }; 8];
        let count = runtime
            .wait_event_queue(stream.queue_fd, &mut events)
            .map_err(|_| 267)?;
        if count != 0 {
            triggered = true;
        }
    }
    if triggered {
        let report = nextmind_optimize_system(runtime, last_snapshot, adaptive_state)?;
        *last_report = Some(report.clone());
        write_line(
            runtime,
            &format!(
                "nextmind.auto trigger={} class={} verdict={}",
                nextmind_pressure_state_label(report.trigger),
                semantic_class_name(report.semantic.class),
                semantic_verdict_name(report.verdict)
            ),
        )?;
    }
    Ok(())
}

fn shell_render_semantic_facts<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    let controller = SystemController::new(runtime);
    let facts = controller.collect_facts().map_err(|_| 260)?;
    for fact in facts {
        match fact {
            SystemFact::Process(process) => write_line(
                runtime,
                &format!(
                    "fact process pid={} name={} state={} class={} budget={} cwd={} image={}",
                    process.handle.pid,
                    process.name,
                    native_process_state_label(process.record.state),
                    scheduler_class_label(process.record.scheduler_class),
                    process.record.scheduler_budget,
                    process.cwd,
                    process.image_path
                ),
            )?,
            SystemFact::Device(device) => {
                if let Some(record) = device.record {
                    write_line(
                        runtime,
                        &format!(
                            "fact device path={} admin={} link={} mtu={} tx={} rx={} dropped={}/{}",
                            device.handle.path,
                            if record.admin_up != 0 { "up" } else { "down" },
                            if record.link_up != 0 { "up" } else { "down" },
                            record.mtu,
                            record.tx_packets,
                            record.rx_packets,
                            record.tx_dropped,
                            record.rx_dropped
                        ),
                    )?;
                } else {
                    write_line(runtime, &format!("fact device path={}", device.handle.path))?;
                }
            }
            SystemFact::Socket(socket) => write_line(
                runtime,
                &format!(
                    "fact socket path={} local={}:{} remote={}:{} connected={} rx={} tx={}",
                    socket.handle.path,
                    render_ipv4(socket.record.local_ipv4),
                    socket.record.local_port,
                    render_ipv4(socket.record.remote_ipv4),
                    socket.record.remote_port,
                    socket.record.connected,
                    socket.record.rx_packets,
                    socket.record.tx_packets
                ),
            )?,
            SystemFact::Resource { id, record } => write_line(
                runtime,
                &format!(
                    "fact resource id={} state={} holder={} waiters={} acquires={} handoffs={}",
                    id,
                    resource_state_name(record.state),
                    record.holder_contract,
                    record.waiting_count,
                    record.acquire_count,
                    record.handoff_count
                ),
            )?,
            SystemFact::Contract { id, record } => write_line(
                runtime,
                &format!(
                    "fact contract id={} resource={} issuer={} kind={} state={}",
                    id,
                    record.resource,
                    record.issuer,
                    contract_kind_name(record.kind),
                    contract_state_name(record.state)
                ),
            )?,
        }
    }
    Ok(())
}

fn shell_semantic_watch_event<B: SyscallBackend>(
    runtime: &Runtime<B>,
    filter: EventFilter,
) -> Result<usize, ExitCode> {
    let controller = SystemController::new(runtime);
    controller
        .subscribe(filter)
        .map(|stream| stream.queue_fd)
        .map_err(|_| 261)
}

fn shell_semantic_wait_event<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
) -> Result<(), ExitCode> {
    let mut events = [NativeEventRecord {
        token: 0,
        events: 0,
        source_kind: 0,
        source_arg0: 0,
        source_arg1: 0,
        source_arg2: 0,
        detail0: 0,
        detail1: 0,
    }; 16];
    let count = runtime
        .wait_event_queue(queue_fd, &mut events)
        .map_err(|_| 262)?;
    for event in events.into_iter().take(count) {
        write_line(
            runtime,
            &format!(
                "semantic-event queue={} token={} source={} arg0={} arg1={} arg2={} detail0={} detail1={}",
                queue_fd,
                event.token,
                event_source_name(&event),
                event.source_arg0,
                event.source_arg1,
                event.source_arg2,
                event.detail0,
                event.detail1
            ),
        )?;
    }
    Ok(())
}

fn shell_semantic_process_action<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
    action: ProcessAction,
) -> Result<(), ExitCode> {
    let controller = SystemController::new(runtime);
    controller
        .act_on_process(ProcessHandle { pid }, action)
        .map_err(|_| 263)?;
    let record = runtime.inspect_process(pid).map_err(|_| 263)?;
    write_line(
        runtime,
        &format!(
            "process-control pid={} state={} class={} budget={}",
            pid,
            native_process_state_label(record.state),
            scheduler_class_label(record.scheduler_class),
            record.scheduler_budget
        ),
    )
}

fn shell_semantic_resource_update<B: SyscallBackend>(
    runtime: &Runtime<B>,
    contract: usize,
    action: ResourceUpdate,
) -> Result<(), ExitCode> {
    let controller = SystemController::new(runtime);
    controller
        .update_resource(ResourceContract { id: contract }, action)
        .map_err(|_| 264)?;
    let contract_record = runtime.inspect_contract(contract).map_err(|_| 264)?;
    let resource = runtime
        .inspect_resource(contract_record.resource as usize)
        .map_err(|_| 264)?;
    write_line(
        runtime,
        &format!(
            "resource-control contract={} resource={} state={}",
            contract,
            contract_record.resource,
            resource_state_name(resource.state)
        ),
    )
}

fn shell_record_learning(
    learning: &mut SemanticFeedbackStore,
    epochs: &[SemanticEntityEpoch],
    subject: &str,
    action: &str,
    success: bool,
) {
    let policy_epoch = epochs
        .iter()
        .find(|entry| entry.subject == subject)
        .map(|entry| entry.policy_epoch)
        .unwrap_or(1);
    learning.record(subject, action, policy_epoch, success);
}

fn shell_send_signal<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
    signal: u8,
) -> Result<(), ExitCode> {
    runtime.send_signal(pid, signal).map_err(|_| 248)?;
    write_line(runtime, &format!("signal-sent pid={pid} signal={signal}"))
}

fn shell_render_jobs<B: SyscallBackend>(
    runtime: &Runtime<B>,
    jobs: &[ShellJob],
) -> Result<(), ExitCode> {
    if jobs.is_empty() {
        return write_line(runtime, "jobs=0");
    }
    for job in jobs {
        let state = if let Some(exit) = job.reaped_exit {
            format!("reaped:{exit}")
        } else {
            let status = runtime
                .inspect_process(job.pid)
                .ok()
                .map(|record| native_process_state_label(record.state).to_string())
                .unwrap_or_else(|| String::from("unknown"));
            format!("live:{status}")
        };
        write_line(
            runtime,
            &format!(
                "job pid={} name={} path={} state={} signals={}",
                job.pid, job.name, job.path, state, job.signal_count
            ),
        )?;
    }
    Ok(())
}

fn shell_render_job_info<B: SyscallBackend>(
    runtime: &Runtime<B>,
    jobs: &[ShellJob],
    pid: u64,
) -> Result<(), ExitCode> {
    let process = runtime.inspect_process(pid).map_err(|_| 252)?;
    let Some(job) = jobs.iter().find(|job| job.pid == pid) else {
        return Err(252);
    };
    let state = if let Some(exit) = job.reaped_exit {
        format!("reaped:{exit}")
    } else {
        format!("live:{}", native_process_state_label(process.state))
    };
    write_line(
        runtime,
        &format!(
            "job-info pid={} name={} path={} state={} signals={} exit={} pending={}",
            job.pid,
            job.name,
            job.path,
            state,
            job.signal_count,
            process.exit_code,
            process.pending_signal_count
        ),
    )
}

fn shell_render_process_record<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
) -> Result<(), ExitCode> {
    let record: NativeProcessRecord = runtime.inspect_process(pid).map_err(|_| 251)?;
    let name = read_process_text(runtime, pid, Runtime::get_process_name).map_err(|_| 251)?;
    let image =
        read_process_text(runtime, pid, Runtime::get_process_image_path).map_err(|_| 251)?;
    let cwd = read_process_text(runtime, pid, Runtime::get_process_cwd).map_err(|_| 251)?;
    write_line(
        runtime,
        &format!(
            "pid={} name={} image={} cwd={} parent={} address-space={} thread={} state={} exit={} fds={} caps={} env={} regions={} threads={} pending={} session-reported={} session-status={} session-stage={} scheduler-class={} scheduler-budget={}",
            record.pid,
            name,
            image,
            cwd,
            record.parent,
            record.address_space,
            record.main_thread,
            native_process_state_label(record.state),
            record.exit_code,
            record.descriptor_count,
            record.capability_count,
            record.environment_count,
            record.memory_region_count,
            record.thread_count,
            record.pending_signal_count,
            record.session_reported,
            record.session_status,
            record.session_stage,
            scheduler_class_label(record.scheduler_class),
            record.scheduler_budget,
        ),
    )
}

fn shell_render_pending_signals<B: SyscallBackend>(
    runtime: &Runtime<B>,
    pid: u64,
    blocked_only: bool,
) -> Result<(), ExitCode> {
    let mut buffer = [0u8; 64];
    let count = if blocked_only {
        runtime
            .blocked_pending_signals(pid, &mut buffer)
            .map_err(|_| 249)?
    } else {
        runtime.pending_signals(pid, &mut buffer).map_err(|_| 250)?
    };
    let rendered = if count == 0 {
        String::from("-")
    } else {
        buffer[..count]
            .iter()
            .map(|signal| format!("{signal}"))
            .collect::<Vec<_>>()
            .join(",")
    };
    write_line(
        runtime,
        &format!(
            "pid={pid} {}={rendered}",
            if blocked_only {
                "blocked-pending-signals"
            } else {
                "pending-signals"
            }
        ),
    )
}

fn shell_render_procfs_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<(), ExitCode> {
    let bytes = read_procfs_all(runtime, path)?;
    let text = core::str::from_utf8(&bytes).map_err(|_| 203)?;
    shell_emit_lines(runtime, text)
}

fn contract_state_name(raw: u32) -> &'static str {
    match NativeContractState::from_raw(raw) {
        Some(NativeContractState::Active) => "active",
        Some(NativeContractState::Suspended) => "suspended",
        Some(NativeContractState::Revoked) => "revoked",
        None => "unknown",
    }
}

fn resource_state_name(raw: u32) -> &'static str {
    match NativeResourceState::from_raw(raw) {
        Some(NativeResourceState::Active) => "active",
        Some(NativeResourceState::Suspended) => "suspended",
        Some(NativeResourceState::Retired) => "retired",
        None => "unknown",
    }
}

fn resource_kind_name(raw: u32) -> &'static str {
    match NativeResourceKind::from_raw(raw) {
        Some(NativeResourceKind::Memory) => "memory",
        Some(NativeResourceKind::Storage) => "storage",
        Some(NativeResourceKind::Channel) => "channel",
        Some(NativeResourceKind::Device) => "device",
        Some(NativeResourceKind::Namespace) => "namespace",
        Some(NativeResourceKind::Surface) => "surface",
        None => "unknown",
    }
}

fn device_class_name(raw: u32) -> &'static str {
    match raw {
        0 => "generic",
        1 => "network",
        2 => "storage",
        3 => "graphics",
        4 => "audio",
        5 => "input",
        _ => "unknown",
    }
}

fn contract_kind_name(raw: u32) -> &'static str {
    match NativeContractKind::from_raw(raw) {
        Some(NativeContractKind::Execution) => "execution",
        Some(NativeContractKind::Memory) => "memory",
        Some(NativeContractKind::Io) => "io",
        Some(NativeContractKind::Device) => "device",
        Some(NativeContractKind::Display) => "display",
        Some(NativeContractKind::Observe) => "observe",
        None => "unknown",
    }
}

fn object_kind_name(raw: u32) -> &'static str {
    match NativeObjectKind::from_raw(raw) {
        Some(NativeObjectKind::File) => "file",
        Some(NativeObjectKind::Directory) => "directory",
        Some(NativeObjectKind::Symlink) => "symlink",
        Some(NativeObjectKind::Socket) => "socket",
        Some(NativeObjectKind::Device) => "device",
        Some(NativeObjectKind::Driver) => "driver",
        Some(NativeObjectKind::Process) => "process",
        Some(NativeObjectKind::Memory) => "memory",
        Some(NativeObjectKind::Channel) => "channel",
        Some(NativeObjectKind::EventQueue) => "event-queue",
        Some(NativeObjectKind::SleepQueue) => "sleep-queue",
        None => "unknown",
    }
}

fn shell_render_domains<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    let mut ids = vec![0u64; 16];
    let count = runtime.list_domains(&mut ids).map_err(|_| 206)?;
    ids.truncate(count);
    let mut name = [0u8; 64];
    for id in ids {
        let copied = runtime
            .get_domain_name(id as usize, &mut name)
            .map_err(|_| 207)?;
        let label = core::str::from_utf8(&name[..copied]).map_err(|_| 208)?;
        let info = runtime.inspect_domain(id as usize).map_err(|_| 209)?;
        write_line(
            runtime,
            &format!(
                "domain id={} owner={} resources={} contracts={} name={}",
                info.id, info.owner, info.resource_count, info.contract_count, label
            ),
        )?;
    }
    Ok(())
}

fn shell_render_resources<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    let mut ids = vec![0u64; 16];
    let count = runtime.list_resources(&mut ids).map_err(|_| 210)?;
    ids.truncate(count);
    let mut name = [0u8; 64];
    for id in ids {
        let copied = runtime
            .get_resource_name(id as usize, &mut name)
            .map_err(|_| 211)?;
        let label = core::str::from_utf8(&name[..copied]).map_err(|_| 212)?;
        let info = runtime.inspect_resource(id as usize).map_err(|_| 213)?;
        write_line(
            runtime,
            &format!(
                "resource id={} domain={} kind={} state={} holder={} waiters={} name={}",
                info.id,
                info.domain,
                resource_kind_name(info.kind),
                resource_state_name(info.state),
                info.holder_contract,
                info.waiting_count,
                label
            ),
        )?;
    }
    Ok(())
}

fn shell_render_contracts<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    let mut ids = vec![0u64; 16];
    let count = runtime.list_contracts(&mut ids).map_err(|_| 214)?;
    ids.truncate(count);
    let mut label = [0u8; 64];
    for id in ids {
        let copied = runtime
            .get_contract_label(id as usize, &mut label)
            .map_err(|_| 215)?;
        let name = core::str::from_utf8(&label[..copied]).map_err(|_| 216)?;
        let info = runtime.inspect_contract(id as usize).map_err(|_| 217)?;
        write_line(
            runtime,
            &format!(
                "contract id={} domain={} resource={} issuer={} kind={} state={} label={}",
                info.id,
                info.domain,
                info.resource,
                info.issuer,
                contract_kind_name(info.kind),
                contract_state_name(info.state),
                name
            ),
        )?;
    }
    Ok(())
}

fn shell_render_domain_detail<B: SyscallBackend>(
    runtime: &Runtime<B>,
    id: usize,
) -> Result<(), ExitCode> {
    let info = runtime.inspect_domain(id).map_err(|_| 220)?;
    let mut name = [0u8; 128];
    let copied = runtime.get_domain_name(id, &mut name).map_err(|_| 221)?;
    let label = core::str::from_utf8(&name[..copied]).map_err(|_| 222)?;
    write_line(
        runtime,
        &format!(
            "domain id={} owner={} parent={} resources={} contracts={} name={}",
            info.id, info.owner, info.parent, info.resource_count, info.contract_count, label
        ),
    )
}

fn shell_render_resource_detail<B: SyscallBackend>(
    runtime: &Runtime<B>,
    id: usize,
) -> Result<(), ExitCode> {
    let info = runtime.inspect_resource(id).map_err(|_| 223)?;
    let mut name = [0u8; 128];
    let copied = runtime.get_resource_name(id, &mut name).map_err(|_| 224)?;
    let label = core::str::from_utf8(&name[..copied]).map_err(|_| 225)?;
    let waiters = shell_collect_waiters(runtime, id)?;
    write_line(
        runtime,
        &format!(
            "resource id={} domain={} creator={} kind={} state={} arbitration={} governance={} contract_policy={} issuer_policy={} holder={} acquire_count={} handoff_count={} waiters={} name={}",
            info.id,
            info.domain,
            info.creator,
            resource_kind_name(info.kind),
            resource_state_name(info.state),
            resource_arbitration_name(info.arbitration),
            resource_governance_name(info.governance),
            resource_contract_policy_name(info.contract_policy),
            resource_issuer_policy_name(info.issuer_policy),
            info.holder_contract,
            info.acquire_count,
            info.handoff_count,
            waiters.as_str(),
            label
        ),
    )
}

fn shell_render_contract_detail<B: SyscallBackend>(
    runtime: &Runtime<B>,
    id: usize,
) -> Result<(), ExitCode> {
    let info = runtime.inspect_contract(id).map_err(|_| 226)?;
    let mut label = [0u8; 128];
    let copied = runtime
        .get_contract_label(id, &mut label)
        .map_err(|_| 227)?;
    let text = core::str::from_utf8(&label[..copied]).map_err(|_| 228)?;
    write_line(
        runtime,
        &format!(
            "contract id={} domain={} resource={} issuer={} kind={} state={} label={}",
            info.id,
            info.domain,
            info.resource,
            info.issuer,
            contract_kind_name(info.kind),
            contract_state_name(info.state),
            text
        ),
    )
}

fn shell_collect_waiters<B: SyscallBackend>(
    runtime: &Runtime<B>,
    resource: usize,
) -> Result<String, ExitCode> {
    let mut ids = vec![0u64; 8];
    loop {
        let count = runtime
            .list_resource_waiters(resource, &mut ids)
            .map_err(|_| 229)?;
        if count <= ids.len() {
            ids.truncate(count);
            let rendered = if ids.is_empty() {
                String::from("-")
            } else {
                ids.into_iter()
                    .map(|id| format!("{id}"))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            return Ok(rendered);
        }
        ids.resize(count, 0);
    }
}

fn shell_render_waiters<B: SyscallBackend>(
    runtime: &Runtime<B>,
    resource: usize,
) -> Result<(), ExitCode> {
    let rendered = shell_collect_waiters(runtime, resource)?;
    write_line(
        runtime,
        &format!("resource={} waiters={rendered}", resource),
    )
}

fn shell_render_self_view<B: SyscallBackend>(
    runtime: &Runtime<B>,
    context: &SessionContext,
    cwd: &str,
    section: &str,
) -> Result<(), ExitCode> {
    let pid = shell_resolve_self_pid(runtime, context, cwd)?;
    shell_render_procfs_path(runtime, &format!("/proc/{pid}/{section}"))
}

fn shell_render_system_queues<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    shell_render_procfs_path(runtime, "/proc/queues")
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
            let overlay_path = if section == "cmdline" {
                session.runtime_argv_path.as_str()
            } else {
                session.runtime_env_path.as_str()
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
    match section {
        "status" | "stat" | "cmdline" | "cwd" | "environ" | "exe" | "auxv" | "maps"
        | "vmobjects" | "vmdecisions" | "vmepisodes" | "fd" | "caps" | "queues" => {
            shell_render_procfs_path(runtime, &format!("/proc/{pid}/{section}"))
        }
        _ => Err(230),
    }
}

fn shell_render_env<B: SyscallBackend>(
    runtime: &Runtime<B>,
    context: &SessionContext,
    cwd: &str,
) -> Result<(), ExitCode> {
    write_line(runtime, &format!("protocol={}", context.protocol))?;
    write_line(runtime, &format!("process={}", context.process_name))?;
    write_line(runtime, &format!("image={}", context.image_path))?;
    write_line(runtime, &format!("cwd={cwd}"))?;
    write_line(
        runtime,
        &format!("root_mount_path={}", context.root_mount_path),
    )?;
    write_line(
        runtime,
        &format!("root_mount_name={}", context.root_mount_name),
    )?;
    write_line(runtime, &format!("page_size={}", context.page_size))?;
    write_line(runtime, &format!("entry=0x{:x}", context.entry))?;
    write_line(runtime, &format!("image_base=0x{:x}", context.image_base))?;
    write_line(runtime, &format!("stack_top=0x{:x}", context.stack_top))?;
    write_line(runtime, &format!("phdr=0x{:x}", context.phdr))?;
    write_line(runtime, &format!("phent={}", context.phent))?;
    write_line(runtime, &format!("phnum={}", context.phnum))?;
    write_line(
        runtime,
        &format!(
            "outcome_policy={}",
            match context.outcome_policy {
                BootOutcomePolicy::RequireZeroExit => "require-zero-exit",
                BootOutcomePolicy::AllowAnyExit => "allow-any-exit",
            }
        ),
    )
}

fn shell_render_stat_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    follow_symlink: bool,
) -> Result<(), ExitCode> {
    let status = if follow_symlink {
        runtime.stat_path(path).map_err(|_| 231)?
    } else {
        runtime.lstat_path(path).map_err(|_| 232)?
    };
    write_line(
        runtime,
        &format!(
            "path={} kind={} inode={} size={} readable={} writable={} cloexec={} nonblock={}",
            path,
            object_kind_name(status.kind),
            status.inode,
            status.size,
            status.readable,
            status.writable,
            status.cloexec,
            status.nonblock
        ),
    )
}

fn shell_render_statfs_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<(), ExitCode> {
    let status = runtime.statfs_path(path).map_err(|_| 233)?;
    write_line(
        runtime,
        &format!(
            "path={} mounts={} nodes={} read_only={}",
            path, status.mount_count, status.node_count, status.read_only
        ),
    )
}

fn shell_open_path<B: SyscallBackend>(runtime: &Runtime<B>, path: &str) -> Result<(), ExitCode> {
    let fd = runtime.open_path(path).map_err(|_| 234)?;
    write_line(runtime, &format!("opened path={} fd={fd}", path))
}

fn shell_readlink_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<(), ExitCode> {
    let mut buffer = [0u8; 256];
    let count = runtime.readlink_path(path, &mut buffer).map_err(|_| 235)?;
    let target = core::str::from_utf8(&buffer[..count]).map_err(|_| 236)?;
    write_line(runtime, &format!("link {} -> {}", path, target))
}

fn shell_cat_file<B: SyscallBackend>(runtime: &Runtime<B>, path: &str) -> Result<(), ExitCode> {
    let fd = runtime.open_path(path).map_err(|_| 237)?;
    let mut buffer = [0u8; 256];
    loop {
        let count = runtime.read(fd, &mut buffer).map_err(|_| 238)?;
        if count == 0 {
            break;
        }
        let text = core::str::from_utf8(&buffer[..count]).map_err(|_| 239)?;
        shell_emit_lines(runtime, text)?;
    }
    runtime.close(fd).map_err(|_| 240)?;
    Ok(())
}

fn shell_read_file_text<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<String, ExitCode> {
    let fd = runtime.open_path(path).map_err(|_| 237)?;
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 256];
    loop {
        let count = runtime.read(fd, &mut buffer).map_err(|_| 238)?;
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
    runtime.close(fd).map_err(|_| 240)?;
    String::from_utf8(bytes).map_err(|_| 239)
}

fn game_manifest_load<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<GameCompatManifest, ExitCode> {
    let text = shell_read_file_text(runtime, path)?;
    GameCompatManifest::parse(&text).map_err(|_| 283)
}

fn game_plan_resource_kind(kind: CompatLaneKind) -> NativeResourceKind {
    match kind {
        CompatLaneKind::Graphics => NativeResourceKind::Surface,
        CompatLaneKind::Audio => NativeResourceKind::Channel,
        CompatLaneKind::Input => NativeResourceKind::Device,
    }
}

fn game_plan_contract_kind(kind: CompatLaneKind) -> NativeContractKind {
    match kind {
        CompatLaneKind::Graphics => NativeContractKind::Display,
        CompatLaneKind::Audio => NativeContractKind::Io,
        CompatLaneKind::Input => NativeContractKind::Observe,
    }
}

fn game_apply_resource_policy<B: SyscallBackend>(
    runtime: &Runtime<B>,
    resource_id: usize,
    kind: CompatLaneKind,
) -> Result<(), ExitCode> {
    runtime
        .set_resource_arbitration_policy(resource_id, NativeResourceArbitrationPolicy::Fifo)
        .map_err(|_| 284)?;
    let governance = match kind {
        CompatLaneKind::Audio => NativeResourceGovernanceMode::Queueing,
        CompatLaneKind::Graphics | CompatLaneKind::Input => {
            NativeResourceGovernanceMode::ExclusiveLease
        }
    };
    runtime
        .set_resource_governance_mode(resource_id, governance)
        .map_err(|_| 284)?;
    let contract_policy = match kind {
        CompatLaneKind::Graphics => NativeResourceContractPolicy::Display,
        CompatLaneKind::Audio => NativeResourceContractPolicy::Io,
        CompatLaneKind::Input => NativeResourceContractPolicy::Observe,
    };
    runtime
        .set_resource_contract_policy(resource_id, contract_policy)
        .map_err(|_| 284)?;
    runtime
        .set_resource_issuer_policy(resource_id, NativeResourceIssuerPolicy::CreatorOnly)
        .map_err(|_| 284)?;
    Ok(())
}

fn game_render_manifest<B: SyscallBackend>(
    runtime: &Runtime<B>,
    manifest_path: &str,
    manifest: &GameCompatManifest,
) -> Result<(), ExitCode> {
    write_line(
        runtime,
        &format!(
            "game.manifest path={} title={} slug={} exec={} cwd={} argv={}",
            manifest_path,
            manifest.title,
            manifest.slug,
            manifest.executable_path,
            manifest.working_dir,
            manifest.argv.join(" ")
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "game.gfx backend={} profile={}",
            graphics_backend_name(manifest.graphics.backend),
            manifest.graphics.profile
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "game.audio backend={} profile={}",
            audio_backend_name(manifest.audio.backend),
            manifest.audio.profile
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "game.input backend={} profile={}",
            input_backend_name(manifest.input.backend),
            manifest.input.profile
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "game.shim prefix={} saves={} cache={}",
            manifest.shims.prefix, manifest.shims.saves, manifest.shims.cache
        ),
    )
}

fn game_render_plan<B: SyscallBackend>(
    runtime: &Runtime<B>,
    plan: &GameSessionPlan,
) -> Result<(), ExitCode> {
    write_line(
        runtime,
        &format!(
            "game.plan domain={} process={} cwd={} exec={}",
            plan.domain_name, plan.process_name, plan.working_dir, plan.executable_path
        ),
    )?;
    for lane in &plan.lanes {
        write_line(
            runtime,
            &format!(
                "game.plan.lane kind={} resource={} contract={}",
                lane_name(lane.kind),
                lane.resource_name,
                lane.contract_label
            ),
        )?;
    }
    for env in &plan.env_shims {
        write_line(runtime, &format!("game.plan.env {}={}", env.key, env.value))?;
    }
    Ok(())
}

fn game_render_session<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
) -> Result<(), ExitCode> {
    write_line(
        runtime,
        &format!(
            "game.session pid={} title={} slug={} domain={} process={} cwd={} exec={} stopped={} exit={}",
            session.pid,
            session.title,
            session.slug,
            session.domain_id,
            session.process_name,
            session.working_dir,
            session.executable_path,
            session.stopped,
            session
                .exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| String::from("-"))
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "game.session.shim prefix={} saves={} cache={} env-file={} argv-file={} channel-file={}",
            session.prefix_path,
            session.saves_path,
            session.cache_path,
            session.runtime_env_path,
            session.runtime_argv_path,
            session.runtime_channel_path,
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "game.session.gfx pid={} device={} driver={} profile={} submitted={} frames={} presented={} last-frame={} gfx-queue={} present-mode={} completion={} completion-observed={} ops={} bytes={}",
            session.pid,
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
            session.last_frame_payload_bytes,
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "game.session.audio pid={} device={} driver={} profile={} audio-batches={} audio-stream={} audio-route={} audio-latency={} audio-spatialization={} audio-completion={} audio-completion-observed={} audio-ops={} audio-bytes={} audio-token={}",
            session.pid,
            session.audio_device_path,
            session.audio_driver_path,
            session.audio_profile,
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
            session
                .last_audio_invoke_token
                .map(|token| token.to_string())
                .unwrap_or_else(|| String::from("pending"))
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "game.session.input pid={} device={} driver={} profile={} input-batches={} input-frame={} input-family={} input-layout={} input-key-table={} pointer-capture={} input-delivery={} input-delivery-observed={} input-ops={} input-bytes={} input-token={}",
            session.pid,
            session.input_device_path,
            session.input_driver_path,
            session.input_profile,
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
            session
                .last_input_invoke_token
                .map(|token| token.to_string())
                .unwrap_or_else(|| String::from("pending"))
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "game.session.gfx-queue pid={} depth={}",
            session.pid,
            session.pending_graphics_frames.len()
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "game.session.audio-queue pid={} depth={}",
            session.pid,
            session.pending_audio_batches.len()
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "game.session.input-queue pid={} depth={}",
            session.pid,
            session.pending_input_batches.len()
        ),
    )?;
    write_line(
        runtime,
        &format!(
            "game.session.runtime-channel pid={} path={}",
            session.pid, session.runtime_channel_path
        ),
    )?;
    for lane in &session.lanes {
        write_line(
            runtime,
            &format!(
                "game.session.lane kind={} resource-id={} resource={} contract-id={} contract={} claimed={} token={}",
                lane_name(lane.kind),
                lane.resource_id,
                lane.resource_name,
                lane.contract_id,
                lane.contract_label,
                lane.claim_acquired,
                lane.invoke_token
                    .map(|token| token.to_string())
                    .unwrap_or_else(|| String::from("pending"))
            ),
        )?;
        write_line(
            runtime,
            &format!(
                "game.session.watch kind={} queue={} token={}",
                lane_name(lane.kind),
                lane.watch_queue_fd
                    .map(|fd| fd.to_string())
                    .unwrap_or_else(|| String::from("inactive")),
                lane.watch_token
                    .map(|token| token.to_string())
                    .unwrap_or_else(|| String::from("inactive"))
            ),
        )?;
    }
    Ok(())
}

fn game_render_session_summary<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
) -> Result<(), ExitCode> {
    let active_watches = session
        .lanes
        .iter()
        .filter(|lane| lane.watch_queue_fd.is_some())
        .count();
    let mut line = StackLineBuffer::<384>::new();
    line.push_str("game.session.summary pid=")?;
    line.push_u64(session.pid)?;
    line.push_str(" slug=")?;
    line.push_str(&session.slug)?;
    line.push_str(" title=")?;
    line.push_str(&session.title)?;
    line.push_str(" stopped=")?;
    line.push_bool(session.stopped)?;
    line.push_str(" exit=")?;
    if let Some(code) = session.exit_code {
        line.push_i32(code)?;
    } else {
        line.push_byte(b'-')?;
    }
    line.push_str(" lanes=")?;
    line.push_usize(session.lanes.len())?;
    line.push_str(" watches=")?;
    line.push_usize(active_watches)?;
    line.push_str(" pending[gfx=")?;
    line.push_usize(session.pending_graphics_frames.len())?;
    line.push_str(";audio=")?;
    line.push_usize(session.pending_audio_batches.len())?;
    line.push_str(";input=")?;
    line.push_usize(session.pending_input_batches.len())?;
    line.push_str("] submitted[gfx=")?;
    line.push_u64(session.submitted_frames)?;
    line.push_str(";audio=")?;
    line.push_u64(session.submitted_audio_batches)?;
    line.push_str(";input=")?;
    line.push_u64(session.submitted_input_batches)?;
    line.push_byte(b']')?;
    runtime
        .writev(1, &[line.as_bytes(), b"\n"])
        .map_err(|_| 190)?;
    Ok(())
}

fn game_render_watch_summary<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
) -> Result<(), ExitCode> {
    for lane in &session.lanes {
        write_line(
            runtime,
            &format!(
                "game.watch.summary pid={} slug={} kind={} queue={} token={} claimed={}",
                session.pid,
                session.slug,
                lane_name(lane.kind),
                lane.watch_queue_fd
                    .map(|fd| fd.to_string())
                    .unwrap_or_else(|| String::from("inactive")),
                lane.watch_token
                    .map(|token| token.to_string())
                    .unwrap_or_else(|| String::from("inactive")),
                lane.claim_acquired
            ),
        )?;
    }
    Ok(())
}

fn game_load_frame_script<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<FrameScript, ExitCode> {
    let text = shell_read_file_text(runtime, path)?;
    FrameScript::parse(&text).map_err(|_| 291)
}

fn game_load_mix_script<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<MixScript, ExitCode> {
    let text = shell_read_file_text(runtime, path)?;
    MixScript::parse(&text).map_err(|_| 292)
}

fn game_load_input_script<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<InputScript, ExitCode> {
    let text = shell_read_file_text(runtime, path)?;
    InputScript::parse(&text).map_err(|_| 296)
}

fn game_encode_frame(
    session: &GameCompatSession,
    script: &FrameScript,
) -> Result<EncodedFrame, ExitCode> {
    script.validate().map_err(|_| 291)?;
    Ok(script.encode(&session.graphics_profile))
}

fn game_encode_mix(
    session: &GameCompatSession,
    script: &MixScript,
) -> Result<EncodedMix, ExitCode> {
    script.validate().map_err(|_| 292)?;
    Ok(script.encode(&session.audio_profile))
}

fn game_encode_input(
    session: &GameCompatSession,
    script: &InputScript,
) -> Result<EncodedInput, ExitCode> {
    script.validate().map_err(|_| 296)?;
    Ok(script.encode(&session.input_profile))
}

fn game_session_lane(
    session: &GameCompatSession,
    kind: CompatLaneKind,
) -> Result<&GameCompatLaneRuntime, ExitCode> {
    session
        .lanes
        .iter()
        .find(|lane| lane.kind == kind)
        .ok_or(293)
}

fn game_session_lane_mut(
    session: &mut GameCompatSession,
    kind: CompatLaneKind,
) -> Result<&mut GameCompatLaneRuntime, ExitCode> {
    session
        .lanes
        .iter_mut()
        .find(|lane| lane.kind == kind)
        .ok_or(293)
}

fn game_submit_frame<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &mut GameCompatSession,
    encoded: &EncodedFrame,
) -> Result<(bool, &'static str), ExitCode> {
    if session.stopped {
        return Err(295);
    }
    shell_gpu_submit(runtime, &session.graphics_device_path, &encoded.payload)?;
    let (presented, completion_observed) = match encoded.completion.as_str() {
        "fire-and-forget" => (false, "submitted"),
        "wait-present" => {
            let queue_fd = runtime
                .create_event_queue(NativeEventQueueMode::Kqueue)
                .map_err(|_| 298)?;
            let watch_token = ((session.pid & 0xffff_ffff) << 32) | 0x4758_0001u64;
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
                shell_gpu_present(runtime, &session.graphics_device_path, &encoded.frame_tag)
                    .is_ok();
            shell_wait_event_queue(runtime, queue_fd)?;
            runtime
                .remove_graphics_events(queue_fd, &session.graphics_device_path, watch_token)
                .map_err(|_| 299)?;
            (presented, "graphics-event-present")
        }
        "wait-complete" => {
            let queue_fd = runtime
                .create_event_queue(NativeEventQueueMode::Kqueue)
                .map_err(|_| 298)?;
            let watch_token = ((session.pid & 0xffff_ffff) << 32) | 0x4758_0002u64;
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
                shell_gpu_present(runtime, &session.graphics_device_path, &encoded.frame_tag)
                    .is_ok();
            shell_wait_event_queue(runtime, queue_fd)?;
            runtime
                .remove_graphics_events(queue_fd, &session.graphics_device_path, watch_token)
                .map_err(|_| 299)?;
            shell_gpu_complete(runtime, &session.graphics_driver_path, &encoded.payload)?;
            (presented, "graphics-event-complete")
        }
        _ => return Err(291),
    };
    session.last_frame_tag = Some(encoded.frame_tag.clone());
    session.last_graphics_queue = Some(encoded.queue.clone());
    session.last_present_mode = Some(encoded.present_mode.clone());
    session.last_completion_mode = Some(encoded.completion.clone());
    session.last_completion_observed = Some(String::from(completion_observed));
    session.last_frame_op_count = encoded.op_count;
    session.last_frame_payload_bytes = encoded.payload.len();
    session.submitted_frames = session.submitted_frames.saturating_add(1);
    if presented {
        session.presented_frames = session.presented_frames.saturating_add(1);
    }
    session.last_presented = presented;
    session.pending_graphics_frames.push(encoded.clone());
    game_publish_runtime_payload(
        runtime,
        &session.runtime_channel_path,
        "graphics",
        &encoded.frame_tag,
        encoded.payload.as_bytes(),
    )?;
    Ok((presented, completion_observed))
}

fn game_publish_runtime_payload<B: SyscallBackend>(
    runtime: &Runtime<B>,
    channel_path: &str,
    kind: &str,
    tag: &str,
    payload: &[u8],
) -> Result<(), ExitCode> {
    let fd = runtime.open_path(channel_path).map_err(|_| 237)?;
    let mut envelope = format!("kind={kind} tag={tag}\n").into_bytes();
    envelope.extend_from_slice(payload);
    let result = shell_write_all(runtime, fd, &envelope);
    let _ = runtime.close(fd);
    result
}

fn game_submit_mix<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &mut GameCompatSession,
    encoded: &EncodedMix,
) -> Result<(usize, &'static str), ExitCode> {
    if session.stopped {
        return Err(295);
    }
    let (contract_id, resource_id) = {
        let lane = game_session_lane(session, CompatLaneKind::Audio)?;
        (lane.contract_id, lane.resource_id)
    };
    let token = runtime.invoke_contract(contract_id).map_err(|_| 294)?;
    let completion_observed = match encoded.completion.as_str() {
        "fire-and-forget" => "submitted",
        "wait-batch" | "wait-drain" => {
            let queue_fd = runtime
                .create_event_queue(NativeEventQueueMode::Kqueue)
                .map_err(|_| 298)?;
            let watch_token = ((session.pid & 0xffff_ffff) << 32) | (resource_id as u64);
            runtime
                .watch_resource_events(
                    queue_fd,
                    resource_id,
                    watch_token,
                    true,
                    true,
                    true,
                    true,
                    true,
                    true,
                    POLLPRI,
                )
                .map_err(|_| 298)?;
            shell_wait_event_queue(runtime, queue_fd)?;
            runtime
                .remove_resource_events(queue_fd, resource_id, watch_token)
                .map_err(|_| 299)?;
            if encoded.completion == "wait-drain" {
                "resource-drained"
            } else {
                "batch-waited"
            }
        }
        _ => return Err(292),
    };
    let lane = game_session_lane_mut(session, CompatLaneKind::Audio)?;
    lane.invoke_token = Some(token);
    session.last_audio_stream_tag = Some(encoded.stream_tag.clone());
    session.last_audio_route = Some(encoded.route.clone());
    session.last_audio_latency_mode = Some(encoded.latency_mode.clone());
    session.last_audio_spatialization = Some(encoded.spatialization.clone());
    session.last_audio_completion_mode = Some(encoded.completion.clone());
    session.last_audio_completion_observed = Some(String::from(completion_observed));
    session.last_audio_op_count = encoded.op_count;
    session.last_audio_payload_bytes = encoded.payload.len();
    session.submitted_audio_batches = session.submitted_audio_batches.saturating_add(1);
    session.last_audio_invoke_token = Some(token);
    session.pending_audio_batches.push(encoded.clone());
    let audio_fd = runtime
        .open_path(&session.audio_device_path)
        .map_err(|_| 234)?;
    shell_write_all(runtime, audio_fd, encoded.payload.as_bytes())?;
    runtime.close(audio_fd).map_err(|_| 240)?;
    game_publish_runtime_payload(
        runtime,
        &session.runtime_channel_path,
        "audio",
        &encoded.stream_tag,
        encoded.payload.as_bytes(),
    )?;
    Ok((token, completion_observed))
}

fn game_submit_input<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &mut GameCompatSession,
    encoded: &EncodedInput,
) -> Result<(usize, &'static str), ExitCode> {
    if session.stopped {
        return Err(295);
    }
    let (contract_id, resource_id) = {
        let lane = game_session_lane(session, CompatLaneKind::Input)?;
        (lane.contract_id, lane.resource_id)
    };
    let token = runtime.invoke_contract(contract_id).map_err(|_| 297)?;
    let delivery_observed = match encoded.delivery.as_str() {
        "immediate" => "submitted",
        "wait-batch" | "wait-frame" => {
            let queue_fd = runtime
                .create_event_queue(NativeEventQueueMode::Kqueue)
                .map_err(|_| 298)?;
            let watch_token = ((session.pid & 0xffff_ffff) << 32) | (resource_id as u64);
            runtime
                .watch_resource_events(
                    queue_fd,
                    resource_id,
                    watch_token,
                    true,
                    true,
                    true,
                    true,
                    true,
                    true,
                    POLLPRI,
                )
                .map_err(|_| 298)?;
            shell_wait_event_queue(runtime, queue_fd)?;
            runtime
                .remove_resource_events(queue_fd, resource_id, watch_token)
                .map_err(|_| 299)?;
            if encoded.delivery == "wait-frame" {
                "frame-delivered"
            } else {
                "batch-delivered"
            }
        }
        _ => return Err(296),
    };
    let lane = game_session_lane_mut(session, CompatLaneKind::Input)?;
    lane.invoke_token = Some(token);
    session.last_input_frame_tag = Some(encoded.frame_tag.clone());
    session.last_input_family = Some(encoded.device_family.clone());
    session.last_input_layout = Some(encoded.layout.clone());
    session.last_input_key_table = Some(encoded.key_table.clone());
    session.last_pointer_capture = Some(encoded.pointer_capture.clone());
    session.last_input_delivery_mode = Some(encoded.delivery.clone());
    session.last_input_delivery_observed = Some(String::from(delivery_observed));
    session.last_input_op_count = encoded.op_count;
    session.last_input_payload_bytes = encoded.payload.len();
    session.submitted_input_batches = session.submitted_input_batches.saturating_add(1);
    session.last_input_invoke_token = Some(token);
    session.pending_input_batches.push(encoded.clone());
    let input_fd = runtime
        .open_path(&session.input_device_path)
        .map_err(|_| 234)?;
    shell_write_all(runtime, input_fd, encoded.payload.as_bytes())?;
    runtime.close(input_fd).map_err(|_| 240)?;
    game_publish_runtime_payload(
        runtime,
        &session.runtime_channel_path,
        "input",
        &encoded.frame_tag,
        encoded.payload.as_bytes(),
    )?;
    Ok((token, delivery_observed))
}

fn game_watch_token(session: &GameCompatSession, lane: &GameCompatLaneRuntime) -> u64 {
    ((session.pid & 0xffff_ffff) << 32) | (lane.resource_id as u64 & 0xffff_ffff)
}

fn parse_game_lane_kind(value: &str) -> Option<CompatLaneKind> {
    match value {
        "graphics" => Some(CompatLaneKind::Graphics),
        "audio" => Some(CompatLaneKind::Audio),
        "input" => Some(CompatLaneKind::Input),
        _ => None,
    }
}

fn game_start_watch<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &mut GameCompatSession,
    kind: CompatLaneKind,
) -> Result<(usize, u64), ExitCode> {
    if session.stopped {
        return Err(295);
    }
    if game_session_lane(session, kind)?.watch_queue_fd.is_some() {
        return Err(298);
    }
    let token = {
        let lane = game_session_lane(session, kind)?;
        game_watch_token(session, lane)
    };
    let queue_fd = runtime
        .create_event_queue(NativeEventQueueMode::Kqueue)
        .map_err(|_| 298)?;
    match kind {
        CompatLaneKind::Graphics => runtime
            .watch_graphics_events(
                queue_fd,
                &session.graphics_device_path,
                token,
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
            .map_err(|_| 298)?,
        CompatLaneKind::Audio | CompatLaneKind::Input => {
            let lane = game_session_lane(session, kind)?;
            runtime
                .watch_resource_events(
                    queue_fd,
                    lane.resource_id,
                    token,
                    true,
                    true,
                    true,
                    true,
                    true,
                    true,
                    POLLPRI,
                )
                .map_err(|_| 298)?;
        }
    }
    let lane = game_session_lane_mut(session, kind)?;
    lane.watch_queue_fd = Some(queue_fd);
    lane.watch_token = Some(token);
    Ok((queue_fd, token))
}

fn game_stop_watch<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &mut GameCompatSession,
    kind: CompatLaneKind,
) -> Result<(), ExitCode> {
    let (queue_fd, token, resource_id) = {
        let lane = game_session_lane(session, kind)?;
        (
            lane.watch_queue_fd.ok_or(299)?,
            lane.watch_token.ok_or(299)?,
            lane.resource_id,
        )
    };
    match kind {
        CompatLaneKind::Graphics => runtime
            .remove_graphics_events(queue_fd, &session.graphics_device_path, token)
            .map_err(|_| 299)?,
        CompatLaneKind::Audio | CompatLaneKind::Input => runtime
            .remove_resource_events(queue_fd, resource_id, token)
            .map_err(|_| 299)?,
    }
    runtime.close(queue_fd).map_err(|_| 240)?;
    let lane = game_session_lane_mut(session, kind)?;
    lane.watch_queue_fd = None;
    lane.watch_token = None;
    Ok(())
}

fn game_wait_watch<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &GameCompatSession,
    kind: CompatLaneKind,
) -> Result<(), ExitCode> {
    let lane = game_session_lane(session, kind)?;
    let queue_fd = lane.watch_queue_fd.ok_or(299)?;
    shell_wait_event_queue(runtime, queue_fd)
}

fn game_poll_all_watches<B: SyscallBackend>(
    runtime: &Runtime<B>,
    sessions: &[GameCompatSession],
) -> Result<usize, ExitCode> {
    let mut polled = 0usize;
    for session in sessions {
        for lane in &session.lanes {
            let (Some(queue_fd), Some(token)) = (lane.watch_queue_fd, lane.watch_token) else {
                continue;
            };
            shell_wait_event_queue(runtime, queue_fd)?;
            write_line(
                runtime,
                &format!(
                    "game.watch.event pid={} slug={} kind={} queue={} token={}",
                    session.pid,
                    session.slug,
                    lane_name(lane.kind),
                    queue_fd,
                    token
                ),
            )?;
            polled = polled.saturating_add(1);
        }
    }
    if polled == 0 {
        return Err(299);
    }
    write_line(runtime, &format!("game.watch.poll count={polled}"))?;
    Ok(polled)
}

fn game_next_payload<B: SyscallBackend>(
    runtime: &Runtime<B>,
    session: &mut GameCompatSession,
) -> Result<(), ExitCode> {
    if !session.pending_graphics_frames.is_empty() {
        let encoded = session.pending_graphics_frames.remove(0);
        write_line(
            runtime,
            &format!(
                "game.next pid={} kind=graphics tag={} remaining[gfx={};audio={};input={}] payload={}",
                session.pid,
                encoded.frame_tag,
                session.pending_graphics_frames.len(),
                session.pending_audio_batches.len(),
                session.pending_input_batches.len(),
                encoded.payload
            ),
        )?;
        return Ok(());
    }
    if !session.pending_audio_batches.is_empty() {
        let encoded = session.pending_audio_batches.remove(0);
        write_line(
            runtime,
            &format!(
                "game.next pid={} kind=audio tag={} remaining[gfx={};audio={};input={}] payload={}",
                session.pid,
                encoded.stream_tag,
                session.pending_graphics_frames.len(),
                session.pending_audio_batches.len(),
                session.pending_input_batches.len(),
                encoded.payload
            ),
        )?;
        return Ok(());
    }
    if !session.pending_input_batches.is_empty() {
        let encoded = session.pending_input_batches.remove(0);
        write_line(
            runtime,
            &format!(
                "game.next pid={} kind=input tag={} remaining[gfx={};audio={};input={}] payload={}",
                session.pid,
                encoded.frame_tag,
                session.pending_graphics_frames.len(),
                session.pending_audio_batches.len(),
                session.pending_input_batches.len(),
                encoded.payload
            ),
        )?;
        return Ok(());
    }
    Err(299)
}

fn game_ensure_dir_tree<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<(), ExitCode> {
    let normalized = normalize_shell_path(path);
    if normalized == "/" {
        return Ok(());
    }
    let mut current = String::new();
    for segment in normalized.trim_start_matches('/').split('/') {
        current.push('/');
        current.push_str(segment);
        if runtime.mkdir_path(&current).is_err() {
            let status = runtime.stat_path(&current).map_err(|_| 241)?;
            if status.kind != NativeObjectKind::Directory as u32 {
                return Err(241);
            }
        }
    }
    Ok(())
}

fn game_write_runtime_bootstrap<B: SyscallBackend>(
    runtime: &Runtime<B>,
    plan: &GameSessionPlan,
    manifest: &GameCompatManifest,
) -> Result<(String, String, String), ExitCode> {
    if manifest.shims.prefix != "/" {
        game_ensure_dir_tree(runtime, &manifest.shims.prefix)?;
    }
    if manifest.shims.saves != "/" {
        game_ensure_dir_tree(runtime, &manifest.shims.saves)?;
    }
    if manifest.shims.cache != "/" {
        game_ensure_dir_tree(runtime, &manifest.shims.cache)?;
    }
    let env_path = if manifest.shims.prefix == "/" {
        String::from("/session.env")
    } else {
        format!("{}/session.env", manifest.shims.prefix)
    };
    let argv_path = if manifest.shims.prefix == "/" {
        String::from("/session.argv")
    } else {
        format!("{}/session.argv", manifest.shims.prefix)
    };
    let channel_path = if manifest.shims.prefix == "/" {
        String::from("/session.chan")
    } else {
        format!("{}/session.chan", manifest.shims.prefix)
    };
    let env_text = plan
        .env_shims
        .iter()
        .map(|shim| format!("{}={}", shim.key, shim.value))
        .chain(core::iter::once(format!(
            "NGOS_GAME_CHANNEL={channel_path}"
        )))
        .collect::<Vec<_>>()
        .join("\n");
    let argv_text = core::iter::once(plan.executable_path.clone())
        .chain(plan.argv.iter().cloned())
        .collect::<Vec<_>>()
        .join("\n");
    let _ = runtime.mkfile_path(&env_path);
    let _ = runtime.mkfile_path(&argv_path);
    let _ = runtime.mkchan_path(&channel_path);
    let env_fd = runtime.open_path(&env_path).map_err(|_| 237)?;
    shell_write_all(runtime, env_fd, env_text.as_bytes())?;
    runtime.close(env_fd).map_err(|_| 240)?;
    let argv_fd = runtime.open_path(&argv_path).map_err(|_| 237)?;
    shell_write_all(runtime, argv_fd, argv_text.as_bytes())?;
    runtime.close(argv_fd).map_err(|_| 240)?;
    Ok((env_path, argv_path, channel_path))
}

fn game_launch_session<B: SyscallBackend>(
    runtime: &Runtime<B>,
    _current_cwd: &mut String,
    manifest: &GameCompatManifest,
) -> Result<Box<GameCompatSession>, ExitCode> {
    fn rollback_partial_game_session<B: SyscallBackend>(
        runtime: &Runtime<B>,
        pid: Option<u64>,
        lanes: &mut [GameCompatLaneRuntime],
    ) {
        if let Some(pid) = pid {
            let _ = runtime.send_signal(pid, 15);
            let _ = runtime.reap_process(pid);
        }
        for lane in lanes.iter().rev() {
            if lane.claim_acquired {
                let _ = runtime.release_resource(lane.contract_id);
            }
            let _ = runtime.set_contract_state(lane.contract_id, NativeContractState::Suspended);
            let _ = runtime.set_resource_state(lane.resource_id, NativeResourceState::Suspended);
        }
    }

    let plan = manifest.session_plan();
    let (runtime_env_path, runtime_argv_path, runtime_channel_path) =
        game_write_runtime_bootstrap(runtime, &plan, manifest)?;
    let domain_id = runtime
        .create_domain(None, &plan.domain_name)
        .map_err(|_| 284)?;
    let mut lanes = Vec::new();
    for lane in &plan.lanes {
        let resource_id = match runtime.create_resource(
            domain_id,
            game_plan_resource_kind(lane.kind),
            &lane.resource_name,
        ) {
            Ok(resource_id) => resource_id,
            Err(_) => {
                rollback_partial_game_session(runtime, None, &mut lanes);
                return Err(284);
            }
        };
        if game_apply_resource_policy(runtime, resource_id, lane.kind).is_err() {
            rollback_partial_game_session(runtime, None, &mut lanes);
            return Err(284);
        }
        let contract_id = match runtime.create_contract(
            domain_id,
            resource_id,
            game_plan_contract_kind(lane.kind),
            &lane.contract_label,
        ) {
            Ok(contract_id) => contract_id,
            Err(_) => {
                let mut pending = lanes;
                pending.push(GameCompatLaneRuntime {
                    kind: lane.kind,
                    resource_id,
                    resource_name: lane.resource_name.clone(),
                    contract_id: 0,
                    contract_label: lane.contract_label.clone(),
                    claim_acquired: false,
                    invoke_token: None,
                    watch_queue_fd: None,
                    watch_token: None,
                });
                rollback_partial_game_session(runtime, None, &mut pending);
                return Err(284);
            }
        };
        if runtime
            .set_contract_state(contract_id, NativeContractState::Active)
            .is_err()
        {
            let mut pending = lanes;
            pending.push(GameCompatLaneRuntime {
                kind: lane.kind,
                resource_id,
                resource_name: lane.resource_name.clone(),
                contract_id,
                contract_label: lane.contract_label.clone(),
                claim_acquired: false,
                invoke_token: None,
                watch_queue_fd: None,
                watch_token: None,
            });
            rollback_partial_game_session(runtime, None, &mut pending);
            return Err(284);
        }
        if runtime.acquire_resource(contract_id).is_err() {
            let mut pending = lanes;
            pending.push(GameCompatLaneRuntime {
                kind: lane.kind,
                resource_id,
                resource_name: lane.resource_name.clone(),
                contract_id,
                contract_label: lane.contract_label.clone(),
                claim_acquired: false,
                invoke_token: None,
                watch_queue_fd: None,
                watch_token: None,
            });
            rollback_partial_game_session(runtime, None, &mut pending);
            return Err(284);
        }
        lanes.push(GameCompatLaneRuntime {
            kind: lane.kind,
            resource_id,
            resource_name: lane.resource_name.clone(),
            contract_id,
            contract_label: lane.contract_label.clone(),
            claim_acquired: true,
            invoke_token: None,
            watch_queue_fd: None,
            watch_token: None,
        });
    }
    let process_argv = core::iter::once(plan.executable_path.as_str())
        .chain(plan.argv.iter().map(String::as_str))
        .collect::<Vec<_>>();
    let process_env = plan
        .env_shims
        .iter()
        .map(|shim| format!("{}={}", shim.key, shim.value))
        .chain(core::iter::once(format!(
            "NGOS_GAME_CHANNEL={runtime_channel_path}"
        )))
        .collect::<Vec<_>>();
    let process_env_refs = process_env.iter().map(String::as_str).collect::<Vec<_>>();
    let pid = match runtime.spawn_configured_process(
        &plan.process_name,
        &plan.executable_path,
        &plan.working_dir,
        &process_argv,
        &process_env_refs,
    ) {
        Ok(pid) => pid,
        Err(_) => {
            rollback_partial_game_session(runtime, None, &mut lanes);
            return Err(288);
        }
    };

    Ok(Box::new(GameCompatSession {
        title: manifest.title.clone(),
        slug: manifest.slug.clone(),
        pid,
        domain_id,
        process_name: plan.process_name,
        executable_path: plan.executable_path,
        working_dir: plan.working_dir,
        prefix_path: manifest.shims.prefix.clone(),
        saves_path: manifest.shims.saves.clone(),
        cache_path: manifest.shims.cache.clone(),
        runtime_env_path,
        runtime_argv_path,
        runtime_channel_path,
        graphics_device_path: String::from("/dev/gpu0"),
        graphics_driver_path: String::from("/drv/gpu0"),
        graphics_profile: manifest.graphics.profile.clone(),
        audio_device_path: String::from("/dev/audio0"),
        audio_driver_path: String::from("/drv/audio0"),
        audio_profile: manifest.audio.profile.clone(),
        input_device_path: String::from("/dev/input0"),
        input_driver_path: String::from("/drv/input0"),
        input_profile: manifest.input.profile.clone(),
        last_frame_tag: None,
        last_graphics_queue: None,
        last_present_mode: None,
        last_completion_mode: None,
        last_completion_observed: None,
        last_frame_op_count: 0,
        last_frame_payload_bytes: 0,
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
        lanes,
        stopped: false,
        exit_code: None,
    }))
}

fn game_stop_session<B: SyscallBackend>(
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
            runtime
                .release_resource(lane.contract_id)
                .map_err(|_| 289)?;
        }
        runtime
            .set_contract_state(lane.contract_id, NativeContractState::Suspended)
            .map_err(|_| 289)?;
        runtime
            .set_resource_state(
                lane.resource_id,
                ngos_user_abi::NativeResourceState::Suspended,
            )
            .map_err(|_| 289)?;
    }
    Ok(())
}

fn shell_cleanup_game_sessions<B: SyscallBackend>(
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

fn shell_write_all<B: SyscallBackend>(
    runtime: &Runtime<B>,
    fd: usize,
    bytes: &[u8],
) -> Result<(), ExitCode> {
    let mut offset = 0usize;
    while offset < bytes.len() {
        let written = runtime.write(fd, &bytes[offset..]).map_err(|_| 240)?;
        if written == 0 {
            return Err(240);
        }
        offset += written;
    }
    Ok(())
}

fn shell_write_file<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    text: &str,
) -> Result<(), ExitCode> {
    match runtime.stat_path(path) {
        Ok(status) => {
            if status.kind == NativeObjectKind::File as u32 {
                let _ = runtime.unlink_path(path);
                runtime.mkfile_path(path).map_err(|_| 242)?;
            }
        }
        Err(_) => {
            runtime.mkfile_path(path).map_err(|_| 242)?;
        }
    }
    let fd = runtime.open_path(path).map_err(|_| 237)?;
    shell_write_all(runtime, fd, text.as_bytes())?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!("file-written path={path} bytes={}", text.len()),
    )
}

fn shell_append_file<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    text: &str,
) -> Result<(), ExitCode> {
    let fd = runtime.open_path(path).map_err(|_| 237)?;
    let mut drain = [0u8; 256];
    loop {
        let count = runtime.read(fd, &mut drain).map_err(|_| 238)?;
        if count == 0 {
            break;
        }
    }
    shell_write_all(runtime, fd, text.as_bytes())?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!("file-appended path={path} bytes={}", text.len()),
    )
}

fn shell_copy_file<B: SyscallBackend>(
    runtime: &Runtime<B>,
    source: &str,
    destination: &str,
) -> Result<(), ExitCode> {
    let src = runtime.open_path(source).map_err(|_| 237)?;
    let dst = runtime.open_path(destination).map_err(|_| 237)?;
    let mut buffer = [0u8; 256];
    let mut total = 0usize;
    loop {
        let count = runtime.read(src, &mut buffer).map_err(|_| 238)?;
        if count == 0 {
            break;
        }
        shell_write_all(runtime, dst, &buffer[..count])?;
        total += count;
    }
    runtime.close(src).map_err(|_| 240)?;
    runtime.close(dst).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!("file-copied from={source} to={destination} bytes={total}"),
    )
}

fn shell_compare_files<B: SyscallBackend>(
    runtime: &Runtime<B>,
    left: &str,
    right: &str,
) -> Result<(), ExitCode> {
    let left_text = shell_read_file_text(runtime, left)?;
    let right_text = shell_read_file_text(runtime, right)?;
    if left_text == right_text {
        return write_line(
            runtime,
            &format!(
                "files-match left={left} right={right} bytes={}",
                left_text.len()
            ),
        );
    }

    let left_bytes = left_text.as_bytes();
    let right_bytes = right_text.as_bytes();
    let mismatch = left_bytes
        .iter()
        .zip(right_bytes.iter())
        .position(|(left, right)| left != right)
        .unwrap_or_else(|| left_bytes.len().min(right_bytes.len()));
    write_line(
        runtime,
        &format!(
            "files-differ left={left} right={right} offset={mismatch} left-bytes={} right-bytes={}",
            left_bytes.len(),
            right_bytes.len()
        ),
    )
}

fn shell_grep_file<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    needle: &str,
) -> Result<(), ExitCode> {
    let text = shell_read_file_text(runtime, path)?;
    let mut matched = 0usize;
    for (index, line) in text.lines().enumerate() {
        if line.contains(needle) {
            matched += 1;
            write_line(runtime, &format!("grep {path}:{} {}", index + 1, line))?;
        }
    }
    write_line(
        runtime,
        &format!("grep-summary path={path} needle={needle} matches={matched}"),
    )
}

fn shell_assert_file_contains<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    needle: &str,
) -> Result<(), ExitCode> {
    let text = shell_read_file_text(runtime, path)?;
    if text.contains(needle) {
        write_line(
            runtime,
            &format!("assert-file-contains-ok path={path} needle={needle}"),
        )
    } else {
        Err(248)
    }
}

fn shell_mkdir_path<B: SyscallBackend>(runtime: &Runtime<B>, path: &str) -> Result<(), ExitCode> {
    runtime.mkdir_path(path).map_err(|_| 241)?;
    write_line(runtime, &format!("directory-created path={path}"))
}

fn shell_mkfile_path<B: SyscallBackend>(runtime: &Runtime<B>, path: &str) -> Result<(), ExitCode> {
    runtime.mkfile_path(path).map_err(|_| 242)?;
    write_line(runtime, &format!("file-created path={path}"))
}

fn shell_mksock_path<B: SyscallBackend>(runtime: &Runtime<B>, path: &str) -> Result<(), ExitCode> {
    runtime.mksock_path(path).map_err(|_| 242)?;
    write_line(runtime, &format!("socket-created path={path}"))
}

fn shell_symlink_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    target: &str,
) -> Result<(), ExitCode> {
    runtime.symlink_path(path, target).map_err(|_| 243)?;
    write_line(
        runtime,
        &format!("symlink-created path={path} target={target}"),
    )
}

fn shell_rename_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    from: &str,
    to: &str,
) -> Result<(), ExitCode> {
    runtime.rename_path(from, to).map_err(|_| 244)?;
    write_line(runtime, &format!("path-renamed from={from} to={to}"))
}

fn shell_unlink_path<B: SyscallBackend>(runtime: &Runtime<B>, path: &str) -> Result<(), ExitCode> {
    runtime.unlink_path(path).map_err(|_| 245)?;
    write_line(runtime, &format!("path-unlinked path={path}"))
}

fn shell_list_path<B: SyscallBackend>(runtime: &Runtime<B>, path: &str) -> Result<(), ExitCode> {
    let mut buffer = vec![0u8; 512];
    loop {
        let count = runtime.list_path(path, &mut buffer).map_err(|_| 246)?;
        if count < buffer.len() {
            let text = core::str::from_utf8(&buffer[..count]).map_err(|_| 247)?;
            if text.is_empty() {
                return write_line(runtime, &format!("path={path} entries=0"));
            }
            return shell_emit_lines(runtime, text);
        }
        buffer.resize(buffer.len() * 2, 0);
    }
}

fn shell_render_network_interface<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record: NativeNetworkInterfaceRecord = runtime
        .inspect_network_interface(device_path)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "netif path={} admin={} link={} promisc={} mtu={} tx-cap={} rx-cap={} inflight-limit={} inflight={} free-buffers={} mac={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x} addr={} netmask={} gateway={} rx-depth={} tx-depth={} rx-packets={} tx-packets={} tx-completions={} tx-dropped={} rx-dropped={} sockets={}",
            device_path,
            if record.admin_up != 0 { "up" } else { "down" },
            if record.link_up != 0 { "up" } else { "down" },
            if record.promiscuous != 0 { "on" } else { "off" },
            record.mtu,
            record.tx_capacity,
            record.rx_capacity,
            record.tx_inflight_limit,
            record.tx_inflight_depth,
            record.free_buffer_count,
            record.mac[0],
            record.mac[1],
            record.mac[2],
            record.mac[3],
            record.mac[4],
            record.mac[5],
            render_ipv4(record.ipv4_addr),
            render_ipv4(record.ipv4_netmask),
            render_ipv4(record.ipv4_gateway),
            record.rx_ring_depth,
            record.tx_ring_depth,
            record.rx_packets,
            record.tx_packets,
            record.tx_completions,
            record.tx_dropped,
            record.rx_dropped,
            record.attached_socket_count
        ),
    )
}

fn shell_render_device<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record: NativeDeviceRecord = runtime.inspect_device(device_path).map_err(|_| 246)?;
    let graphics_control_reserve = if record.class == 3 {
        if record.reserved0 != 0 {
            "armed"
        } else {
            "released"
        }
    } else {
        "n/a"
    };
    write_line(
        runtime,
        &format!(
            "device path={} class={} state={} queue-depth={} queue-capacity={} control-reserve={} submitted={} completed={} total-latency={} max-latency={} total-queue-wait={} max-queue-wait={} link={} block-size={} capacity-bytes={}",
            device_path,
            device_class_name(record.class),
            record.state,
            record.queue_depth,
            record.queue_capacity,
            graphics_control_reserve,
            record.submitted_requests,
            record.completed_requests,
            record.total_latency_ticks,
            record.max_latency_ticks,
            record.total_queue_wait_ticks,
            record.max_queue_wait_ticks,
            if record.link_up != 0 { "up" } else { "down" },
            record.block_size,
            record.capacity_bytes
        ),
    )
}

fn fixed_text_field(bytes: &[u8]) -> &str {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    core::str::from_utf8(&bytes[..end]).unwrap_or("")
}

fn shell_render_gpu_binding<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record = runtime.inspect_gpu_binding(device_path).map_err(|_| 246)?;
    if record.present == 0 {
        return write_line(
            runtime,
            &format!("gpu-binding device={} status=unavailable", device_path),
        );
    }
    write_line(
        runtime,
        &format!(
            "gpu-binding device={} architecture={} product={} die={} bus-interface={} inf-section={} kernel-service={} vbios={} part={} subsystem=0x{:08x} bar1-mib={} framebuffer-mib={} resizable-bar={} display-engine-confirmed={} msi-source={} msi-supported={} msi-limit={}",
            device_path,
            fixed_text_field(&record.architecture_name),
            fixed_text_field(&record.product_name),
            fixed_text_field(&record.die_name),
            fixed_text_field(&record.bus_interface),
            fixed_text_field(&record.inf_section),
            fixed_text_field(&record.kernel_service),
            fixed_text_field(&record.vbios_version),
            fixed_text_field(&record.part_number),
            record.subsystem_id,
            record.bar1_total_mib,
            record.framebuffer_total_mib,
            record.resizable_bar_enabled,
            record.display_engine_confirmed,
            fixed_text_field(&record.msi_source_name),
            record.msi_supported,
            record.msi_message_limit
        ),
    )
}

fn shell_render_gpu_vbios<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record = runtime.inspect_gpu_vbios(device_path).map_err(|_| 246)?;
    if record.present == 0 {
        return write_line(
            runtime,
            &format!("gpu-vbios device={} status=unavailable", device_path),
        );
    }
    let header_len = core::cmp::min(record.header_len as usize, record.header.len());
    let mut header_hex = String::new();
    for (index, byte) in record.header[..header_len].iter().enumerate() {
        if index != 0 {
            header_hex.push(':');
        }
        let _ = write!(&mut header_hex, "{:02x}", byte);
    }
    write_line(
        runtime,
        &format!(
            "gpu-vbios device={} enabled={} rom-bar=0x{:08x} physical-base=0x{:x} image-len={} vendor=0x{:04x} device=0x{:04x} pcir=0x{:x} bit=0x{:x} nvfw=0x{:x} board={} code={} version={} header-len={} header={}",
            device_path,
            record.enabled,
            record.rom_bar_raw,
            record.physical_base,
            record.image_len,
            record.vendor_id,
            record.device_id,
            record.pcir_offset,
            record.bit_offset,
            record.nvfw_offset,
            fixed_text_field(&record.board_name),
            fixed_text_field(&record.board_code),
            fixed_text_field(&record.version),
            record.header_len,
            header_hex
        ),
    )
}

fn shell_render_gpu_gsp<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record = runtime.inspect_gpu_gsp(device_path).map_err(|_| 246)?;
    if record.present == 0 {
        return write_line(
            runtime,
            &format!("gpu-gsp device={} status=unavailable", device_path),
        );
    }
    write_line(
        runtime,
        &format!(
            "gpu-gsp device={} ready={} completions={} failures={} firmware-known={} firmware-version={} blackwell-blob={} blobs={}",
            device_path,
            record.loopback_ready,
            record.loopback_completions,
            record.loopback_failures,
            record.firmware_known,
            fixed_text_field(&record.firmware_version),
            record.blackwell_blob_present,
            fixed_text_field(&record.blob_summary),
        ),
    )
}

fn shell_render_gpu_interrupt<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record = runtime
        .inspect_gpu_interrupt(device_path)
        .map_err(|_| 246)?;
    if record.present == 0 {
        return write_line(
            runtime,
            &format!("gpu-irq device={} status=unavailable", device_path),
        );
    }
    write_line(
        runtime,
        &format!(
            "gpu-irq device={} vector={} delivered={} msi-supported={} message-limit={} windows-max={} hardware-confirmed={}",
            device_path,
            record.vector,
            record.delivered_count,
            record.msi_supported,
            record.message_limit,
            record.windows_interrupt_message_maximum,
            record.hardware_servicing_confirmed
        ),
    )
}

fn shell_render_gpu_display<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record = runtime.inspect_gpu_display(device_path).map_err(|_| 246)?;
    if record.present == 0 {
        return write_line(
            runtime,
            &format!("gpu-display device={} status=unavailable", device_path),
        );
    }
    write_line(
        runtime,
        &format!(
            "gpu-display device={} pipes={} planned={} hardware-confirmed={}",
            device_path,
            record.active_pipes,
            record.planned_frames,
            record.hardware_programming_confirmed
        ),
    )
}

fn shell_render_gpu_power<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record = runtime.inspect_gpu_power(device_path).map_err(|_| 247)?;
    if record.present == 0 {
        return write_line(
            runtime,
            &format!("gpu-power device={} status=unavailable", device_path),
        );
    }
    write_line(
        runtime,
        &format!(
            "gpu-power device={} pstate=P{} graphics-mhz={} memory-mhz={} boost-mhz={} hardware-confirmed={}",
            device_path,
            record.pstate,
            record.graphics_clock_mhz,
            record.memory_clock_mhz,
            record.boost_clock_mhz,
            record.hardware_power_management_confirmed
        ),
    )
}

fn parse_gpu_power_state(text: &str) -> Option<u32> {
    match text.trim() {
        "P0" | "p0" => Some(0),
        "P5" | "p5" => Some(5),
        "P8" | "p8" => Some(8),
        "P12" | "p12" => Some(12),
        _ => None,
    }
}

fn shell_set_gpu_power<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    state_text: &str,
) -> Result<(), ExitCode> {
    let Some(pstate) = parse_gpu_power_state(state_text) else {
        return write_line(
            runtime,
            &format!(
                "gpu-power-set device={} state={} status=invalid",
                device_path, state_text
            ),
        );
    };
    if runtime.set_gpu_power_state(device_path, pstate).is_err() {
        return write_line(
            runtime,
            &format!(
                "gpu-power-set device={} state=P{} status=unavailable",
                device_path, pstate
            ),
        );
    }
    write_line(
        runtime,
        &format!(
            "gpu-power-set device={} state=P{} status=ok",
            device_path, pstate
        ),
    )
}

fn shell_render_gpu_media<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record = runtime.inspect_gpu_media(device_path).map_err(|_| 247)?;
    if record.present == 0 {
        return write_line(
            runtime,
            &format!("gpu-media device={} status=unavailable", device_path),
        );
    }
    write_line(
        runtime,
        &format!(
            "gpu-media device={} sessions={} codec={} width={} height={} bitrate-kbps={} hardware-confirmed={}",
            device_path,
            record.sessions,
            record.codec,
            record.width,
            record.height,
            record.bitrate_kbps,
            record.hardware_media_confirmed
        ),
    )
}

fn shell_start_gpu_media<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    width_text: &str,
    height_text: &str,
    bitrate_text: &str,
    codec_text: &str,
) -> Result<(), ExitCode> {
    let Ok(width) = width_text.parse::<u32>() else {
        return write_line(
            runtime,
            &format!("gpu-media-start device={} status=invalid", device_path),
        );
    };
    let Ok(height) = height_text.parse::<u32>() else {
        return write_line(
            runtime,
            &format!("gpu-media-start device={} status=invalid", device_path),
        );
    };
    let Ok(bitrate_kbps) = bitrate_text.parse::<u32>() else {
        return write_line(
            runtime,
            &format!("gpu-media-start device={} status=invalid", device_path),
        );
    };
    if width == 0 || height == 0 || bitrate_kbps == 0 {
        return write_line(
            runtime,
            &format!("gpu-media-start device={} status=invalid", device_path),
        );
    }
    let codec = match codec_text {
        "h264" => 0,
        "hevc" => 1,
        "av1" => 2,
        _ => {
            return write_line(
                runtime,
                &format!("gpu-media-start device={} status=invalid", device_path),
            );
        }
    };
    if runtime
        .start_gpu_media_session(device_path, width, height, bitrate_kbps, codec)
        .is_err()
    {
        return write_line(
            runtime,
            &format!("gpu-media-start device={} status=unavailable", device_path),
        );
    }
    write_line(
        runtime,
        &format!(
            "gpu-media-start device={} width={} height={} bitrate-kbps={} codec={} status=ok",
            device_path, width, height, bitrate_kbps, codec_text
        ),
    )
}

fn shell_render_gpu_neural<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record = runtime.inspect_gpu_neural(device_path).map_err(|_| 247)?;
    if record.present == 0 {
        return write_line(
            runtime,
            &format!("gpu-neural device={} status=unavailable", device_path),
        );
    }
    write_line(
        runtime,
        &format!(
            "gpu-neural device={} model-loaded={} semantics={} committed={} hardware-confirmed={}",
            device_path,
            record.model_loaded,
            record.active_semantics,
            record.last_commit_completed,
            record.hardware_neural_confirmed
        ),
    )
}

fn shell_inject_gpu_neural<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    semantic_label: &str,
) -> Result<(), ExitCode> {
    if semantic_label.trim().is_empty() {
        return write_line(
            runtime,
            &format!("gpu-neural-inject device={} status=invalid", device_path),
        );
    }
    if runtime
        .inject_gpu_neural_semantic(device_path, semantic_label)
        .is_err()
    {
        return write_line(
            runtime,
            &format!(
                "gpu-neural-inject device={} status=unavailable",
                device_path
            ),
        );
    }
    write_line(
        runtime,
        &format!(
            "gpu-neural-inject device={} semantic={} status=ok",
            device_path, semantic_label
        ),
    )
}

fn shell_commit_gpu_neural<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    if runtime.commit_gpu_neural_frame(device_path).is_err() {
        return write_line(
            runtime,
            &format!(
                "gpu-neural-commit device={} status=unavailable",
                device_path
            ),
        );
    }
    write_line(
        runtime,
        &format!("gpu-neural-commit device={} status=ok", device_path),
    )
}

fn shell_render_gpu_tensor<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record = runtime.inspect_gpu_tensor(device_path).map_err(|_| 247)?;
    if record.present == 0 {
        return write_line(
            runtime,
            &format!("gpu-tensor device={} status=unavailable", device_path),
        );
    }
    write_line(
        runtime,
        &format!(
            "gpu-tensor device={} active-jobs={} last-kernel={} hardware-confirmed={}",
            device_path,
            record.active_jobs,
            record.last_kernel_id,
            record.hardware_tensor_confirmed
        ),
    )
}

fn shell_dispatch_gpu_tensor<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    kernel_text: &str,
) -> Result<(), ExitCode> {
    let Ok(kernel_id) = kernel_text.parse::<u32>() else {
        return write_line(
            runtime,
            &format!("gpu-tensor-dispatch device={} status=invalid", device_path),
        );
    };
    if kernel_id == 0 {
        return write_line(
            runtime,
            &format!("gpu-tensor-dispatch device={} status=invalid", device_path),
        );
    }
    if runtime
        .dispatch_gpu_tensor_kernel(device_path, kernel_id)
        .is_err()
    {
        return write_line(
            runtime,
            &format!(
                "gpu-tensor-dispatch device={} status=unavailable",
                device_path
            ),
        );
    }
    write_line(
        runtime,
        &format!(
            "gpu-tensor-dispatch device={} kernel={} status=ok",
            device_path, kernel_id
        ),
    )
}

fn shell_gpu_submit<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    payload: &str,
) -> Result<(), ExitCode> {
    let fd = runtime.open_path(device_path).map_err(|_| 234)?;
    shell_write_all(runtime, fd, payload.as_bytes())?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!(
            "gpu-submit device={} bytes={} payload={}",
            device_path,
            payload.len(),
            payload
        ),
    )
}

fn shell_gpu_queue_capacity<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    queue_capacity: usize,
) -> Result<(), ExitCode> {
    runtime
        .configure_device_queue(device_path, queue_capacity)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "gpu-queue-capacity device={} queue-capacity={}",
            device_path, queue_capacity
        ),
    )
}

const GPU_PRESENT_OPCODE: u32 = 0x4750_0001;
const GPU_DRIVER_RESET_OPCODE: u32 = 0x4750_1001;
const GPU_DRIVER_RETIRE_OPCODE: u32 = 0x4750_1002;

fn shell_gpu_probe_submit<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    payload: &str,
) -> Result<(), ExitCode> {
    let before = runtime.inspect_device(device_path).ok();
    match runtime.open_path(device_path) {
        Ok(fd) => {
            let outcome = shell_write_all(runtime, fd, payload.as_bytes());
            let close_result = runtime.close(fd);
            let after = runtime.inspect_device(device_path).ok();
            match (outcome, close_result, before, after) {
                (Ok(()), Ok(()), Some(before), Some(after))
                    if after.submitted_requests > before.submitted_requests =>
                {
                    write_line(
                        runtime,
                        &format!(
                            "gpu-probe-submit device={} bytes={} outcome=submitted payload={}",
                            device_path,
                            payload.len(),
                            payload
                        ),
                    )
                }
                _ => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-submit device={} bytes={} outcome=error",
                        device_path,
                        payload.len()
                    ),
                ),
            }
        }
        Err(_) => write_line(
            runtime,
            &format!(
                "gpu-probe-submit device={} bytes={} outcome=error",
                device_path,
                payload.len()
            ),
        ),
    }
}

fn shell_gpu_present<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    frame_token: &str,
) -> Result<(), ExitCode> {
    let response = runtime
        .present_gpu_frame(device_path, frame_token.as_bytes())
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "gpu-present device={} opcode=0x{:08x} response=0x{:08x} frame={}",
            device_path, GPU_PRESENT_OPCODE, response, frame_token
        ),
    )
}

fn shell_gpu_probe_present<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    frame_token: &str,
) -> Result<(), ExitCode> {
    let before = runtime.inspect_device(device_path).ok();
    match runtime.open_path(device_path) {
        Ok(fd) => {
            let outcome = runtime.control(fd, GPU_PRESENT_OPCODE);
            let close_result = runtime.close(fd);
            let after = runtime.inspect_device(device_path).ok();
            match (outcome, close_result, before, after) {
                (Ok(response), Ok(()), Some(before), Some(after))
                    if after.submitted_requests > before.submitted_requests =>
                {
                    write_line(
                        runtime,
                        &format!(
                            "gpu-probe-present device={} opcode=0x{:08x} response=0x{:08x} outcome=presented frame={}",
                            device_path, GPU_PRESENT_OPCODE, response, frame_token
                        ),
                    )
                }
                _ => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-present device={} opcode=0x{:08x} outcome=error frame={}",
                        device_path, GPU_PRESENT_OPCODE, frame_token
                    ),
                ),
            }
        }
        Err(_) => write_line(
            runtime,
            &format!(
                "gpu-probe-present device={} opcode=0x{:08x} outcome=error frame={}",
                device_path, GPU_PRESENT_OPCODE, frame_token
            ),
        ),
    }
}

fn shell_gpu_driver_reset<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
) -> Result<(), ExitCode> {
    let fd = runtime.open_path(driver_path).map_err(|_| 234)?;
    let canceled = runtime
        .control(fd, GPU_DRIVER_RESET_OPCODE)
        .map_err(|_| 246)?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!(
            "gpu-driver-reset driver={} opcode=0x{:08x} canceled={}",
            driver_path, GPU_DRIVER_RESET_OPCODE, canceled
        ),
    )
}

fn shell_gpu_probe_driver_reset<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
) -> Result<(), ExitCode> {
    match runtime.open_path(driver_path) {
        Ok(fd) => {
            let outcome = runtime.control(fd, GPU_DRIVER_RESET_OPCODE);
            let close_result = runtime.close(fd);
            match (outcome, close_result) {
                (Ok(canceled), Ok(())) => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-driver-reset driver={} opcode=0x{:08x} canceled={} outcome=reset",
                        driver_path, GPU_DRIVER_RESET_OPCODE, canceled
                    ),
                ),
                _ => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-driver-reset driver={} opcode=0x{:08x} outcome=error",
                        driver_path, GPU_DRIVER_RESET_OPCODE
                    ),
                ),
            }
        }
        Err(_) => write_line(
            runtime,
            &format!(
                "gpu-probe-driver-reset driver={} opcode=0x{:08x} outcome=error",
                driver_path, GPU_DRIVER_RESET_OPCODE
            ),
        ),
    }
}

fn shell_gpu_driver_retire<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
) -> Result<(), ExitCode> {
    let fd = runtime.open_path(driver_path).map_err(|_| 234)?;
    let canceled = runtime
        .control(fd, GPU_DRIVER_RETIRE_OPCODE)
        .map_err(|_| 246)?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!(
            "gpu-driver-retire driver={} opcode=0x{:08x} canceled={}",
            driver_path, GPU_DRIVER_RETIRE_OPCODE, canceled
        ),
    )
}

fn shell_gpu_probe_driver_retire<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
) -> Result<(), ExitCode> {
    match runtime.open_path(driver_path) {
        Ok(fd) => {
            let outcome = runtime.control(fd, GPU_DRIVER_RETIRE_OPCODE);
            let close_result = runtime.close(fd);
            match (outcome, close_result) {
                (Ok(canceled), Ok(())) => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-driver-retire driver={} opcode=0x{:08x} canceled={} outcome=retired",
                        driver_path, GPU_DRIVER_RETIRE_OPCODE, canceled
                    ),
                ),
                _ => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-driver-retire driver={} opcode=0x{:08x} outcome=error",
                        driver_path, GPU_DRIVER_RETIRE_OPCODE
                    ),
                ),
            }
        }
        Err(_) => write_line(
            runtime,
            &format!(
                "gpu-probe-driver-retire driver={} opcode=0x{:08x} outcome=error",
                driver_path, GPU_DRIVER_RETIRE_OPCODE
            ),
        ),
    }
}

fn shell_gpu_driver_bind<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    driver_path: &str,
) -> Result<(), ExitCode> {
    runtime
        .bind_device_driver(device_path, driver_path)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "gpu-driver-bind device={} driver={}",
            device_path, driver_path
        ),
    )
}

fn shell_gpu_probe_driver_bind<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    driver_path: &str,
) -> Result<(), ExitCode> {
    let before_device = runtime.inspect_device(device_path).ok();
    let before_driver = runtime.inspect_driver(driver_path).ok();
    match runtime.bind_device_driver(device_path, driver_path) {
        Ok(()) => {
            let after_device = runtime.inspect_device(device_path).ok();
            let after_driver = runtime.inspect_driver(driver_path).ok();
            match (before_device, after_device, before_driver, after_driver) {
                (
                    Some(before_device),
                    Some(after_device),
                    Some(before_driver),
                    Some(after_driver),
                ) if before_device.state != after_device.state
                    || before_driver.bound_device_count != after_driver.bound_device_count =>
                {
                    write_line(
                        runtime,
                        &format!(
                            "gpu-probe-driver-bind device={} driver={} outcome=bound",
                            device_path, driver_path
                        ),
                    )
                }
                _ => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-driver-bind device={} driver={} outcome=error",
                        device_path, driver_path
                    ),
                ),
            }
        }
        Err(_) => write_line(
            runtime,
            &format!(
                "gpu-probe-driver-bind device={} driver={} outcome=error",
                device_path, driver_path
            ),
        ),
    }
}

fn shell_gpu_driver_unbind<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    runtime.unbind_device_driver(device_path).map_err(|_| 246)?;
    write_line(
        runtime,
        &format!("gpu-driver-unbind device={}", device_path),
    )
}

fn shell_gpu_probe_driver_unbind<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let before_device = runtime.inspect_device(device_path).ok();
    match runtime.unbind_device_driver(device_path) {
        Ok(()) => {
            let after_device = runtime.inspect_device(device_path).ok();
            match (before_device, after_device) {
                (Some(before_device), Some(after_device))
                    if before_device.state != after_device.state =>
                {
                    write_line(
                        runtime,
                        &format!(
                            "gpu-probe-driver-unbind device={} outcome=unbound",
                            device_path
                        ),
                    )
                }
                _ => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-driver-unbind device={} outcome=error",
                        device_path
                    ),
                ),
            }
        }
        Err(_) => write_line(
            runtime,
            &format!(
                "gpu-probe-driver-unbind device={} outcome=error",
                device_path
            ),
        ),
    }
}

fn gpu_request_kind_name(kind: u32) -> &'static str {
    match kind {
        0 => "read",
        1 => "write",
        2 => "control",
        _ => "unknown",
    }
}

fn gpu_request_state_name(state: u32) -> &'static str {
    match state {
        0 => "queued",
        1 => "inflight",
        2 => "completed",
        3 => "failed",
        4 => "canceled",
        _ => "unknown",
    }
}

fn shell_gpu_request<B: SyscallBackend>(
    runtime: &Runtime<B>,
    request_id: u64,
) -> Result<(), ExitCode> {
    let record: NativeDeviceRequestRecord = runtime
        .inspect_device_request(request_id)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "gpu-request id={} issuer={} kind={} state={} opcode=0x{:08x} buffer={} payload={} response={} submitted={} started={} completed={}",
            request_id,
            record.issuer,
            gpu_request_kind_name(record.kind),
            gpu_request_state_name(record.state),
            record.opcode as u32,
            record.buffer_id,
            record.payload_len,
            record.response_len,
            record.submitted_tick,
            record.started_tick,
            record.completed_tick
        ),
    )
}

fn shell_gpu_buffer_create<B: SyscallBackend>(
    runtime: &Runtime<B>,
    length: usize,
) -> Result<(), ExitCode> {
    let buffer_id = runtime.create_gpu_buffer(length).map_err(|_| 246)?;
    write_line(
        runtime,
        &format!("gpu-buffer-create id={} length={}", buffer_id, length),
    )
}

fn shell_gpu_buffer_write<B: SyscallBackend>(
    runtime: &Runtime<B>,
    buffer_id: u64,
    offset: usize,
    payload: &str,
) -> Result<(), ExitCode> {
    let written = runtime
        .write_gpu_buffer(buffer_id, offset, payload.as_bytes())
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "gpu-buffer-write id={} offset={} bytes={} payload={}",
            buffer_id, offset, written, payload
        ),
    )
}

fn shell_gpu_buffer<B: SyscallBackend>(
    runtime: &Runtime<B>,
    buffer_id: u64,
) -> Result<(), ExitCode> {
    let record = runtime.inspect_gpu_buffer(buffer_id).map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "gpu-buffer id={} owner={} length={} used={}",
            buffer_id, record.owner, record.length, record.used_len
        ),
    )
}

fn shell_gpu_scanout<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record: NativeGpuScanoutRecord =
        runtime.inspect_gpu_scanout(device_path).map_err(|_| 246)?;
    let mut buffer = vec![0u8; record.last_frame_len as usize];
    let copied = runtime
        .read_gpu_scanout_frame(device_path, &mut buffer)
        .map_err(|_| 246)?;
    buffer.truncate(copied);
    let frame = String::from_utf8_lossy(&buffer);
    write_line(
        runtime,
        &format!(
            "gpu-scanout device={} presented={} last-frame-bytes={} frame={}",
            device_path, record.presented_frames, copied, frame
        ),
    )
}

fn shell_gpu_perf<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    let record: NativeDeviceRecord = runtime.inspect_device(device_path).map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "gpu-perf device={} submitted={} completed={} total-latency={} max-latency={} total-queue-wait={} max-queue-wait={}",
            device_path,
            record.submitted_requests,
            record.completed_requests,
            record.total_latency_ticks,
            record.max_latency_ticks,
            record.total_queue_wait_ticks,
            record.max_queue_wait_ticks
        ),
    )
}

fn shell_gpu_submit_buffer<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    buffer_id: u64,
) -> Result<(), ExitCode> {
    let submitted = runtime
        .submit_gpu_buffer(device_path, buffer_id)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "gpu-submit-buffer device={} buffer={} submitted={}",
            device_path, buffer_id, submitted
        ),
    )
}

fn shell_gpu_probe_submit_buffer<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    buffer_id: u64,
) -> Result<(), ExitCode> {
    let before = runtime.inspect_device(device_path).ok();
    match runtime.submit_gpu_buffer(device_path, buffer_id) {
        Ok(submitted) => {
            let after = runtime.inspect_device(device_path).ok();
            match (before, after) {
                (Some(before), Some(after))
                    if after.submitted_requests > before.submitted_requests =>
                {
                    write_line(
                        runtime,
                        &format!(
                            "gpu-probe-submit-buffer device={} buffer={} submitted={} outcome=submitted",
                            device_path, buffer_id, submitted
                        ),
                    )
                }
                _ => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-submit-buffer device={} buffer={} outcome=error",
                        device_path, buffer_id
                    ),
                ),
            }
        }
        Err(_) => write_line(
            runtime,
            &format!(
                "gpu-probe-submit-buffer device={} buffer={} outcome=error",
                device_path, buffer_id
            ),
        ),
    }
}

fn shell_gpu_driver_read<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
) -> Result<(), ExitCode> {
    if let Ok(record) = runtime.inspect_driver(driver_path)
        && record.queued_requests == 0
        && record.in_flight_requests == 0
    {
        return write_line(
            runtime,
            &format!("gpu-driver-read driver={} outcome=empty", driver_path),
        );
    }
    let fd = runtime.open_path(driver_path).map_err(|_| 234)?;
    let mut buffer = [0u8; 512];
    let count = runtime.read(fd, &mut buffer).map_err(|_| 238)?;
    runtime.close(fd).map_err(|_| 240)?;
    if count == 0 {
        return write_line(
            runtime,
            &format!("gpu-driver-read driver={} outcome=empty", driver_path),
        );
    }
    let prefix_len = buffer[..count]
        .iter()
        .position(|byte| *byte == b'\n')
        .map(|index| index + 1)
        .unwrap_or(count);
    let header = core::str::from_utf8(&buffer[..prefix_len]).map_err(|_| 239)?;
    let payload = core::str::from_utf8(&buffer[prefix_len..count]).map_err(|_| 239)?;
    let header = header.trim_end();
    write_line(
        runtime,
        &format!(
            "gpu-driver-read driver={} outcome=request header={} payload={}",
            driver_path, header, payload
        ),
    )
}

fn shell_gpu_complete<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    payload: &str,
) -> Result<(), ExitCode> {
    let fd = runtime.open_path(driver_path).map_err(|_| 234)?;
    shell_write_all(runtime, fd, payload.as_bytes())?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!(
            "gpu-complete driver={} bytes={} payload={}",
            driver_path,
            payload.len(),
            payload
        ),
    )
}

fn shell_gpu_complete_request<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    request_id: u64,
    payload: &str,
) -> Result<(), ExitCode> {
    let encoded = format!("request:{request_id}\n{payload}");
    let fd = runtime.open_path(driver_path).map_err(|_| 234)?;
    shell_write_all(runtime, fd, encoded.as_bytes())?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!(
            "gpu-complete-request driver={} request={} bytes={} payload={}",
            driver_path,
            request_id,
            payload.len(),
            payload
        ),
    )
}

fn shell_gpu_fail_request<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    request_id: u64,
    payload: &str,
) -> Result<(), ExitCode> {
    let encoded = format!("failed-request:{request_id}\n{payload}");
    let fd = runtime.open_path(driver_path).map_err(|_| 234)?;
    shell_write_all(runtime, fd, encoded.as_bytes())?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!(
            "gpu-fail-request driver={} request={} bytes={} payload={}",
            driver_path,
            request_id,
            payload.len(),
            payload
        ),
    )
}

fn shell_gpu_cancel_request<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    request_id: u64,
    payload: &str,
) -> Result<(), ExitCode> {
    let encoded = format!("cancel-request:{request_id}\n{payload}");
    let fd = runtime.open_path(driver_path).map_err(|_| 234)?;
    shell_write_all(runtime, fd, encoded.as_bytes())?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!(
            "gpu-cancel-request driver={} request={} bytes={} payload={}",
            driver_path,
            request_id,
            payload.len(),
            payload
        ),
    )
}

fn shell_gpu_probe_complete<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    payload: &str,
) -> Result<(), ExitCode> {
    let before = runtime.inspect_driver(driver_path).ok();
    match runtime.open_path(driver_path) {
        Ok(fd) => {
            let outcome = shell_write_all(runtime, fd, payload.as_bytes());
            let close_result = runtime.close(fd);
            let after = runtime.inspect_driver(driver_path).ok();
            match (outcome, close_result, before, after) {
                (Ok(()), Ok(()), Some(before), Some(after))
                    if after.completed_requests > before.completed_requests =>
                {
                    write_line(
                        runtime,
                        &format!(
                            "gpu-probe-complete driver={} bytes={} outcome=completed payload={}",
                            driver_path,
                            payload.len(),
                            payload
                        ),
                    )
                }
                _ => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-complete driver={} bytes={} outcome=error",
                        driver_path,
                        payload.len()
                    ),
                ),
            }
        }
        Err(_) => write_line(
            runtime,
            &format!(
                "gpu-probe-complete driver={} bytes={} outcome=error",
                driver_path,
                payload.len()
            ),
        ),
    }
}

fn shell_gpu_probe_complete_request<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    request_id: u64,
    payload: &str,
) -> Result<(), ExitCode> {
    let before = runtime.inspect_driver(driver_path).ok();
    let encoded = format!("request:{request_id}\n{payload}");
    match runtime.open_path(driver_path) {
        Ok(fd) => {
            let outcome = shell_write_all(runtime, fd, encoded.as_bytes());
            let close_result = runtime.close(fd);
            let after = runtime.inspect_driver(driver_path).ok();
            match (outcome, close_result, before, after) {
                (Ok(()), Ok(()), Some(before), Some(after))
                    if after.completed_requests > before.completed_requests =>
                {
                    write_line(
                        runtime,
                        &format!(
                            "gpu-probe-complete-request driver={} request={} bytes={} outcome=completed payload={}",
                            driver_path,
                            request_id,
                            payload.len(),
                            payload
                        ),
                    )
                }
                _ => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-complete-request driver={} request={} bytes={} outcome=error",
                        driver_path,
                        request_id,
                        payload.len()
                    ),
                ),
            }
        }
        Err(_) => write_line(
            runtime,
            &format!(
                "gpu-probe-complete-request driver={} request={} bytes={} outcome=error",
                driver_path,
                request_id,
                payload.len()
            ),
        ),
    }
}

fn shell_gpu_read<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
) -> Result<(), ExitCode> {
    if let Ok(record) = runtime.inspect_device(device_path)
        && record.submitted_requests == 0
        && record.completed_requests == 0
    {
        return write_line(
            runtime,
            &format!("gpu-read device={} outcome=empty", device_path),
        );
    }
    let fd = runtime.open_path(device_path).map_err(|_| 234)?;
    let mut buffer = [0u8; 512];
    let count = runtime.read(fd, &mut buffer).map_err(|_| 238)?;
    runtime.close(fd).map_err(|_| 240)?;
    if count == 0 {
        return write_line(
            runtime,
            &format!("gpu-read device={} outcome=empty", device_path),
        );
    }
    let payload = core::str::from_utf8(&buffer[..count]).map_err(|_| 239)?;
    write_line(
        runtime,
        &format!(
            "gpu-read device={} bytes={} payload={}",
            device_path, count, payload
        ),
    )
}

fn shell_gpu_probe_fail_request<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    request_id: u64,
    payload: &str,
) -> Result<(), ExitCode> {
    let before = runtime.inspect_driver(driver_path).ok();
    let encoded = format!("failed-request:{request_id}\n{payload}");
    match runtime.open_path(driver_path) {
        Ok(fd) => {
            let outcome = shell_write_all(runtime, fd, encoded.as_bytes());
            let close_result = runtime.close(fd);
            let after = runtime.inspect_driver(driver_path).ok();
            match (outcome, close_result, before, after) {
                (Ok(()), Ok(()), Some(before), Some(after))
                    if after.completed_requests > before.completed_requests =>
                {
                    write_line(
                        runtime,
                        &format!(
                            "gpu-probe-fail-request driver={} request={} bytes={} outcome=failed payload={}",
                            driver_path,
                            request_id,
                            payload.len(),
                            payload
                        ),
                    )
                }
                _ => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-fail-request driver={} request={} bytes={} outcome=error",
                        driver_path,
                        request_id,
                        payload.len()
                    ),
                ),
            }
        }
        Err(_) => write_line(
            runtime,
            &format!(
                "gpu-probe-fail-request driver={} request={} bytes={} outcome=error",
                driver_path,
                request_id,
                payload.len()
            ),
        ),
    }
}

fn shell_gpu_probe_cancel_request<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    request_id: u64,
    payload: &str,
) -> Result<(), ExitCode> {
    let before = runtime.inspect_device_request(request_id).ok();
    let encoded = format!("cancel-request:{request_id}\n{payload}");
    match runtime.open_path(driver_path) {
        Ok(fd) => {
            let outcome = shell_write_all(runtime, fd, encoded.as_bytes());
            let close_result = runtime.close(fd);
            let after = runtime.inspect_device_request(request_id).ok();
            match (outcome, close_result, before, after) {
                (Ok(()), Ok(()), Some(before), Some(after))
                    if before.state != 4 && after.state == 4 =>
                {
                    write_line(
                        runtime,
                        &format!(
                            "gpu-probe-cancel-request driver={} request={} bytes={} outcome=canceled payload={}",
                            driver_path,
                            request_id,
                            payload.len(),
                            payload
                        ),
                    )
                }
                _ => write_line(
                    runtime,
                    &format!(
                        "gpu-probe-cancel-request driver={} request={} bytes={} outcome=error",
                        driver_path,
                        request_id,
                        payload.len()
                    ),
                ),
            }
        }
        Err(_) => write_line(
            runtime,
            &format!(
                "gpu-probe-cancel-request driver={} request={} bytes={} outcome=error",
                driver_path,
                request_id,
                payload.len()
            ),
        ),
    }
}

fn shell_watch_gpu_lease<B: SyscallBackend>(
    runtime: &Runtime<B>,
    resource: usize,
    token: u64,
) -> Result<usize, ExitCode> {
    let queue_fd = runtime
        .create_event_queue(NativeEventQueueMode::Kqueue)
        .map_err(|_| 246)?;
    runtime
        .watch_resource_events(
            queue_fd, resource, token, true, true, true, true, true, true, POLLPRI,
        )
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "gpu-lease-watch queue={} resource={} token={}",
            queue_fd, resource, token
        ),
    )?;
    Ok(queue_fd)
}

fn shell_remove_gpu_lease_watch<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
    resource: usize,
    token: u64,
) -> Result<(), ExitCode> {
    runtime
        .remove_resource_events(queue_fd, resource, token)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "gpu-lease-unwatch queue={} resource={} token={}",
            queue_fd, resource, token
        ),
    )
}

fn shell_wait_gpu_lease<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
) -> Result<(), ExitCode> {
    let mut records = [NativeEventRecord {
        token: 0,
        events: 0,
        source_kind: 0,
        source_arg0: 0,
        source_arg1: 0,
        source_arg2: 0,
        detail0: 0,
        detail1: 0,
    }; 16];
    let count = runtime
        .wait_event_queue(queue_fd, &mut records)
        .map_err(|_| 246)?;
    for record in &records[..count] {
        let kind = if record.source_kind == NativeEventSourceKind::Resource as u32 {
            match record.detail0 {
                0 => "claimed",
                1 => "queued",
                2 => "canceled",
                3 => "released",
                4 => "handed-off",
                5 => "revoked",
                _ => "unknown",
            }
        } else {
            "unknown"
        };
        write_line(
            runtime,
            &format!(
                "gpu-lease-event queue={} token={} resource={} contract={} kind={} events=0x{:x}",
                queue_fd, record.token, record.source_arg0, record.source_arg1, kind, record.events
            ),
        )?;
    }
    Ok(())
}

fn parse_readiness_interest(token: &str) -> Option<(bool, bool, bool)> {
    match token {
        "read" => Some((true, false, false)),
        "write" => Some((false, true, false)),
        "priority" => Some((false, false, true)),
        "readwrite" => Some((true, true, false)),
        "readpriority" => Some((true, false, true)),
        "writepriority" => Some((false, true, true)),
        "all" => Some((true, true, true)),
        _ => None,
    }
}

fn shell_watch_fd_readiness<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    readable: bool,
    writable: bool,
    priority: bool,
) -> Result<usize, ExitCode> {
    let fd = runtime.open_path(path).map_err(|_| 234)?;
    runtime
        .register_readiness(fd, readable, writable, priority)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "fd-watch fd={} path={} readable={} writable={} priority={}",
            fd, path, readable as u8, writable as u8, priority as u8
        ),
    )?;
    Ok(fd)
}

fn shell_collect_readiness<B: SyscallBackend>(runtime: &Runtime<B>) -> Result<(), ExitCode> {
    let mut records = [NativeReadinessRecord {
        owner: 0,
        fd: 0,
        readable: 0,
        writable: 0,
        priority: 0,
        reserved: 0,
    }; 16];
    let count = runtime.collect_readiness(&mut records).map_err(|_| 246)?;
    if count == 0 {
        return write_line(runtime, "fd-ready count=0");
    }
    for record in &records[..count] {
        write_line(
            runtime,
            &format!(
                "fd-ready owner={} fd={} readable={} writable={} priority={}",
                record.owner, record.fd, record.readable, record.writable, record.priority
            ),
        )?;
    }
    Ok(())
}

fn shell_render_driver<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
) -> Result<(), ExitCode> {
    let record: NativeDriverRecord = runtime.inspect_driver(driver_path).map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "driver path={} state={} bound-devices={} queued={} inflight={} completed={}",
            driver_path,
            record.state,
            record.bound_device_count,
            record.queued_requests,
            record.in_flight_requests,
            record.completed_requests
        ),
    )
}

fn encode_block_request_bytes(request: &NativeBlockIoRequest) -> &[u8] {
    unsafe {
        core::slice::from_raw_parts(
            (request as *const NativeBlockIoRequest).cast::<u8>(),
            core::mem::size_of::<NativeBlockIoRequest>(),
        )
    }
}

fn default_block_request_security(
    object_id: u64,
    subject_id: u64,
    rights: ngos_user_abi::BlockRightsMask,
) -> (
    ngos_user_abi::CapabilityToken,
    ngos_user_abi::SecurityLabel,
    ngos_user_abi::ProvenanceTag,
    ngos_user_abi::IntegrityTag,
) {
    let label = ngos_user_abi::SecurityLabel::new(
        ngos_user_abi::ConfidentialityLevel::Internal,
        ngos_user_abi::IntegrityLevel::Verified,
    );
    let integrity = ngos_user_abi::IntegrityTag::zeroed(ngos_user_abi::IntegrityTagKind::None);
    let capability = ngos_user_abi::CapabilityToken {
        object_id,
        rights,
        issuer_id: subject_id,
        subject_id,
        generation: 1,
        revocation_epoch: 1,
        delegation_depth: 0,
        delegated: 0,
        nonce: object_id ^ subject_id ^ rights.0,
        expiry_epoch: u64::MAX,
        authenticator: integrity,
    };
    let provenance = ngos_user_abi::ProvenanceTag {
        origin_kind: ngos_user_abi::ProvenanceOriginKind::Subject,
        reserved0: 0,
        origin_id: subject_id,
        parent_origin_id: 0,
        parent_measurement: [0; 32],
        edge_id: object_id,
        measurement: integrity,
    };
    (capability, label, provenance, integrity)
}

fn try_decode_block_request(bytes: &[u8]) -> Option<NativeBlockIoRequest> {
    if bytes.len() < core::mem::size_of::<NativeBlockIoRequest>() {
        return None;
    }
    let request = unsafe { (bytes.as_ptr() as *const NativeBlockIoRequest).read_unaligned() };
    if request.magic != NATIVE_BLOCK_IO_MAGIC || request.version != NATIVE_BLOCK_IO_VERSION {
        return None;
    }
    Some(request)
}

fn shell_submit_block_read<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    sector: u64,
    sector_count: u32,
) -> Result<(), ExitCode> {
    let record = runtime.inspect_device(device_path).map_err(|_| 246)?;
    if record.block_size == 0 {
        return Err(246);
    }
    let rights = ngos_user_abi::BlockRightsMask::READ.union(ngos_user_abi::BlockRightsMask::SUBMIT);
    let (capability, label, provenance, integrity) =
        default_block_request_security(0x5354_4f52_4147_4530, 1, rights);
    let request = NativeBlockIoRequest {
        magic: NATIVE_BLOCK_IO_MAGIC,
        version: NATIVE_BLOCK_IO_VERSION,
        op: NATIVE_BLOCK_IO_OP_READ,
        sector,
        sector_count,
        block_size: record.block_size,
        rights,
        capability,
        label,
        provenance,
        integrity,
    };
    let fd = runtime.open_path(device_path).map_err(|_| 234)?;
    shell_write_all(runtime, fd, encode_block_request_bytes(&request))?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!(
            "blk-read device={} sector={} sectors={} block-size={}",
            device_path, sector, sector_count, record.block_size
        ),
    )
}

fn shell_render_network_socket<B: SyscallBackend>(
    runtime: &Runtime<B>,
    socket_path: &str,
) -> Result<(), ExitCode> {
    let record: NativeNetworkSocketRecord = runtime
        .inspect_network_socket(socket_path)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "netsock path={} local={}:{} remote={}:{} connected={} rx-depth={} rx-limit={} rx-packets={} tx-packets={} dropped={}",
            socket_path,
            render_ipv4(record.local_ipv4),
            record.local_port,
            render_ipv4(record.remote_ipv4),
            record.remote_port,
            if record.connected != 0 { "yes" } else { "no" },
            record.rx_depth,
            record.rx_queue_limit,
            record.rx_packets,
            record.tx_packets,
            record.dropped_packets
        ),
    )
}

#[allow(clippy::too_many_arguments)]
fn shell_net_admin<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    mtu: usize,
    tx_capacity: usize,
    rx_capacity: usize,
    tx_inflight_limit: usize,
    admin_up: bool,
    promiscuous: bool,
) -> Result<(), ExitCode> {
    runtime
        .configure_network_interface_admin(
            device_path,
            mtu,
            tx_capacity,
            rx_capacity,
            tx_inflight_limit,
            admin_up,
            promiscuous,
        )
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "netif-admin path={} mtu={} tx-cap={} rx-cap={} inflight-limit={} admin={} promisc={}",
            device_path,
            mtu,
            tx_capacity,
            rx_capacity,
            tx_inflight_limit,
            if admin_up { "up" } else { "down" },
            if promiscuous { "on" } else { "off" }
        ),
    )
}

fn shell_net_config<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    addr: [u8; 4],
    netmask: [u8; 4],
    gateway: [u8; 4],
) -> Result<(), ExitCode> {
    runtime
        .configure_network_interface_ipv4(device_path, addr, netmask, gateway)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "netif-configured path={} addr={} netmask={} gateway={}",
            device_path,
            render_ipv4(addr),
            render_ipv4(netmask),
            render_ipv4(gateway)
        ),
    )
}

fn shell_udp_bind<B: SyscallBackend>(
    runtime: &Runtime<B>,
    socket_path: &str,
    device_path: &str,
    local_port: u16,
    remote_ipv4: [u8; 4],
    remote_port: u16,
) -> Result<(), ExitCode> {
    runtime
        .bind_udp_socket(
            socket_path,
            device_path,
            local_port,
            remote_ipv4,
            remote_port,
        )
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "udp-bound socket={} device={} local-port={} remote={}:{}",
            socket_path,
            device_path,
            local_port,
            render_ipv4(remote_ipv4),
            remote_port
        ),
    )
}

fn shell_poll_path<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
    interest: u32,
) -> Result<(), ExitCode> {
    let fd = runtime.open_path(path).map_err(|_| 234)?;
    let events = runtime.poll(fd, interest).map_err(|_| 234)?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!(
            "poll path={} interest=0x{:x} ready=0x{:x}",
            path, interest, events
        ),
    )
}

fn shell_net_send<B: SyscallBackend>(
    runtime: &Runtime<B>,
    socket_path: &str,
    payload: &str,
) -> Result<(), ExitCode> {
    let fd = runtime.open_path(socket_path).map_err(|_| 234)?;
    shell_write_all(runtime, fd, payload.as_bytes())?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!("net-send socket={} bytes={}", socket_path, payload.len()),
    )
}

fn shell_net_sendto<B: SyscallBackend>(
    runtime: &Runtime<B>,
    socket_path: &str,
    remote_ip: [u8; 4],
    remote_port: u16,
    payload: &str,
) -> Result<(), ExitCode> {
    let written = runtime
        .send_udp_to(socket_path, remote_ip, remote_port, payload.as_bytes())
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "net-sendto socket={} remote={}:{} bytes={}",
            socket_path,
            render_ipv4(remote_ip),
            remote_port,
            written
        ),
    )
}

fn shell_net_recv<B: SyscallBackend>(
    runtime: &Runtime<B>,
    socket_path: &str,
) -> Result<(), ExitCode> {
    let fd = runtime.open_path(socket_path).map_err(|_| 234)?;
    let mut buffer = [0u8; 512];
    let count = runtime.read(fd, &mut buffer).map_err(|_| 238)?;
    runtime.close(fd).map_err(|_| 240)?;
    let text = core::str::from_utf8(&buffer[..count]).map_err(|_| 239)?;
    write_line(
        runtime,
        &format!(
            "net-recv socket={} bytes={} payload={}",
            socket_path, count, text
        ),
    )
}

fn shell_net_recvfrom<B: SyscallBackend>(
    runtime: &Runtime<B>,
    socket_path: &str,
) -> Result<(), ExitCode> {
    let mut buffer = [0u8; 512];
    let (count, meta) = runtime
        .recv_udp_from(socket_path, &mut buffer)
        .map_err(|_| 246)?;
    let text = core::str::from_utf8(&buffer[..count]).map_err(|_| 239)?;
    write_line(
        runtime,
        &format!(
            "net-recvfrom socket={} remote={}:{} bytes={} payload={}",
            socket_path,
            render_ipv4(meta.remote_ipv4),
            meta.remote_port,
            count,
            text
        ),
    )
}

fn shell_driver_read<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
) -> Result<(), ExitCode> {
    let fd = runtime.open_path(driver_path).map_err(|_| 234)?;
    let mut buffer = [0u8; 512];
    let count = runtime.read(fd, &mut buffer).map_err(|_| 238)?;
    runtime.close(fd).map_err(|_| 240)?;
    let prefix_len = buffer[..count]
        .iter()
        .position(|byte| *byte == b'\n')
        .map(|index| index + 1)
        .unwrap_or(count);
    let text = core::str::from_utf8(&buffer[..prefix_len]).map_err(|_| 239)?;
    shell_emit_lines(runtime, text)?;
    if prefix_len < count {
        let payload_bytes = &buffer[prefix_len..count];
        if !payload_bytes.is_empty() {
            if let Some(request) = try_decode_block_request(payload_bytes) {
                write_line(
                    runtime,
                    &format!(
                        "block-request path={} op={} sector={} sectors={} block-size={}",
                        driver_path,
                        match request.op {
                            NATIVE_BLOCK_IO_OP_READ => "read",
                            _ => "unknown",
                        },
                        request.sector,
                        request.sector_count,
                        request.block_size
                    ),
                )?;
            } else if let Ok(payload) = core::str::from_utf8(payload_bytes) {
                write_line(
                    runtime,
                    &format!(
                        "driver-payload path={} bytes={} text={}",
                        driver_path,
                        payload_bytes.len(),
                        payload
                    ),
                )?;
            } else {
                write_line(
                    runtime,
                    &format!(
                        "driver-payload path={} bytes={} encoding=binary",
                        driver_path,
                        payload_bytes.len()
                    ),
                )?;
            }
        }
    }
    Ok(())
}

fn shell_driver_inject_udp<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    src_ip: [u8; 4],
    src_port: u16,
    dst_ip: [u8; 4],
    dst_port: u16,
    payload: &str,
) -> Result<(), ExitCode> {
    let frame = build_udp_ipv4_frame(
        [0x02, 0xaa, 0xbb, 0xcc, 0xdd, 0xee],
        [0x02, 0x11, 0x22, 0x33, 0x44, 0x55],
        src_ip,
        dst_ip,
        src_port,
        dst_port,
        payload.as_bytes(),
    );
    let fd = runtime.open_path(driver_path).map_err(|_| 234)?;
    shell_write_all(runtime, fd, &frame)?;
    runtime.close(fd).map_err(|_| 240)?;
    write_line(
        runtime,
        &format!(
            "net-inject driver={} src={}:{} dst={}:{} bytes={}",
            driver_path,
            render_ipv4(src_ip),
            src_port,
            render_ipv4(dst_ip),
            dst_port,
            payload.len()
        ),
    )
}

fn shell_set_net_link<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    link_up: bool,
) -> Result<(), ExitCode> {
    runtime
        .set_network_interface_link_state(device_path, link_up)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "netif-link path={} state={}",
            device_path,
            if link_up { "up" } else { "down" }
        ),
    )
}

fn shell_udp_connect<B: SyscallBackend>(
    runtime: &Runtime<B>,
    socket_path: &str,
    remote_ipv4: [u8; 4],
    remote_port: u16,
) -> Result<(), ExitCode> {
    runtime
        .connect_udp_socket(socket_path, remote_ipv4, remote_port)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "udp-connected socket={} remote={}:{}",
            socket_path,
            render_ipv4(remote_ipv4),
            remote_port
        ),
    )
}

fn shell_complete_net_tx<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    completions: usize,
) -> Result<(), ExitCode> {
    let completed = runtime
        .complete_network_tx(driver_path, completions)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "net-complete driver={} completed={}",
            driver_path, completed
        ),
    )
}

fn shell_create_event_queue<B: SyscallBackend>(
    runtime: &Runtime<B>,
    mode: NativeEventQueueMode,
) -> Result<usize, ExitCode> {
    let fd = runtime.create_event_queue(mode).map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "queue-created fd={} mode={}",
            fd,
            match mode {
                NativeEventQueueMode::Kqueue => "kqueue",
                NativeEventQueueMode::Epoll => "epoll",
            }
        ),
    )?;
    Ok(fd)
}

fn shell_watch_network_events<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
    device_path: &str,
    socket_path: Option<&str>,
    token: u64,
) -> Result<(), ExitCode> {
    runtime
        .watch_network_events(
            queue_fd,
            device_path,
            socket_path,
            token,
            true,
            true,
            true,
            POLLPRI,
        )
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "net-watch queue={} device={} socket={} token={}",
            queue_fd,
            device_path,
            socket_path.unwrap_or("-"),
            token
        ),
    )
}

fn shell_remove_network_watch<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
    device_path: &str,
    socket_path: Option<&str>,
    token: u64,
) -> Result<(), ExitCode> {
    runtime
        .remove_network_events(queue_fd, device_path, socket_path, token)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "net-unwatch queue={} device={} socket={} token={}",
            queue_fd,
            device_path,
            socket_path.unwrap_or("-"),
            token
        ),
    )
}

fn parse_resource_watch_kinds(
    raw: Option<&str>,
) -> Option<(bool, bool, bool, bool, bool, bool, String)> {
    let Some(raw) = raw else {
        return Some((true, true, true, true, true, true, String::from("all")));
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "all" {
        return Some((true, true, true, true, true, true, String::from("all")));
    }
    let mut claimed = false;
    let mut queued = false;
    let mut canceled = false;
    let mut released = false;
    let mut handed_off = false;
    let mut revoked = false;
    for token in trimmed.split(',') {
        match token.trim() {
            "claimed" => claimed = true,
            "queued" => queued = true,
            "canceled" => canceled = true,
            "released" => released = true,
            "handed-off" => handed_off = true,
            "revoked" => revoked = true,
            _ => return None,
        }
    }
    Some((
        claimed,
        queued,
        canceled,
        released,
        handed_off,
        revoked,
        trimmed.to_string(),
    ))
}

fn shell_watch_resource_events<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
    resource: usize,
    token: u64,
    claimed: bool,
    queued: bool,
    canceled: bool,
    released: bool,
    handed_off: bool,
    revoked: bool,
    kinds_label: &str,
) -> Result<(), ExitCode> {
    runtime
        .watch_resource_events(
            queue_fd, resource, token, claimed, queued, canceled, released, handed_off, revoked,
            POLLPRI,
        )
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "resource-watch queue={} resource={} token={} kinds={}",
            queue_fd, resource, token, kinds_label
        ),
    )
}

fn shell_remove_resource_watch<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
    resource: usize,
    token: u64,
) -> Result<(), ExitCode> {
    runtime
        .remove_resource_events(queue_fd, resource, token)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "resource-unwatch queue={} resource={} token={}",
            queue_fd, resource, token
        ),
    )
}

fn shell_watch_graphics_events<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
    device_path: &str,
    token: u64,
) -> Result<(), ExitCode> {
    runtime
        .watch_graphics_events(
            queue_fd,
            device_path,
            token,
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
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "gpu-watch queue={} device={} token={}",
            queue_fd, device_path, token
        ),
    )
}

fn shell_remove_graphics_watch<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
    device_path: &str,
    token: u64,
) -> Result<(), ExitCode> {
    runtime
        .remove_graphics_events(queue_fd, device_path, token)
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "gpu-unwatch queue={} device={} token={}",
            queue_fd, device_path, token
        ),
    )
}

fn shell_wait_event_queue<B: SyscallBackend>(
    runtime: &Runtime<B>,
    queue_fd: usize,
) -> Result<(), ExitCode> {
    let mut records = [NativeEventRecord {
        token: 0,
        events: 0,
        source_kind: 0,
        source_arg0: 0,
        source_arg1: 0,
        source_arg2: 0,
        detail0: 0,
        detail1: 0,
    }; 16];
    let count = runtime
        .wait_event_queue(queue_fd, &mut records)
        .map_err(|_| 246)?;
    for record in &records[..count] {
        let source = match NativeEventSourceKind::from_raw(record.source_kind) {
            Some(NativeEventSourceKind::Resource) => {
                let kind = match record.detail0 {
                    0 => "claimed",
                    1 => "queued",
                    2 => "canceled",
                    3 => "released",
                    4 => "handed-off",
                    5 => "revoked",
                    _ => "unknown",
                };
                format!(
                    "resource id={} contract={} kind={}",
                    record.source_arg0, record.source_arg1, kind
                )
            }
            Some(NativeEventSourceKind::Network) => {
                let kind = match NativeNetworkEventKind::from_raw(record.detail1) {
                    Some(NativeNetworkEventKind::LinkChanged) => "link-changed",
                    Some(NativeNetworkEventKind::RxReady) => "rx-ready",
                    Some(NativeNetworkEventKind::TxDrained) => "tx-drained",
                    None => "unknown",
                };
                format!(
                    "network iface={} socket={} kind={}",
                    record.source_arg0,
                    if record.detail0 != 0 {
                        record.source_arg1.to_string()
                    } else {
                        "-".to_string()
                    },
                    kind
                )
            }
            Some(NativeEventSourceKind::Graphics) => {
                match NativeGraphicsEventKind::from_raw(record.detail1) {
                    Some(NativeGraphicsEventKind::Submitted) => format!(
                        "graphics device={} request={} kind=submitted",
                        record.source_arg0, record.source_arg1
                    ),
                    Some(NativeGraphicsEventKind::Completed) => format!(
                        "graphics device={} request={} kind=completed",
                        record.source_arg0, record.source_arg1
                    ),
                    Some(NativeGraphicsEventKind::Failed) => format!(
                        "graphics device={} request={} kind=failed",
                        record.source_arg0, record.source_arg1
                    ),
                    Some(NativeGraphicsEventKind::Drained) => format!(
                        "graphics device={} request={} kind=drained",
                        record.source_arg0, record.source_arg1
                    ),
                    Some(NativeGraphicsEventKind::Canceled) => format!(
                        "graphics device={} request={} kind=canceled",
                        record.source_arg0, record.source_arg1
                    ),
                    Some(NativeGraphicsEventKind::Faulted) => format!(
                        "graphics device={} token={} kind=faulted",
                        record.source_arg0, record.source_arg1
                    ),
                    Some(NativeGraphicsEventKind::Recovered) => format!(
                        "graphics device={} token={} kind=recovered",
                        record.source_arg0, record.source_arg1
                    ),
                    Some(NativeGraphicsEventKind::Retired) => format!(
                        "graphics device={} token={} kind=retired",
                        record.source_arg0, record.source_arg1
                    ),
                    Some(NativeGraphicsEventKind::LeaseReleased) => format!(
                        "graphics device={} contract={} kind=lease-released",
                        record.source_arg0, record.source_arg1
                    ),
                    Some(NativeGraphicsEventKind::LeaseAcquired) => format!(
                        "graphics device={} contract={} kind=lease-acquired",
                        record.source_arg0, record.source_arg1
                    ),
                    None => format!(
                        "graphics device={} token={} kind=unknown",
                        record.source_arg0, record.source_arg1
                    ),
                }
            }
            Some(kind) => format!("other:{kind:?}"),
            None => "unknown".to_string(),
        };
        write_line(
            runtime,
            &format!(
                "queue-event queue={} token={} events=0x{:x} source={}",
                queue_fd, record.token, record.events, source
            ),
        )?;
    }
    Ok(())
}

fn shell_get_fd_status_flags<B: SyscallBackend>(
    runtime: &Runtime<B>,
    fd: usize,
) -> Result<(), ExitCode> {
    let flags = runtime.fcntl(fd, FcntlCmd::GetFl).map_err(|_| 246)?;
    write_line(
        runtime,
        &format!("fcntl-getfl fd={} flags=0x{:x}", fd, flags),
    )
}

fn shell_get_fd_descriptor_flags<B: SyscallBackend>(
    runtime: &Runtime<B>,
    fd: usize,
) -> Result<(), ExitCode> {
    let flags = runtime.fcntl(fd, FcntlCmd::GetFd).map_err(|_| 246)?;
    write_line(
        runtime,
        &format!("fcntl-getfd fd={} flags=0x{:x}", fd, flags),
    )
}

fn shell_set_fd_nonblock<B: SyscallBackend>(
    runtime: &Runtime<B>,
    fd: usize,
    nonblock: bool,
) -> Result<(), ExitCode> {
    let flags = runtime
        .fcntl(fd, FcntlCmd::SetFl { nonblock })
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "nonblock-fd fd={} nonblock={} flags=0x{:x}",
            fd, nonblock as u8, flags
        ),
    )
}

fn shell_set_fd_cloexec<B: SyscallBackend>(
    runtime: &Runtime<B>,
    fd: usize,
    cloexec: bool,
) -> Result<(), ExitCode> {
    let flags = runtime
        .fcntl(fd, FcntlCmd::SetFd { cloexec })
        .map_err(|_| 246)?;
    write_line(
        runtime,
        &format!(
            "cloexec-fd fd={} cloexec={} flags=0x{:x}",
            fd, cloexec as u8, flags
        ),
    )
}

fn resource_arbitration_name(raw: u32) -> &'static str {
    match NativeResourceArbitrationPolicy::from_raw(raw) {
        Some(NativeResourceArbitrationPolicy::Fifo) => "fifo",
        Some(NativeResourceArbitrationPolicy::Lifo) => "lifo",
        None => "unknown",
    }
}

fn resource_governance_name(raw: u32) -> &'static str {
    match NativeResourceGovernanceMode::from_raw(raw) {
        Some(NativeResourceGovernanceMode::Queueing) => "queueing",
        Some(NativeResourceGovernanceMode::ExclusiveLease) => "exclusive-lease",
        None => "unknown",
    }
}

fn resource_contract_policy_name(raw: u32) -> &'static str {
    match NativeResourceContractPolicy::from_raw(raw) {
        Some(NativeResourceContractPolicy::Any) => "any",
        Some(NativeResourceContractPolicy::Execution) => "execution",
        Some(NativeResourceContractPolicy::Memory) => "memory",
        Some(NativeResourceContractPolicy::Io) => "io",
        Some(NativeResourceContractPolicy::Device) => "device",
        Some(NativeResourceContractPolicy::Display) => "display",
        Some(NativeResourceContractPolicy::Observe) => "observe",
        None => "unknown",
    }
}

fn resource_issuer_policy_name(raw: u32) -> &'static str {
    match NativeResourceIssuerPolicy::from_raw(raw) {
        Some(NativeResourceIssuerPolicy::AnyIssuer) => "any-issuer",
        Some(NativeResourceIssuerPolicy::CreatorOnly) => "creator-only",
        Some(NativeResourceIssuerPolicy::DomainOwnerOnly) => "domain-owner-only",
        None => "unknown",
    }
}

fn run_session_shell<B: SyscallBackend>(
    runtime: &Runtime<B>,
    context: &SessionContext,
) -> ExitCode {
    if write_line(runtime, "ngos shell").is_err() {
        return 192;
    }
    let mut script = Vec::new();
    let mut chunk = [0u8; 256];
    loop {
        let read = match runtime.read(0, &mut chunk) {
            Ok(read) => read,
            Err(_) => return 193,
        };
        if read == 0 {
            break;
        }
        script.extend_from_slice(&chunk[..read]);
    }

    let text = match core::str::from_utf8(&script) {
        Ok(text) => text,
        Err(_) => return 194,
    };

    let mut last_status = 0;
    let mut current_cwd = context.cwd.clone();
    let mut jobs = Vec::<ShellJob>::new();
    let mut aliases = Vec::<ShellAlias>::new();
    let mut variables = Vec::<ShellVariable>::new();
    let mut history = Vec::<String>::new();
    let mut shell_mode = ShellMode::Direct;
    let mut semantic_learning = SemanticFeedbackStore::new();
    let mut nextmind_auto_state = NextMindAutoState {
        enabled: false,
        streams: Vec::new(),
    };
    let mut nextmind_last_report = None::<NextMindDecisionReport>;
    let mut nextmind_last_snapshot = None::<ngos_user_abi::NativeSystemSnapshotRecord>;
    let mut nextmind_adaptive_state = AdaptiveState::new();
    let mut nextmind_context = SemanticContext::new();
    let mut nextmind_entity_epochs = Vec::<SemanticEntityEpoch>::new();
    let mut game_sessions = Vec::<GameCompatSession>::new();
    let mut edit_session = EditSessionState::new();
    let mut shell_functions = Vec::<ShellFunction>::new();
    let mut shell_call_stack = Vec::<ShellCallFrame>::new();
    let mut last_spawned_pid = None::<u64>;
    shell_sync_runtime_variables(&mut variables, last_status, &current_cwd, last_spawned_pid);
    let mut pending_lines = text
        .lines()
        .map(|line| line.to_string())
        .collect::<Vec<_>>();
    let mut line_index = 0usize;
    while line_index < pending_lines.len() {
        merge_multiline_lang_block(&mut pending_lines, line_index);
        let raw_line = pending_lines[line_index].clone();
        line_index += 1;
        for (guard, command) in shell_parse_guarded_commands(&raw_line) {
            match guard {
                ShellCommandGuard::Always => {}
                ShellCommandGuard::OnSuccess if last_status != 0 => continue,
                ShellCommandGuard::OnFailure if last_status == 0 => continue,
                ShellCommandGuard::OnSuccess | ShellCommandGuard::OnFailure => {}
            }
            if command.is_empty() || command.starts_with('#') {
                continue;
            }
            history.push(command.to_string());
            if nextmind_drain_auto_events(
                runtime,
                &nextmind_auto_state,
                &mut nextmind_last_snapshot,
                &mut nextmind_adaptive_state,
                &mut nextmind_last_report,
            )
            .is_err()
            {
                return 267;
            }
            shell_sync_runtime_variables(
                &mut variables,
                last_status,
                &current_cwd,
                last_spawned_pid,
            );
            let expanded_command = shell_expand_aliases(&command, &aliases);
            let line = shell_expand_variables(&expanded_command, &variables);
            if line.is_empty() {
                continue;
            }
            let previous_status = last_status;
            last_status = 0;
            if line == "help" {
                if write_line(
                    runtime,
                    "help session mode pwd env cd alias unalias aliases set unset vars history last-status true false repeat assert-status assert-file-contains source-file let print if while for break continue calc fn call return functions workspace-summary workspace-members workspace-topology workspace-audit crate-info crate-files crate-deps crate-audit crate-hotspots docs-list doc-show doc-search doc-links manifest-show rust-files rust-symbols crate-symbols unsafe-audit todo-rust source-hotspots ps jobs job-info fg kill pause resume renice pending-signals blocked-signals spawn-path reap process-info proc cat self status stat cmdline cwd environ exe auxv maps vmobjects vmdecisions vmepisodes fd fdinfo dup-fd close-fd fcntl-getfl fcntl-getfd nonblock-fd cloexec-fd fd-watch fd-ready vm-map-anon vm-probe-map-anon vm-brk vm-probe-brk vm-quarantine vm-release vm-load-word vm-store-word vm-probe-store-word vm-sync-range vm-protect vm-unmap vm-advise vm-pressure vm-pressure-global caps queues system-queues stat-path lstat-path statfs-path open-path readlink-path cat-file head-file tail-file wc-file hex-file cat-numbered find-text find-tree-text replace-text replace-line insert-line delete-line append-line insert-before insert-after touch-file truncate-file move-path grep-tree copy-tree mirror-tree tree-path find-path edit-open edit-status edit-show edit-set edit-insert edit-append edit-delete edit-write edit-abort write-file append-file copy-file cmp-file grep-file mkdir-path mkfile-path mksock-path symlink-path rename-path unlink-path list-path game-manifest game-plan game-launch game-simulate game-sessions game-status game-stop game-next game-gfx-plan game-gfx-submit game-gfx-status game-gfx-next game-audio-plan game-audio-submit game-audio-status game-audio-next game-input-plan game-input-submit game-input-status game-input-next game-watch-start game-watch-status game-watch-status-all game-watch-poll-all game-watch-wait game-watch-stop device gpu-evidence gpu-vbios gpu-gsp gpu-irq gpu-display gpu-power gpu-power-set gpu-media gpu-media-start gpu-neural gpu-neural-inject gpu-neural-commit gpu-tensor gpu-tensor-dispatch driver gpu-queue-capacity gpu-buffer-create gpu-buffer-write gpu-buffer gpu-scanout gpu-perf gpu-submit-buffer gpu-probe-submit-buffer gpu-request gpu-submit gpu-probe-submit gpu-present gpu-probe-present gpu-driver-read gpu-driver-bind gpu-probe-driver-bind gpu-driver-unbind gpu-probe-driver-unbind gpu-driver-reset gpu-probe-driver-reset gpu-driver-retire gpu-probe-driver-retire gpu-complete gpu-complete-request gpu-fail-request gpu-cancel-request gpu-probe-complete gpu-probe-complete-request gpu-probe-fail-request gpu-probe-cancel-request gpu-read gpu-watch gpu-unwatch gpu-lease-watch gpu-lease-unwatch gpu-lease-wait blk-read netif net-config net-admin net-link udp-bind udp-connect netsock net-send net-sendto net-recv net-recvfrom driver-read net-driver-read net-complete net-inject-udp queue-create net-watch net-unwatch resource-watch resource-unwatch queue-wait poll-path domains domain resources resource waiters contracts contract mkdomain mkresource mkcontract claim releaseclaim release transfer cancelclaim invoke contract-state resource-state resource-policy resource-governance resource-contract-policy resource-issuer-policy observe intent learn semantic-watch semantic-wait nextmind.observe nextmind.optimize nextmind.auto nextmind.explain smoke vfs-smoke wasm-smoke echo exit",
                )
                .is_err()
                {
                    return 195;
                }
                continue;
            }
            if let Some(result) = try_handle_shell_lang_command(
                runtime,
                &line,
                &mut variables,
                &mut shell_functions,
                &mut shell_call_stack,
                &mut pending_lines,
                line_index,
            ) {
                match result {
                    Ok(()) => {}
                    Err(code) if code == 2 => return 199,
                    Err(_) => return 205,
                }
                continue;
            }
            if let Some(result) = try_handle_game_agent_command(
                runtime,
                &mut current_cwd,
                &line,
                &mut game_sessions,
                &mut jobs,
                &mut last_spawned_pid,
                &mut last_status,
            ) {
                match result {
                    Ok(()) => {}
                    Err(code) => return code,
                }
                continue;
            }
            if let Some(result) = try_handle_code_agent_command(runtime, &current_cwd, &line) {
                match result {
                    Ok(()) => {}
                    Err(code) if code == 2 => return 199,
                    Err(_) => return 205,
                }
                continue;
            }
            if let Some(result) = try_handle_workflow_agent_command(runtime, &current_cwd, &line) {
                match result {
                    Ok(()) => {}
                    Err(code) if code == 2 => return 199,
                    Err(_) => return 205,
                }
                continue;
            }
            if let Some(result) = try_handle_project_agent_command(runtime, &current_cwd, &line) {
                match result {
                    Ok(()) => {}
                    Err(code) if code == 2 => return 199,
                    Err(_) => return 205,
                }
                continue;
            }
            if let Some(result) = try_handle_rust_agent_command(runtime, &current_cwd, &line) {
                match result {
                    Ok(()) => {}
                    Err(code) if code == 2 => return 199,
                    Err(_) => return 205,
                }
                continue;
            }
            if let Some(result) = try_handle_analysis_agent_command(runtime, &current_cwd, &line) {
                match result {
                    Ok(()) => {}
                    Err(code) if code == 2 => return 199,
                    Err(_) => return 205,
                }
                continue;
            }
            if let Some(result) = nextmind_agents::try_handle_nextmind_agent_command(
                runtime,
                &line,
                &mut nextmind_agents::NextMindAgentState {
                    last_snapshot: &mut nextmind_last_snapshot,
                    adaptive_state: &mut nextmind_adaptive_state,
                    context: &mut nextmind_context,
                    entity_epochs: &mut nextmind_entity_epochs,
                    auto_state: &mut nextmind_auto_state,
                    last_report: &mut nextmind_last_report,
                    last_status: &mut last_status,
                },
            ) {
                match result {
                    Ok(()) => {}
                    Err(code) if code == 2 => return 199,
                    Err(code) => return code,
                }
                continue;
            }
            if let Some(result) = shell_state_agents::try_handle_shell_state_agent_command(
                runtime,
                &current_cwd,
                &line,
                &mut shell_mode,
                &semantic_learning,
                previous_status,
                &mut pending_lines,
                line_index,
                &mut last_status,
            ) {
                match result {
                    Ok(()) => {}
                    Err(code) if code == 2 => return 199,
                    Err(code) => return code,
                }
                continue;
            }
            if let Some(result) = session_agents::try_handle_session_agent_command(
                runtime,
                context,
                &mut current_cwd,
                &mut aliases,
                &mut variables,
                &history,
                &mut pending_lines,
                line_index,
                &line,
            ) {
                match result {
                    Ok(()) => {}
                    Err(code) if code == 2 => return 199,
                    Err(code) => return code,
                }
                continue;
            }
            if let Some(result) = semantic_agents::try_handle_semantic_agent_command(
                runtime,
                &current_cwd,
                &line,
                &mut last_status,
                &mut semantic_learning,
                &nextmind_entity_epochs,
            ) {
                match result {
                    Ok(()) => {}
                    Err(code) if code == 2 => return 199,
                    Err(code) => return code,
                }
                continue;
            }
            if let Some(result) = intent_agents::try_handle_intent_agent_command(
                runtime,
                &current_cwd,
                shell_mode,
                &line,
                &mut last_status,
                &mut semantic_learning,
                &nextmind_entity_epochs,
            ) {
                match result {
                    Ok(()) => {}
                    Err(code) if code == 2 => return 199,
                    Err(code) => return code,
                }
                continue;
            }
            if let Some(result) = proc_agents::try_handle_proc_agent_command(
                runtime,
                context,
                &current_cwd,
                &line,
                &mut jobs,
                &game_sessions,
                &mut last_spawned_pid,
            ) {
                match result {
                    Ok(()) => {}
                    Err(code) if code == 2 => return 199,
                    Err(code) => return code,
                }
                continue;
            }
            if let Some(result) =
                vfs_agents::try_handle_vfs_agent_command(runtime, &current_cwd, &line)
            {
                match result {
                    Ok(()) => {}
                    Err(code) if code == 2 => return 199,
                    Err(_) => return 205,
                }
                continue;
            }
            if let Some(result) = gpu_agents::try_handle_gpu_agent_command(
                runtime,
                &current_cwd,
                &mut variables,
                &line,
                &mut last_status,
            ) {
                match result {
                    Ok(()) => {}
                    Err(code) if code == 2 => return 199,
                    Err(code) => return code,
                }
                continue;
            }
            if let Some(result) = surface_agents::try_handle_surface_agent_command(
                runtime,
                context,
                &current_cwd,
                &mut variables,
                &line,
                &mut last_status,
            ) {
                match result {
                    Ok(surface_agents::SurfaceAgentOutcome::Continue) => {}
                    Ok(surface_agents::SurfaceAgentOutcome::Exit(code)) => {
                        shell_cleanup_game_sessions(runtime, &mut game_sessions, &mut jobs);
                        return code;
                    }
                    Err(code) if code == 2 => return 199,
                    Err(code) => return code,
                }
                continue;
            }
            if let Some(result) = network_agents::try_handle_network_agent_command(
                runtime,
                &current_cwd,
                &mut variables,
                &line,
                &mut last_status,
            ) {
                match result {
                    Ok(()) => {}
                    Err(code) if code == 2 => return 199,
                    Err(code) => return code,
                }
                continue;
            }
            if let Some(result) =
                resource_agents::try_handle_resource_agent_command(runtime, &line, &mut last_status)
            {
                match result {
                    Ok(()) => {}
                    Err(code) if code == 2 => return 199,
                    Err(code) => return code,
                }
                continue;
            }
            if let Some(result) = try_handle_fd_agent_command(runtime, &line) {
                match result {
                    Ok(()) => {}
                    Err(code) if code == 2 => {
                        return 199;
                    }
                    Err(_) => return 205,
                }
                continue;
            }
            if let Some(result) = try_handle_vm_agent_command(runtime, &line) {
                match result {
                    Ok(()) => {}
                    Err(code) if code == 2 => {
                        return 199;
                    }
                    Err(_) => return 205,
                }
                continue;
            }
            if let Some(result) = try_handle_dev_agent_command(runtime, &current_cwd, &line) {
                match result {
                    Ok(()) => {}
                    Err(code) if code == 2 => {
                        return 199;
                    }
                    Err(_) => return 205,
                }
                continue;
            }
            if let Some(result) =
                path_agents::try_handle_path_agent_command(runtime, &current_cwd, &line)
            {
                match result {
                    Ok(()) => {}
                    Err(code) if code == 2 => {
                        return 199;
                    }
                    Err(_) => return 205,
                }
                continue;
            }
            if let Some(result) =
                try_handle_edit_agent_command(runtime, &current_cwd, &mut edit_session, &line)
            {
                match result {
                    Ok(()) => {}
                    Err(code) if code == 2 => {
                        return 199;
                    }
                    Err(_) => return 205,
                }
                continue;
            }
            if line == "system-queues" {
                if shell_render_system_queues(runtime).is_err() {
                    return 205;
                }
                continue;
            }
            last_status = 127;
            if write_line(runtime, "unknown-command").is_err() {
                return 199;
            }
        }
    }
    shell_cleanup_game_sessions(runtime, &mut game_sessions, &mut jobs);
    last_status
}

fn run_program<B: SyscallBackend>(runtime: &Runtime<B>, bootstrap: &BootstrapArgs<'_>) -> ExitCode {
    #[cfg(target_os = "none")]
    debug_break(ngos_user_abi::USER_DEBUG_MARKER_MAIN, 0);

    if bootstrap.argc != bootstrap.argv.len() || bootstrap.argc == 0 {
        return 1;
    }
    if !image_path_matches_program(bootstrap.argv[0]) {
        return 103;
    }
    if bootstrap_has_arg(bootstrap, COMPAT_WORKER_ARG) {
        return run_native_game_compat_worker(runtime, bootstrap);
    }
    let boot_mode = bootstrap.is_boot_mode();
    let boot_flag = bootstrap.has_flag(BOOT_ARG_FLAG);
    if boot_mode && !boot_flag {
        return 104;
    }
    if boot_mode {
        let context = match parse_boot_context(bootstrap) {
            Ok(context) => context,
            Err(_) => return 105,
        };
        if context.page_size != 4096 {
            return 106;
        }
        if context.entry == 0 {
            return 107;
        }
        if context.process_name != PROGRAM_NAME {
            return 108;
        }
        if !image_path_matches_program(&context.image_path) {
            return 109;
        }
        if context.cwd != "/" {
            return 110;
        }
        if context.root_mount_path != "/" {
            return 111;
        }
        if context.root_mount_name != "rootfs" {
            return 112;
        }
        if context.image_base == 0 {
            return 113;
        }
        if context.stack_top == 0 {
            return 114;
        }
        if context.phdr == 0 {
            return 115;
        }
        if context.phent == 0 {
            return 116;
        }
        if context.phnum == 0 {
            return 117;
        }
        let framebuffer = match &context.framebuffer {
            Some(framebuffer) => framebuffer,
            None => return 118,
        };
        if framebuffer.width == 0 {
            return 119;
        }
        if framebuffer.height == 0 {
            return 120;
        }
        if framebuffer.pitch == 0 {
            return 121;
        }
        if framebuffer.bpp == 0 {
            return 122;
        }
        if context.memory_region_count == 0 {
            return 123;
        }
        if context.usable_memory_bytes == 0 {
            return 124;
        }
        if context.module_phys_start == 0 || context.module_phys_end <= context.module_phys_start {
            return 125;
        }
        if context.kernel_phys_start == 0 || context.kernel_phys_end <= context.kernel_phys_start {
            return 126;
        }
        match context.boot_outcome_policy {
            BootOutcomePolicy::RequireZeroExit | BootOutcomePolicy::AllowAnyExit => {}
        }
        if bootstrap.env_value(BOOT_ENV_PROOF_PREFIX) == Some("wasm") {
            let _ = write_line(runtime, "boot.proof=wasm");
            return run_native_wasm_boot_smoke(runtime);
        }
        if bootstrap.env_value(BOOT_ENV_PROOF_PREFIX) == Some("vfs") {
            let _ = write_line(runtime, "boot.proof=vfs");
            return run_native_vfs_boot_smoke(runtime);
        }
        let desktop_code = run_boot_desktop(runtime, &context);
        if desktop_code != 0 {
            return desktop_code;
        }
        return run_native_vm_boot_smoke(runtime);
    }

    run_native_surface_smoke(runtime, true)
}

fn run_boot_desktop<B: SyscallBackend>(
    runtime: &Runtime<B>,
    context: &ngos_user_abi::bootstrap::BootContext,
) -> ExitCode {
    let framebuffer = match &context.framebuffer {
        Some(framebuffer) => framebuffer,
        None => return 118,
    };
    let frame = match build_boot_desktop_frame(framebuffer) {
        Some(frame) => frame,
        None => return 127,
    };
    let encoded = frame.encode("desktop-boot");

    if write_line(
        runtime,
        &format!(
            "desktop.boot stage=compose surface={}x{} frame={} queue={} present={} completion={} ops={}",
            frame.width,
            frame.height,
            frame.frame_tag,
            frame.queue,
            frame.present_mode,
            frame.completion,
            frame.ops.len()
        ),
    )
    .is_err()
    {
        return 190;
    }
    let queue_configured = shell_gpu_queue_capacity(runtime, "/dev/gpu0", 32).is_ok();
    if !queue_configured
        && write_line(
            runtime,
            "desktop.boot stage=queue-config queue=default reason=unsupported",
        )
        .is_err()
    {
        return 190;
    }
    if shell_gpu_submit(runtime, "/dev/gpu0", &encoded.payload).is_err() {
        return 234;
    }
    let presented = shell_gpu_present(runtime, "/dev/gpu0", &encoded.frame_tag).is_ok();
    if presented {
        if write_line(
            runtime,
            &format!(
                "desktop.boot stage=presented frame={} payload={} framebuffer={}x{}",
                encoded.frame_tag,
                encoded.payload.len(),
                framebuffer.width,
                framebuffer.height
            ),
        )
        .is_err()
        {
            return 190;
        }
    } else if write_line(
        runtime,
        &format!(
            "desktop.boot stage=submitted frame={} payload={} framebuffer={}x{} present=pending",
            encoded.frame_tag,
            encoded.payload.len(),
            framebuffer.width,
            framebuffer.height
        ),
    )
    .is_err()
    {
        return 190;
    }
    0
}

fn build_boot_desktop_frame(
    framebuffer: &ngos_user_abi::bootstrap::FramebufferContext,
) -> Option<FrameScript> {
    let width = u32::try_from(framebuffer.width).ok()?;
    let height = u32::try_from(framebuffer.height).ok()?;
    if width < 640 || height < 480 {
        return None;
    }

    let top_bar_h = (height / 13).max(60);
    let dock_h = (height / 11).max(78);
    let sidebar_w = (width / 5).max(240);
    let margin = (width / 42).max(24);
    let gap = (margin / 2).max(16);
    let widget_h = (height / 7).max(96);
    let card_h = (height / 10).max(74);
    let dock_y = height.saturating_sub(dock_h);
    let sidebar_h = height.saturating_sub(top_bar_h).saturating_sub(dock_h);
    let workspace_x = sidebar_w.saturating_add(margin);
    let workspace_y = top_bar_h.saturating_add(margin);
    let workspace_w = width.saturating_sub(workspace_x).saturating_sub(margin);
    let workspace_h = dock_y.saturating_sub(workspace_y).saturating_sub(margin);
    let inspector_w = (workspace_w / 4).max(240);
    let canvas_w = workspace_w.saturating_sub(inspector_w).saturating_sub(gap);
    let main_window_h = workspace_h
        .saturating_sub((workspace_h / 3).max(180))
        .saturating_sub(gap);
    let bottom_window_h = workspace_h
        .saturating_sub(main_window_h)
        .saturating_sub(gap);
    let main_window_x = workspace_x;
    let main_window_y = workspace_y;
    let inspector_x = main_window_x.saturating_add(canvas_w).saturating_add(gap);
    let inspector_y = workspace_y;
    let bottom_window_x = workspace_x;
    let bottom_window_y = main_window_y
        .saturating_add(main_window_h)
        .saturating_add(gap);

    let mut ops = vec![
        DrawOp::Clear {
            color: rgba(0x0b, 0x11, 0x1a),
        },
        DrawOp::GradientRect {
            x: 0,
            y: 0,
            width,
            height,
            top_left: rgba(0x0b, 0x11, 0x1a),
            top_right: rgba(0x14, 0x1d, 0x2f),
            bottom_left: rgba(0x10, 0x18, 0x25),
            bottom_right: rgba(0x06, 0x0b, 0x12),
        },
        DrawOp::Rect {
            x: 0,
            y: 0,
            width,
            height,
            color: rgba(0x0f, 0x17, 0x24),
        },
        DrawOp::Rect {
            x: 0,
            y: 0,
            width,
            height: top_bar_h,
            color: rgba(0x1a, 0x23, 0x33),
        },
        DrawOp::Rect {
            x: 0,
            y: top_bar_h,
            width: sidebar_w,
            height: sidebar_h,
            color: rgba(0x13, 0x1b, 0x28),
        },
        DrawOp::Rect {
            x: 0,
            y: dock_y,
            width,
            height: dock_h,
            color: rgba(0x16, 0x1e, 0x2d),
        },
        DrawOp::Rect {
            x: main_window_x,
            y: main_window_y,
            width: canvas_w,
            height: main_window_h,
            color: rgba(0x1f, 0x2a, 0x3d),
        },
        DrawOp::ShadowRect {
            x: main_window_x.saturating_sub(gap / 2),
            y: main_window_y.saturating_sub(gap / 2),
            width: canvas_w.saturating_add(gap),
            height: main_window_h.saturating_add(gap),
            blur: gap,
            color: rgbaa(0x00, 0x00, 0x00, 0x56),
        },
        DrawOp::RoundedRect {
            x: main_window_x,
            y: main_window_y,
            width: canvas_w,
            height: main_window_h,
            radius: gap,
            color: rgbaa(0xf6, 0xfb, 0xff, 0x14),
        },
        DrawOp::Rect {
            x: main_window_x,
            y: main_window_y,
            width: canvas_w,
            height: (top_bar_h / 2).max(28),
            color: rgba(0x2a, 0x34, 0x48),
        },
        DrawOp::Rect {
            x: main_window_x,
            y: main_window_y,
            width: 6,
            height: (top_bar_h / 2).max(28),
            color: rgba(0x4b, 0x92, 0xe8),
        },
        DrawOp::Rect {
            x: inspector_x,
            y: inspector_y,
            width: inspector_w,
            height: main_window_h,
            color: rgba(0x1b, 0x24, 0x35),
        },
        DrawOp::ShadowRect {
            x: inspector_x.saturating_sub(gap / 2),
            y: inspector_y.saturating_sub(gap / 2),
            width: inspector_w.saturating_add(gap),
            height: main_window_h.saturating_add(gap),
            blur: gap,
            color: rgbaa(0x00, 0x00, 0x00, 0x50),
        },
        DrawOp::RoundedRect {
            x: inspector_x,
            y: inspector_y,
            width: inspector_w,
            height: main_window_h,
            radius: gap,
            color: rgbaa(0xf6, 0xfb, 0xff, 0x12),
        },
        DrawOp::Rect {
            x: inspector_x,
            y: inspector_y,
            width: inspector_w,
            height: (top_bar_h / 2).max(28),
            color: rgba(0x26, 0x31, 0x45),
        },
        DrawOp::Rect {
            x: bottom_window_x,
            y: bottom_window_y,
            width: workspace_w,
            height: bottom_window_h,
            color: rgba(0x18, 0x22, 0x31),
        },
        DrawOp::ShadowRect {
            x: bottom_window_x.saturating_sub(gap / 2),
            y: bottom_window_y.saturating_sub(gap / 2),
            width: workspace_w.saturating_add(gap),
            height: bottom_window_h.saturating_add(gap),
            blur: gap,
            color: rgbaa(0x00, 0x00, 0x00, 0x4a),
        },
        DrawOp::RoundedRect {
            x: bottom_window_x,
            y: bottom_window_y,
            width: workspace_w,
            height: bottom_window_h,
            radius: gap,
            color: rgbaa(0xf6, 0xfb, 0xff, 0x10),
        },
        DrawOp::Rect {
            x: bottom_window_x,
            y: bottom_window_y,
            width: workspace_w,
            height: (top_bar_h / 2).max(28),
            color: rgba(0x25, 0x30, 0x42),
        },
        DrawOp::Rect {
            x: main_window_x + gap,
            y: main_window_y + top_bar_h / 2,
            width: canvas_w.saturating_sub(gap * 2),
            height: main_window_h.saturating_sub(top_bar_h),
            color: rgba(0x2d, 0x3c, 0x56),
        },
        DrawOp::Rect {
            x: main_window_x + gap * 2,
            y: main_window_y + top_bar_h / 2 + gap,
            width: canvas_w / 2,
            height: (main_window_h / 2).max(120),
            color: rgba(0x46, 0x81, 0xd8),
        },
        DrawOp::Rect {
            x: main_window_x + canvas_w / 2 + gap,
            y: main_window_y + top_bar_h / 2 + gap,
            width: canvas_w
                .saturating_sub(canvas_w / 2)
                .saturating_sub(gap * 3),
            height: (main_window_h / 3).max(96),
            color: rgba(0x2f, 0xb0, 0x8b),
        },
        DrawOp::Rect {
            x: main_window_x + canvas_w / 2 + gap,
            y: main_window_y + main_window_h / 2,
            width: canvas_w
                .saturating_sub(canvas_w / 2)
                .saturating_sub(gap * 3),
            height: (main_window_h / 3).max(96),
            color: rgba(0xc8, 0x73, 0x31),
        },
    ];

    for (x, y, w, h, color) in [
        (
            margin / 2,
            top_bar_h + margin,
            width / 3,
            height / 3,
            rgbaa(0x2d, 0xb6, 0xb0, 0x24),
        ),
        (
            width / 3,
            height / 5,
            width / 2,
            height / 3,
            rgbaa(0x74, 0x8b, 0xff, 0x20),
        ),
        (
            width / 2,
            height / 3,
            width / 3,
            height / 3,
            rgbaa(0xff, 0x9a, 0x5a, 0x18),
        ),
        (
            width / 5,
            dock_y.saturating_sub(height / 5),
            width / 2,
            height / 4,
            rgbaa(0x6d, 0xd8, 0xf8, 0x10),
        ),
    ] {
        ops.push(DrawOp::Rect {
            x,
            y,
            width: w,
            height: h,
            color,
        });
    }
    for band in 0..6 {
        ops.push(DrawOp::Rect {
            x: 0,
            y: (height / 6) * band,
            width,
            height: (height / 8).max(48),
            color: if band % 2 == 0 {
                rgbaa(0xff, 0xff, 0xff, 0x06)
            } else {
                rgbaa(0x78, 0x92, 0xbd, 0x08)
            },
        });
    }
    for (x, y, w, h) in [
        (
            main_window_x.saturating_sub(gap / 2),
            main_window_y.saturating_sub(gap / 2),
            canvas_w + gap,
            main_window_h + gap,
        ),
        (
            inspector_x.saturating_sub(gap / 2),
            inspector_y.saturating_sub(gap / 2),
            inspector_w + gap,
            main_window_h + gap,
        ),
        (
            bottom_window_x.saturating_sub(gap / 2),
            bottom_window_y.saturating_sub(gap / 2),
            workspace_w + gap,
            bottom_window_h + gap,
        ),
    ] {
        ops.push(DrawOp::Rect {
            x,
            y,
            width: w,
            height: h,
            color: rgbaa(0x00, 0x00, 0x00, 0x38),
        });
        ops.push(DrawOp::Rect {
            x: x + 1,
            y: y + 1,
            width: w.saturating_sub(2),
            height: h.saturating_sub(2),
            color: rgbaa(0xff, 0xff, 0xff, 0x10),
        });
    }
    ops.push(DrawOp::Rect {
        x: 0,
        y: dock_y,
        width,
        height: dock_h,
        color: rgbaa(0xff, 0xff, 0xff, 0x10),
    });
    ops.push(DrawOp::Rect {
        x: 0,
        y: 0,
        width,
        height: top_bar_h,
        color: rgbaa(0xff, 0xff, 0xff, 0x0c),
    });

    let title_button = (top_bar_h / 8).max(10);
    let title_spacing = title_button + gap / 2;
    for (base_x, base_y) in [
        (main_window_x + gap, main_window_y + gap / 2),
        (inspector_x + gap, inspector_y + gap / 2),
        (bottom_window_x + gap, bottom_window_y + gap / 2),
    ] {
        for (index, color) in [
            rgba(0xf0, 0x6b, 0x63),
            rgba(0xf3, 0xc9, 0x57),
            rgba(0x67, 0xd8, 0x84),
        ]
        .into_iter()
        .enumerate()
        {
            ops.push(DrawOp::Rect {
                x: base_x + index as u32 * title_spacing,
                y: base_y,
                width: title_button,
                height: title_button,
                color,
            });
        }
        ops.push(DrawOp::Rect {
            x: base_x + title_spacing * 4,
            y: base_y,
            width: (width / 10).max(100),
            height: title_button,
            color: rgba(0x3a, 0x47, 0x60),
        });
    }
    ops.push(DrawOp::Line {
        x0: main_window_x,
        y0: main_window_y + (top_bar_h / 2).max(28),
        x1: main_window_x + canvas_w,
        y1: main_window_y + (top_bar_h / 2).max(28),
        color: rgba(0x58, 0x9d, 0xf0),
    });
    ops.push(DrawOp::Line {
        x0: inspector_x,
        y0: inspector_y + (top_bar_h / 2).max(28),
        x1: inspector_x + inspector_w,
        y1: inspector_y + (top_bar_h / 2).max(28),
        color: rgba(0x3a, 0x48, 0x61),
    });
    ops.push(DrawOp::Line {
        x0: bottom_window_x,
        y0: bottom_window_y + (top_bar_h / 2).max(28),
        x1: bottom_window_x + workspace_w,
        y1: bottom_window_y + (top_bar_h / 2).max(28),
        color: rgba(0x3a, 0x48, 0x61),
    });

    let sidebar_inner_w = sidebar_w.saturating_sub(margin * 2);
    let sidebar_x = margin;
    let mut sidebar_y = top_bar_h + margin;
    let sidebar_blocks = [
        (widget_h, rgba(0x25, 0x31, 0x47)),
        (card_h, rgba(0x1c, 0x64, 0x7d)),
        (card_h, rgba(0x6b, 0x46, 0x74)),
        ((height / 5).max(128), rgba(0x2b, 0x35, 0x4d)),
    ];
    for (block_h, color) in sidebar_blocks {
        ops.push(DrawOp::Rect {
            x: sidebar_x,
            y: sidebar_y,
            width: sidebar_inner_w,
            height: block_h,
            color,
        });
        ops.push(DrawOp::Rect {
            x: sidebar_x + gap,
            y: sidebar_y + gap,
            width: sidebar_inner_w.saturating_sub(gap * 2),
            height: (block_h / 4).max(22),
            color: rgba(0x34, 0x42, 0x5a),
        });
        ops.push(DrawOp::Rect {
            x: sidebar_x + gap,
            y: sidebar_y + gap * 3,
            width: (sidebar_inner_w / 3).max(54),
            height: (block_h / 6).max(18),
            color: rgba(0x4a, 0x92, 0xe6),
        });
        for row in 0..3 {
            ops.push(DrawOp::Rect {
                x: sidebar_x + gap,
                y: sidebar_y + gap * 5 + row * ((block_h / 7).max(16)),
                width: sidebar_inner_w.saturating_sub(gap * 2),
                height: (block_h / 10).max(12),
                color: if row == 0 {
                    rgba(0x3a, 0x4e, 0x6a)
                } else {
                    rgba(0x2b, 0x37, 0x4d)
                },
            });
        }
        if block_h == widget_h {
            ops.push(DrawOp::Rect {
                x: sidebar_x + gap / 2,
                y: sidebar_y + gap * 5 + (block_h / 7).max(16),
                width: 4,
                height: (block_h / 10).max(12),
                color: rgba(0x58, 0x9d, 0xf0),
            });
        }
        sidebar_y = sidebar_y.saturating_add(block_h).saturating_add(gap);
    }

    let top_chip_w = (width / 9).max(120);
    for index in 0..4 {
        ops.push(DrawOp::Rect {
            x: margin + index * (top_chip_w + gap / 2),
            y: gap / 2,
            width: top_chip_w,
            height: top_bar_h.saturating_sub(gap),
            color: if index == 0 {
                rgba(0x2f, 0x74, 0xd0)
            } else {
                rgba(0x26, 0x30, 0x42)
            },
        });
        ops.push(DrawOp::Rect {
            x: margin + index * (top_chip_w + gap / 2) + gap,
            y: top_bar_h.saturating_sub(gap / 2 + 4),
            width: top_chip_w.saturating_sub(gap * 2),
            height: 3,
            color: if index == 0 || index == 2 {
                rgba(0x58, 0x9d, 0xf0)
            } else {
                rgba(0x3b, 0x47, 0x5f)
            },
        });
    }

    let right_cluster_w = (width / 7).max(180);
    ops.push(DrawOp::Rect {
        x: width.saturating_sub(right_cluster_w).saturating_sub(margin),
        y: gap / 2,
        width: right_cluster_w,
        height: top_bar_h.saturating_sub(gap),
        color: rgba(0x31, 0x3d, 0x53),
    });
    for index in 0..3 {
        ops.push(DrawOp::Rect {
            x: width.saturating_sub(right_cluster_w).saturating_sub(margin)
                + gap
                + index * ((right_cluster_w / 4).max(34)),
            y: gap,
            width: (right_cluster_w / 6).max(18),
            height: top_bar_h.saturating_sub(gap * 2),
            color: rgba(0x44, 0x52, 0x69),
        });
    }
    for (index, color) in [
        rgba(0x67, 0xd8, 0x84),
        rgba(0xf3, 0xc9, 0x57),
        rgba(0xf0, 0x6b, 0x63),
    ]
    .into_iter()
    .enumerate()
    {
        ops.push(DrawOp::Rect {
            x: width.saturating_sub(right_cluster_w).saturating_sub(margin) + right_cluster_w
                - gap * 2
                - (index as u32 + 1) * ((right_cluster_w / 7).max(18)),
            y: gap,
            width: (right_cluster_w / 10).max(10),
            height: top_bar_h.saturating_sub(gap * 2),
            color,
        });
    }
    ops.push(DrawOp::Line {
        x0: width.saturating_sub(right_cluster_w).saturating_sub(margin) + gap,
        y0: top_bar_h,
        x1: inspector_x + gap,
        y1: workspace_y + gap,
        color: rgba(0x67, 0xd8, 0x84),
    });
    ops.push(DrawOp::Line {
        x0: margin + top_chip_w / 2,
        y0: top_bar_h,
        x1: main_window_x + gap * 2,
        y1: main_window_y + top_bar_h / 2 + gap,
        color: rgba(0x58, 0x9d, 0xf0),
    });

    let inspector_card_w = inspector_w.saturating_sub(gap * 2);
    let inspector_card_x = inspector_x + gap;
    let mut inspector_card_y = inspector_y + gap;
    for (card_index, (card_height, color)) in [
        ((height / 6).max(120), rgba(0x2d, 0x39, 0x50)),
        ((height / 8).max(90), rgba(0x25, 0x54, 0x66)),
        ((height / 7).max(100), rgba(0x5e, 0x55, 0x90)),
        ((height / 9).max(82), rgba(0x2f, 0x6a, 0x91)),
    ]
    .into_iter()
    .enumerate()
    {
        ops.push(DrawOp::Rect {
            x: inspector_card_x,
            y: inspector_card_y,
            width: inspector_card_w,
            height: card_height,
            color,
        });
        ops.push(DrawOp::Rect {
            x: inspector_card_x + gap,
            y: inspector_card_y + gap,
            width: inspector_card_w.saturating_sub(gap * 2),
            height: (card_height / 5).max(18),
            color: rgba(0x3a, 0x49, 0x64),
        });
        ops.push(DrawOp::Rect {
            x: inspector_card_x + inspector_card_w.saturating_sub(gap * 3),
            y: inspector_card_y + gap + 4,
            width: (inspector_card_w / 7).max(16),
            height: (card_height / 10).max(10),
            color: match card_index {
                0 => rgba(0x67, 0xd8, 0x84),
                1 => rgba(0x58, 0x9d, 0xf0),
                2 => rgba(0xf3, 0xc9, 0x57),
                _ => rgba(0xf0, 0x6b, 0x63),
            },
        });
        for metric in 0..2 {
            ops.push(DrawOp::Rect {
                x: inspector_card_x + gap,
                y: inspector_card_y + gap * 3 + metric * ((card_height / 3).max(24)),
                width: inspector_card_w.saturating_sub(gap * 2),
                height: (card_height / 8).max(14),
                color: if metric == 0 {
                    rgba(0x4b, 0x92, 0xe8)
                } else {
                    rgba(0x2f, 0x6c, 0x90)
                },
            });
        }
        for slice in 0..3 {
            ops.push(DrawOp::Rect {
                x: inspector_card_x + gap,
                y: inspector_card_y + card_height.saturating_sub(gap * 2)
                    - slice * ((card_height / 9).max(10)),
                width: (inspector_card_w / 5)
                    .saturating_add(slice * ((inspector_card_w / 10).max(10))),
                height: (card_height / 14).max(8),
                color: match card_index {
                    0 => rgba(0x67, 0xd8, 0x84),
                    1 => rgba(0x58, 0x9d, 0xf0),
                    2 => rgba(0xf3, 0xc9, 0x57),
                    _ => rgba(0xf0, 0x6b, 0x63),
                },
            });
        }
        inspector_card_y = inspector_card_y
            .saturating_add(card_height)
            .saturating_add(gap);
    }

    let content_panel_x = main_window_x + gap * 2;
    let content_panel_y = main_window_y + top_bar_h / 2 + gap;
    let content_panel_w = canvas_w.saturating_sub(gap * 4);
    let content_panel_h = main_window_h.saturating_sub(top_bar_h + gap * 2);
    let left_nav_w = (content_panel_w / 5).max(96);
    let feed_x = content_panel_x + left_nav_w + gap;
    let feed_w = content_panel_w.saturating_sub(left_nav_w + gap * 2);
    ops.push(DrawOp::Rect {
        x: content_panel_x,
        y: content_panel_y,
        width: left_nav_w,
        height: content_panel_h,
        color: rgba(0x24, 0x33, 0x48),
    });
    for row in 0..5 {
        ops.push(DrawOp::Rect {
            x: content_panel_x + gap / 2,
            y: content_panel_y + gap / 2 + row * ((content_panel_h / 6).max(22)),
            width: left_nav_w.saturating_sub(gap),
            height: (content_panel_h / 9).max(16),
            color: if row == 1 {
                rgba(0x4a, 0x92, 0xe6)
            } else {
                rgba(0x2f, 0x3d, 0x55)
            },
        });
        if row == 0 || row == 3 {
            ops.push(DrawOp::Rect {
                x: content_panel_x + left_nav_w.saturating_sub(gap + 10),
                y: content_panel_y + gap / 2 + row * ((content_panel_h / 6).max(22)) + 3,
                width: 6,
                height: ((content_panel_h / 9).max(16)).saturating_sub(6),
                color: if row == 0 {
                    rgba(0x67, 0xd8, 0x84)
                } else {
                    rgba(0xf3, 0xc9, 0x57)
                },
            });
        }
    }
    ops.push(DrawOp::Rect {
        x: feed_x,
        y: content_panel_y,
        width: feed_w,
        height: (content_panel_h / 6).max(30),
        color: rgba(0x32, 0x40, 0x56),
    });
    for col in 0..3 {
        ops.push(DrawOp::Rect {
            x: feed_x + gap + col * ((feed_w / 4).max(58)),
            y: content_panel_y + gap / 2,
            width: (feed_w / 6).max(34),
            height: (content_panel_h / 10).max(14),
            color: match col {
                0 => rgba(0x58, 0x9d, 0xf0),
                1 => rgba(0x67, 0xd8, 0x84),
                _ => rgba(0xf3, 0xc9, 0x57),
            },
        });
    }
    for card in 0..3 {
        let card_y = content_panel_y
            + (content_panel_h / 5).max(40)
            + card * ((content_panel_h / 4).max(46));
        ops.push(DrawOp::Rect {
            x: feed_x,
            y: card_y,
            width: feed_w,
            height: (content_panel_h / 5).max(36),
            color: if card == 0 {
                rgba(0x41, 0x55, 0x73)
            } else {
                rgba(0x2f, 0x3f, 0x57)
            },
        });
        ops.push(DrawOp::Rect {
            x: feed_x + gap,
            y: card_y + gap / 2,
            width: feed_w / 2,
            height: (content_panel_h / 12).max(12),
            color: rgba(0x51, 0x62, 0x7e),
        });
        for pulse in 0..4 {
            ops.push(DrawOp::Rect {
                x: feed_x + feed_w.saturating_sub(gap * 2) - pulse * ((feed_w / 10).max(18)),
                y: card_y + gap,
                width: (feed_w / 18).max(8),
                height: match card {
                    0 => (content_panel_h / 12).max(12) + pulse * 2,
                    1 => (content_panel_h / 8).max(16).saturating_sub(pulse * 2),
                    _ => (content_panel_h / 14).max(10) + (pulse % 2) * 6,
                },
                color: match card {
                    0 => rgba(0x58, 0x9d, 0xf0),
                    1 => rgba(0x67, 0xd8, 0x84),
                    _ => rgba(0xf3, 0xc9, 0x57),
                },
            });
        }
        if card == 0 {
            ops.push(DrawOp::Rect {
                x: feed_x,
                y: card_y,
                width: 5,
                height: (content_panel_h / 5).max(36),
                color: rgba(0x58, 0x9d, 0xf0),
            });
        }
    }

    let lane_y = bottom_window_y + gap;
    let lane_h = bottom_window_h.saturating_sub(gap * 2);
    let lane_gap = gap;
    let lane_w = workspace_w.saturating_sub(lane_gap * 4) / 3;
    for (index, color) in [
        rgba(0x21, 0x30, 0x43),
        rgba(0x2b, 0x5d, 0x84),
        rgba(0x6f, 0x44, 0x38),
    ]
    .into_iter()
    .enumerate()
    {
        let lane_x = bottom_window_x + lane_gap + index as u32 * (lane_w + lane_gap);
        ops.push(DrawOp::Rect {
            x: lane_x,
            y: lane_y,
            width: lane_w,
            height: lane_h,
            color,
        });
        ops.push(DrawOp::Rect {
            x: lane_x + gap,
            y: lane_y + gap,
            width: lane_w.saturating_sub(gap * 2),
            height: (lane_h / 3).max(58),
            color: rgba(0x39, 0x4d, 0x69),
        });
        ops.push(DrawOp::Rect {
            x: lane_x + gap,
            y: lane_y + lane_h / 2,
            width: lane_w.saturating_sub(gap * 2),
            height: (lane_h / 4).max(44),
            color: rgba(0x18, 0x24, 0x34),
        });
        ops.push(DrawOp::Rect {
            x: lane_x + gap,
            y: lane_y + lane_h.saturating_sub((lane_h / 5).max(34)) - gap,
            width: lane_w / 3,
            height: (lane_h / 5).max(34),
            color: match index {
                0 => rgba(0x67, 0xd8, 0x84),
                1 => rgba(0x58, 0x9d, 0xf0),
                _ => rgba(0xf3, 0xc9, 0x57),
            },
        });
        for row in 0..2 {
            ops.push(DrawOp::Rect {
                x: lane_x + lane_w / 2,
                y: lane_y + gap + row * ((lane_h / 4).max(28)),
                width: lane_w.saturating_sub(lane_w / 2).saturating_sub(gap * 2),
                height: (lane_h / 8).max(16),
                color: rgba(0x31, 0x46, 0x5e),
            });
        }
        for tick in 0..4 {
            ops.push(DrawOp::Rect {
                x: lane_x + gap + tick * ((lane_w / 6).max(18)),
                y: lane_y + lane_h / 2 + gap,
                width: (lane_w / 12).max(8),
                height: match index {
                    0 => (lane_h / 8).max(14) + tick * 2,
                    1 => (lane_h / 5).max(18).saturating_sub(tick * 3),
                    _ => (lane_h / 9).max(10) + (tick % 2) * 8,
                },
                color: match index {
                    0 => rgba(0x67, 0xd8, 0x84),
                    1 => rgba(0x58, 0x9d, 0xf0),
                    _ => rgba(0xf3, 0xc9, 0x57),
                },
            });
        }
    }

    let dock_item_w = (width / 14).max(78);
    let dock_item_h = dock_h.saturating_sub(gap * 2);
    for index in 0..6 {
        ops.push(DrawOp::Rect {
            x: margin + index * (dock_item_w + gap),
            y: dock_y + gap,
            width: dock_item_w,
            height: dock_item_h,
            color: if index == 1 || index == 3 {
                rgba(0x3d, 0x7f, 0xd6)
            } else {
                rgba(0x2a, 0x36, 0x4d)
            },
        });
        ops.push(DrawOp::Rect {
            x: margin + index * (dock_item_w + gap) + gap,
            y: dock_y + gap * 2,
            width: dock_item_w.saturating_sub(gap * 2),
            height: dock_item_h.saturating_sub(gap * 2),
            color: rgba(0x1f, 0x2a, 0x3b),
        });
        if index == 1 || index == 3 || index == 4 {
            ops.push(DrawOp::Rect {
                x: margin + index * (dock_item_w + gap) + dock_item_w / 3,
                y: dock_y + dock_item_h + gap / 2,
                width: dock_item_w / 3,
                height: 4,
                color: rgba(0x58, 0x9d, 0xf0),
            });
        }
    }
    ops.push(DrawOp::Line {
        x0: margin + dock_item_w * 3 + gap * 3,
        y0: dock_y + gap,
        x1: margin + dock_item_w * 3 + gap * 3,
        y1: dock_y + dock_item_h,
        color: rgba(0x45, 0x55, 0x6b),
    });
    ops.push(DrawOp::Rect {
        x: width
            .saturating_sub((width / 6).max(164))
            .saturating_sub(margin),
        y: dock_y + gap,
        width: (width / 6).max(164),
        height: dock_item_h,
        color: rgba(0x3f, 0x4b, 0x64),
    });
    ops.push(DrawOp::Rect {
        x: width
            .saturating_sub((width / 6).max(164))
            .saturating_sub(margin)
            + gap,
        y: dock_y + gap * 2,
        width: (width / 6).max(164).saturating_sub(gap * 2),
        height: dock_item_h.saturating_sub(gap * 2),
        color: rgba(0x2c, 0x37, 0x4b),
    });
    for (slot, color) in [
        rgba(0x67, 0xd8, 0x84),
        rgba(0x4b, 0x92, 0xe8),
        rgba(0xf3, 0xc9, 0x57),
    ]
    .into_iter()
    .enumerate()
    {
        ops.push(DrawOp::Rect {
            x: width
                .saturating_sub((width / 6).max(164))
                .saturating_sub(margin)
                + gap * 2
                + slot as u32 * (((width / 6).max(164) / 4).max(24)),
            y: dock_y + dock_item_h / 2,
            width: (((width / 6).max(164)) / 7).max(12),
            height: dock_item_h / 5,
            color,
        });
    }
    ops.push(DrawOp::Line {
        x0: feed_x + feed_w.saturating_sub(gap * 2),
        y0: content_panel_y + (content_panel_h / 3),
        x1: margin + (dock_item_w + gap) * 4 + dock_item_w / 2,
        y1: dock_y + gap,
        color: rgba(0x58, 0x9d, 0xf0),
    });
    ops.push(DrawOp::Line {
        x0: inspector_card_x + inspector_card_w / 2,
        y0: inspector_y + gap,
        x1: bottom_window_x + lane_w + lane_gap,
        y1: lane_y,
        color: rgba(0x67, 0xd8, 0x84),
    });

    for (x0, y0, x1, y1, color) in [
        (
            0,
            top_bar_h,
            width.saturating_sub(1),
            top_bar_h,
            rgba(0x4d, 0x5b, 0x74),
        ),
        (
            sidebar_w,
            top_bar_h,
            sidebar_w,
            dock_y,
            rgba(0x39, 0x4a, 0x63),
        ),
        (
            0,
            dock_y,
            width.saturating_sub(1),
            dock_y,
            rgba(0x4a, 0x5d, 0x78),
        ),
        (
            main_window_x,
            bottom_window_y.saturating_sub(gap / 2),
            width.saturating_sub(margin),
            bottom_window_y.saturating_sub(gap / 2),
            rgba(0x32, 0x42, 0x58),
        ),
    ] {
        ops.push(DrawOp::Line {
            x0,
            y0,
            x1,
            y1,
            color,
        });
    }

    Some(FrameScript {
        width,
        height,
        frame_tag: String::from("ngos-desktop-boot"),
        queue: String::from("graphics"),
        present_mode: String::from("mailbox"),
        completion: String::from("wait-present"),
        ops,
    })
}

const fn rgba(r: u8, g: u8, b: u8) -> RgbaColor {
    RgbaColor { r, g, b, a: 0xff }
}

const fn rgbaa(r: u8, g: u8, b: u8, a: u8) -> RgbaColor {
    RgbaColor { r, g, b, a }
}

#[inline(never)]
fn run_native_surface_core<B: SyscallBackend>(
    runtime: &Runtime<B>,
    validate_stdio: bool,
) -> ExitCode {
    if validate_stdio && runtime.fcntl(1, FcntlCmd::GetFl).unwrap_or(usize::MAX) != 0 {
        return 2;
    }
    if validate_stdio && runtime.poll(1, POLLOUT).unwrap_or(0) & POLLOUT == 0 {
        return 3;
    }
    let dup_fd = match runtime.dup(1) {
        Ok(fd) => fd,
        Err(_) => return 4,
    };
    debug_break(0x4e47_4f53_4253_3030, dup_fd as u64);
    if runtime
        .fcntl(dup_fd, FcntlCmd::SetFd { cloexec: true })
        .unwrap_or(usize::MAX)
        != 2
    {
        return 5;
    }
    debug_break(0x4e47_4f53_4253_3041, dup_fd as u64);
    if runtime.fcntl(dup_fd, FcntlCmd::GetFd).unwrap_or(usize::MAX) != 2 {
        return 6;
    }
    debug_break(0x4e47_4f53_4253_3042, dup_fd as u64);
    if runtime.close(dup_fd).is_err() {
        return 7;
    }
    debug_break(0x4e47_4f53_4253_3043, dup_fd as u64);

    debug_break(0x4e47_4f53_4253_3031, 0);
    let storage = match runtime.inspect_device("/dev/storage0") {
        Ok(record) => record,
        Err(_) => return 131,
    };
    debug_break(0x4e47_4f53_4253_3032, storage.block_size as u64);
    if storage.block_size != 512 || storage.capacity_bytes < 512 {
        return 132;
    }
    debug_break(0x4e47_4f53_4253_3033, storage.capacity_bytes);
    let driver = match runtime.inspect_driver("/drv/storage0") {
        Ok(record) => record,
        Err(_) => return 133,
    };
    debug_break(0x4e47_4f53_4253_3034, driver.bound_device_count);
    if driver.bound_device_count != 1 {
        return 134;
    }
    let block_fd = match runtime.open_path("/dev/storage0") {
        Ok(fd) => fd,
        Err(_) => return 135,
    };
    debug_break(0x4e47_4f53_4253_3035, block_fd as u64);
    let rights = ngos_user_abi::BlockRightsMask::READ.union(ngos_user_abi::BlockRightsMask::SUBMIT);
    let (capability, label, provenance, integrity) =
        default_block_request_security(0x5354_4f52_4147_4530, 1, rights);
    let request = NativeBlockIoRequest {
        magic: NATIVE_BLOCK_IO_MAGIC,
        version: NATIVE_BLOCK_IO_VERSION,
        op: NATIVE_BLOCK_IO_OP_READ,
        sector: 0,
        sector_count: 1,
        block_size: storage.block_size,
        rights,
        capability,
        label,
        provenance,
        integrity,
    };
    debug_break(0x4e47_4f53_4253_3036, request.block_size as u64);
    let request_bytes = encode_block_request_bytes(&request);
    debug_break(0x4e47_4f53_4253_3037, request_bytes.len() as u64);
    if runtime.write(block_fd, request_bytes).is_err() {
        let _ = runtime.close(block_fd);
        return 136;
    }
    debug_break(0x4e47_4f53_4253_3038, 0);
    let driver_fd = match runtime.open_path("/drv/storage0") {
        Ok(fd) => fd,
        Err(_) => {
            let _ = runtime.close(block_fd);
            return 137;
        }
    };
    debug_break(0x4e47_4f53_4253_3039, driver_fd as u64);
    if runtime.poll(driver_fd, POLLIN).unwrap_or(0) != POLLIN {
        let _ = runtime.close(driver_fd);
        let _ = runtime.close(block_fd);
        return 138;
    }
    debug_break(0x4e47_4f53_4253_3044, driver_fd as u64);
    let mut driver_bytes = [0u8; 512];
    let driver_read = match runtime.read(driver_fd, &mut driver_bytes) {
        Ok(count) => count,
        Err(_) => {
            let _ = runtime.close(driver_fd);
            let _ = runtime.close(block_fd);
            return 139;
        }
    };
    debug_break(0x4e47_4f53_4253_3045, driver_read as u64);
    if driver_read == 0 {
        let _ = runtime.close(driver_fd);
        let _ = runtime.close(block_fd);
        return 140;
    }
    let driver_payload = &driver_bytes[..driver_read];
    let driver_prefix_len = driver_payload
        .iter()
        .position(|byte| *byte == b'\n')
        .map(|index| index + 1)
        .unwrap_or(driver_payload.len());
    let driver_request = match try_decode_block_request(&driver_payload[driver_prefix_len..]) {
        Some(request) => request,
        None => {
            let _ = runtime.close(driver_fd);
            let _ = runtime.close(block_fd);
            return 141;
        }
    };
    debug_break(0x4e47_4f53_4253_3046, driver_request.sector_count as u64);
    if driver_request.op != NATIVE_BLOCK_IO_OP_READ
        || driver_request.sector != 0
        || driver_request.sector_count != 1
    {
        let _ = runtime.close(driver_fd);
        let _ = runtime.close(block_fd);
        return 142;
    }
    let completion_payload = b"sector0:eb58904d5357494e";
    if runtime.write(driver_fd, completion_payload).is_err() {
        let _ = runtime.close(driver_fd);
        let _ = runtime.close(block_fd);
        return 143;
    }
    if runtime.poll(block_fd, POLLIN).unwrap_or(0) != POLLIN {
        let _ = runtime.close(driver_fd);
        let _ = runtime.close(block_fd);
        return 144;
    }
    debug_break(0x4e47_4f53_4253_3047, block_fd as u64);
    let mut sector0 = [0u8; 512];
    let sector0_read = match runtime.read(block_fd, &mut sector0) {
        Ok(count) => count,
        Err(_) => {
            let _ = runtime.close(driver_fd);
            let _ = runtime.close(block_fd);
            return 145;
        }
    };
    debug_break(0x4e47_4f53_4253_3048, sector0_read as u64);
    let _ = runtime.close(driver_fd);
    let _ = runtime.close(block_fd);
    let completion = &sector0[..sector0_read];
    if completion != completion_payload {
        return 146;
    }

    let domain = match runtime.create_domain(None, "graphics") {
        Ok(id) => id,
        Err(_) => return 8,
    };
    let resource = match runtime.create_resource(domain, NativeResourceKind::Device, "gpu0") {
        Ok(id) => id,
        Err(_) => return 9,
    };
    let contract =
        match runtime.create_contract(domain, resource, NativeContractKind::Display, "scanout") {
            Ok(id) => id,
            Err(_) => return 10,
        };
    let mirror =
        match runtime.create_contract(domain, resource, NativeContractKind::Display, "mirror") {
            Ok(id) => id,
            Err(_) => return 11,
        };
    let recorder =
        match runtime.create_contract(domain, resource, NativeContractKind::Display, "record") {
            Ok(id) => id,
            Err(_) => return 12,
        };

    let mut domain_ids = [0u64; 4];
    let domain_count = match runtime.list_domains(&mut domain_ids) {
        Ok(count) if count >= 1 && domain_ids[0] as usize == domain => count,
        _ => return 12,
    };
    debug_break(0x4e47_4f53_4253_3049, domain_count as u64);
    let mut resource_ids = [0u64; 4];
    if !matches!(
        runtime.list_resources(&mut resource_ids),
        Ok(count) if count >= 1 && resource_ids[0] as usize == resource
    ) {
        return 13;
    }
    debug_break(0x4e47_4f53_4253_3050, resource_ids[0]);
    let mut contract_ids = [0u64; 4];
    if !matches!(
        runtime.list_contracts(&mut contract_ids),
        Ok(count) if count >= 3
            && contract_ids[0] as usize == contract
            && contract_ids[1] as usize == mirror
            && contract_ids[2] as usize == recorder
    ) {
        return 14;
    }
    debug_break(0x4e47_4f53_4253_3051, contract_ids[2]);

    let domain_info = match runtime.inspect_domain(domain) {
        Ok(info) => info,
        Err(_) => return 14,
    };
    debug_break(0x4e47_4f53_4253_3052, domain_info.contract_count);
    if domain_info.id as usize != domain
        || domain_info.resource_count != 1
        || domain_info.contract_count != 3
        || domain_info.owner == 0
        || domain_count < 1
    {
        return 15;
    }

    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 16,
    };
    debug_break(0x4e47_4f53_4253_3053, resource_info.acquire_count);
    if resource_info.id as usize != resource
        || resource_info.domain as usize != domain
        || resource_info.kind != NativeResourceKind::Device as u32
        || resource_info.arbitration != NativeResourceArbitrationPolicy::Fifo as u32
        || resource_info.governance != NativeResourceGovernanceMode::Queueing as u32
        || resource_info.holder_contract != 0
        || resource_info.waiting_count != 0
        || resource_info.acquire_count != 0
        || resource_info.handoff_count != 0
    {
        return 17;
    }

    let contract_info = match runtime.inspect_contract(contract) {
        Ok(info) => info,
        Err(_) => return 18,
    };
    debug_break(0x4e47_4f53_4253_3054, contract_info.state as u64);
    if contract_info.id as usize != contract
        || contract_info.domain as usize != domain
        || contract_info.resource as usize != resource
        || contract_info.kind != NativeContractKind::Display as u32
        || contract_info.state != NativeContractState::Active as u32
    {
        return 19;
    }

    debug_break(0x4e47_4f53_4253_3059, contract as u64);
    if runtime
        .set_contract_state(contract, NativeContractState::Suspended)
        .is_err()
    {
        return 20;
    }
    debug_break(0x4e47_4f53_4253_3055, contract as u64);
    let contract_info = match runtime.inspect_contract(contract) {
        Ok(info) => info,
        Err(_) => return 21,
    };
    debug_break(0x4e47_4f53_4253_3561, contract_info.state as u64);
    if contract_info.state != NativeContractState::Suspended as u32 {
        return 22;
    }
    debug_break(0x4e47_4f53_4253_3062, contract as u64);
    if runtime.invoke_contract(contract) != Err(ngos_user_abi::Errno::Access) {
        return 23;
    }
    debug_break(0x4e47_4f53_4253_3056, contract as u64);
    debug_break(0x4e47_4f53_4253_3060, contract as u64);
    if runtime
        .set_contract_state(contract, NativeContractState::Active)
        .is_err()
    {
        return 24;
    }
    let contract_info = match runtime.inspect_contract(contract) {
        Ok(info) => info,
        Err(_) => return 25,
    };
    if contract_info.state != NativeContractState::Active as u32 {
        return 26;
    }
    let invocation_count = match runtime.invoke_contract(contract) {
        Ok(count) => count,
        Err(_) => return 27,
    };
    debug_break(0x4e47_4f53_4253_3057, invocation_count as u64);
    if invocation_count != 1 {
        return 28;
    }
    if runtime
        .set_resource_arbitration_policy(resource, NativeResourceArbitrationPolicy::Fifo)
        .is_err()
    {
        return 29;
    }
    match runtime.claim_resource(contract) {
        Ok(ResourceClaimOutcome::Acquired {
            resource: id,
            acquire_count: 1,
        }) if id == resource => {}
        _ => return 30,
    }
    debug_break(0x4e47_4f53_4253_3058, resource as u64);
    match runtime.claim_resource(mirror) {
        Ok(ResourceClaimOutcome::Queued {
            resource: id,
            holder_contract,
            position: 1,
        }) if id == resource && holder_contract == contract => {}
        _ => return 31,
    }
    match runtime.claim_resource(recorder) {
        Ok(ResourceClaimOutcome::Queued {
            resource: id,
            holder_contract,
            position: 2,
        }) if id == resource && holder_contract == contract => {}
        _ => return 32,
    }
    let mut waiters = [0u64; 4];
    if !matches!(
        runtime.list_resource_waiters(resource, &mut waiters),
        Ok(2) if waiters[0] as usize == mirror && waiters[1] as usize == recorder
    ) {
        return 33;
    }
    if write_line(
        runtime,
        &format!(
            "resource.core.queue resource={} holder={} waiters={}:{} acquires=1 handoffs=0",
            resource, contract, mirror, recorder
        ),
    )
    .is_err()
    {
        return 102;
    }
    match runtime.cancel_resource_claim(mirror) {
        Ok(ResourceCancelOutcome {
            resource: id,
            waiting_count: 1,
        }) if id == resource => {}
        _ => return 34,
    }
    if !matches!(
        runtime.list_resource_waiters(resource, &mut waiters),
        Ok(1) if waiters[0] as usize == recorder
    ) {
        return 35;
    }
    if write_line(
        runtime,
        &format!(
            "resource.core.cancel resource={} canceled={} waiters={} acquires=1 handoffs=0",
            resource, mirror, recorder
        ),
    )
    .is_err()
    {
        return 103;
    }
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 36,
    };
    if resource_info.arbitration != NativeResourceArbitrationPolicy::Fifo as u32
        || resource_info.governance != NativeResourceGovernanceMode::Queueing as u32
        || resource_info.holder_contract as usize != contract
        || resource_info.waiting_count != 1
        || resource_info.acquire_count != 1
        || resource_info.handoff_count != 0
    {
        return 37;
    }
    match runtime.release_claimed_resource(contract) {
        Ok(ResourceReleaseOutcome::HandedOff {
            resource: id,
            contract: handoff,
            acquire_count: 2,
            handoff_count: 1,
        }) if id == resource && handoff == recorder => {}
        _ => return 38,
    }
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 39,
    };
    if resource_info.governance != NativeResourceGovernanceMode::Queueing as u32
        || resource_info.holder_contract as usize != recorder
        || resource_info.waiting_count != 0
        || resource_info.acquire_count != 2
        || resource_info.handoff_count != 1
    {
        return 40;
    }
    if write_line(
        runtime,
        &format!(
            "resource.core.handoff resource={} from={} to={} waiters=0 acquires=2 handoffs=1",
            resource, contract, recorder
        ),
    )
    .is_err()
    {
        return 104;
    }
    let transferred = match runtime.transfer_resource(recorder, mirror) {
        Ok(id) => id,
        Err(_) => return 41,
    };
    if transferred != resource {
        return 42;
    }
    if !matches!(runtime.list_resource_waiters(resource, &mut waiters), Ok(0)) {
        return 43;
    }
    let released = match runtime.release_resource(mirror) {
        Ok(id) => id,
        Err(_) => return 44,
    };
    if released != resource {
        return 45;
    }
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 46,
    };
    if resource_info.governance != NativeResourceGovernanceMode::Queueing as u32
        || resource_info.holder_contract != 0
        || resource_info.waiting_count != 0
        || resource_info.acquire_count != 3
        || resource_info.handoff_count != 2
    {
        return 47;
    }
    if write_line(
        runtime,
        &format!(
            "resource.core.transfer resource={} from={} to={} holder=0 acquires=3 handoffs=2",
            resource, recorder, mirror
        ),
    )
    .is_err()
    {
        return 105;
    }

    match runtime.claim_resource(mirror) {
        Ok(ResourceClaimOutcome::Acquired {
            resource: id,
            acquire_count: 4,
        }) if id == resource => {}
        _ => return 48,
    }
    match runtime.claim_resource(recorder) {
        Ok(ResourceClaimOutcome::Queued {
            resource: id,
            holder_contract,
            position: 1,
        }) if id == resource && holder_contract == mirror => {}
        _ => return 49,
    }
    if runtime
        .set_contract_state(recorder, NativeContractState::Suspended)
        .is_err()
    {
        return 50;
    }
    if !matches!(runtime.list_resource_waiters(resource, &mut waiters), Ok(0)) {
        return 51;
    }
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 52,
    };
    if resource_info.holder_contract as usize != mirror
        || resource_info.waiting_count != 0
        || resource_info.acquire_count != 4
        || resource_info.handoff_count != 2
    {
        return 53;
    }
    match runtime.release_claimed_resource(mirror) {
        Ok(ResourceReleaseOutcome::Released { resource: id }) if id == resource => {}
        _ => return 54,
    }
    debug_break(0x4e47_4f53_4253_3633, resource as u64);
    if runtime.claim_resource(recorder) != Err(ngos_user_abi::Errno::Access) {
        return 55;
    }
    if runtime
        .set_contract_state(recorder, NativeContractState::Active)
        .is_err()
    {
        return 56;
    }
    match runtime.claim_resource(recorder) {
        Ok(ResourceClaimOutcome::Acquired {
            resource: id,
            acquire_count: 5,
        }) if id == resource => {}
        _ => return 57,
    }
    if runtime
        .set_contract_state(recorder, NativeContractState::Revoked)
        .is_err()
    {
        return 58;
    }
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 59,
    };
    if resource_info.holder_contract != 0
        || resource_info.waiting_count != 0
        || resource_info.acquire_count != 5
        || resource_info.handoff_count != 2
    {
        return 60;
    }
    if write_line(
        runtime,
        &format!(
            "resource.core.revoked resource={} contract={} holder=0 waiters=0 acquires=5 handoffs=2",
            resource, recorder
        ),
    )
    .is_err()
    {
        return 106;
    }
    if runtime
        .set_resource_governance_mode(resource, NativeResourceGovernanceMode::ExclusiveLease)
        .is_err()
    {
        return 61;
    }
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 62,
    };
    if resource_info.governance != NativeResourceGovernanceMode::ExclusiveLease as u32 {
        return 63;
    }
    match runtime.claim_resource(contract) {
        Ok(ResourceClaimOutcome::Acquired {
            resource: id,
            acquire_count: 6,
        }) if id == resource => {}
        _ => return 64,
    }
    if runtime.claim_resource(mirror) != Err(ngos_user_abi::Errno::Busy) {
        return 65;
    }
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 66,
    };
    if resource_info.holder_contract as usize != contract
        || resource_info.waiting_count != 0
        || resource_info.acquire_count != 6
        || resource_info.handoff_count != 2
    {
        return 67;
    }
    if write_line(
        runtime,
        &format!(
            "resource.core.exclusive resource={} holder={} refused={} reason=busy acquires=6 handoffs=2",
            resource, contract, mirror
        ),
    )
    .is_err()
    {
        return 107;
    }
    let released = match runtime.release_claimed_resource(contract) {
        Ok(ResourceReleaseOutcome::Released { resource: id }) => id,
        _ => return 68,
    };
    if released != resource {
        return 69;
    }
    let writer = match runtime.create_contract(domain, resource, NativeContractKind::Io, "writer") {
        Ok(id) => id,
        Err(_) => return 70,
    };
    if runtime
        .set_resource_contract_policy(resource, NativeResourceContractPolicy::Io)
        .is_err()
    {
        return 71;
    }
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 72,
    };
    if resource_info.contract_policy != NativeResourceContractPolicy::Io as u32
        || resource_info.governance != NativeResourceGovernanceMode::ExclusiveLease as u32
    {
        return 73;
    }
    match runtime.claim_resource(writer) {
        Ok(ResourceClaimOutcome::Acquired {
            resource: id,
            acquire_count: 7,
        }) if id == resource => {}
        _ => return 74,
    }
    if runtime.claim_resource(contract) != Err(ngos_user_abi::Errno::Access) {
        return 75;
    }
    match runtime.release_claimed_resource(writer) {
        Ok(ResourceReleaseOutcome::Released { resource: id }) if id == resource => {}
        _ => return 76,
    }
    if write_line(
        runtime,
        &format!(
            "resource.core.contract-policy resource={} policy=io acquired={} refused={} reason=access",
            resource, writer, contract
        ),
    )
    .is_err()
    {
        return 108;
    }
    debug_break(0x4e47_4f53_4253_3634, resource as u64);
    if runtime.create_contract(domain, resource, NativeContractKind::Display, "overlay")
        != Err(ngos_user_abi::Errno::Access)
    {
        return 77;
    }
    if runtime
        .set_resource_issuer_policy(resource, NativeResourceIssuerPolicy::CreatorOnly)
        .is_err()
    {
        return 78;
    }
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 79,
    };
    if resource_info.issuer_policy != NativeResourceIssuerPolicy::CreatorOnly as u32 {
        return 80;
    }
    if runtime
        .create_contract(domain, resource, NativeContractKind::Io, "writer-2")
        .is_err()
    {
        return 81;
    }
    if write_line(
        runtime,
        &format!(
            "resource.core.issuer-policy resource={} policy=creator-only allowed_domain={} outcome=ok",
            resource, domain
        ),
    )
    .is_err()
    {
        return 109;
    }
    if runtime
        .set_resource_state(resource, NativeResourceState::Suspended)
        .is_err()
    {
        return 82;
    }
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 83,
    };
    if resource_info.state != NativeResourceState::Suspended as u32
        || resource_info.holder_contract != 0
        || resource_info.waiting_count != 0
    {
        return 84;
    }
    if runtime.claim_resource(writer) != Err(ngos_user_abi::Errno::Access) {
        return 85;
    }
    if runtime.invoke_contract(contract) != Err(ngos_user_abi::Errno::Access) {
        return 86;
    }
    if runtime.create_contract(domain, resource, NativeContractKind::Io, "writer-3")
        != Err(ngos_user_abi::Errno::Access)
    {
        return 87;
    }
    if write_line(
        runtime,
        &format!(
            "resource.core.suspended resource={} claims=refused invoke=refused create-contract=refused",
            resource
        ),
    )
    .is_err()
    {
        return 110;
    }
    if runtime
        .set_resource_state(resource, NativeResourceState::Active)
        .is_err()
    {
        return 88;
    }
    let writer3 =
        match runtime.create_contract(domain, resource, NativeContractKind::Io, "writer-3") {
            Ok(id) => id,
            Err(_) => return 89,
        };
    if runtime
        .set_resource_state(resource, NativeResourceState::Retired)
        .is_err()
    {
        return 90;
    }
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => return 91,
    };
    if resource_info.state != NativeResourceState::Retired as u32
        || resource_info.holder_contract != 0
        || resource_info.waiting_count != 0
    {
        return 92;
    }
    let writer3_info = match runtime.inspect_contract(writer3) {
        Ok(info) => info,
        Err(_) => return 93,
    };
    if writer3_info.state != NativeContractState::Revoked as u32 {
        return 94;
    }
    if runtime.create_contract(domain, resource, NativeContractKind::Io, "writer-4")
        != Err(ngos_user_abi::Errno::Access)
    {
        return 95;
    }
    if write_line(
        runtime,
        &format!(
            "resource.core.retired resource={} writer={} state=retired revoked=true create-contract=refused",
            resource, writer3
        ),
    )
    .is_err()
    {
        return 111;
    }

    let mut text = [0u8; 16];
    let copied = match runtime.get_domain_name(domain, &mut text) {
        Ok(count) => count,
        Err(_) => return 96,
    };
    if &text[..copied] != b"graphics" {
        return 97;
    }

    let copied = match runtime.get_resource_name(resource, &mut text) {
        Ok(count) => count,
        Err(_) => return 98,
    };
    if &text[..copied] != b"gpu0" {
        return 99;
    }

    let copied = match runtime.get_contract_label(contract, &mut text) {
        Ok(count) => count,
        Err(_) => return 100,
    };
    if &text[..copied] != b"scanout" {
        return 101;
    }

    if write_line(
        runtime,
        &format!(
            "resource.core.final resource={} state=retired governance=exclusive-lease contract-policy=io issuer-policy=creator-only holder=0 waiters=0 acquires=7 handoffs=2",
            resource
        ),
    )
    .is_err()
    {
        return 112;
    }

    0
}

fn run_native_surface_smoke<B: SyscallBackend>(
    runtime: &Runtime<B>,
    validate_stdio: bool,
) -> ExitCode {
    let core_code = run_native_surface_core(runtime, validate_stdio);
    if core_code != 0 {
        return core_code;
    }
    debug_break(0x4e47_4f53_5653_3030, 0);
    let eventing_code = run_native_eventing_resource_smoke(runtime);
    if eventing_code != 0 {
        return eventing_code;
    }
    debug_break(0x4e47_4f53_5653_3032, 0);
    let vm_smoke_code = run_native_vm_boot_smoke(runtime);
    if vm_smoke_code != 0 {
        return vm_smoke_code;
    }
    debug_break(0x4e47_4f53_5653_3031, 0);
    debug_break(0x4e47_4f53_4753_3030, 0);
    let game_smoke_code = run_native_game_stack_smoke(runtime);
    if game_smoke_code != 0 {
        return game_smoke_code;
    }
    debug_break(0x4e47_4f53_4753_3031, 0);
    let payload = b"ngos-userland-native: native abi ok\n";
    match runtime.write(1, payload) {
        Ok(wrote) if wrote == payload.len() => 0,
        _ => 102,
    }
}

fn run_native_eventing_resource_smoke<B: SyscallBackend>(runtime: &Runtime<B>) -> ExitCode {
    let domain = match runtime.create_domain(None, "eventing-smoke") {
        Ok(id) => id,
        Err(_) => return 200,
    };
    let resource = match runtime.create_resource(domain, NativeResourceKind::Device, "queue0") {
        Ok(id) => id,
        Err(_) => return 201,
    };
    let primary =
        match runtime.create_contract(domain, resource, NativeContractKind::Display, "primary") {
            Ok(id) => id,
            Err(_) => return 202,
        };
    let mirror =
        match runtime.create_contract(domain, resource, NativeContractKind::Display, "mirror") {
            Ok(id) => id,
            Err(_) => return 203,
        };
    let queue_fd = match runtime.create_event_queue(NativeEventQueueMode::Epoll) {
        Ok(fd) => fd,
        Err(_) => return 204,
    };
    if runtime
        .fcntl(queue_fd, FcntlCmd::SetFl { nonblock: true })
        .is_err()
    {
        let _ = runtime.close(queue_fd);
        return 205;
    }
    let watch_token = ((resource as u64) << 32) | 0x515;
    if runtime
        .watch_resource_events(
            queue_fd,
            resource,
            watch_token,
            false,
            true,
            false,
            false,
            true,
            true,
            POLLPRI,
        )
        .is_err()
    {
        let _ = runtime.close(queue_fd);
        return 206;
    }

    let mut events = [NativeEventRecord {
        token: 0,
        events: 0,
        source_kind: 0,
        source_arg0: 0,
        source_arg1: 0,
        source_arg2: 0,
        detail0: 0,
        detail1: 0,
    }; 4];
    if runtime.wait_event_queue(queue_fd, &mut events) != Err(Errno::Again) {
        let _ = runtime.remove_resource_events(queue_fd, resource, watch_token);
        let _ = runtime.close(queue_fd);
        return 207;
    }

    match runtime.claim_resource(primary) {
        Ok(ResourceClaimOutcome::Acquired {
            resource: id,
            acquire_count: 1,
        }) if id == resource => {}
        _ => {
            let _ = runtime.remove_resource_events(queue_fd, resource, watch_token);
            let _ = runtime.close(queue_fd);
            return 208;
        }
    }
    match runtime.claim_resource(mirror) {
        Ok(ResourceClaimOutcome::Queued {
            resource: id,
            holder_contract,
            position: 1,
        }) if id == resource && holder_contract == primary => {}
        _ => {
            let _ = runtime.remove_resource_events(queue_fd, resource, watch_token);
            let _ = runtime.close(queue_fd);
            return 209;
        }
    }

    let queued_count = match runtime.wait_event_queue(queue_fd, &mut events) {
        Ok(count) => count,
        Err(_) => {
            let _ = runtime.remove_resource_events(queue_fd, resource, watch_token);
            let _ = runtime.close(queue_fd);
            return 210;
        }
    };
    if queued_count != 1
        || events[0].token != watch_token
        || events[0].events != POLLPRI
        || events[0].source_kind != NativeEventSourceKind::Resource as u32
        || events[0].source_arg0 as usize != resource
        || events[0].source_arg1 as usize != mirror
        || events[0].detail0 != 1
    {
        let _ = runtime.remove_resource_events(queue_fd, resource, watch_token);
        let _ = runtime.close(queue_fd);
        return 211;
    }
    if write_line(
        runtime,
        &format!(
            "resource.smoke.queued queue={} resource={} contract={} holder={}",
            queue_fd, resource, mirror, primary
        ),
    )
    .is_err()
    {
        let _ = runtime.remove_resource_events(queue_fd, resource, watch_token);
        let _ = runtime.close(queue_fd);
        return 212;
    }

    match runtime.release_claimed_resource(primary) {
        Ok(ResourceReleaseOutcome::HandedOff {
            resource: id,
            contract: handoff,
            acquire_count: 2,
            handoff_count: 1,
        }) if id == resource && handoff == mirror => {}
        _ => {
            let _ = runtime.remove_resource_events(queue_fd, resource, watch_token);
            let _ = runtime.close(queue_fd);
            return 213;
        }
    }
    let handed_off_count = match runtime.wait_event_queue(queue_fd, &mut events) {
        Ok(count) => count,
        Err(_) => {
            let _ = runtime.remove_resource_events(queue_fd, resource, watch_token);
            let _ = runtime.close(queue_fd);
            return 214;
        }
    };
    if handed_off_count != 1
        || events[0].token != watch_token
        || events[0].events != POLLPRI
        || events[0].source_kind != NativeEventSourceKind::Resource as u32
        || events[0].source_arg0 as usize != resource
        || events[0].source_arg1 as usize != mirror
        || events[0].detail0 != 4
    {
        let _ = runtime.remove_resource_events(queue_fd, resource, watch_token);
        let _ = runtime.close(queue_fd);
        return 215;
    }
    if write_line(
        runtime,
        &format!(
            "resource.smoke.handoff queue={} resource={} contract={} handoff_count=1",
            queue_fd, resource, mirror
        ),
    )
    .is_err()
    {
        let _ = runtime.remove_resource_events(queue_fd, resource, watch_token);
        let _ = runtime.close(queue_fd);
        return 216;
    }

    if runtime
        .remove_resource_events(queue_fd, resource, watch_token)
        .is_err()
    {
        let _ = runtime.close(queue_fd);
        return 217;
    }
    match runtime.release_claimed_resource(mirror) {
        Ok(ResourceReleaseOutcome::Released { resource: id }) if id == resource => {}
        _ => {
            let _ = runtime.close(queue_fd);
            return 218;
        }
    }
    if runtime.wait_event_queue(queue_fd, &mut events) != Err(Errno::Again) {
        let _ = runtime.close(queue_fd);
        return 219;
    }
    let resource_info = match runtime.inspect_resource(resource) {
        Ok(info) => info,
        Err(_) => {
            let _ = runtime.close(queue_fd);
            return 220;
        }
    };
    if resource_info.holder_contract != 0
        || resource_info.waiting_count != 0
        || resource_info.acquire_count != 2
        || resource_info.handoff_count != 1
    {
        let _ = runtime.close(queue_fd);
        return 221;
    }
    if write_line(
        runtime,
        &format!(
            "resource.smoke.final resource={} holder=0 waiters=0 acquires=2 handoffs=1",
            resource
        ),
    )
    .is_err()
    {
        let _ = runtime.close(queue_fd);
        return 222;
    }
    if runtime.close(queue_fd).is_err() {
        return 223;
    }
    0
}

fn run_native_vm_boot_smoke<B: SyscallBackend>(runtime: &Runtime<B>) -> ExitCode {
    let pid = 1u64;
    if let Err(code) =
        ensure_vm_smoke_backing_file(runtime, "/lib/libvm-smoke.so", 0x3000, 0x5a, 160, 163, 164)
    {
        return code;
    }
    let mapped = match runtime.map_anonymous_memory(pid, 0x2000, true, true, false, "boot-vm-smoke")
    {
        Ok(start) => start,
        Err(_) => return 170,
    };
    if write_line(
        runtime,
        &format!("vm.smoke.map pid={pid} start={mapped} len=8192"),
    )
    .is_err()
    {
        return 171;
    }

    if runtime.store_memory_word(pid, mapped, 7).is_err() {
        return 172;
    }
    if runtime
        .protect_memory_range(pid, mapped + 0x1000, 0x1000, true, false, false)
        .is_err()
    {
        return 173;
    }
    if runtime.load_memory_word(pid, mapped + 0x1000).is_err() {
        return 174;
    }
    if runtime.store_memory_word(pid, mapped + 0x1000, 9) != Err(Errno::Fault) {
        return 175;
    }
    if write_line(
        runtime,
        &format!(
            "vm.smoke.protect pid={pid} addr={} perms=r--",
            mapped + 0x1000
        ),
    )
    .is_err()
    {
        return 176;
    }

    let maps = match path_contains_all_markers(
        runtime,
        "/proc/1/maps",
        &["[anon:boot-vm-smoke]", "r--p 00000000 [anon:boot-vm-smoke]"],
    ) {
        Ok(value) => value,
        Err(code) => return code,
    };
    if !maps {
        return 177;
    }

    let heap_grow = match runtime.set_process_break(pid, 0x4000_7000) {
        Ok(end) => end,
        Err(_) => return 199,
    };
    if heap_grow != 0x4000_7000 {
        return 200;
    }
    let heap_shrink = match runtime.set_process_break(pid, 0x4000_3000) {
        Ok(end) => end,
        Err(_) => return 201,
    };
    if heap_shrink != 0x4000_3000 {
        return 202;
    }
    let heap_vmdecisions =
        match path_contains_all_markers(runtime, "/proc/1/vmdecisions", &["agent=brk"]) {
            Ok(value) => value,
            Err(code) => return code,
        };
    if !heap_vmdecisions {
        return 203;
    }

    if runtime.sync_memory_range(pid, mapped, 0x1000).is_err() {
        return 179;
    }
    if runtime
        .unmap_memory_range(pid, mapped + 0x1000, 0x1000)
        .is_err()
    {
        return 181;
    }
    if runtime.load_memory_word(pid, mapped + 0x1000) != Err(Errno::Fault) {
        return 182;
    }
    if write_line(
        runtime,
        &format!(
            "vm.smoke.unmap pid={pid} addr={} len=4096 outcome=ok",
            mapped + 0x1000
        ),
    )
    .is_err()
    {
        return 184;
    }
    let vmdecisions = match path_contains_all_markers(
        runtime,
        "/proc/1/vmdecisions",
        &["agent=map", "agent=protect", "agent=unmap"],
    ) {
        Ok(value) => value,
        Err(code) => return code,
    };
    if !vmdecisions {
        return 178;
    }
    let vmepisodes = match path_contains_all_markers(
        runtime,
        "/proc/1/vmepisodes",
        &[
            "kind=heap",
            "grew=yes",
            "shrank=yes",
            "kind=region",
            "protected=yes",
            "unmapped=yes",
        ],
    ) {
        Ok(value) => value,
        Err(code) => return code,
    };
    if !vmepisodes {
        return 204;
    }
    if write_line(
        runtime,
        &format!("vm.smoke.heap pid={pid} kind=heap grew=yes shrank=yes"),
    )
    .is_err()
    {
        return 205;
    }
    if write_line(
        runtime,
        &format!("vm.smoke.region pid={pid} kind=region protected=yes unmapped=yes"),
    )
    .is_err()
    {
        return 198;
    }

    let cow_child = match runtime.spawn_process_copy_vm("c", "/c", pid) {
        Ok(child) => child,
        Err(_) => return 192,
    };
    if runtime.store_memory_word(cow_child, mapped, 21).is_err() {
        return 193;
    }
    let child_vmobjects =
        match path_contains_all_markers(runtime, "/proc/2/vmobjects", &["[cow]", "depth=1"]) {
            Ok(value) => value,
            Err(code) => return code,
        };
    if !child_vmobjects {
        return 194;
    }
    let child_vmdecisions = match path_contains_all_markers(
        runtime,
        "/proc/2/vmdecisions",
        &["agent=shadow-reuse", "agent=cow-populate"],
    ) {
        Ok(value) => value,
        Err(code) => return code,
    };
    if !child_vmdecisions {
        return 195;
    }
    if write_line(
        runtime,
        &format!(
            "vm.smoke.cow.observe pid={cow_child} source={pid} object=[cow] depth=1 kind=fault cow=yes"
        ),
    )
    .is_err()
    {
        return 197;
    }
    if write_line(
        runtime,
        &format!("vm.smoke.cow pid={cow_child} source={pid} addr={mapped} outcome=ok"),
    )
    .is_err()
    {
        return 196;
    }

    let file_mapped = match runtime.map_file_memory(
        pid,
        "/lib/libvm-smoke.so",
        0x2000,
        0x1000,
        true,
        false,
        true,
        true,
    ) {
        Ok(start) => start,
        Err(_) => return 185,
    };
    if runtime.store_memory_word(pid, file_mapped, 13) != Err(Errno::Fault) {
        return 186;
    }
    if runtime
        .protect_memory_range(pid, file_mapped, 0x2000, true, true, false)
        .is_err()
    {
        return 187;
    }
    if runtime.store_memory_word(pid, file_mapped, 13).is_err() {
        return 188;
    }
    if runtime.sync_memory_range(pid, file_mapped, 0x2000).is_err() {
        return 189;
    }
    let file_maps = match path_contains_all_markers(
        runtime,
        "/proc/1/maps",
        &["rw-p 00001000 /lib/libvm-smoke.so"],
    ) {
        Ok(value) => value,
        Err(code) => return code,
    };
    if !file_maps {
        return 183;
    }
    let file_vmdecisions = match path_contains_all_markers(
        runtime,
        "/proc/1/vmdecisions",
        &[
            "agent=map-file",
            "agent=protect",
            "agent=sync",
            "/lib/libvm-smoke.so",
        ],
    ) {
        Ok(value) => value,
        Err(code) => return code,
    };
    if !file_vmdecisions {
        return 191;
    }
    if runtime
        .unmap_memory_range(pid, file_mapped, 0x2000)
        .is_err()
    {
        return 192;
    }

    if let Err(code) = run_vm_stress_hardening(runtime, pid) {
        return code;
    }
    if let Err(code) = run_vm_pressure_hardening(runtime, pid) {
        return code;
    }
    if let Err(code) = run_vm_global_pressure_hardening(runtime, pid) {
        return code;
    }

    if write_line(
        runtime,
        &format!(
            "vm.smoke.production pid={pid} stress=yes pressure=yes global-pressure=yes workloads=anon,cow,file,heap,region outcome=ok"
        ),
    )
    .is_err()
    {
        return 246;
    }

    0
}

fn run_native_wasm_boot_smoke<B: SyscallBackend>(runtime: &Runtime<B>) -> ExitCode {
    let pid = 1u64;
    let observer = "semantic-observer";
    if write_line(
        runtime,
        &format!("wasm.smoke.start component={observer} pid={pid} artifact=boot-proof"),
    )
    .is_err()
    {
        return 260;
    }

    match execute_wasm_component(
        runtime,
        WASM_BOOT_PROOF_COMPONENT,
        pid,
        &[WasmCapability::ObserveProcessCapabilityCount],
    ) {
        Err(WasmExecutionError::MissingCapability { capability, .. })
            if capability == WasmCapability::ObserveSystemProcessCount => {}
        Ok(_) => return 261,
        Err(_) => return 262,
    }
    if write_line(
        runtime,
        "wasm.smoke.refusal component=semantic-observer missing=observe-system-process-count outcome=expected",
    )
    .is_err()
    {
        return 263;
    }

    let report = match execute_wasm_component(
        runtime,
        WASM_BOOT_PROOF_COMPONENT,
        pid,
        &[
            WasmCapability::ObserveProcessCapabilityCount,
            WasmCapability::ObserveSystemProcessCount,
        ],
    ) {
        Ok(report) => report,
        Err(_) => return 264,
    };
    if write_line(
        runtime,
        "wasm.smoke.grants component=semantic-observer grants=observe-process-capability-count,observe-system-process-count",
    )
    .is_err()
    {
        return 265;
    }
    if write_line(
        runtime,
        &format!(
            "wasm.smoke.observe component={observer} pid={} capabilities={} processes={}",
            report.observation.pid,
            report.observation.process_capability_count,
            report.observation.process_count
        ),
    )
    .is_err()
    {
        return 266;
    }
    if write_line(
        runtime,
        &format!(
            "wasm.smoke.recovery component={observer} refusal=observe-system-process-count recovered=yes verdict={}",
            report.verdict.marker_name()
        ),
    )
    .is_err()
    {
        return 267;
    }
    if write_line(
        runtime,
        &format!(
            "wasm.smoke.result component={observer} verdict={} outcome=ok",
            report.verdict.marker_name()
        ),
    )
    .is_err()
    {
        return 269;
    }
    let identity = "process-identity";
    if write_line(
        runtime,
        &format!("wasm.smoke.start component={identity} pid={pid} artifact=process-identity"),
    )
    .is_err()
    {
        return 271;
    }
    match execute_wasm_component(
        runtime,
        WASM_PROCESS_IDENTITY_COMPONENT,
        pid,
        &[WasmCapability::ObserveProcessStatusBytes],
    ) {
        Err(WasmExecutionError::MissingCapability { capability, .. })
            if capability == WasmCapability::ObserveProcessCwdRoot => {}
        Ok(_) => return 272,
        Err(_) => return 273,
    }
    if write_line(
        runtime,
        "wasm.smoke.refusal component=process-identity missing=observe-process-cwd-root outcome=expected",
    )
    .is_err()
    {
        return 274;
    }
    let identity_report = match execute_wasm_component(
        runtime,
        WASM_PROCESS_IDENTITY_COMPONENT,
        pid,
        &[
            WasmCapability::ObserveProcessStatusBytes,
            WasmCapability::ObserveProcessCwdRoot,
        ],
    ) {
        Ok(report) => report,
        Err(_) => return 275,
    };
    if write_line(
        runtime,
        "wasm.smoke.grants component=process-identity grants=observe-process-status-bytes,observe-process-cwd-root",
    )
    .is_err()
    {
        return 276;
    }
    if write_line(
        runtime,
        &format!(
            "wasm.smoke.observe component={identity} pid={} status-bytes={} cwd-root={}",
            identity_report.observation.pid,
            identity_report.observation.process_status_bytes,
            if identity_report.observation.process_cwd_root {
                "yes"
            } else {
                "no"
            }
        ),
    )
    .is_err()
    {
        return 277;
    }
    if write_line(
        runtime,
        &format!(
            "wasm.smoke.recovery component={identity} refusal=observe-process-cwd-root recovered=yes verdict={}",
            identity_report.verdict.marker_name()
        ),
    )
    .is_err()
    {
        return 278;
    }
    if write_line(
        runtime,
        &format!(
            "wasm.smoke.result component={identity} verdict={} outcome=ok",
            identity_report.verdict.marker_name()
        ),
    )
    .is_err()
    {
        return 279;
    }
    if write_line(runtime, "wasm-smoke-ok").is_err() {
        return 270;
    }
    0
}

fn run_native_vfs_boot_smoke<B: SyscallBackend>(runtime: &Runtime<B>) -> ExitCode {
    let pid = 1u64;
    let root = "/vfs";
    let bin_dir = "/vfs/bin";
    let app_path = "/vfs/bin/app";
    let link_path = "/vfs/link";
    if write_line(runtime, "vfs.smoke.step=mkdir-root").is_err() {
        return 249;
    }
    if runtime.mkdir_path(root).is_err() && runtime.stat_path(root).is_err() {
        return 250;
    }
    if write_line(runtime, "vfs.smoke.step=mkdir-bin").is_err() {
        return 249;
    }
    if runtime.mkdir_path(bin_dir).is_err() && runtime.stat_path(bin_dir).is_err() {
        return 251;
    }
    if write_line(runtime, "vfs.smoke.step=mkfile-app").is_err() {
        return 249;
    }
    if runtime.mkfile_path(app_path).is_err() && runtime.stat_path(app_path).is_err() {
        return 252;
    }
    if write_line(runtime, "vfs.smoke.step=symlink-link").is_err() {
        return 249;
    }
    if let Err(errno) = runtime.symlink_path(link_path, app_path) {
        let _ = write_line(
            runtime,
            &format!("vfs.smoke.fail step=symlink errno={}", errno as u32),
        );
        return 253;
    }

    let mount = match runtime.statfs_path(root) {
        Ok(status) => status,
        Err(_) => return 254,
    };
    let kind = match runtime.stat_path(app_path) {
        Ok(status) => status,
        Err(_) => return 255,
    };
    let link = match runtime.lstat_path(link_path) {
        Ok(status) => status,
        Err(_) => return 256,
    };
    let mut target = [0u8; 64];
    let target_len = match runtime.readlink_path(link_path, &mut target) {
        Ok(count) => count,
        Err(_) => return 257,
    };
    if &target[..target_len] != app_path.as_bytes() {
        return 258;
    }

    let fd = match runtime.open_path(link_path) {
        Ok(fd) => fd,
        Err(_) => return 259,
    };
    if runtime.close(fd).is_err() {
        return 260;
    }
    if runtime.rename_path(app_path, "/vfs/bin/app2").is_err() {
        return 261;
    }
    if runtime.stat_path("/vfs/bin/app2").is_err() {
        return 262;
    }
    if runtime.open_path("/vfs/bin/app2").is_err() {
        return 263;
    }
    if runtime.rename_path(bin_dir, "/vfs/bin/subdir").is_ok() {
        return 264;
    }
    if runtime.unlink_path(link_path).is_err() {
        return 265;
    }
    if runtime.readlink_path(link_path, &mut target).is_ok() {
        return 266;
    }

    if write_line(
        runtime,
        &format!(
            "vfs.smoke.mount pid={pid} path={root} mounts={} nodes={} read_only={} outcome=ok",
            mount.mount_count, mount.node_count, mount.read_only
        ),
    )
    .is_err()
    {
        return 267;
    }
    if write_line(
        runtime,
        &format!(
            "vfs.smoke.create pid={pid} path={app_path} kind={} inode={} outcome=ok",
            kind.kind, kind.inode
        ),
    )
    .is_err()
    {
        return 268;
    }
    if write_line(
        runtime,
        &format!(
            "vfs.smoke.symlink pid={pid} path={link_path} target={app_path} kind={} inode={} outcome=ok",
            link.kind, link.inode
        ),
    )
    .is_err()
    {
        return 269;
    }
    if write_line(
        runtime,
        &format!(
            "vfs.smoke.rename pid={pid} from={app_path} to=/vfs/bin/app2 refusal=invalid-subtree yes outcome=ok"
        ),
    )
    .is_err()
    {
        return 270;
    }
    if write_line(
        runtime,
        &format!("vfs.smoke.unlink pid={pid} path={link_path} after-unlink=missing outcome=ok"),
    )
    .is_err()
    {
        return 271;
    }
    if write_line(
        runtime,
        &format!(
            "vfs.smoke.coherence pid={pid} descriptor=open-path-open readlink=stable statfs=ok outcome=ok"
        ),
    )
    .is_err()
    {
        return 272;
    }
    if write_line(runtime, "vfs-smoke-ok").is_err() {
        return 273;
    }
    0
}

#[cfg(target_os = "none")]
fn debug_break(marker: u64, value: u64) {
    unsafe {
        core::arch::asm!(
            "mov rax, {marker}",
            "mov rdi, {value}",
            "int3",
            marker = in(reg) marker,
            value = in(reg) value,
            options(nostack)
        );
    }
}

#[cfg(not(target_os = "none"))]
fn debug_break(_marker: u64, _value: u64) {}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::{Cell, RefCell};
    use ngos_user_abi::{
        AT_ENTRY, AT_PAGESZ, BootSessionStage, BootSessionStatus, Errno, NativeContractKind,
        NativeContractRecord, NativeContractState, NativeDeviceRecord, NativeDomainRecord,
        NativeDriverRecord, NativeEventRecord, NativeEventSourceKind, NativeFileStatusRecord,
        NativeFileSystemStatusRecord, NativeObjectKind, NativeProcessRecord,
        NativeResourceArbitrationPolicy, NativeResourceCancelRecord, NativeResourceClaimRecord,
        NativeResourceContractPolicy, NativeResourceEventWatchConfig, NativeResourceGovernanceMode,
        NativeResourceIssuerPolicy, NativeResourceKind, NativeResourceRecord,
        NativeResourceReleaseRecord, NativeResourceState, NativeSchedulerClass,
        NativeSystemSnapshotRecord, SYS_ACQUIRE_RESOURCE, SYS_BLOCKED_PENDING_SIGNALS,
        SYS_BOOT_REPORT, SYS_CANCEL_RESOURCE_CLAIM, SYS_CHDIR_PATH, SYS_CLAIM_RESOURCE, SYS_CLOSE,
        SYS_CONFIGURE_NETIF_ADMIN, SYS_CREATE_CONTRACT, SYS_CREATE_DOMAIN, SYS_CREATE_EVENT_QUEUE,
        SYS_CREATE_RESOURCE, SYS_DUP, SYS_FCNTL, SYS_GET_CONTRACT_LABEL, SYS_GET_DOMAIN_NAME,
        SYS_GET_PROCESS_CWD, SYS_GET_PROCESS_IMAGE_PATH, SYS_GET_PROCESS_NAME,
        SYS_GET_RESOURCE_NAME, SYS_INSPECT_CONTRACT, SYS_INSPECT_DEVICE, SYS_INSPECT_DOMAIN,
        SYS_INSPECT_DRIVER, SYS_INSPECT_PROCESS, SYS_INSPECT_RESOURCE, SYS_INSPECT_SYSTEM_SNAPSHOT,
        SYS_INVOKE_CONTRACT, SYS_LIST_CONTRACTS, SYS_LIST_DOMAINS, SYS_LIST_PATH,
        SYS_LIST_PROCESSES, SYS_LIST_RESOURCE_WAITERS, SYS_LIST_RESOURCES, SYS_LOAD_MEMORY_WORD,
        SYS_LSTAT_PATH, SYS_MAP_ANONYMOUS_MEMORY, SYS_MAP_FILE_MEMORY, SYS_MKCHAN_PATH,
        SYS_MKDIR_PATH, SYS_MKFILE_PATH, SYS_OPEN_PATH, SYS_PAUSE_PROCESS, SYS_PENDING_SIGNALS,
        SYS_POLL, SYS_PROTECT_MEMORY_RANGE, SYS_READ, SYS_READ_PROCFS, SYS_READLINK_PATH,
        SYS_REAP_PROCESS, SYS_RECLAIM_MEMORY_PRESSURE, SYS_RECLAIM_MEMORY_PRESSURE_GLOBAL,
        SYS_RELEASE_CLAIMED_RESOURCE, SYS_RELEASE_RESOURCE, SYS_REMOVE_GRAPHICS_EVENTS,
        SYS_REMOVE_NET_EVENTS, SYS_REMOVE_PROCESS_EVENTS, SYS_REMOVE_RESOURCE_EVENTS,
        SYS_RENAME_PATH, SYS_RENICE_PROCESS, SYS_RESUME_PROCESS, SYS_SEND_SIGNAL,
        SYS_SET_CONTRACT_STATE, SYS_SET_PROCESS_ARGS, SYS_SET_PROCESS_BREAK, SYS_SET_PROCESS_CWD,
        SYS_SET_PROCESS_ENV, SYS_SET_RESOURCE_CONTRACT_POLICY, SYS_SET_RESOURCE_GOVERNANCE,
        SYS_SET_RESOURCE_ISSUER_POLICY, SYS_SET_RESOURCE_POLICY, SYS_SET_RESOURCE_STATE,
        SYS_SPAWN_CONFIGURED_PROCESS, SYS_SPAWN_PATH_PROCESS, SYS_SPAWN_PROCESS_COPY_VM,
        SYS_STAT_PATH, SYS_STATFS_PATH, SYS_STORE_MEMORY_WORD, SYS_SYMLINK_PATH,
        SYS_SYNC_MEMORY_RANGE, SYS_TRANSFER_RESOURCE, SYS_UNLINK_PATH, SYS_UNMAP_MEMORY_RANGE,
        SYS_WAIT_EVENT_QUEUE, SYS_WATCH_GRAPHICS_EVENTS, SYS_WATCH_NET_EVENTS,
        SYS_WATCH_PROCESS_EVENTS, SYS_WATCH_RESOURCE_EVENTS, SYS_WRITE, SYS_WRITEV, SyscallFrame,
        SyscallReturn, UserIoVec,
    };
    use ngos_user_runtime::Runtime as UserRuntime;

    #[derive(Clone, Debug)]
    struct RecordedProcessBootstrap {
        pid: u64,
        name: String,
        image_path: String,
        cwd: String,
        argv: Vec<String>,
        envp: Vec<String>,
    }

    #[derive(Clone, Debug)]
    struct VmMappingRecord {
        pid: u64,
        start: u64,
        len: u64,
        readable: bool,
        writable: bool,
        executable: bool,
        label: String,
        file_path: Option<String>,
        private: bool,
        cow: bool,
        present: bool,
        reclaimed: bool,
        words: Vec<(u64, u32)>,
    }

    struct RecordingBackend {
        frames: RefCell<Vec<SyscallFrame>>,
        stdin: RefCell<Vec<u8>>,
        stdin_offset: Cell<usize>,
        stdout: RefCell<Vec<u8>>,
        next_fd: Cell<usize>,
        next_pid: Cell<u64>,
        process_bootstraps: RefCell<Vec<RecordedProcessBootstrap>>,
        open_files: RefCell<Vec<(usize, String)>>,
        read_offsets: RefCell<Vec<(usize, usize)>>,
        created_paths: RefCell<Vec<(String, NativeObjectKind)>>,
        symlink_targets: RefCell<Vec<(String, String)>>,
        channel_messages: RefCell<Vec<(String, Vec<Vec<u8>>)>>,
        file_contents: RefCell<Vec<(String, Vec<u8>)>>,
        vm_mappings: RefCell<Vec<VmMappingRecord>>,
        vm_decisions: RefCell<Vec<(u64, String)>>,
        vm_episodes: RefCell<Vec<(u64, String)>>,
        next_vm_addr: Cell<u64>,
        event_queue_pending: RefCell<Vec<(usize, Vec<NativeEventRecord>)>>,
        resource_event_watches: RefCell<Vec<(usize, u64, NativeResourceEventWatchConfig)>>,
        event_queue_nonblock: RefCell<Vec<(usize, bool)>>,
        resource_event_queues: RefCell<Vec<usize>>,
    }

    impl Default for RecordingBackend {
        fn default() -> Self {
            Self {
                frames: RefCell::new(Vec::new()),
                stdin: RefCell::new(Vec::new()),
                stdin_offset: Cell::new(0),
                stdout: RefCell::new(Vec::new()),
                next_fd: Cell::new(7),
                next_pid: Cell::new(77),
                process_bootstraps: RefCell::new(Vec::new()),
                open_files: RefCell::new(Vec::new()),
                read_offsets: RefCell::new(Vec::new()),
                created_paths: RefCell::new(Vec::new()),
                symlink_targets: RefCell::new(Vec::new()),
                channel_messages: RefCell::new(Vec::new()),
                file_contents: RefCell::new(vec![
                    (String::from("/motd"), b"ngos host motd\n".to_vec()),
                    (String::from("/etc/motd"), b"ngos host motd\n".to_vec()),
                    (
                        String::from("/proc/1/status"),
                        b"Name:\tngos-userland-native\nState:\tRunning\nPid:\t1\nCwd:\t/\n"
                            .to_vec(),
                    ),
                ]),
                vm_mappings: RefCell::new(Vec::new()),
                vm_decisions: RefCell::new(Vec::new()),
                vm_episodes: RefCell::new(Vec::new()),
                next_vm_addr: Cell::new(0x1000_0000),
                event_queue_pending: RefCell::new(Vec::new()),
                resource_event_watches: RefCell::new(Vec::new()),
                event_queue_nonblock: RefCell::new(Vec::new()),
                resource_event_queues: RefCell::new(Vec::new()),
            }
        }
    }

    impl RecordingBackend {
        fn with_stdin(input: &[u8]) -> Self {
            Self {
                frames: RefCell::new(Vec::new()),
                stdin: RefCell::new(input.to_vec()),
                stdin_offset: Cell::new(0),
                stdout: RefCell::new(Vec::new()),
                next_fd: Cell::new(7),
                next_pid: Cell::new(77),
                process_bootstraps: RefCell::new(Vec::new()),
                open_files: RefCell::new(Vec::new()),
                read_offsets: RefCell::new(Vec::new()),
                created_paths: RefCell::new(Vec::new()),
                symlink_targets: RefCell::new(Vec::new()),
                channel_messages: RefCell::new(Vec::new()),
                file_contents: RefCell::new(vec![
                    (String::from("/motd"), b"ngos host motd\n".to_vec()),
                    (String::from("/etc/motd"), b"ngos host motd\n".to_vec()),
                    (
                        String::from("/proc/1/status"),
                        b"Name:\tngos-userland-native\nState:\tRunning\nPid:\t1\nCwd:\t/\n"
                            .to_vec(),
                    ),
                ]),
                vm_mappings: RefCell::new(Vec::new()),
                vm_decisions: RefCell::new(Vec::new()),
                vm_episodes: RefCell::new(Vec::new()),
                next_vm_addr: Cell::new(0x1000_0000),
                event_queue_pending: RefCell::new(Vec::new()),
                resource_event_watches: RefCell::new(Vec::new()),
                event_queue_nonblock: RefCell::new(Vec::new()),
                resource_event_queues: RefCell::new(Vec::new()),
            }
        }

        fn opened_path(&self, fd: usize) -> Option<String> {
            self.open_files
                .borrow()
                .iter()
                .rev()
                .find(|(open_fd, _)| *open_fd == fd)
                .map(|(_, path)| path.clone())
        }

        fn file_content(&self, path: &str) -> Vec<u8> {
            if let Some(payload) = self.recorded_procfs_payload(path) {
                return payload;
            }
            match path {
                "/proc/1/cwd" => return b"/".to_vec(),
                "/proc/1/exe" => return b"/bin/ngos-userland-native".to_vec(),
                "/proc/1/fd" => {
                    return b"0 [stdio:stdin]\n1 [stdio:stdout]\n2 [stdio:stderr]\n".to_vec()
                }
                "/proc/1/fdinfo/0" => {
                    return b"fd:\t0\npath:\t[stdio:stdin]\nkind:\tFile\npos:\t0\nflags:\tcloexec=false nonblock=false\nrights:\t0x3\n"
                        .to_vec()
                }
                "/proc/1/maps" => return self.render_proc_maps(1),
                "/proc/1/vmdecisions" => return self.render_vm_decisions(1),
                "/proc/1/vmobjects" => return self.render_vm_objects(1),
                "/proc/1/vmepisodes" => return self.render_vm_episodes(1),
                "/proc/2/vmdecisions" => return self.render_vm_decisions(2),
                "/proc/2/vmobjects" => return self.render_vm_objects(2),
                "/proc/2/vmepisodes" => return self.render_vm_episodes(2),
                "/proc/3/vmdecisions" => return self.render_vm_decisions(3),
                "/proc/3/vmobjects" => return self.render_vm_objects(3),
                "/proc/3/vmepisodes" => return self.render_vm_episodes(3),
                "/proc/1/cmdline" => return b"ngos-userland-native\0".to_vec(),
                _ => {}
            }
            self.file_contents
                .borrow()
                .iter()
                .find(|(candidate, _)| candidate == path)
                .map(|(_, bytes)| bytes.clone())
                .unwrap_or_default()
        }

        fn created_kind(&self, path: &str) -> Option<NativeObjectKind> {
            self.created_paths
                .borrow()
                .iter()
                .find(|(candidate, _)| candidate == path)
                .map(|(_, kind)| *kind)
        }

        fn symlink_target(&self, path: &str) -> Option<String> {
            self.symlink_targets
                .borrow()
                .iter()
                .find(|(candidate, _)| candidate == path)
                .map(|(_, target)| target.clone())
        }

        fn path_exists(&self, path: &str) -> bool {
            self.created_kind(path).is_some()
                || self
                    .file_contents
                    .borrow()
                    .iter()
                    .any(|(candidate, _)| candidate == path)
                || self.symlink_target(path).is_some()
                || path.starts_with("/proc/")
                || path.starts_with("/dev/")
                || path.starts_with("/drv/")
        }

        fn record_created_path(&self, path: &str, kind: NativeObjectKind) {
            let mut created = self.created_paths.borrow_mut();
            if let Some((_, existing_kind)) =
                created.iter_mut().find(|(candidate, _)| candidate == path)
            {
                *existing_kind = kind;
            } else {
                created.push((path.to_string(), kind));
            }
        }

        fn record_symlink_path(&self, path: &str, target: &str) {
            self.record_created_path(path, NativeObjectKind::Symlink);
            let mut symlinks = self.symlink_targets.borrow_mut();
            if let Some((_, existing_target)) =
                symlinks.iter_mut().find(|(candidate, _)| candidate == path)
            {
                *existing_target = target.to_string();
            } else {
                symlinks.push((path.to_string(), target.to_string()));
            }
        }

        fn rewrite_path_prefix(path: &str, from: &str, to: &str) -> String {
            if path == from {
                return to.to_string();
            }
            let prefix = format!("{from}/");
            if let Some(rest) = path.strip_prefix(&prefix) {
                return format!("{to}/{rest}");
            }
            path.to_string()
        }

        fn push_channel_message(&self, path: &str, bytes: &[u8]) {
            let mut channels = self.channel_messages.borrow_mut();
            if let Some((_, queue)) = channels.iter_mut().find(|(candidate, _)| candidate == path) {
                queue.push(bytes.to_vec());
            } else {
                channels.push((path.to_string(), vec![bytes.to_vec()]));
            }
        }

        fn pop_channel_message(&self, path: &str) -> Option<Vec<u8>> {
            let mut channels = self.channel_messages.borrow_mut();
            let (_, queue) = channels
                .iter_mut()
                .find(|(candidate, _)| candidate == path)?;
            if queue.is_empty() {
                None
            } else {
                Some(queue.remove(0))
            }
        }

        fn write_file_content_at(&self, path: &str, offset: usize, bytes: &[u8]) {
            let mut files = self.file_contents.borrow_mut();
            let content = if let Some((_, content)) =
                files.iter_mut().find(|(candidate, _)| candidate == path)
            {
                content
            } else {
                files.push((path.to_string(), Vec::new()));
                &mut files.last_mut().expect("just pushed").1
            };
            if content.len() < offset {
                content.resize(offset, 0);
            }
            let required_len = offset.saturating_add(bytes.len());
            if content.len() < required_len {
                content.resize(required_len, 0);
            }
            content[offset..required_len].copy_from_slice(bytes);
        }

        fn read_offset(&self, fd: usize) -> usize {
            self.read_offsets
                .borrow()
                .iter()
                .find(|(open_fd, _)| *open_fd == fd)
                .map(|(_, offset)| *offset)
                .unwrap_or(0)
        }

        fn service_block_request(&self, path: &str, bytes: &[u8]) -> bool {
            if path != "/dev/storage0" {
                return false;
            }
            let Some(request) = try_decode_block_request(bytes) else {
                return false;
            };
            let header = format!(
                "request:1 kind={} device={} opcode={}\n",
                if request.op == NATIVE_BLOCK_IO_OP_READ {
                    "Read"
                } else {
                    "Write"
                },
                "/dev/storage0",
                request.op
            );
            let mut driver_payload = header.into_bytes();
            driver_payload.extend_from_slice(encode_block_request_bytes(&request));
            let block_payload = if request.op == NATIVE_BLOCK_IO_OP_READ {
                let total_len = request.sector_count as usize * request.block_size as usize;
                let mut sector = vec![0u8; total_len];
                if total_len >= 3 {
                    sector[0] = 0xeb;
                    sector[1] = 0x58;
                    sector[2] = 0x90;
                }
                if total_len >= 11 {
                    sector[3..11].copy_from_slice(b"MSWIN4.1");
                }
                sector
            } else {
                Vec::new()
            };
            let mut files = self.file_contents.borrow_mut();
            files.retain(|(candidate, _)| {
                candidate != "/drv/storage0" && candidate != "/dev/storage0"
            });
            files.push((String::from("/drv/storage0"), driver_payload));
            files.push((String::from("/dev/storage0"), block_payload));
            true
        }

        fn service_block_completion(&self, path: &str, bytes: &[u8]) -> bool {
            if path != "/drv/storage0" {
                return false;
            }
            let mut files = self.file_contents.borrow_mut();
            files.retain(|(candidate, _)| candidate != "/dev/storage0");
            files.push((String::from("/dev/storage0"), bytes.to_vec()));
            true
        }

        fn rename_path(&self, from: &str, to: &str) -> Result<(), Errno> {
            if from == to {
                return Err(Errno::Inval);
            }
            if to.starts_with(&(from.to_string() + "/")) {
                return Err(Errno::Inval);
            }
            if !self.path_exists(from) {
                return Err(Errno::NoEnt);
            }
            if self.path_exists(to) {
                return Err(Errno::Busy);
            }

            let mut created = self.created_paths.borrow_mut();
            for entry in created.iter_mut() {
                entry.0 = Self::rewrite_path_prefix(&entry.0, from, to);
            }
            created.sort_by(|a, b| a.0.cmp(&b.0));

            let mut symlinks = self.symlink_targets.borrow_mut();
            for entry in symlinks.iter_mut() {
                entry.0 = Self::rewrite_path_prefix(&entry.0, from, to);
            }
            symlinks.sort_by(|a, b| a.0.cmp(&b.0));

            let mut files = self.file_contents.borrow_mut();
            for entry in files.iter_mut() {
                entry.0 = Self::rewrite_path_prefix(&entry.0, from, to);
            }
            files.sort_by(|a, b| a.0.cmp(&b.0));
            Ok(())
        }

        fn unlink_path(&self, path: &str) -> Result<(), Errno> {
            if !self.path_exists(path) {
                return Err(Errno::NoEnt);
            }
            self.created_paths
                .borrow_mut()
                .retain(|(candidate, _)| candidate != path);
            self.symlink_targets
                .borrow_mut()
                .retain(|(candidate, _)| candidate != path);
            self.file_contents
                .borrow_mut()
                .retain(|(candidate, _)| candidate != path);
            Ok(())
        }

        fn set_read_offset(&self, fd: usize, offset: usize) {
            let mut offsets = self.read_offsets.borrow_mut();
            if let Some((_, current)) = offsets.iter_mut().find(|(open_fd, _)| *open_fd == fd) {
                *current = offset;
            } else {
                offsets.push((fd, offset));
            }
        }

        fn record_spawned_process(&self, pid: u64, name: &str, path: &str) {
            let mut bootstraps = self.process_bootstraps.borrow_mut();
            if let Some(existing) = bootstraps.iter_mut().find(|entry| entry.pid == pid) {
                existing.name = name.to_string();
                existing.image_path = path.to_string();
                if existing.argv.is_empty() {
                    existing.argv.push(name.to_string());
                }
                return;
            }
            bootstraps.push(RecordedProcessBootstrap {
                pid,
                name: name.to_string(),
                image_path: path.to_string(),
                cwd: String::from("/"),
                argv: vec![name.to_string()],
                envp: Vec::new(),
            });
        }

        fn with_recorded_process_mut<F>(&self, pid: u64, update: F) -> Result<(), Errno>
        where
            F: FnOnce(&mut RecordedProcessBootstrap),
        {
            let mut bootstraps = self.process_bootstraps.borrow_mut();
            let Some(process) = bootstraps.iter_mut().find(|entry| entry.pid == pid) else {
                return Err(Errno::Srch);
            };
            update(process);
            Ok(())
        }

        fn recorded_process(&self, pid: u64) -> Option<RecordedProcessBootstrap> {
            self.process_bootstraps
                .borrow()
                .iter()
                .find(|entry| entry.pid == pid)
                .cloned()
        }

        fn decode_string_table(
            &self,
            ptr: usize,
            len: usize,
            count: usize,
        ) -> Result<Vec<String>, Errno> {
            if count == 0 {
                return if len == 0 {
                    Ok(Vec::new())
                } else {
                    Err(Errno::Inval)
                };
            }
            let bytes = unsafe { core::slice::from_raw_parts(ptr as *const u8, len) };
            if bytes.last().copied() != Some(0) {
                return Err(Errno::Inval);
            }
            let values = bytes
                .split(|byte| *byte == 0)
                .take(count)
                .map(|segment| String::from_utf8(segment.to_vec()).map_err(|_| Errno::Inval))
                .collect::<Result<Vec<_>, _>>()?;
            if values.len() != count {
                return Err(Errno::Inval);
            }
            let expected = values
                .iter()
                .fold(0usize, |acc, value| acc + value.len() + 1);
            if expected != len {
                return Err(Errno::Inval);
            }
            Ok(values)
        }

        fn parse_proc_pid_section(path: &str) -> Option<(u64, &str)> {
            let rest = path.strip_prefix("/proc/")?;
            let (pid, section) = rest.split_once('/')?;
            Some((pid.parse::<u64>().ok()?, section))
        }

        fn recorded_procfs_payload(&self, path: &str) -> Option<Vec<u8>> {
            let (pid, section) = Self::parse_proc_pid_section(path)?;
            let process = self.recorded_process(pid)?;
            match section {
                "cmdline" => Some(process.argv.join("\n").into_bytes()),
                "environ" => Some(process.envp.join("\n").into_bytes()),
                "cwd" => Some(process.cwd.into_bytes()),
                "exe" => Some(process.image_path.into_bytes()),
                "status" => Some(
                    format!(
                        "Name:\t{}\nState:\tExited\nPid:\t{}\nCwd:\t{}\n",
                        process.name, process.pid, process.cwd
                    )
                    .into_bytes(),
                ),
                _ => None,
            }
        }

        fn alloc_vm_addr(&self, len: u64) -> u64 {
            let start = self.next_vm_addr.get();
            let advance = len.max(0x1000).next_multiple_of(0x1000);
            self.next_vm_addr.set(start + advance + 0x1000);
            start
        }

        fn push_vm_decision(&self, pid: u64, line: String) {
            self.vm_decisions.borrow_mut().push((pid, line));
        }

        fn push_vm_episode(&self, pid: u64, line: String) {
            self.vm_episodes.borrow_mut().push((pid, line));
        }

        fn ensure_heap_mapping(&self, pid: u64) {
            let mut mappings = self.vm_mappings.borrow_mut();
            if mappings
                .iter()
                .any(|mapping| mapping.pid == pid && mapping.present && mapping.label == "[heap]")
            {
                return;
            }
            mappings.push(VmMappingRecord {
                pid,
                start: 0x4000_0000,
                len: 0x4000,
                readable: true,
                writable: true,
                executable: false,
                label: String::from("[heap]"),
                file_path: None,
                private: true,
                cow: false,
                present: true,
                reclaimed: false,
                words: Vec::new(),
            });
        }

        fn mapping_resident_pages(mapping: &VmMappingRecord) -> u64 {
            let mut pages = Vec::new();
            for (addr, _) in &mapping.words {
                let page = addr.saturating_sub(mapping.start) / 0x1000;
                if !pages.contains(&page) {
                    pages.push(page);
                }
            }
            pages.len() as u64
        }

        fn load_vm_word(&self, pid: u64, addr: u64) -> Result<(u32, bool), Errno> {
            let mut mappings = self.vm_mappings.borrow_mut();
            let Some(mapping) = mappings.iter_mut().find(|mapping| {
                mapping.pid == pid
                    && mapping.present
                    && addr >= mapping.start
                    && addr < mapping.start.saturating_add(mapping.len)
            }) else {
                return Err(Errno::Fault);
            };
            if !mapping.readable {
                return Err(Errno::Fault);
            }
            let restored = mapping.reclaimed;
            mapping.reclaimed = false;
            if mapping.file_path.is_some()
                && !mapping
                    .words
                    .iter()
                    .any(|(word_addr, _)| *word_addr == addr)
            {
                mapping.words.push((addr, 0));
            }
            Ok((
                mapping
                    .words
                    .iter()
                    .find(|(word_addr, _)| *word_addr == addr)
                    .map(|(_, value)| *value)
                    .unwrap_or(0),
                restored,
            ))
        }

        fn store_vm_word(&self, pid: u64, addr: u64, value: u32) -> Result<bool, Errno> {
            let mut mappings = self.vm_mappings.borrow_mut();
            let Some(mapping) = mappings.iter_mut().find(|mapping| {
                mapping.pid == pid
                    && mapping.present
                    && addr >= mapping.start
                    && addr < mapping.start.saturating_add(mapping.len)
            }) else {
                return Err(Errno::Fault);
            };
            if !mapping.writable {
                return Err(Errno::Fault);
            }
            let was_cow = mapping.cow;
            mapping.reclaimed = false;
            if let Some((_, current)) = mapping
                .words
                .iter_mut()
                .find(|(word_addr, _)| *word_addr == addr)
            {
                *current = value;
            } else {
                mapping.words.push((addr, value));
            }
            Ok(was_cow)
        }

        fn protect_vm_range(
            &self,
            pid: u64,
            start: u64,
            len: u64,
            readable: bool,
            writable: bool,
            executable: bool,
        ) -> Result<(), Errno> {
            let mut mappings = self.vm_mappings.borrow_mut();
            let Some(index) = mappings.iter().position(|mapping| {
                mapping.pid == pid
                    && mapping.present
                    && start >= mapping.start
                    && start.saturating_add(len) <= mapping.start.saturating_add(mapping.len)
            }) else {
                return Err(Errno::Fault);
            };
            let mapping = mappings.remove(index);
            let end = start + len;
            let mapping_end = mapping.start + mapping.len;
            let mut replacements = Vec::new();
            if start > mapping.start {
                let mut before = mapping.clone();
                before.len = start - mapping.start;
                before.words.retain(|(addr, _)| *addr < start);
                replacements.push(before);
            }
            let mut middle = mapping.clone();
            middle.start = start;
            middle.len = len;
            middle.readable = readable;
            middle.writable = writable;
            middle.executable = executable;
            middle
                .words
                .retain(|(addr, _)| *addr >= start && *addr < end);
            replacements.push(middle);
            if end < mapping_end {
                let mut after = mapping;
                after.start = end;
                after.len = mapping_end - end;
                after.words.retain(|(addr, _)| *addr >= end);
                replacements.push(after);
            }
            mappings.splice(index..index, replacements);
            Ok(())
        }

        fn unmap_vm_range(&self, pid: u64, start: u64, len: u64) -> Result<(), Errno> {
            let mut mappings = self.vm_mappings.borrow_mut();
            let Some(index) = mappings.iter().position(|mapping| {
                mapping.pid == pid
                    && mapping.present
                    && start >= mapping.start
                    && start.saturating_add(len) <= mapping.start.saturating_add(mapping.len)
            }) else {
                return Err(Errno::Fault);
            };
            let mapping = mappings.remove(index);
            let end = start + len;
            let mapping_end = mapping.start + mapping.len;
            let mut replacements = Vec::new();
            if start > mapping.start {
                let mut before = mapping.clone();
                before.len = start - mapping.start;
                before.words.retain(|(addr, _)| *addr < start);
                replacements.push(before);
            }
            if end < mapping_end {
                let mut after = mapping;
                after.start = end;
                after.len = mapping_end - end;
                after.words.retain(|(addr, _)| *addr >= end);
                replacements.push(after);
            }
            mappings.splice(index..index, replacements);
            Ok(())
        }

        fn reclaim_vm_pressure(&self, scope: Option<u64>, target_pages: u64) -> u64 {
            let mut mappings = self.vm_mappings.borrow_mut();
            let mut candidates = mappings
                .iter_mut()
                .filter(|mapping| {
                    mapping.present
                        && mapping.file_path.is_some()
                        && scope.is_none_or(|pid| mapping.pid == pid)
                        && Self::mapping_resident_pages(mapping) > 0
                })
                .collect::<Vec<_>>();
            candidates.sort_by(|left, right| {
                Self::mapping_resident_pages(right)
                    .cmp(&Self::mapping_resident_pages(left))
                    .then_with(|| left.start.cmp(&right.start))
            });
            let actor_pid = scope.unwrap_or(1);
            self.push_vm_decision(
                actor_pid,
                format!("agent=pressure-trigger pid={actor_pid} target-pages={target_pages}"),
            );
            let mut reclaimed = 0u64;
            for mapping in candidates {
                if reclaimed >= target_pages {
                    break;
                }
                let resident = Self::mapping_resident_pages(mapping);
                if resident == 0 {
                    continue;
                }
                if let Some(path) = mapping.file_path.as_ref() {
                    self.push_vm_decision(
                        mapping.pid,
                        format!(
                            "agent=sync pid={} start={} len={} path={}",
                            mapping.pid, mapping.start, mapping.len, path
                        ),
                    );
                    self.push_vm_decision(
                        mapping.pid,
                        format!(
                            "agent=advice pid={} start={} len={} path={}",
                            mapping.pid, mapping.start, mapping.len, path
                        ),
                    );
                    self.push_vm_decision(
                        mapping.pid,
                        format!(
                            "agent=pressure-victim pid={} reclaimed-pages={} path={}",
                            mapping.pid, resident, path
                        ),
                    );
                }
                mapping.words.clear();
                mapping.reclaimed = true;
                reclaimed = reclaimed.saturating_add(resident);
                self.push_vm_episode(
                    mapping.pid,
                    format!("kind=reclaim pid={} evicted=yes restored=no", mapping.pid),
                );
            }
            reclaimed.min(target_pages)
        }

        fn render_proc_maps(&self, pid: u64) -> Vec<u8> {
            let mut lines = vec![
                String::from("0000000000400000-0000000000401000 r-x /bin/ngos-userland-native"),
                String::from("00007fffffff0000-0000800000000000 rw- [stack]"),
            ];
            for mapping in self
                .vm_mappings
                .borrow()
                .iter()
                .filter(|mapping| mapping.pid == pid && mapping.present)
            {
                let perms = format!(
                    "{}{}{}{}",
                    if mapping.readable { "r" } else { "-" },
                    if mapping.writable { "w" } else { "-" },
                    if mapping.executable { "x" } else { "-" },
                    if mapping.private { "p" } else { "-" }
                );
                let tail = match &mapping.file_path {
                    Some(path) => path.clone(),
                    None => format!("[anon:{}]", mapping.label),
                };
                let offset = if mapping.file_path.is_some() {
                    0x1000
                } else {
                    0
                };
                lines.push(format!(
                    "{:016x}-{:016x} {} {:08x} {}",
                    mapping.start,
                    mapping.start + mapping.len,
                    perms,
                    offset,
                    tail
                ));
            }
            lines.join("\n").into_bytes()
        }

        fn render_vm_decisions(&self, pid: u64) -> Vec<u8> {
            self.vm_decisions
                .borrow()
                .iter()
                .filter(|(entry_pid, _)| *entry_pid == pid)
                .map(|(_, line)| line.clone())
                .collect::<Vec<_>>()
                .join("\n")
                .into_bytes()
        }

        fn render_vm_objects(&self, pid: u64) -> Vec<u8> {
            self.vm_mappings
                .borrow()
                .iter()
                .filter(|mapping| mapping.pid == pid && mapping.present)
                .map(|mapping| {
                    let resident = Self::mapping_resident_pages(mapping);
                    let dirty = resident;
                    format!(
                        "pid={} start={} len={} kind={} depth={} {} path={} resident={} dirty={}",
                        pid,
                        mapping.start,
                        mapping.len,
                        if mapping.file_path.is_some() {
                            "file"
                        } else {
                            "anon"
                        },
                        if mapping.cow { 1 } else { 0 },
                        if mapping.cow { "[cow]" } else { "[owned]" },
                        mapping
                            .file_path
                            .as_deref()
                            .unwrap_or(mapping.label.as_str()),
                        resident,
                        dirty
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
                .into_bytes()
        }

        fn render_vm_episodes(&self, pid: u64) -> Vec<u8> {
            self.vm_episodes
                .borrow()
                .iter()
                .filter(|(entry_pid, _)| *entry_pid == pid)
                .map(|(_, line)| line.clone())
                .collect::<Vec<_>>()
                .join("\n")
                .into_bytes()
        }

        fn push_event_queue_record(&self, queue_fd: usize, record: NativeEventRecord) {
            let mut pending = self.event_queue_pending.borrow_mut();
            if let Some((_, records)) = pending.iter_mut().find(|(fd, _)| *fd == queue_fd) {
                records.push(record);
            } else {
                pending.push((queue_fd, vec![record]));
            }
        }

        fn take_event_queue_records(
            &self,
            queue_fd: usize,
            capacity: usize,
        ) -> Result<Vec<NativeEventRecord>, Errno> {
            let mut pending = self.event_queue_pending.borrow_mut();
            let Some((_, records)) = pending.iter_mut().find(|(fd, _)| *fd == queue_fd) else {
                return Err(Errno::Again);
            };
            if records.is_empty() {
                let nonblock = self
                    .event_queue_nonblock
                    .borrow()
                    .iter()
                    .find(|(fd, _)| *fd == queue_fd)
                    .map(|(_, value)| *value)
                    .unwrap_or(false);
                let has_resource_watch = self
                    .resource_event_watches
                    .borrow()
                    .iter()
                    .any(|(fd, _, _)| *fd == queue_fd);
                let is_resource_queue = self.resource_event_queues.borrow().contains(&queue_fd);
                if nonblock && (has_resource_watch || is_resource_queue) {
                    return Err(Errno::Again);
                }
                if capacity == 0 {
                    return Ok(Vec::new());
                }
                return Ok(vec![NativeEventRecord {
                    token: queue_fd as u64,
                    events: POLLPRI,
                    source_kind: if has_resource_watch {
                        NativeEventSourceKind::Resource as u32
                    } else {
                        NativeEventSourceKind::Network as u32
                    },
                    source_arg0: if has_resource_watch { 42 } else { 99 },
                    source_arg1: if has_resource_watch { 43 } else { 0 },
                    source_arg2: 0,
                    detail0: if has_resource_watch { 4 } else { 1 },
                    detail1: 0,
                }]);
            }
            let count = capacity.min(records.len());
            Ok(records.drain(..count).collect())
        }

        fn register_resource_event_watch(
            &self,
            queue_fd: usize,
            resource: u64,
            config: NativeResourceEventWatchConfig,
        ) {
            let mut resource_queues = self.resource_event_queues.borrow_mut();
            if !resource_queues.contains(&queue_fd) {
                resource_queues.push(queue_fd);
            }
            let mut watches = self.resource_event_watches.borrow_mut();
            if let Some((_, _, existing)) =
                watches.iter_mut().find(|(fd, watch_resource, existing)| {
                    *fd == queue_fd && *watch_resource == resource && existing.token == config.token
                })
            {
                *existing = config;
            } else {
                watches.push((queue_fd, resource, config));
            }
        }

        fn remove_resource_event_watch(
            &self,
            queue_fd: usize,
            resource: u64,
            token: u64,
        ) -> Result<(), Errno> {
            let mut watches = self.resource_event_watches.borrow_mut();
            let original = watches.len();
            watches.retain(|(fd, watch_resource, config)| {
                !(*fd == queue_fd && *watch_resource == resource && config.token == token)
            });
            if watches.len() == original {
                return Err(Errno::NoEnt);
            }
            Ok(())
        }

        fn emit_resource_event(&self, resource: u64, contract: u64, detail0: u32) {
            let watches = self.resource_event_watches.borrow().clone();
            for (queue_fd, watch_resource, config) in watches {
                if watch_resource != resource {
                    continue;
                }
                let interested = match detail0 {
                    0 => config.claimed != 0,
                    1 => config.queued != 0,
                    2 => config.canceled != 0,
                    3 => config.released != 0,
                    4 => config.handed_off != 0,
                    5 => config.revoked != 0,
                    _ => false,
                };
                if !interested {
                    continue;
                }
                self.push_event_queue_record(
                    queue_fd,
                    NativeEventRecord {
                        token: config.token,
                        events: config.poll_events,
                        source_kind: NativeEventSourceKind::Resource as u32,
                        source_arg0: resource,
                        source_arg1: contract,
                        source_arg2: 0,
                        detail0,
                        detail1: 0,
                    },
                );
            }
        }
    }

    fn set_replayed_contract_state(states: &mut Vec<(u64, u32)>, contract: u64, state: u32) {
        if let Some((_, current)) = states.iter_mut().find(|(id, _)| *id == contract) {
            *current = state;
        } else {
            states.push((contract, state));
        }
    }

    fn replayed_contract_state(states: &[(u64, u32)], contract: u64) -> u32 {
        states
            .iter()
            .rev()
            .find(|(id, _)| *id == contract)
            .map(|(_, state)| *state)
            .unwrap_or(NativeContractState::Active as u32)
    }

    fn replayed_process_scheduler(frames: &[SyscallFrame], pid: u64) -> (u32, u32) {
        frames
            .iter()
            .rev()
            .find(|entry| entry.number == SYS_RENICE_PROCESS && entry.arg0 as u64 == pid)
            .map(|entry| (entry.arg1 as u32, entry.arg2 as u32))
            .unwrap_or_else(|| {
                if pid == 1 {
                    (NativeSchedulerClass::Interactive as u32, 2)
                } else {
                    (NativeSchedulerClass::LatencyCritical as u32, 4)
                }
            })
    }

    fn replayed_process_state(frames: &[SyscallFrame], pid: u64) -> u32 {
        if pid >= 77 {
            return 4;
        }
        frames
            .iter()
            .rev()
            .find_map(|entry| {
                if entry.arg0 as u64 != pid {
                    return None;
                }
                match entry.number {
                    SYS_PAUSE_PROCESS => Some(3),
                    SYS_RESUME_PROCESS => Some(1),
                    _ => None,
                }
            })
            .unwrap_or(2)
    }

    fn replayed_system_snapshot(frames: &[SyscallFrame]) -> NativeSystemSnapshotRecord {
        let renice_count = frames
            .iter()
            .filter(|entry| entry.number == SYS_RENICE_PROCESS)
            .count() as u64;
        let pause_count = frames
            .iter()
            .filter(|entry| entry.number == SYS_PAUSE_PROCESS)
            .count() as u64;
        let net_admin_count = frames
            .iter()
            .filter(|entry| entry.number == SYS_CONFIGURE_NETIF_ADMIN)
            .count() as u64;
        let run_queue = 5u64
            .saturating_sub(renice_count.saturating_mul(2))
            .saturating_sub(pause_count);
        let cpu_busy = if run_queue > 0 { 96 } else { 54 };
        let socket_limit: u64 = if net_admin_count == 0 { 16 } else { 32 };
        let socket_depth: u64 = if net_admin_count == 0 { 15 } else { 6 };
        let saturated = u64::from(socket_depth >= socket_limit.saturating_sub(1));
        NativeSystemSnapshotRecord {
            current_tick: 100 + frames.len() as u64,
            busy_ticks: 80 + (frames.len() as u64).saturating_mul(cpu_busy) / 100,
            process_count: 3,
            active_process_count: 3u64.saturating_sub(pause_count.min(1)),
            blocked_process_count: pause_count.min(1),
            queued_processes: run_queue,
            queued_latency_critical: 1,
            queued_interactive: 1u64.min(run_queue),
            queued_normal: run_queue.saturating_sub(2).min(2),
            queued_background: run_queue.saturating_sub(3),
            deferred_task_count: 1,
            sleeping_processes: pause_count.min(1),
            total_event_queue_count: frames
                .iter()
                .filter(|entry| entry.number == SYS_WATCH_PROCESS_EVENTS)
                .count() as u64
                + frames
                    .iter()
                    .filter(|entry| entry.number == SYS_WATCH_RESOURCE_EVENTS)
                    .count() as u64,
            total_event_queue_pending: if run_queue > 2 { 10 } else { 2 },
            total_event_queue_waiters: if run_queue > 2 { 3 } else { 1 },
            total_socket_count: 2,
            saturated_socket_count: saturated,
            total_socket_rx_depth: socket_depth,
            total_socket_rx_limit: socket_limit,
            max_socket_rx_depth: socket_depth.saturating_sub(2),
            total_network_tx_dropped: 4,
            total_network_rx_dropped: 3,
            running_pid: 1,
            reserved0: 0,
            reserved1: 0,
        }
    }

    fn pop_next_active_waiter(waiters: &mut Vec<u64>, states: &[(u64, u32)]) -> Option<u64> {
        while let Some(waiter) = waiters.first().copied() {
            waiters.remove(0);
            if replayed_contract_state(states, waiter) == NativeContractState::Active as u32 {
                return Some(waiter);
            }
        }
        None
    }

    fn replayed_contract_kind(frames: &[SyscallFrame], contract: u64) -> u32 {
        let mut next_contract = 43u64;
        for entry in frames {
            if entry.number == SYS_CREATE_CONTRACT {
                if next_contract == contract {
                    return entry.arg2 as u32;
                }
                next_contract += 1;
            }
        }
        NativeContractKind::Display as u32
    }

    fn replayed_contract_resource(frames: &[SyscallFrame], contract: u64) -> u64 {
        let mut next_contract = 43u64;
        for entry in frames {
            if entry.number == SYS_CREATE_CONTRACT {
                if next_contract == contract {
                    return entry.arg1 as u64;
                }
                next_contract += 1;
            }
        }
        42
    }

    fn replayed_resource_contract_policy(frames: &[SyscallFrame], resource: u64) -> u32 {
        frames
            .iter()
            .rev()
            .find(|entry| {
                entry.number == SYS_SET_RESOURCE_CONTRACT_POLICY && entry.arg0 as u64 == resource
            })
            .map(|entry| entry.arg1 as u32)
            .unwrap_or(NativeResourceContractPolicy::Any as u32)
    }

    fn replayed_resource_operational_state(frames: &[SyscallFrame], resource: u64) -> u32 {
        frames
            .iter()
            .rev()
            .find(|entry| entry.number == SYS_SET_RESOURCE_STATE && entry.arg0 as u64 == resource)
            .map(|entry| entry.arg1 as u32)
            .unwrap_or(NativeResourceState::Active as u32)
    }

    fn replayed_resource_governance(frames: &[SyscallFrame], resource: u64) -> u32 {
        frames
            .iter()
            .rev()
            .find(|entry| {
                entry.number == SYS_SET_RESOURCE_GOVERNANCE && entry.arg0 as u64 == resource
            })
            .map(|entry| entry.arg1 as u32)
            .unwrap_or(NativeResourceGovernanceMode::Queueing as u32)
    }

    fn replay_resource_state(frames: &[SyscallFrame], resource: u64) -> (u64, u64, u64, Vec<u64>) {
        let mut holder_contract = 0u64;
        let mut acquire_count = 0u64;
        let mut handoff_count = 0u64;
        let mut fifo = true;
        let mut exclusive = false;
        let mut resource_state = NativeResourceState::Active as u32;
        let mut waiters = Vec::new();
        let mut states = Vec::new();
        for entry in frames {
            match entry.number {
                SYS_SET_RESOURCE_POLICY if entry.arg0 as u64 == resource => {
                    fifo = entry.arg1 == NativeResourceArbitrationPolicy::Fifo as usize;
                }
                SYS_SET_RESOURCE_GOVERNANCE if entry.arg0 as u64 == resource => {
                    exclusive = entry.arg1 == NativeResourceGovernanceMode::ExclusiveLease as usize;
                    if exclusive {
                        waiters.clear();
                    }
                }
                SYS_SET_CONTRACT_STATE => {
                    let contract = entry.arg0 as u64;
                    let state = entry.arg1 as u32;
                    set_replayed_contract_state(&mut states, contract, state);
                    waiters.retain(|waiter| *waiter != contract);
                    if holder_contract == contract && state != NativeContractState::Active as u32 {
                        holder_contract =
                            pop_next_active_waiter(&mut waiters, &states).unwrap_or(0);
                        if holder_contract != 0 {
                            acquire_count += 1;
                            handoff_count += 1;
                        }
                    }
                }
                SYS_SET_RESOURCE_STATE if entry.arg0 as u64 == resource => {
                    resource_state = entry.arg1 as u32;
                    if resource_state != NativeResourceState::Active as u32 {
                        holder_contract = 0;
                        waiters.clear();
                    }
                }
                SYS_CLAIM_RESOURCE => {
                    if replayed_contract_resource(frames, entry.arg0 as u64) != resource {
                        continue;
                    }
                    if resource_state != NativeResourceState::Active as u32 {
                        continue;
                    }
                    let contract = entry.arg0 as u64;
                    if replayed_contract_state(&states, contract)
                        != NativeContractState::Active as u32
                    {
                        continue;
                    }
                    if holder_contract == 0 {
                        holder_contract = contract;
                        acquire_count += 1;
                    } else if exclusive {
                        continue;
                    } else if holder_contract != contract && !waiters.contains(&contract) {
                        if fifo {
                            waiters.push(contract);
                        } else {
                            waiters.insert(0, contract);
                        }
                    }
                }
                SYS_CANCEL_RESOURCE_CLAIM => {
                    let contract = entry.arg0 as u64;
                    if replayed_contract_resource(frames, contract) != resource {
                        continue;
                    }
                    waiters.retain(|waiter| *waiter != contract);
                }
                SYS_TRANSFER_RESOURCE
                    if replayed_contract_resource(frames, entry.arg0 as u64) == resource
                        && holder_contract == entry.arg0 as u64 =>
                {
                    waiters.retain(|waiter| *waiter != entry.arg1 as u64);
                    holder_contract = entry.arg1 as u64;
                    acquire_count += 1;
                    handoff_count += 1;
                }
                SYS_RELEASE_CLAIMED_RESOURCE
                    if replayed_contract_resource(frames, entry.arg0 as u64) == resource
                        && holder_contract == entry.arg0 as u64 =>
                {
                    if let Some(next) = pop_next_active_waiter(&mut waiters, &states) {
                        holder_contract = next;
                        acquire_count += 1;
                        handoff_count += 1;
                    } else {
                        holder_contract = 0;
                    }
                }
                SYS_RELEASE_RESOURCE
                    if replayed_contract_resource(frames, entry.arg0 as u64) == resource
                        && holder_contract == entry.arg0 as u64 =>
                {
                    holder_contract = 0;
                }
                _ => {}
            }
        }
        (holder_contract, acquire_count, handoff_count, waiters)
    }

    impl ngos_user_abi::SyscallBackend for RecordingBackend {
        unsafe fn syscall(&self, frame: SyscallFrame) -> SyscallReturn {
            self.frames.borrow_mut().push(frame);
            match frame.number {
                SYS_READ => {
                    if frame.arg0 != 0 {
                        let Some(path) = self.opened_path(frame.arg0) else {
                            return SyscallReturn::err(Errno::Badf);
                        };
                        if self.created_kind(&path) == Some(NativeObjectKind::Channel) {
                            let payload = self.pop_channel_message(&path).unwrap_or_default();
                            let read = payload.len().min(frame.arg2);
                            if read != 0 {
                                unsafe {
                                    core::ptr::copy_nonoverlapping(
                                        payload.as_ptr(),
                                        frame.arg1 as *mut u8,
                                        read,
                                    );
                                }
                            }
                            return SyscallReturn::ok(read);
                        }
                        let payload = self.file_content(&path);
                        let offset = self.read_offset(frame.arg0);
                        let read = payload.len().saturating_sub(offset).min(frame.arg2);
                        if read != 0 {
                            unsafe {
                                core::ptr::copy_nonoverlapping(
                                    payload.as_ptr().add(offset),
                                    frame.arg1 as *mut u8,
                                    read,
                                );
                            }
                            self.set_read_offset(frame.arg0, offset + read);
                        }
                        return SyscallReturn::ok(read);
                    }
                    let stdin = self.stdin.borrow();
                    let offset = self.stdin_offset.get();
                    let remaining = stdin.len().saturating_sub(offset);
                    let read = remaining.min(frame.arg2);
                    if read != 0 {
                        unsafe {
                            core::ptr::copy_nonoverlapping(
                                stdin.as_ptr().add(offset),
                                frame.arg1 as *mut u8,
                                read,
                            );
                        }
                        self.stdin_offset.set(offset + read);
                    }
                    SyscallReturn::ok(read)
                }
                SYS_CREATE_DOMAIN => {
                    let count = self
                        .frames
                        .borrow()
                        .iter()
                        .filter(|entry| entry.number == SYS_CREATE_DOMAIN)
                        .count();
                    SyscallReturn::ok(40 + count)
                }
                SYS_CREATE_RESOURCE => {
                    let count = self
                        .frames
                        .borrow()
                        .iter()
                        .filter(|entry| entry.number == SYS_CREATE_RESOURCE)
                        .count();
                    SyscallReturn::ok(41 + count)
                }
                SYS_CREATE_CONTRACT => {
                    let resource = frame.arg1 as u64;
                    let frames = self.frames.borrow();
                    if replayed_resource_operational_state(&frames, resource)
                        != NativeResourceState::Active as u32
                    {
                        return SyscallReturn::err(Errno::Access);
                    }
                    let policy = replayed_resource_contract_policy(
                        &frames[..frames.len().saturating_sub(1)],
                        resource,
                    );
                    if policy != NativeResourceContractPolicy::Any as u32
                        && frame.arg2 as u32 != policy - 1
                    {
                        return SyscallReturn::err(Errno::Access);
                    }
                    let count = self
                        .frames
                        .borrow()
                        .iter()
                        .filter(|entry| entry.number == SYS_CREATE_CONTRACT)
                        .count();
                    SyscallReturn::ok(42 + count)
                }
                SYS_LIST_DOMAINS => {
                    let ptr = frame.arg0 as *mut u64;
                    unsafe {
                        *ptr = 41;
                    }
                    SyscallReturn::ok(1)
                }
                SYS_LIST_RESOURCES => {
                    let ptr = frame.arg0 as *mut u64;
                    unsafe {
                        *ptr = 42;
                    }
                    SyscallReturn::ok(1)
                }
                SYS_LIST_CONTRACTS => {
                    let ptr = frame.arg0 as *mut u64;
                    unsafe {
                        *ptr = 43;
                        *ptr.add(1) = 44;
                        *ptr.add(2) = 45;
                    }
                    SyscallReturn::ok(3)
                }
                SYS_LIST_PROCESSES => {
                    let ptr = frame.arg0 as *mut u64;
                    let spawned = self.next_pid.get().saturating_sub(77) as usize;
                    unsafe {
                        *ptr = 1;
                        for index in 0..spawned {
                            *ptr.add(1 + index) = 77 + index as u64;
                        }
                        *ptr.add(1 + spawned) = 88;
                    }
                    SyscallReturn::ok(spawned + 2)
                }
                SYS_INSPECT_PROCESS => {
                    let ptr = frame.arg1 as *mut NativeProcessRecord;
                    let pid = frame.arg0 as u64;
                    let frames = self.frames.borrow();
                    let state = replayed_process_state(&frames, pid);
                    let (scheduler_class, scheduler_budget) =
                        replayed_process_scheduler(&frames, pid);
                    let exit_code = if pid >= 77 && pid < self.next_pid.get() {
                        137
                    } else {
                        0
                    };
                    unsafe {
                        ptr.write(NativeProcessRecord {
                            pid,
                            parent: 0,
                            address_space: 4,
                            main_thread: pid,
                            state,
                            exit_code,
                            descriptor_count: 3,
                            capability_count: 2,
                            environment_count: 1,
                            memory_region_count: 2,
                            thread_count: 1,
                            pending_signal_count: if pid == 1 { 1 } else { 0 },
                            session_reported: 0,
                            session_status: 0,
                            session_stage: 0,
                            scheduler_class,
                            scheduler_budget,
                            cpu_runtime_ticks: match pid {
                                88 => 91,
                                77.. => 48 + pid.saturating_sub(77),
                                _ => 12,
                            },
                            execution_contract: 0,
                            memory_contract: 0,
                            io_contract: 0,
                            observe_contract: 0,
                            reserved: 0,
                        });
                    }
                    SyscallReturn::ok(0)
                }
                SYS_INSPECT_SYSTEM_SNAPSHOT => {
                    let ptr = frame.arg0 as *mut NativeSystemSnapshotRecord;
                    let frames = self.frames.borrow();
                    unsafe {
                        ptr.write(replayed_system_snapshot(&frames));
                    }
                    SyscallReturn::ok(0)
                }
                SYS_GET_PROCESS_NAME => {
                    let ptr = frame.arg1 as *mut u8;
                    let payload = match frame.arg0 as u64 {
                        pid if pid >= 77 && pid < self.next_pid.get() => self
                            .recorded_process(pid)
                            .map(|process| process.name.into_bytes())
                            .unwrap_or_else(|| b"worker".to_vec()),
                        88 => b"bg-net-pump".to_vec(),
                        _ => b"ngos-userland-native".to_vec(),
                    };
                    let count = payload.len().min(frame.arg2);
                    unsafe {
                        core::ptr::copy_nonoverlapping(payload.as_ptr(), ptr, count);
                    }
                    SyscallReturn::ok(count)
                }
                SYS_GET_PROCESS_IMAGE_PATH => {
                    let ptr = frame.arg1 as *mut u8;
                    let payload = match frame.arg0 as u64 {
                        pid if pid >= 77 && pid < self.next_pid.get() => self
                            .recorded_process(pid)
                            .map(|process| process.image_path.into_bytes())
                            .unwrap_or_else(|| b"/bin/worker".to_vec()),
                        88 => b"/bin/bg-net-pump".to_vec(),
                        _ => b"/bin/ngos-userland-native".to_vec(),
                    };
                    let count = payload.len().min(frame.arg2);
                    unsafe {
                        core::ptr::copy_nonoverlapping(payload.as_ptr(), ptr, count);
                    }
                    SyscallReturn::ok(count)
                }
                SYS_GET_PROCESS_CWD => {
                    let ptr = frame.arg1 as *mut u8;
                    let payload = match frame.arg0 as u64 {
                        pid if pid >= 77 && pid < self.next_pid.get() => self
                            .recorded_process(pid)
                            .map(|process| process.cwd.into_bytes())
                            .unwrap_or_else(|| b"/".to_vec()),
                        _ => b"/".to_vec(),
                    };
                    let count = payload.len().min(frame.arg2);
                    unsafe {
                        core::ptr::copy_nonoverlapping(payload.as_ptr(), ptr, count);
                    }
                    SyscallReturn::ok(count)
                }
                SYS_CHDIR_PATH
                | SYS_PAUSE_PROCESS
                | SYS_RESUME_PROCESS
                | SYS_RENICE_PROCESS
                | SYS_WATCH_NET_EVENTS
                | SYS_WATCH_GRAPHICS_EVENTS
                | SYS_WATCH_PROCESS_EVENTS
                | SYS_REMOVE_PROCESS_EVENTS
                | SYS_REMOVE_GRAPHICS_EVENTS
                | SYS_REMOVE_NET_EVENTS => SyscallReturn::ok(0),
                SYS_WATCH_RESOURCE_EVENTS => {
                    let config = unsafe {
                        (frame.arg2 as *const NativeResourceEventWatchConfig).read_unaligned()
                    };
                    self.register_resource_event_watch(frame.arg0, frame.arg1 as u64, config);
                    SyscallReturn::ok(0)
                }
                SYS_REMOVE_RESOURCE_EVENTS => {
                    match self.remove_resource_event_watch(
                        frame.arg0,
                        frame.arg1 as u64,
                        frame.arg2 as u64,
                    ) {
                        Ok(()) => SyscallReturn::ok(0),
                        Err(errno) => SyscallReturn::err(errno),
                    }
                }
                SYS_RECLAIM_MEMORY_PRESSURE => {
                    let reclaimed =
                        self.reclaim_vm_pressure(Some(frame.arg0 as u64), frame.arg1 as u64);
                    SyscallReturn::ok(reclaimed as usize)
                }
                SYS_RECLAIM_MEMORY_PRESSURE_GLOBAL => {
                    let reclaimed = self.reclaim_vm_pressure(None, frame.arg0 as u64);
                    SyscallReturn::ok(reclaimed as usize)
                }
                SYS_MAP_ANONYMOUS_MEMORY => {
                    let pid = frame.arg0 as u64;
                    let len = frame.arg1 as u64;
                    let perms = frame.arg2;
                    let label = unsafe {
                        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                            frame.arg3 as *const u8,
                            frame.arg4,
                        ))
                    };
                    let start = self.alloc_vm_addr(len);
                    self.vm_mappings.borrow_mut().push(VmMappingRecord {
                        pid,
                        start,
                        len,
                        readable: (perms & 1) != 0,
                        writable: (perms & 2) != 0,
                        executable: (perms & 4) != 0,
                        label: label.to_string(),
                        file_path: None,
                        private: true,
                        cow: false,
                        present: true,
                        reclaimed: false,
                        words: Vec::new(),
                    });
                    self.push_vm_decision(
                        pid,
                        format!("agent=map pid={pid} start={start} len={len} label={label}"),
                    );
                    SyscallReturn::ok(start as usize)
                }
                SYS_LOAD_MEMORY_WORD => {
                    match self.load_vm_word(frame.arg0 as u64, frame.arg1 as u64) {
                        Ok((value, restored)) => {
                            if restored {
                                self.push_vm_episode(
                                    frame.arg0 as u64,
                                    format!(
                                        "kind=reclaim pid={} evicted=yes restored=yes",
                                        frame.arg0 as u64
                                    ),
                                );
                            }
                            SyscallReturn::ok(value as usize)
                        }
                        Err(errno) => SyscallReturn::err(errno),
                    }
                }
                SYS_STORE_MEMORY_WORD => {
                    let pid = frame.arg0 as u64;
                    let addr = frame.arg1 as u64;
                    let value = frame.arg2 as u32;
                    match self.store_vm_word(pid, addr, value) {
                        Ok(was_cow) => {
                            if was_cow {
                                self.push_vm_episode(
                                    pid,
                                    format!("kind=fault pid={pid} addr={addr} cow=yes"),
                                );
                            }
                            SyscallReturn::ok(0)
                        }
                        Err(errno) => SyscallReturn::err(errno),
                    }
                }
                SYS_SYNC_MEMORY_RANGE => {
                    let pid = frame.arg0 as u64;
                    self.push_vm_decision(
                        pid,
                        format!(
                            "agent=sync pid={pid} start={} len={}",
                            frame.arg1, frame.arg2
                        ),
                    );
                    SyscallReturn::ok(0)
                }
                SYS_PROTECT_MEMORY_RANGE => {
                    let pid = frame.arg0 as u64;
                    let start = frame.arg1 as u64;
                    let len = frame.arg2 as u64;
                    let readable = frame.arg3 != 0;
                    let writable = frame.arg4 != 0;
                    let executable = frame.arg5 != 0;
                    match self.protect_vm_range(pid, start, len, readable, writable, executable) {
                        Ok(()) => {
                            self.push_vm_decision(
                                pid,
                                format!("agent=protect pid={pid} start={start} len={len}"),
                            );
                            self.push_vm_episode(
                                pid,
                                format!("kind=region pid={pid} protected=yes unmapped=no"),
                            );
                            SyscallReturn::ok(0)
                        }
                        Err(errno) => SyscallReturn::err(errno),
                    }
                }
                SYS_UNMAP_MEMORY_RANGE => {
                    let pid = frame.arg0 as u64;
                    let start = frame.arg1 as u64;
                    let len = frame.arg2 as u64;
                    match self.unmap_vm_range(pid, start, len) {
                        Ok(()) => {
                            self.push_vm_decision(
                                pid,
                                format!("agent=unmap pid={pid} start={start} len={len}"),
                            );
                            self.push_vm_episode(
                                pid,
                                format!("kind=region pid={pid} protected=no unmapped=yes"),
                            );
                            SyscallReturn::ok(0)
                        }
                        Err(errno) => SyscallReturn::err(errno),
                    }
                }
                SYS_SET_PROCESS_BREAK => {
                    let pid = frame.arg0 as u64;
                    let requested_end = frame.arg1 as u64;
                    self.ensure_heap_mapping(pid);
                    let mut mappings = self.vm_mappings.borrow_mut();
                    let Some(mapping) = mappings.iter_mut().find(|mapping| {
                        mapping.pid == pid && mapping.present && mapping.label == "[heap]"
                    }) else {
                        return SyscallReturn::err(Errno::NoEnt);
                    };
                    if requested_end <= mapping.start {
                        return SyscallReturn::err(Errno::Inval);
                    }
                    let old_end = mapping.start.saturating_add(mapping.len);
                    mapping.len = requested_end
                        .saturating_sub(mapping.start)
                        .max(0x1000)
                        .next_multiple_of(0x1000);
                    let new_end = mapping.start.saturating_add(mapping.len);
                    let grew = new_end > old_end;
                    let shrank = new_end < old_end;
                    drop(mappings);
                    self.push_vm_decision(
                        pid,
                        format!("agent=brk pid={pid} old-end={old_end} new-end={new_end}"),
                    );
                    self.push_vm_episode(
                        pid,
                        format!(
                            "kind=heap pid={pid} grew={} shrank={} old-end={} new-end={}",
                            if grew { "yes" } else { "no" },
                            if shrank { "yes" } else { "no" },
                            old_end,
                            new_end,
                        ),
                    );
                    SyscallReturn::ok(new_end as usize)
                }
                SYS_MAP_FILE_MEMORY => {
                    let pid = frame.arg0 as u64;
                    let path = unsafe {
                        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                            frame.arg1 as *const u8,
                            frame.arg2,
                        ))
                    };
                    let len = frame.arg3 as u64;
                    let perms = frame.arg5;
                    let start = self.alloc_vm_addr(len);
                    self.vm_mappings.borrow_mut().push(VmMappingRecord {
                        pid,
                        start,
                        len,
                        readable: (perms & 1) != 0,
                        writable: (perms & 2) != 0,
                        executable: (perms & 4) != 0,
                        label: path.to_string(),
                        file_path: Some(path.to_string()),
                        private: true,
                        cow: true,
                        present: true,
                        reclaimed: false,
                        words: Vec::new(),
                    });
                    self.push_vm_decision(
                        pid,
                        format!("agent=map-file pid={pid} start={start} len={len} path={path}"),
                    );
                    SyscallReturn::ok(start as usize)
                }
                SYS_SPAWN_PROCESS_COPY_VM => {
                    let source_pid = frame.arg4 as u64;
                    let child_pid = if self
                        .vm_mappings
                        .borrow()
                        .iter()
                        .any(|mapping| mapping.pid == 2)
                    {
                        3u64
                    } else {
                        2u64
                    };
                    let clones = self
                        .vm_mappings
                        .borrow()
                        .iter()
                        .filter(|mapping| mapping.pid == source_pid && mapping.present)
                        .cloned()
                        .map(|mut mapping| {
                            mapping.pid = child_pid;
                            mapping.cow = true;
                            mapping
                        })
                        .collect::<Vec<_>>();
                    self.vm_mappings.borrow_mut().extend(clones);
                    self.push_vm_decision(
                        child_pid,
                        format!("agent=shadow-reuse pid={child_pid} source-pid={source_pid}"),
                    );
                    self.push_vm_decision(
                        child_pid,
                        format!("agent=cow-populate pid={child_pid} source-pid={source_pid}"),
                    );
                    self.push_vm_episode(
                        child_pid,
                        format!("kind=fault pid={child_pid} source_pid={source_pid} cow=yes"),
                    );
                    SyscallReturn::ok(child_pid as usize)
                }
                SYS_CREATE_EVENT_QUEUE => {
                    let fd = self.next_fd.get();
                    self.next_fd.set(fd + 1);
                    self.event_queue_pending.borrow_mut().push((fd, Vec::new()));
                    self.event_queue_nonblock.borrow_mut().push((fd, false));
                    SyscallReturn::ok(fd)
                }
                SYS_WAIT_EVENT_QUEUE => {
                    match self.take_event_queue_records(frame.arg0, frame.arg2) {
                        Ok(records) => {
                            let ptr = frame.arg1 as *mut NativeEventRecord;
                            for (index, record) in records.iter().enumerate() {
                                unsafe {
                                    ptr.add(index).write(*record);
                                }
                            }
                            SyscallReturn::ok(records.len())
                        }
                        Err(errno) => SyscallReturn::err(errno),
                    }
                }
                SYS_SEND_SIGNAL => SyscallReturn::ok(0),
                SYS_PENDING_SIGNALS => {
                    let ptr = frame.arg1 as *mut u8;
                    unsafe {
                        *ptr = 9;
                    }
                    SyscallReturn::ok(1)
                }
                SYS_BLOCKED_PENDING_SIGNALS => SyscallReturn::ok(0),
                SYS_SPAWN_PATH_PROCESS => {
                    let name = unsafe {
                        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                            frame.arg0 as *const u8,
                            frame.arg1,
                        ))
                    };
                    let path = unsafe {
                        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                            frame.arg2 as *const u8,
                            frame.arg3,
                        ))
                    };
                    let pid = self.next_pid.get();
                    self.next_pid.set(pid + 1);
                    self.record_spawned_process(pid, name, path);
                    SyscallReturn::ok(pid as usize)
                }
                SYS_SPAWN_CONFIGURED_PROCESS => {
                    let config = unsafe {
                        (frame.arg0 as *const ngos_user_abi::NativeSpawnProcessConfig)
                            .read_unaligned()
                    };
                    let name = unsafe {
                        core::str::from_utf8(core::slice::from_raw_parts(
                            config.name_ptr as *const u8,
                            config.name_len,
                        ))
                    };
                    let name = match name {
                        Ok(name) => name,
                        Err(_) => return SyscallReturn::err(Errno::Inval),
                    };
                    let path = unsafe {
                        core::str::from_utf8(core::slice::from_raw_parts(
                            config.path_ptr as *const u8,
                            config.path_len,
                        ))
                    };
                    let path = match path {
                        Ok(path) => path,
                        Err(_) => return SyscallReturn::err(Errno::Inval),
                    };
                    let cwd = unsafe {
                        core::str::from_utf8(core::slice::from_raw_parts(
                            config.cwd_ptr as *const u8,
                            config.cwd_len,
                        ))
                    };
                    let cwd = match cwd {
                        Ok(cwd) => cwd.to_string(),
                        Err(_) => return SyscallReturn::err(Errno::Inval),
                    };
                    let argv = match self.decode_string_table(
                        config.argv_ptr,
                        config.argv_len,
                        config.argv_count,
                    ) {
                        Ok(argv) => argv,
                        Err(error) => return SyscallReturn::err(error),
                    };
                    let envp = match self.decode_string_table(
                        config.envp_ptr,
                        config.envp_len,
                        config.envp_count,
                    ) {
                        Ok(envp) => envp,
                        Err(error) => return SyscallReturn::err(error),
                    };
                    let pid = self.next_pid.get();
                    self.next_pid.set(pid + 1);
                    self.record_spawned_process(pid, name, path);
                    if self
                        .with_recorded_process_mut(pid, |process| {
                            process.cwd = cwd;
                            process.argv = argv;
                            process.envp = envp;
                        })
                        .is_err()
                    {
                        return SyscallReturn::err(Errno::Srch);
                    }
                    SyscallReturn::ok(pid as usize)
                }
                SYS_SET_PROCESS_ARGS => {
                    let pid = frame.arg0 as u64;
                    let argv = match self.decode_string_table(frame.arg1, frame.arg2, frame.arg3) {
                        Ok(argv) => argv,
                        Err(error) => return SyscallReturn::err(error),
                    };
                    if self
                        .with_recorded_process_mut(pid, |process| process.argv = argv)
                        .is_err()
                    {
                        return SyscallReturn::err(Errno::Srch);
                    }
                    SyscallReturn::ok(0)
                }
                SYS_SET_PROCESS_ENV => {
                    let pid = frame.arg0 as u64;
                    let envp = match self.decode_string_table(frame.arg1, frame.arg2, frame.arg3) {
                        Ok(envp) => envp,
                        Err(error) => return SyscallReturn::err(error),
                    };
                    if self
                        .with_recorded_process_mut(pid, |process| process.envp = envp)
                        .is_err()
                    {
                        return SyscallReturn::err(Errno::Srch);
                    }
                    SyscallReturn::ok(0)
                }
                SYS_SET_PROCESS_CWD => {
                    let pid = frame.arg0 as u64;
                    let cwd = unsafe {
                        core::str::from_utf8(core::slice::from_raw_parts(
                            frame.arg1 as *const u8,
                            frame.arg2,
                        ))
                    };
                    let cwd = match cwd {
                        Ok(cwd) => cwd.to_string(),
                        Err(_) => return SyscallReturn::err(Errno::Inval),
                    };
                    if self
                        .with_recorded_process_mut(pid, |process| process.cwd = cwd)
                        .is_err()
                    {
                        return SyscallReturn::err(Errno::Srch);
                    }
                    SyscallReturn::ok(0)
                }
                SYS_REAP_PROCESS => SyscallReturn::ok(137),
                SYS_READ_PROCFS => {
                    let path =
                        unsafe { core::slice::from_raw_parts(frame.arg0 as *const u8, frame.arg1) };
                    let path = match core::str::from_utf8(path) {
                        Ok(path) => path,
                        Err(_) => return SyscallReturn::err(Errno::Inval),
                    };
                    let payload = if let Some(payload) = self.recorded_procfs_payload(path) {
                        payload
                    } else {
                        match path {
                            "/proc/1/status" => {
                                b"Name:\tngos-userland-native\nState:\tRunning\nPid:\t1\nCwd:\t/\n"
                                    .to_vec()
                            }
                            "/proc/1/cwd" => b"/".to_vec(),
                            "/proc/1/exe" => b"/bin/ngos-userland-native".to_vec(),
                            "/proc/1/fd" => {
                                b"0 [stdio:stdin]\n1 [stdio:stdout]\n2 [stdio:stderr]\n".to_vec()
                            }
                            "/proc/1/fdinfo/0" => {
                                b"fd:\t0\npath:\t[stdio:stdin]\nkind:\tFile\npos:\t0\nflags:\tcloexec=false nonblock=false\nrights:\t0x3\n"
                                    .to_vec()
                            }
                            "/proc/1/maps" => self.render_proc_maps(1),
                            "/proc/1/vmdecisions" => self.render_vm_decisions(1),
                            "/proc/1/vmobjects" => self.render_vm_objects(1),
                            "/proc/1/vmepisodes" => self.render_vm_episodes(1),
                            "/proc/2/vmdecisions" => self.render_vm_decisions(2),
                            "/proc/2/vmobjects" => self.render_vm_objects(2),
                            "/proc/2/vmepisodes" => self.render_vm_episodes(2),
                            "/proc/3/vmdecisions" => self.render_vm_decisions(3),
                            "/proc/3/vmobjects" => self.render_vm_objects(3),
                            "/proc/3/vmepisodes" => self.render_vm_episodes(3),
                            "/proc/1/cmdline" => b"ngos-userland-native\0".to_vec(),
                            _ => return SyscallReturn::err(Errno::NoEnt),
                        }
                    };
                    let count = payload.len().min(frame.arg3);
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            payload.as_ptr(),
                            frame.arg2 as *mut u8,
                            count,
                        );
                    }
                    SyscallReturn::ok(count)
                }
                SYS_OPEN_PATH => {
                    let path =
                        unsafe { core::slice::from_raw_parts(frame.arg0 as *const u8, frame.arg1) };
                    let path = match core::str::from_utf8(path) {
                        Ok(path) => path,
                        Err(_) => return SyscallReturn::err(Errno::Inval),
                    };
                    if !self.path_exists(path)
                        && path != "/proc/1/status"
                        && path != "/proc/1/cwd"
                        && !path.starts_with("/dev/")
                        && !path.starts_with("/drv/")
                    {
                        return SyscallReturn::err(Errno::NoEnt);
                    }
                    let fd = self.next_fd.get();
                    self.next_fd.set(fd + 1);
                    self.open_files.borrow_mut().push((fd, path.to_string()));
                    self.set_read_offset(fd, 0);
                    SyscallReturn::ok(fd)
                }
                SYS_MKDIR_PATH => {
                    let path = unsafe {
                        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                            frame.arg0 as *const u8,
                            frame.arg1,
                        ))
                    };
                    self.record_created_path(path, NativeObjectKind::Directory);
                    SyscallReturn::ok(0)
                }
                SYS_MKFILE_PATH => {
                    let path = unsafe {
                        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                            frame.arg0 as *const u8,
                            frame.arg1,
                        ))
                    };
                    self.record_created_path(path, NativeObjectKind::File);
                    SyscallReturn::ok(0)
                }
                SYS_MKCHAN_PATH => {
                    let path = unsafe {
                        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                            frame.arg0 as *const u8,
                            frame.arg1,
                        ))
                    };
                    self.record_created_path(path, NativeObjectKind::Channel);
                    SyscallReturn::ok(0)
                }
                SYS_SYMLINK_PATH => {
                    let path = unsafe {
                        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                            frame.arg0 as *const u8,
                            frame.arg1,
                        ))
                    };
                    let target = unsafe {
                        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                            frame.arg2 as *const u8,
                            frame.arg3,
                        ))
                    };
                    self.record_symlink_path(path, target);
                    SyscallReturn::ok(0)
                }
                SYS_RENAME_PATH => {
                    let from = unsafe {
                        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                            frame.arg0 as *const u8,
                            frame.arg1,
                        ))
                    };
                    let to = unsafe {
                        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                            frame.arg2 as *const u8,
                            frame.arg3,
                        ))
                    };
                    match self.rename_path(from, to) {
                        Ok(()) => SyscallReturn::ok(0),
                        Err(errno) => SyscallReturn::err(errno),
                    }
                }
                SYS_UNLINK_PATH => {
                    let path = unsafe {
                        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                            frame.arg0 as *const u8,
                            frame.arg1,
                        ))
                    };
                    match self.unlink_path(path) {
                        Ok(()) => SyscallReturn::ok(0),
                        Err(errno) => SyscallReturn::err(errno),
                    }
                }
                SYS_LIST_PATH => {
                    let path = unsafe {
                        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                            frame.arg0 as *const u8,
                            frame.arg1,
                        ))
                    };
                    let ptr = frame.arg2 as *mut u8;
                    let payload: &[u8] = if path == "/dev" {
                        b"net0\tDevice\nstorage0\tDevice\ngpu0\tDevice\naudio0\tDevice\ninput0\tDevice\n"
                    } else if path == "/drv" {
                        b"net0\tDriver\nstorage0\tDriver\ngpu0\tDriver\naudio0\tDriver\ninput0\tDriver\n"
                    } else {
                        b"note-2\tFile\n"
                    };
                    let count = payload.len().min(frame.arg3);
                    unsafe {
                        core::ptr::copy_nonoverlapping(payload.as_ptr(), ptr, count);
                    }
                    SyscallReturn::ok(count)
                }
                SYS_READLINK_PATH => {
                    let path = unsafe {
                        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                            frame.arg0 as *const u8,
                            frame.arg1,
                        ))
                    };
                    let payload = if path == "/proc/1/cwd" {
                        b"/".to_vec()
                    } else if let Some(target) = self.symlink_target(path) {
                        target.into_bytes()
                    } else {
                        return SyscallReturn::err(Errno::NoEnt);
                    };
                    let ptr = frame.arg2 as *mut u8;
                    let count = payload.len().min(frame.arg3);
                    unsafe {
                        core::ptr::copy_nonoverlapping(payload.as_ptr(), ptr, count);
                    }
                    SyscallReturn::ok(count)
                }
                SYS_STAT_PATH | SYS_LSTAT_PATH => {
                    let ptr = frame.arg2 as *mut NativeFileStatusRecord;
                    let path = unsafe {
                        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                            frame.arg0 as *const u8,
                            frame.arg1,
                        ))
                    };
                    if !self.path_exists(path) && !path.starts_with("/proc/") {
                        return SyscallReturn::err(Errno::NoEnt);
                    }
                    unsafe {
                        ptr.write(NativeFileStatusRecord {
                            inode: 99,
                            size: 4096,
                            kind: self.created_kind(path).unwrap_or(NativeObjectKind::File) as u32,
                            cloexec: 0,
                            nonblock: 0,
                            readable: 1,
                            writable: 1,
                        });
                    }
                    SyscallReturn::ok(0)
                }
                SYS_STATFS_PATH => {
                    let ptr = frame.arg2 as *mut NativeFileSystemStatusRecord;
                    unsafe {
                        ptr.write(NativeFileSystemStatusRecord {
                            mount_count: 1,
                            node_count: 3,
                            read_only: 0,
                            reserved: 0,
                        });
                    }
                    SyscallReturn::ok(0)
                }
                SYS_INSPECT_DEVICE => {
                    let ptr = frame.arg2 as *mut NativeDeviceRecord;
                    let path = unsafe {
                        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                            frame.arg0 as *const u8,
                            frame.arg1,
                        ))
                    };
                    unsafe {
                        ptr.write(if path == "/dev/storage0" {
                            NativeDeviceRecord {
                                class: 2,
                                state: 1,
                                reserved0: 0,
                                queue_depth: 0,
                                queue_capacity: 128,
                                submitted_requests: 4,
                                completed_requests: 4,
                                total_latency_ticks: 0,
                                max_latency_ticks: 0,
                                total_queue_wait_ticks: 0,
                                max_queue_wait_ticks: 0,
                                link_up: 1,
                                reserved1: 0,
                                block_size: 512,
                                reserved2: 0,
                                capacity_bytes: 128 * 1024 * 1024,
                            }
                        } else if path == "/dev/audio0" {
                            NativeDeviceRecord {
                                class: 4,
                                state: 1,
                                reserved0: 0,
                                queue_depth: 1,
                                queue_capacity: 128,
                                submitted_requests: 6,
                                completed_requests: 5,
                                total_latency_ticks: 48,
                                max_latency_ticks: 16,
                                total_queue_wait_ticks: 12,
                                max_queue_wait_ticks: 4,
                                link_up: 1,
                                reserved1: 0,
                                block_size: 0,
                                reserved2: 0,
                                capacity_bytes: 0,
                            }
                        } else if path == "/dev/input0" {
                            NativeDeviceRecord {
                                class: 5,
                                state: 1,
                                reserved0: 0,
                                queue_depth: 2,
                                queue_capacity: 64,
                                submitted_requests: 8,
                                completed_requests: 7,
                                total_latency_ticks: 28,
                                max_latency_ticks: 8,
                                total_queue_wait_ticks: 10,
                                max_queue_wait_ticks: 3,
                                link_up: 1,
                                reserved1: 0,
                                block_size: 0,
                                reserved2: 0,
                                capacity_bytes: 0,
                            }
                        } else {
                            NativeDeviceRecord {
                                class: 1,
                                state: 1,
                                reserved0: 0,
                                queue_depth: 0,
                                queue_capacity: 256,
                                submitted_requests: 2,
                                completed_requests: 2,
                                total_latency_ticks: 6,
                                max_latency_ticks: 3,
                                total_queue_wait_ticks: 2,
                                max_queue_wait_ticks: 1,
                                link_up: 1,
                                reserved1: 0,
                                block_size: 0,
                                reserved2: 0,
                                capacity_bytes: 0,
                            }
                        });
                    }
                    SyscallReturn::ok(0)
                }
                SYS_INSPECT_DRIVER => {
                    let ptr = frame.arg2 as *mut NativeDriverRecord;
                    let path = unsafe {
                        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                            frame.arg0 as *const u8,
                            frame.arg1,
                        ))
                    };
                    unsafe {
                        ptr.write(if path == "/drv/audio0" {
                            NativeDriverRecord {
                                state: 1,
                                reserved: 0,
                                bound_device_count: 1,
                                queued_requests: 1,
                                in_flight_requests: 1,
                                completed_requests: 5,
                            }
                        } else if path == "/drv/input0" {
                            NativeDriverRecord {
                                state: 1,
                                reserved: 0,
                                bound_device_count: 1,
                                queued_requests: 1,
                                in_flight_requests: 0,
                                completed_requests: 7,
                            }
                        } else {
                            NativeDriverRecord {
                                state: 1,
                                reserved: 0,
                                bound_device_count: 1,
                                queued_requests: 0,
                                in_flight_requests: 0,
                                completed_requests: 4,
                            }
                        });
                    }
                    SyscallReturn::ok(0)
                }
                SYS_INSPECT_DOMAIN => {
                    let ptr = frame.arg1 as *mut NativeDomainRecord;
                    unsafe {
                        ptr.write(NativeDomainRecord {
                            id: 41,
                            owner: 1,
                            parent: 0,
                            resource_count: 1,
                            contract_count: 3,
                        });
                    }
                    SyscallReturn::ok(0)
                }
                SYS_INSPECT_RESOURCE => {
                    let ptr = frame.arg1 as *mut NativeResourceRecord;
                    let frames = self.frames.borrow();
                    let (holder_contract, acquire_count, handoff_count, waiters) =
                        replay_resource_state(&frames, frame.arg0 as u64);
                    unsafe {
                        ptr.write(NativeResourceRecord {
                            id: 42,
                            domain: 41,
                            creator: 1,
                            holder_contract,
                            kind: NativeResourceKind::Device as u32,
                            state: replayed_resource_operational_state(&frames, frame.arg0 as u64),
                            arbitration: NativeResourceArbitrationPolicy::Fifo as u32,
                            governance: replayed_resource_governance(&frames, frame.arg0 as u64),
                            contract_policy: replayed_resource_contract_policy(
                                &frames,
                                frame.arg0 as u64,
                            ),
                            issuer_policy: frames
                                .iter()
                                .rev()
                                .find(|entry| entry.number == SYS_SET_RESOURCE_ISSUER_POLICY)
                                .map(|entry| entry.arg1 as u32)
                                .unwrap_or(NativeResourceIssuerPolicy::AnyIssuer as u32),
                            waiting_count: waiters.len() as u64,
                            acquire_count,
                            handoff_count,
                        });
                    }
                    SyscallReturn::ok(0)
                }
                SYS_LIST_RESOURCE_WAITERS => {
                    let ptr = frame.arg1 as *mut u64;
                    let frames = self.frames.borrow();
                    let (_, _, _, waiters) = replay_resource_state(&frames, frame.arg0 as u64);
                    for (index, waiter) in waiters.iter().take(frame.arg2).enumerate() {
                        unsafe {
                            *ptr.add(index) = *waiter;
                        }
                    }
                    SyscallReturn::ok(waiters.len())
                }
                SYS_SET_RESOURCE_POLICY => SyscallReturn::ok(0),
                SYS_SET_RESOURCE_CONTRACT_POLICY => SyscallReturn::ok(0),
                SYS_SET_RESOURCE_ISSUER_POLICY => SyscallReturn::ok(0),
                SYS_SET_RESOURCE_STATE => SyscallReturn::ok(0),
                SYS_ACQUIRE_RESOURCE => SyscallReturn::ok(1),
                SYS_CLAIM_RESOURCE => {
                    let ptr = frame.arg1 as *mut NativeResourceClaimRecord;
                    let frames = self.frames.borrow();
                    let states = frames
                        .iter()
                        .filter(|entry| entry.number == SYS_SET_CONTRACT_STATE)
                        .map(|entry| (entry.arg0 as u64, entry.arg1 as u32))
                        .collect::<Vec<_>>();
                    if replayed_contract_state(&states, frame.arg0 as u64)
                        != NativeContractState::Active as u32
                    {
                        return SyscallReturn::err(Errno::Access);
                    }
                    let resource = replayed_contract_resource(&frames, frame.arg0 as u64);
                    if replayed_resource_operational_state(&frames, resource)
                        != NativeResourceState::Active as u32
                    {
                        return SyscallReturn::err(Errno::Access);
                    }
                    let (holder_contract, acquire_count, _, waiters) =
                        replay_resource_state(&frames, resource);
                    let contract = frame.arg0 as u64;
                    let policy = replayed_resource_contract_policy(&frames, resource);
                    let contract_kind = replayed_contract_kind(&frames, contract);
                    let allowed_kind = match policy {
                        x if x == NativeResourceContractPolicy::Any as u32 => None,
                        x => Some(x - 1),
                    };
                    if let Some(expected) = allowed_kind
                        && contract_kind != expected
                    {
                        return SyscallReturn::err(Errno::Access);
                    }
                    if holder_contract != 0
                        && holder_contract != contract
                        && replayed_resource_governance(&frames, resource)
                            == NativeResourceGovernanceMode::ExclusiveLease as u32
                    {
                        return SyscallReturn::err(Errno::Busy);
                    }
                    unsafe {
                        if holder_contract == contract {
                            ptr.write(NativeResourceClaimRecord {
                                resource,
                                holder_contract: contract,
                                acquire_count,
                                position: 0,
                                queued: 0,
                                reserved: 0,
                            });
                            self.emit_resource_event(resource, contract, 0);
                        } else {
                            let position = waiters
                                .iter()
                                .position(|waiter| *waiter == contract)
                                .map(|index| index as u64 + 1)
                                .unwrap_or(0);
                            ptr.write(NativeResourceClaimRecord {
                                resource,
                                holder_contract,
                                acquire_count: 0,
                                position,
                                queued: 1,
                                reserved: 0,
                            });
                            self.emit_resource_event(resource, contract, 1);
                        }
                    }
                    SyscallReturn::ok(0)
                }
                SYS_CANCEL_RESOURCE_CLAIM => {
                    let ptr = frame.arg1 as *mut NativeResourceCancelRecord;
                    let frames = self.frames.borrow();
                    let resource = replayed_contract_resource(&frames, frame.arg0 as u64);
                    let (holder_contract, _, _, waiters) =
                        replay_resource_state(&frames[..frames.len().saturating_sub(1)], resource);
                    let contract = frame.arg0 as u64;
                    if holder_contract == contract {
                        return SyscallReturn::err(Errno::Busy);
                    }
                    if !waiters.contains(&contract) {
                        return SyscallReturn::err(Errno::Inval);
                    }
                    let (_, _, _, new_waiters) = replay_resource_state(&frames, resource);
                    unsafe {
                        ptr.write(NativeResourceCancelRecord {
                            resource,
                            waiting_count: new_waiters.len() as u64,
                        });
                    }
                    self.emit_resource_event(resource, contract, 2);
                    SyscallReturn::ok(0)
                }
                SYS_SET_RESOURCE_GOVERNANCE => SyscallReturn::ok(0),
                SYS_RELEASE_CLAIMED_RESOURCE => {
                    let ptr = frame.arg1 as *mut NativeResourceReleaseRecord;
                    let frames = self.frames.borrow();
                    let resource = replayed_contract_resource(&frames, frame.arg0 as u64);
                    let (old_holder, _, old_handoffs, _) =
                        replay_resource_state(&frames[..frames.len().saturating_sub(1)], resource);
                    let (holder_contract, acquire_count, handoff_count, _) =
                        replay_resource_state(&frames, resource);
                    unsafe {
                        if old_holder == frame.arg0 as u64
                            && holder_contract != 0
                            && holder_contract != frame.arg0 as u64
                            && handoff_count > old_handoffs
                        {
                            ptr.write(NativeResourceReleaseRecord {
                                resource,
                                handoff_contract: holder_contract,
                                acquire_count,
                                handoff_count,
                                handed_off: 1,
                                reserved: 0,
                            });
                        } else {
                            ptr.write(NativeResourceReleaseRecord {
                                resource,
                                handoff_contract: 0,
                                acquire_count: 0,
                                handoff_count,
                                handed_off: 0,
                                reserved: 0,
                            });
                        }
                    }
                    self.emit_resource_event(resource, frame.arg0 as u64, 3);
                    if old_holder == frame.arg0 as u64
                        && holder_contract != 0
                        && holder_contract != frame.arg0 as u64
                        && handoff_count > old_handoffs
                    {
                        self.emit_resource_event(resource, holder_contract, 4);
                    }
                    SyscallReturn::ok(0)
                }
                SYS_INSPECT_CONTRACT => {
                    let ptr = frame.arg1 as *mut NativeContractRecord;
                    let frames = self.frames.borrow();
                    let mut state = replayed_contract_state(
                        &frames
                            .iter()
                            .filter(|entry| entry.number == SYS_SET_CONTRACT_STATE)
                            .map(|entry| (entry.arg0 as u64, entry.arg1 as u32))
                            .collect::<Vec<_>>(),
                        frame.arg0 as u64,
                    );
                    let resource = replayed_contract_resource(&frames, frame.arg0 as u64);
                    if replayed_resource_operational_state(&frames, resource)
                        == NativeResourceState::Retired as u32
                    {
                        state = NativeContractState::Revoked as u32;
                    }
                    unsafe {
                        ptr.write(NativeContractRecord {
                            id: frame.arg0 as u64,
                            domain: 41,
                            resource,
                            issuer: 1,
                            kind: replayed_contract_kind(&self.frames.borrow(), frame.arg0 as u64),
                            state,
                        });
                    }
                    SyscallReturn::ok(0)
                }
                SYS_SET_CONTRACT_STATE => {
                    if frame.arg1 as u32 == NativeContractState::Revoked as u32 {
                        let resource =
                            replayed_contract_resource(&self.frames.borrow(), frame.arg0 as u64);
                        self.emit_resource_event(resource, frame.arg0 as u64, 5);
                    }
                    SyscallReturn::ok(0)
                }
                SYS_INVOKE_CONTRACT => {
                    let frames = self.frames.borrow();
                    let resource = replayed_contract_resource(&frames, frame.arg0 as u64);
                    if replayed_resource_operational_state(&frames, resource)
                        != NativeResourceState::Active as u32
                    {
                        return SyscallReturn::err(Errno::Access);
                    }
                    let state = replayed_contract_state(
                        &frames
                            .iter()
                            .filter(|entry| entry.number == SYS_SET_CONTRACT_STATE)
                            .map(|entry| (entry.arg0 as u64, entry.arg1 as u32))
                            .collect::<Vec<_>>(),
                        frame.arg0 as u64,
                    );
                    if state != NativeContractState::Active as u32 {
                        SyscallReturn::err(Errno::Access)
                    } else {
                        let start = frames
                            .iter()
                            .enumerate()
                            .rev()
                            .find(|(_, entry)| {
                                entry.number == SYS_SET_CONTRACT_STATE
                                    && entry.arg1 == NativeContractState::Active as usize
                            })
                            .map(|(index, _)| index)
                            .unwrap_or(0);
                        let count = frames
                            .iter()
                            .skip(start)
                            .filter(|entry| entry.number == SYS_INVOKE_CONTRACT)
                            .count();
                        SyscallReturn::ok(count)
                    }
                }
                SYS_RELEASE_RESOURCE => {
                    let frames = self.frames.borrow();
                    let resource = replayed_contract_resource(&frames, frame.arg0 as u64);
                    SyscallReturn::ok(resource as usize)
                }
                SYS_TRANSFER_RESOURCE => {
                    let frames = self.frames.borrow();
                    let resource = replayed_contract_resource(&frames, frame.arg0 as u64);
                    let (holder_contract, _, _, _) =
                        replay_resource_state(&frames[..frames.len().saturating_sub(1)], resource);
                    if holder_contract == frame.arg0 as u64 {
                        SyscallReturn::ok(resource as usize)
                    } else {
                        SyscallReturn::err(Errno::Inval)
                    }
                }
                SYS_GET_DOMAIN_NAME => {
                    let bytes = b"graphics";
                    let ptr = frame.arg1 as *mut u8;
                    unsafe {
                        core::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
                    }
                    SyscallReturn::ok(bytes.len())
                }
                SYS_GET_RESOURCE_NAME => {
                    let bytes = b"gpu0";
                    let ptr = frame.arg1 as *mut u8;
                    unsafe {
                        core::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
                    }
                    SyscallReturn::ok(bytes.len())
                }
                SYS_GET_CONTRACT_LABEL => {
                    let bytes = b"scanout";
                    let ptr = frame.arg1 as *mut u8;
                    unsafe {
                        core::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
                    }
                    SyscallReturn::ok(bytes.len())
                }
                SYS_FCNTL => match frame.arg1 & 0xff {
                    0 => SyscallReturn::ok(
                        self.event_queue_nonblock
                            .borrow()
                            .iter()
                            .find(|(fd, _)| *fd == frame.arg0)
                            .map(|(_, nonblock)| usize::from(*nonblock))
                            .unwrap_or(0),
                    ),
                    1 => SyscallReturn::ok(2),
                    2 => {
                        let nonblock = (frame.arg1 >> 8) != 0;
                        if let Some((_, value)) = self
                            .event_queue_nonblock
                            .borrow_mut()
                            .iter_mut()
                            .find(|(fd, _)| *fd == frame.arg0)
                        {
                            *value = nonblock;
                        }
                        SyscallReturn::ok(usize::from(nonblock))
                    }
                    3 => SyscallReturn::ok((frame.arg1 >> 8) << 1),
                    _ => SyscallReturn::err(Errno::Inval),
                },
                SYS_POLL => SyscallReturn::ok(frame.arg1),
                SYS_DUP => SyscallReturn::ok(3),
                SYS_CLOSE => {
                    self.open_files
                        .borrow_mut()
                        .retain(|(open_fd, _)| *open_fd != frame.arg0);
                    self.read_offsets
                        .borrow_mut()
                        .retain(|(open_fd, _)| *open_fd != frame.arg0);
                    self.event_queue_pending
                        .borrow_mut()
                        .retain(|(queue_fd, _)| *queue_fd != frame.arg0);
                    self.event_queue_nonblock
                        .borrow_mut()
                        .retain(|(queue_fd, _)| *queue_fd != frame.arg0);
                    self.resource_event_queues
                        .borrow_mut()
                        .retain(|queue_fd| *queue_fd != frame.arg0);
                    self.resource_event_watches
                        .borrow_mut()
                        .retain(|(queue_fd, _, _)| *queue_fd != frame.arg0);
                    SyscallReturn::ok(0)
                }
                SYS_BOOT_REPORT => SyscallReturn::ok(0),
                SYS_WRITE => {
                    if frame.arg0 == 1 || frame.arg0 == 2 {
                        let bytes = unsafe {
                            core::slice::from_raw_parts(frame.arg1 as *const u8, frame.arg2)
                        };
                        self.stdout.borrow_mut().extend_from_slice(bytes);
                    } else if let Some(path) = self.opened_path(frame.arg0) {
                        let bytes = unsafe {
                            core::slice::from_raw_parts(frame.arg1 as *const u8, frame.arg2)
                        };
                        if self.created_kind(&path) == Some(NativeObjectKind::Channel) {
                            self.push_channel_message(&path, bytes);
                            return SyscallReturn::ok(bytes.len());
                        }
                        if !self.service_block_request(&path, bytes)
                            && !self.service_block_completion(&path, bytes)
                        {
                            let offset = self.read_offset(frame.arg0);
                            self.write_file_content_at(&path, offset, bytes);
                            self.set_read_offset(frame.arg0, offset.saturating_add(bytes.len()));
                        }
                    }
                    SyscallReturn::ok(frame.arg2)
                }
                SYS_WRITEV => {
                    if frame.arg0 == 1 || frame.arg0 == 2 {
                        let iovecs = unsafe {
                            core::slice::from_raw_parts(frame.arg1 as *const UserIoVec, frame.arg2)
                        };
                        let mut total = 0usize;
                        for iov in iovecs {
                            let bytes = unsafe {
                                core::slice::from_raw_parts(iov.base as *const u8, iov.len)
                            };
                            self.stdout.borrow_mut().extend_from_slice(bytes);
                            total += bytes.len();
                        }
                        SyscallReturn::ok(total)
                    } else {
                        SyscallReturn::err(Errno::Badf)
                    }
                }
                _ => SyscallReturn::err(Errno::Inval),
            }
        }
    }

    #[test]
    fn native_program_accepts_basic_bootstrap_and_emits_descriptor_syscalls_then_write() {
        let runtime = UserRuntime::new(RecordingBackend::default());
        let argv = ["ngos-userland-native", "--boot"];
        let envp = ["TERM=dumb"];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &[]);
        assert_eq!(main(&runtime, &bootstrap), 0);
        let frames = runtime.backend().frames.borrow();
        let frame_numbers = frames.iter().map(|frame| frame.number).collect::<Vec<_>>();
        let expected_prefix = vec![
            SYS_FCNTL,
            SYS_POLL,
            SYS_DUP,
            SYS_FCNTL,
            SYS_FCNTL,
            SYS_CLOSE,
            SYS_INSPECT_DEVICE,
            SYS_INSPECT_DRIVER,
            SYS_OPEN_PATH,
            SYS_WRITE,
            SYS_OPEN_PATH,
            SYS_POLL,
            SYS_READ,
            SYS_WRITE,
            SYS_POLL,
            SYS_READ,
            SYS_CLOSE,
            SYS_CLOSE,
            SYS_CREATE_DOMAIN,
            SYS_CREATE_RESOURCE,
            SYS_CREATE_CONTRACT,
            SYS_CREATE_CONTRACT,
            SYS_CREATE_CONTRACT,
            SYS_LIST_DOMAINS,
            SYS_LIST_RESOURCES,
            SYS_LIST_CONTRACTS,
            SYS_INSPECT_DOMAIN,
            SYS_INSPECT_RESOURCE,
            SYS_INSPECT_CONTRACT,
            SYS_SET_CONTRACT_STATE,
            SYS_INSPECT_CONTRACT,
            SYS_INVOKE_CONTRACT,
            SYS_SET_CONTRACT_STATE,
            SYS_INSPECT_CONTRACT,
            SYS_INVOKE_CONTRACT,
            SYS_SET_RESOURCE_POLICY,
            SYS_CLAIM_RESOURCE,
            SYS_CLAIM_RESOURCE,
            SYS_CLAIM_RESOURCE,
            SYS_LIST_RESOURCE_WAITERS,
            SYS_CANCEL_RESOURCE_CLAIM,
            SYS_LIST_RESOURCE_WAITERS,
            SYS_INSPECT_RESOURCE,
            SYS_RELEASE_CLAIMED_RESOURCE,
            SYS_INSPECT_RESOURCE,
            SYS_TRANSFER_RESOURCE,
            SYS_LIST_RESOURCE_WAITERS,
            SYS_RELEASE_RESOURCE,
            SYS_INSPECT_RESOURCE,
            SYS_CLAIM_RESOURCE,
            SYS_CLAIM_RESOURCE,
            SYS_SET_CONTRACT_STATE,
            SYS_LIST_RESOURCE_WAITERS,
            SYS_INSPECT_RESOURCE,
            SYS_RELEASE_CLAIMED_RESOURCE,
            SYS_CLAIM_RESOURCE,
            SYS_SET_CONTRACT_STATE,
            SYS_CLAIM_RESOURCE,
            SYS_SET_CONTRACT_STATE,
            SYS_INSPECT_RESOURCE,
            SYS_SET_RESOURCE_GOVERNANCE,
            SYS_INSPECT_RESOURCE,
            SYS_CLAIM_RESOURCE,
            SYS_CLAIM_RESOURCE,
            SYS_INSPECT_RESOURCE,
            SYS_RELEASE_CLAIMED_RESOURCE,
            SYS_CREATE_CONTRACT,
            SYS_SET_RESOURCE_CONTRACT_POLICY,
            SYS_INSPECT_RESOURCE,
            SYS_CLAIM_RESOURCE,
            SYS_CLAIM_RESOURCE,
            SYS_RELEASE_CLAIMED_RESOURCE,
            SYS_CREATE_CONTRACT,
            SYS_SET_RESOURCE_ISSUER_POLICY,
            SYS_INSPECT_RESOURCE,
            SYS_CREATE_CONTRACT,
            SYS_SET_RESOURCE_STATE,
            SYS_INSPECT_RESOURCE,
            SYS_CLAIM_RESOURCE,
            SYS_INVOKE_CONTRACT,
            SYS_CREATE_CONTRACT,
            SYS_SET_RESOURCE_STATE,
            SYS_CREATE_CONTRACT,
            SYS_SET_RESOURCE_STATE,
            SYS_INSPECT_RESOURCE,
            SYS_INSPECT_CONTRACT,
            SYS_CREATE_CONTRACT,
            SYS_GET_DOMAIN_NAME,
            SYS_GET_RESOURCE_NAME,
            SYS_GET_CONTRACT_LABEL,
            SYS_WRITE,
        ];
        let mut cursor = 0usize;
        for expected in expected_prefix {
            let Some(found) = frame_numbers[cursor..]
                .iter()
                .position(|number| *number == expected)
            else {
                panic!("missing expected syscall {expected} after position {cursor}");
            };
            cursor += found + 1;
        }
        assert!(frame_numbers.contains(&SYS_MAP_ANONYMOUS_MEMORY));
        assert!(frame_numbers.contains(&SYS_MAP_FILE_MEMORY));
        assert!(frame_numbers.contains(&SYS_SPAWN_PROCESS_COPY_VM));
        assert!(frame_numbers.ends_with(&[SYS_WRITE]));
        let first_poll = frames
            .iter()
            .find(|frame| frame.number == SYS_POLL)
            .unwrap();
        assert_eq!(first_poll.arg1 as u32, POLLOUT);
        let first_open = frames
            .iter()
            .find(|frame| frame.number == SYS_OPEN_PATH)
            .unwrap();
        assert_eq!(first_open.arg1, "/dev/storage0".len());
        let stdout_write = frames
            .iter()
            .find(|frame| frame.number == SYS_WRITE && frame.arg0 == 1)
            .unwrap();
        assert!(stdout_write.arg2 > 0);
    }

    #[test]
    fn native_program_accepts_bootstrap_with_framebuffer_metadata() {
        let runtime = UserRuntime::new(RecordingBackend::default());
        let argv = ["ngos-userland-native", "--boot"];
        let envp = [
            "NGOS_BOOT=1",
            "NGOS_BOOT_PROTOCOL=limine",
            "NGOS_BOOT_MODULE=ngos-userland-native",
            "NGOS_BOOT_MODULE_LEN=12288",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_BOOT_MODULE_PHYS_START=0x200000",
            "NGOS_BOOT_MODULE_PHYS_END=0x203000",
            "NGOS_IMAGE_PATH=ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
            "NGOS_FRAMEBUFFER_PRESENT=1",
            "NGOS_FRAMEBUFFER_WIDTH=1920",
            "NGOS_FRAMEBUFFER_HEIGHT=1080",
            "NGOS_FRAMEBUFFER_PITCH=7680",
            "NGOS_FRAMEBUFFER_BPP=32",
            "NGOS_MEMORY_REGION_COUNT=2",
            "NGOS_USABLE_MEMORY_BYTES=8388608",
            "NGOS_PHYSICAL_MEMORY_OFFSET=0x0",
            "NGOS_RSDP=0xdeadbeef",
            "NGOS_KERNEL_PHYS_START=0x100000",
            "NGOS_KERNEL_PHYS_END=0x101000",
            "NGOS_BOOT_OUTCOME_POLICY=require-zero-exit",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
    }

    #[test]
    fn native_program_accepts_bootstrap_with_allow_any_exit_policy() {
        let runtime = UserRuntime::new(RecordingBackend::default());
        let argv = ["ngos-userland-native", "--boot"];
        let envp = [
            "NGOS_BOOT=1",
            "NGOS_BOOT_PROTOCOL=limine",
            "NGOS_BOOT_MODULE=ngos-userland-native",
            "NGOS_BOOT_MODULE_LEN=12288",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_BOOT_MODULE_PHYS_START=0x200000",
            "NGOS_BOOT_MODULE_PHYS_END=0x203000",
            "NGOS_IMAGE_PATH=ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
            "NGOS_FRAMEBUFFER_PRESENT=1",
            "NGOS_FRAMEBUFFER_WIDTH=1920",
            "NGOS_FRAMEBUFFER_HEIGHT=1080",
            "NGOS_FRAMEBUFFER_PITCH=7680",
            "NGOS_FRAMEBUFFER_BPP=32",
            "NGOS_MEMORY_REGION_COUNT=2",
            "NGOS_USABLE_MEMORY_BYTES=8388608",
            "NGOS_PHYSICAL_MEMORY_OFFSET=0x0",
            "NGOS_RSDP=0xdeadbeef",
            "NGOS_KERNEL_PHYS_START=0x100000",
            "NGOS_KERNEL_PHYS_END=0x101000",
            "NGOS_BOOT_OUTCOME_POLICY=allow-any-exit",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
    }

    #[test]
    fn native_program_accepts_bootstrap_with_kernel_image_path() {
        let runtime = UserRuntime::new(RecordingBackend::default());
        let argv = ["ngos-userland-native", "--boot"];
        let envp = [
            "NGOS_BOOT=1",
            "NGOS_BOOT_PROTOCOL=limine",
            "NGOS_BOOT_MODULE=ngos-userland-native",
            "NGOS_BOOT_MODULE_LEN=12288",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_BOOT_MODULE_PHYS_START=0x200000",
            "NGOS_BOOT_MODULE_PHYS_END=0x203000",
            "NGOS_IMAGE_PATH=/kernel/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
            "NGOS_FRAMEBUFFER_PRESENT=1",
            "NGOS_FRAMEBUFFER_WIDTH=1920",
            "NGOS_FRAMEBUFFER_HEIGHT=1080",
            "NGOS_FRAMEBUFFER_PITCH=7680",
            "NGOS_FRAMEBUFFER_BPP=32",
            "NGOS_MEMORY_REGION_COUNT=2",
            "NGOS_USABLE_MEMORY_BYTES=8388608",
            "NGOS_PHYSICAL_MEMORY_OFFSET=0x0",
            "NGOS_RSDP=0xdeadbeef",
            "NGOS_KERNEL_PHYS_START=0x100000",
            "NGOS_KERNEL_PHYS_END=0x101000",
            "NGOS_BOOT_OUTCOME_POLICY=require-zero-exit",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
    }

    #[test]
    fn boot_desktop_frame_uses_framebuffer_dimensions_and_presents_graphics_queue() {
        let framebuffer = ngos_user_abi::bootstrap::FramebufferContext {
            width: 1920,
            height: 1080,
            pitch: 7680,
            bpp: 32,
        };

        let frame = build_boot_desktop_frame(&framebuffer).expect("desktop frame should build");

        assert_eq!(frame.width, 1920);
        assert_eq!(frame.height, 1080);
        assert_eq!(frame.frame_tag, "ngos-desktop-boot");
        assert_eq!(frame.queue, "graphics");
        assert_eq!(frame.present_mode, "mailbox");
        assert_eq!(frame.completion, "wait-present");
        assert!(frame.ops.len() >= 35);
    }

    #[test]
    fn native_program_reports_kernel_launch_session_stages() {
        let runtime = UserRuntime::new(RecordingBackend::default());
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let frames = runtime.backend().frames.borrow();
        let reports = frames
            .iter()
            .filter(|frame| frame.number == SYS_BOOT_REPORT)
            .collect::<Vec<_>>();
        assert_eq!(reports.len(), 3);
        assert_eq!(reports[0].arg0, BootSessionStatus::Success as usize);
        assert_eq!(reports[0].arg1, BootSessionStage::Bootstrap as usize);
        assert_eq!(reports[1].arg1, BootSessionStage::NativeRuntime as usize);
        assert_eq!(reports[2].arg0, BootSessionStatus::Success as usize);
        assert_eq!(reports[2].arg1, BootSessionStage::Complete as usize);
    }

    #[test]
    fn native_program_runs_kernel_launch_shell_script_from_stdin() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"help\n# comment line\npwd; env\nset NOTE shell-note\nvars\nalias ll list-path .\nalias say echo $NOTE\naliases\nsession\nps\nprocess-info 1\nkill 1 9\npending-signals 1\nblocked-signals 1\nspawn-path worker /bin/worker\njobs\njob-info 77\nfg 77\njobs\nproc 1 maps\nstatus\nfd\nfdinfo 0\nstat-path /proc/1/status\nlstat-path /proc/1/status\nstatfs-path /proc/1/status\nopen-path /proc/1/status\nreadlink-path /proc/1/cwd\ncat-file /motd\nmkdir-path /shell-tmp\ncd /shell-tmp\npwd\nmkfile-path note\nwrite-file note $NOTE\nappend-file note -extra\ngrep-file note shell\nassert-file-contains note shell-note-extra\ncat-file note\nmkfile-path copy\ncopy-file note copy\ncmp-file note copy\ncat-file copy\nmkfile-path script\nwrite-file script echo sourced-script\nsource-file script\nfalse || echo recovered\nassert-status 0\nrepeat 2 echo looped\ntrue && echo chained\nfalse && echo skipped\ntrue || echo skipped-2\nlast-status\nsymlink-path current-note note\nrename-path note note-2\nll\nreadlink-path current-note\nunalias ll\nunset NOTE\nhistory\nunlink-path current-note\ncd /\ndomains\ndomain 41\nresources\nresource 42\nwaiters 42\ncontracts\ncontract 43\nmkdomain render\nmkresource 41 device gpu1\nmkcontract 41 42 display mirror-2\nresource-policy 42 lifo\nresource-governance 42 exclusive-lease\nresource-contract-policy 42 display\nresource-issuer-policy 42 creator-only\nclaim 43\nwaiters 42\nreleaseclaim 43\ncontract-state 43 suspended\ncontract-state 43 active\nresource-state 42 suspended\nresource-state 42 active\ncat /proc/1/fd\nsay\necho hello-kernel\nexit 7\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 7);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("ngos shell"));
        assert!(stdout.contains("help session mode pwd env cd alias unalias"));
        assert!(stdout.contains("game-manifest game-plan game-launch game-simulate"));
        assert!(stdout.contains("game-simulate game-sessions"));
        assert!(stdout.contains("game-next"));
        assert!(stdout.contains("game-gfx-plan game-gfx-submit game-gfx-status"));
        assert!(stdout.contains("game-gfx-next"));
        assert!(stdout.contains("game-audio-plan game-audio-submit game-audio-status"));
        assert!(stdout.contains("game-audio-next"));
        assert!(stdout.contains("game-input-plan game-input-submit game-input-status"));
        assert!(
            stdout.contains("game-watch-start game-watch-status game-watch-status-all game-watch-poll-all game-watch-wait game-watch-stop")
        );
        assert!(stdout.contains("game-watch-start"));
        assert!(stdout.contains("game-watch-status"));
        assert!(stdout.contains("game-watch-status-all"));
        assert!(stdout.contains("game-watch-poll-all"));
        assert!(stdout.contains("game-watch-wait"));
        assert!(stdout.contains("game-watch-stop"));
        assert!(stdout.contains("var NOTE=shell-note"));
        assert!(stdout.contains("alias ll='list-path .'"));
        assert!(stdout.contains("alias say='echo shell-note'"));
        assert!(stdout.contains("pid=1 name=ngos-userland-native image=/bin/ngos-userland-native cwd=/ parent=0 address-space=4 thread=1 state=Running exit=0 fds=3 caps=2 env=1 regions=2 threads=1 pending=1 session-reported=0 session-status=0 session-stage=0 scheduler-class=interactive scheduler-budget=2"));
        assert!(stdout.contains("signal-sent pid=1 signal=9"));
        assert!(stdout.contains("pid=1 pending-signals=9"));
        assert!(stdout.contains("pid=1 blocked-pending-signals=-"));
        assert!(stdout.contains("process-spawned pid=77 name=worker path=/bin/worker"));
        assert!(
            stdout.contains("job pid=77 name=worker path=/bin/worker state=live:Exited signals=0")
        );
        assert!(stdout.contains("job-info pid=77 name=worker path=/bin/worker state=live:Exited signals=0 exit=137 pending=0"));
        assert!(stdout.contains("foreground-complete pid=77 exit=137"));
        assert!(
            stdout.contains("job pid=77 name=worker path=/bin/worker state=reaped:137 signals=0")
        );
        assert!(stdout.contains("\n/\n"));
        assert!(stdout.contains("outcome_policy=require-zero-exit"));
        assert!(stdout.contains("protocol=kernel-launch"));
        assert!(stdout.contains("pid=1 name=ngos-userland-native state=Running cwd=/"));
        assert!(stdout.contains("/bin/ngos-userland-native"));
        assert!(stdout.contains("fd:\t0"));
        assert!(stdout.contains("path=/proc/1/status kind=file inode=99 size=4096"));
        assert!(stdout.contains("path=/proc/1/status mounts=1 nodes=3 read_only=0"));
        assert!(stdout.contains("opened path=/proc/1/status fd=7"));
        assert!(stdout.contains("link /shell-tmp/current-note -> /shell-tmp/note"));
        assert!(stdout.contains("ngos host motd"));
        assert!(stdout.contains("directory-created path=/shell-tmp"));
        assert!(stdout.contains("cwd-updated path=/shell-tmp"));
        assert!(stdout.contains("file-created path=/shell-tmp/note"));
        assert!(stdout.contains("file-written path=/shell-tmp/note bytes=10"));
        assert!(stdout.contains("file-appended path=/shell-tmp/note bytes=6"));
        assert!(stdout.contains("grep-summary path=/shell-tmp/note needle=shell matches=1"));
        assert!(
            stdout.contains("assert-file-contains-ok path=/shell-tmp/note needle=shell-note-extra")
        );
        assert!(stdout.contains("shell-note-extra"));
        assert!(stdout.contains("file-created path=/shell-tmp/copy"));
        assert!(stdout.contains("file-copied from=/shell-tmp/note to=/shell-tmp/copy bytes=16"));
        assert!(stdout.contains("files-match left=/shell-tmp/note right=/shell-tmp/copy bytes=16"));
        assert!(stdout.contains("file-created path=/shell-tmp/script"));
        assert!(stdout.contains("file-written path=/shell-tmp/script bytes=19"));
        assert!(stdout.contains("script-loaded path=/shell-tmp/script"));
        assert!(stdout.contains("sourced-script"));
        assert!(stdout.contains("recovered"));
        assert!(stdout.contains("assert-status-ok expected=0"));
        assert!(stdout.contains("repeat-expanded count=2"));
        assert!(stdout.matches("looped").count() >= 2);
        assert!(stdout.contains("chained"));
        assert!(!stdout.contains("skipped\n"));
        assert!(!stdout.contains("skipped-2\n"));
        assert!(stdout.contains("last-status=0"));
        assert!(
            stdout.contains("symlink-created path=/shell-tmp/current-note target=/shell-tmp/note")
        );
        assert!(stdout.contains("path-renamed from=/shell-tmp/note to=/shell-tmp/note-2"));
        assert!(stdout.contains("note-2\tFile"));
        assert!(stdout.contains("link /shell-tmp/current-note -> /shell-tmp/note"));
        assert!(stdout.contains("history 1 help"));
        assert!(stdout.contains("history 2 pwd"));
        assert!(stdout.contains("history "));
        assert!(stdout.contains("path-unlinked path=/shell-tmp/current-note"));
        assert!(stdout.contains("domain id=41 owner=1 resources=1 contracts=3 name=graphics"));
        assert!(
            stdout.contains("domain id=41 owner=1 parent=0 resources=1 contracts=3 name=graphics")
        );
        assert!(stdout.contains(
            "resource id=42 domain=41 kind=device state=active holder=0 waiters=0 name=gpu0"
        ));
        assert!(stdout.contains("resource id=42 domain=41 creator=1 kind=device state=active arbitration=fifo governance=queueing"));
        assert!(stdout.contains("resource=42 waiters=-"));
        assert!(stdout.contains(
            "contract id=43 domain=41 resource=42 issuer=1 kind=display state=active label=scanout"
        ));
        assert!(stdout.contains("domain-created id=41 name=render"));
        assert!(stdout.contains("resource-created id=42 domain=41 kind=device name=gpu1"));
        assert!(
            stdout.contains(
                "contract-created id=43 domain=41 resource=42 kind=display label=mirror-2"
            )
        );
        assert!(stdout.contains("resource-policy-updated id=42 policy=lifo"));
        assert!(stdout.contains("resource-governance-updated id=42 mode=exclusive-lease"));
        assert!(stdout.contains("resource-contract-policy-updated id=42 policy=display"));
        assert!(stdout.contains("resource-issuer-policy-updated id=42 policy=creator-only"));
        assert!(stdout.contains("claim-acquired contract=43 resource=42 acquire_count=1"));
        assert!(stdout.contains("claim-released contract=43 resource=42"));
        assert!(stdout.contains("contract-state-updated id=43 state=suspended"));
        assert!(stdout.contains("contract-state-updated id=43 state=active"));
        assert!(stdout.contains("resource-state-updated id=42 state=suspended"));
        assert!(stdout.contains("resource-state-updated id=42 state=active"));
        assert!(stdout.contains("0 [stdio:stdin]"));
        assert!(stdout.contains("hello-kernel"));
    }

    #[test]
    fn native_program_runs_vm_pressure_global_command_from_shell() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"vm-pressure-global 5\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("vm-pressure-global target-pages=5 reclaimed-pages=0"));

        let frames = runtime.backend().frames.borrow();
        assert!(frames.iter().any(|frame| {
            frame.number == SYS_RECLAIM_MEMORY_PRESSURE_GLOBAL && frame.arg0 == 5
        }));
    }

    #[test]
    fn native_shell_can_watch_and_unwatch_resource_events_through_queue_interface() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"queue-create epoll\nresource-watch $LAST_QUEUE_FD 42 900 all\nclaim 43\nqueue-wait $LAST_QUEUE_FD\nresource-unwatch $LAST_QUEUE_FD 42 900\nresource-watch $LAST_QUEUE_FD 42 901 queued\nclaim 44\nqueue-wait $LAST_QUEUE_FD\nlast-status\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("queue-created fd="));
        assert!(stdout.contains("resource-watch queue="));
        assert!(stdout.contains("token=900 kinds=all"));
        assert!(stdout.contains("claim-acquired contract=43 resource=42 acquire_count=1"));
        assert!(stdout.contains("queue-event queue="));
        assert!(stdout.contains("source=resource id=42 contract=43 kind=claimed"));
        assert!(stdout.contains("resource-unwatch queue="));
        assert!(stdout.contains("token=901 kinds=queued"));
        assert!(stdout.contains("claim-queued contract=44 resource=42 holder=43 position=1"));
        assert!(stdout.contains("source=resource id=42 contract=44 kind=queued"));
        assert!(stdout.contains("last-status=0"));
    }

    #[test]
    fn native_shell_rejects_invalid_resource_watch_kind_list() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"queue-create epoll\nresource-watch $LAST_QUEUE_FD 42 900 invalid-kind\nlast-status\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("queue-created fd="));
        assert!(stdout.contains(
            "usage: resource-watch <queue-fd> <resource> <token> [all|claimed,queued,canceled,released,handed-off,revoked]"
        ));
        assert!(stdout.contains("last-status=2"));
    }

    #[test]
    fn native_shell_controls_fd_flags_and_observes_empty_resource_queue_nonblocking() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"queue-create epoll\nfcntl-getfl $LAST_QUEUE_FD\nfcntl-getfd $LAST_QUEUE_FD\nnonblock-fd $LAST_QUEUE_FD on\nfcntl-getfl $LAST_QUEUE_FD\ncloexec-fd $LAST_QUEUE_FD on\nfcntl-getfd $LAST_QUEUE_FD\nresource-watch $LAST_QUEUE_FD 42 900 queued\nresource-unwatch $LAST_QUEUE_FD 42 900\nqueue-wait $LAST_QUEUE_FD\nlast-status\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("fcntl-getfl fd="));
        assert!(stdout.contains("flags=0x0"));
        assert!(stdout.contains("nonblock-fd fd="));
        assert!(stdout.contains("nonblock=1"));
        assert!(stdout.contains("flags=0x2"));
        assert!(stdout.contains("cloexec-fd fd="));
        assert!(stdout.contains("cloexec=1"));
        assert!(stdout.contains("resource-watch queue="));
        assert!(stdout.contains("resource-unwatch queue="));
        assert!(stdout.contains("last-status=246"));
    }

    #[test]
    fn native_shell_runs_vfs_smoke_command_and_reports_vfs_markers() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(b"vfs-smoke\nexit 0\n"));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("vfs.smoke.mount pid=1 path=/vfs"));
        assert!(stdout.contains("vfs.smoke.create pid=1 path=/vfs/bin/app"));
        assert!(stdout.contains("vfs.smoke.symlink pid=1 path=/vfs/link target=/vfs/bin/app"));
        assert!(stdout.contains("vfs.smoke.rename pid=1 from=/vfs/bin/app to=/vfs/bin/app2"));
        assert!(stdout.contains("vfs.smoke.unlink pid=1 path=/vfs/link after-unlink=missing"));
        assert!(stdout.contains("vfs.smoke.coherence pid=1 descriptor=open-path-open"));
        assert!(stdout.contains("vfs-smoke-ok"));
    }

    #[test]
    fn native_shell_runs_wasm_smoke_command_and_reports_wasm_markers() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(b"wasm-smoke\nexit 0\n"));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("wasm.smoke.start component=semantic-observer pid=1"));
        assert!(stdout.contains("wasm.smoke.refusal component=semantic-observer missing=observe-system-process-count outcome=expected"));
        assert!(stdout.contains("wasm.smoke.grants component=semantic-observer grants=observe-process-capability-count,observe-system-process-count"));
        assert!(stdout.contains(
            "wasm.smoke.observe component=semantic-observer pid=1 capabilities=2 processes=2"
        ));
        assert!(stdout.contains("wasm.smoke.recovery component=semantic-observer refusal=observe-system-process-count recovered=yes verdict=ready"));
        assert!(
            stdout
                .contains("wasm.smoke.result component=semantic-observer verdict=ready outcome=ok")
        );
        assert!(stdout.contains("wasm.smoke.start component=process-identity pid=1"));
        assert!(stdout.contains("wasm.smoke.refusal component=process-identity missing=observe-process-cwd-root outcome=expected"));
        assert!(stdout.contains("wasm.smoke.grants component=process-identity grants=observe-process-status-bytes,observe-process-cwd-root"));
        assert!(
            stdout.contains("wasm.smoke.observe component=process-identity pid=1 status-bytes=")
        );
        assert!(stdout.contains(
            "wasm.smoke.recovery component=process-identity refusal=observe-process-cwd-root recovered=yes verdict=ready"
        ));
        assert!(
            stdout
                .contains("wasm.smoke.result component=process-identity verdict=ready outcome=ok")
        );
        assert!(stdout.contains("wasm-smoke-ok"));
    }

    #[test]
    fn native_shell_can_transfer_and_release_resource_through_contract_commands() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"claim 43\nclaim 44\ntransfer 43 44\nresource 42\nrelease 44\nresource 42\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("claim-acquired contract=43 resource=42 acquire_count=1"));
        assert!(stdout.contains("claim-queued contract=44 resource=42 holder=43 position=1"));
        assert!(stdout.contains("resource-transferred source=43 target=44 resource=42"));
        assert!(stdout.contains("resource id=42 domain=41 creator=1 kind=device state=active arbitration=fifo governance=queueing"));
        assert!(stdout.contains("holder=44"));
        assert!(stdout.contains("resource-released contract=44 resource=42"));
        assert!(stdout.contains("holder=0"));
    }

    #[test]
    fn native_shell_reports_resource_governance_refusal_and_recovery_semantically() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"claim 43\nresource-governance 42 exclusive-lease\nclaim 44\nlast-status\nresource-governance 42 queueing\nclaim 44\nreleaseclaim 43\nresource 42\nrelease 44\nresource 42\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("claim-acquired contract=43 resource=42 acquire_count=1"));
        assert!(stdout.contains("resource-governance-updated id=42 mode=exclusive-lease"));
        assert!(stdout.contains("claim-refused contract=44 errno=EBUSY code=16"));
        assert!(stdout.contains("last-status=241"));
        assert!(stdout.contains("resource-governance-updated id=42 mode=queueing"));
        assert!(stdout.contains("claim-queued contract=44 resource=42 holder=43 position=1"));
        assert!(
            stdout.contains("claim-handed-off resource=42 to=44 acquire_count=2 handoff_count=1")
        );
        assert!(stdout.contains("holder=44"));
        assert!(stdout.contains("resource-released contract=44 resource=42"));
        assert!(stdout.contains("holder=0"));
    }

    #[test]
    fn native_shell_reports_contract_and_resource_state_refusal_and_recovery_semantically() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"contract-state 43 suspended\ninvoke 43\nlast-status\ncontract-state 43 active\ninvoke 43\nresource-state 42 suspended\nclaim 43\nlast-status\nresource-state 42 active\nclaim 43\nreleaseclaim 43\nresource 42\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("contract-state-updated id=43 state=suspended"));
        assert!(stdout.contains("invoke-refused contract=43 errno=EACCES code=13"));
        assert!(stdout.contains("last-status=244"));
        assert!(stdout.contains("contract-state-updated id=43 state=active"));
        assert!(stdout.contains("invoked contract=43 count=1"));
        assert!(stdout.contains("resource-state-updated id=42 state=suspended"));
        assert!(stdout.contains("claim-refused contract=43 errno=EACCES code=13"));
        assert!(stdout.contains("resource-state-updated id=42 state=active"));
        assert!(stdout.contains("claim-acquired contract=43 resource=42 acquire_count=1"));
        assert!(stdout.contains("claim-released contract=43 resource=42"));
        assert!(stdout.contains("holder=0"));
    }

    #[test]
    fn native_program_runs_signal_commands_from_stdin() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"kill 1 9\npending-signals 1\nblocked-signals 1\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("signal-sent pid=1 signal=9"));
        assert!(stdout.contains("pid=1 pending-signals=9"));
        assert!(stdout.contains("pid=1 blocked-pending-signals=-"));
    }

    #[test]
    fn native_shell_unifies_direct_and_semantic_control_modes() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"mode semantic\nobserve system\nrenice 1 background 1\npause 1\nresume 1\nintent optimize process 1\nlearn\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("mode=semantic"));
        assert!(stdout.contains("fact process pid=1 name=ngos-userland-native state=Running class=interactive budget=2 cwd=/ image=/bin/ngos-userland-native"));
        assert!(stdout.contains("fact device path=/dev/net0"));
        assert!(stdout.contains("process-control pid=1"));
        assert!(stdout.contains("class=background budget=1"));
        assert!(stdout.contains("class=latency-critical budget=4"));
        assert!(stdout.contains("learn subject=process:1 action=renice policy-epoch="));
        assert!(stdout.contains("learn subject=process:1 action=pause policy-epoch="));
        assert!(stdout.contains("learn subject=process:1 action=resume policy-epoch="));
        assert!(stdout.contains("learn subject=process:1 action=optimize policy-epoch="));
    }

    #[test]
    fn native_shell_nextmind_optimizes_mixed_pressure_and_explains_actions() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"mode semantic\nnextmind.optimize\nnextmind.observe\nnextmind.explain last\nnextmind.auto on\ntrue\nnextmind.auto off\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("nextmind.metrics label=before state="));
        assert!(stdout.contains("nextmind.metrics label=after state="));
        assert!(stdout.contains("nextmind.metrics label=current state="));
        assert!(stdout.contains("nextmind.action reason="));
        assert!(stdout.contains("nextmind.verdict=improved"));
        assert!(stdout.contains("nextmind.explain trigger="));
        assert!(stdout.contains("verdict=improved thresholds=runq>3,cpu>=75,socket>=80,event>=75"));
        assert!(stdout.contains("nextmind.auto=on"));
        assert!(stdout.contains("nextmind.auto trigger="));
        assert!(stdout.contains("nextmind.auto=off"));
    }

    #[test]
    fn native_shell_runs_game_compat_launch_lifecycle() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-manifest /games/orbit.manifest\ngame-plan /games/orbit.manifest\ngame-launch /games/orbit.manifest\ngame-status\ngame-stop 77\ngame-status\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("game.manifest path=/games/orbit.manifest title=Orbit Runner slug=orbit-runner exec=/bin/worker cwd=/games/orbit argv=--fullscreen"));
        assert!(stdout.contains("game.gfx backend=vulkan profile=frame-pace"));
        assert!(stdout.contains("game.audio backend=native-mixer profile=spatial-mix"));
        assert!(stdout.contains("game.input backend=native-input profile=gamepad-first"));
        assert!(stdout.contains("game.plan domain=compat-game-orbit-runner process=game-orbit-runner cwd=/games/orbit exec=/bin/worker"));
        assert!(stdout.contains(
            "game.plan.lane kind=graphics resource=orbit-runner-gfx contract=frame-pace-display"
        ));
        assert!(stdout.contains(
            "game.plan.lane kind=audio resource=orbit-runner-audio contract=spatial-mix-mix"
        ));
        assert!(stdout.contains(
            "game.plan.lane kind=input resource=orbit-runner-input contract=gamepad-first-capture"
        ));
        assert!(stdout.contains("game.plan.env NGOS_COMPAT_PREFIX=/compat/orbit"));
        assert!(stdout.contains("game.session pid=77 title=Orbit Runner slug=orbit-runner"));
        assert!(stdout.contains(
            "game.session.shim prefix=/compat/orbit saves=/saves/orbit cache=/cache/orbit"
        ));
        assert!(stdout.contains("game.session.lane kind=graphics"));
        assert!(stdout.contains("game.session.lane kind=audio"));
        assert!(stdout.contains("game.session.lane kind=input"));
        assert!(
            stdout.contains("game.session pid=77 title=Orbit Runner slug=orbit-runner domain=")
        );
        assert!(stdout.contains("stopped=true exit="));
    }

    #[test]
    fn native_shell_translates_and_submits_game_graphics_frame() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\nmkfile-path /games/orbit.frame\nappend-line /games/orbit.frame surface=1280x720\nappend-line /games/orbit.frame frame=orbit-001\nappend-line /games/orbit.frame queue=graphics\nappend-line /games/orbit.frame present-mode=mailbox\nappend-line /games/orbit.frame completion=wait-complete\nappend-line /games/orbit.frame clear=#112233\nappend-line /games/orbit.frame line=0,0,1279,719,#44ccffff\nappend-line /games/orbit.frame rect=10,20,200,100,#ff8800ff\nappend-line /games/orbit.frame sprite=ship-main,400,220,96,96\nappend-line /games/orbit.frame blit=hud-overlay,0,0,1280,64\ngame-launch /games/orbit.manifest\ngame-gfx-plan 77 /games/orbit.frame\ngame-gfx-submit 77 /games/orbit.frame\ncat-file /compat/orbit/session.chan\ngame-status\ngame-gfx-status 77\ngame-gfx-next 77\ngame-gfx-next 77\nlast-status\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("game.gfx.plan pid=77 frame=orbit-001 ops=5"));
        assert!(stdout.contains("queue=graphics present-mode=mailbox completion=wait-complete"));
        assert!(stdout.contains("gpu-submit device=/dev/gpu0"));
        assert!(stdout.contains("queue-event queue="));
        assert!(stdout.contains("gpu-complete driver=/drv/gpu0"));
        assert!(stdout.contains("kind=graphics tag=orbit-001"));
        assert!(stdout.contains("game.gfx.submit pid=77 frame=orbit-001 ops=5"));
        assert!(stdout.contains(
            "game.gfx.status pid=77 device=/dev/gpu0 driver=/drv/gpu0 profile=frame-pace submitted=1 frames=0 presented=false last-frame=orbit-001 queue=graphics present-mode=mailbox completion=wait-complete completion-observed=graphics-event-complete ops=5"
        ));
        assert!(stdout.contains("game.session.gfx-queue pid=77 depth=1"));
        assert!(stdout.contains("game.gfx.next pid=77 frame=orbit-001 queue=graphics present-mode=mailbox completion=wait-complete remaining=0 payload=ngos-gfx-translate/v1"));
        assert!(stdout.contains("game.gfx.queue pid=77 depth=0"));
        assert!(stdout.contains("last-status=299"));
    }

    #[test]
    fn native_shell_rejects_stopped_game_graphics_submit_and_clears_pending_payloads() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\nmkfile-path /games/orbit.frame\nappend-line /games/orbit.frame surface=1280x720\nappend-line /games/orbit.frame frame=orbit-001\nappend-line /games/orbit.frame queue=graphics\nappend-line /games/orbit.frame present-mode=mailbox\nappend-line /games/orbit.frame completion=wait-complete\nappend-line /games/orbit.frame clear=#112233\nappend-line /games/orbit.frame rect=10,20,200,100,#ff8800ff\ngame-launch /games/orbit.manifest\ngame-gfx-submit 77 /games/orbit.frame\ngame-stop 77\ngame-gfx-submit 77 /games/orbit.frame\ngame-next 77\ngame-status\nlast-status\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("game.gfx.submit pid=77 frame=orbit-001 ops=2"));
        assert!(stdout.contains("game.next pid=77 depth[gfx=0;audio=0;input=0]"));
        assert!(stdout.contains("game.session.gfx-queue pid=77 depth=0"));
        assert!(stdout.contains("game.session.audio-queue pid=77 depth=0"));
        assert!(stdout.contains("game.session.input-queue pid=77 depth=0"));
        assert!(stdout.contains("stopped=true exit="));
        assert!(stdout.contains("last-status=0"));
    }

    #[test]
    fn native_shell_translates_and_submits_game_audio_mix() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\nmkfile-path /games/orbit.mix\nappend-line /games/orbit.mix rate=48000\nappend-line /games/orbit.mix channels=2\nappend-line /games/orbit.mix stream=orbit-intro\nappend-line /games/orbit.mix route=music\nappend-line /games/orbit.mix latency-mode=interactive\nappend-line /games/orbit.mix spatialization=world-3d\nappend-line /games/orbit.mix completion=wait-drain\nappend-line /games/orbit.mix tone=lead,440,120,0.800,-0.250,sine\nappend-line /games/orbit.mix clip=ambience,hangar-loop,2,0.650,0.100\ngame-launch /games/orbit.manifest\ngame-audio-plan 77 /games/orbit.mix\ngame-audio-submit 77 /games/orbit.mix\ngame-status\ngame-audio-status 77\ngame-audio-next 77\ngame-audio-next 77\nlast-status\ngame-stop 77\ngame-audio-submit 77 /games/orbit.mix\nlast-status\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("game.audio.plan pid=77 stream=orbit-intro ops=2"));
        assert!(stdout.contains("route=music latency-mode=interactive spatialization=world-3d"));
        assert!(stdout.contains("completion=wait-drain"));
        assert!(stdout.contains("game.audio.submit pid=77 stream=orbit-intro ops=2"));
        assert!(stdout.contains("batches=1 token="));
        assert!(stdout.contains("completion-observed=resource-drained"));
        assert!(stdout.contains("queue-event queue="));
        assert!(stdout.contains(
            "game.audio.status pid=77 device=/dev/audio0 driver=/drv/audio0 profile=spatial-mix claimed="
        ));
        assert!(stdout.contains(" token="));
        assert!(stdout.contains(
            "stream=orbit-intro route=music latency-mode=interactive spatialization=world-3d completion=wait-drain completion-observed=resource-drained ops=2 bytes="
        ));
        assert!(stdout.contains("device-queue=1/128 device-submitted=6 device-completed=5"));
        assert!(stdout.contains("driver-queued=1 driver-inflight=1 driver-completed=5"));
        assert!(stdout.contains("game.session.audio-queue pid=77 depth=1"));
        assert!(stdout.contains("game.audio.next pid=77 stream=orbit-intro route=music latency-mode=interactive spatialization=world-3d completion=wait-drain remaining=0 payload=ngos-audio-translate/v1"));
        assert!(stdout.contains("game.audio.queue pid=77 depth=0"));
        assert!(stdout.contains("last-status=295"));
    }

    #[test]
    fn native_shell_translates_and_submits_game_input_batch() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\nmkfile-path /games/orbit.input\nappend-line /games/orbit.input device=gamepad\nappend-line /games/orbit.input family=dualshock\nappend-line /games/orbit.input frame=input-001\nappend-line /games/orbit.input layout=gamepad-standard\nappend-line /games/orbit.input key-table=us-game\nappend-line /games/orbit.input pointer-capture=relative-lock\nappend-line /games/orbit.input delivery=wait-frame\nappend-line /games/orbit.input button=cross,press\nappend-line /games/orbit.input axis=left-x,0.750\nappend-line /games/orbit.input pointer=4,-2,0,1\ngame-launch /games/orbit.manifest\ngame-input-plan 77 /games/orbit.input\ngame-input-submit 77 /games/orbit.input\ngame-status\ngame-input-status 77\ngame-input-next 77\ngame-input-next 77\nlast-status\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("game.input.plan pid=77 frame=input-001 ops=3"));
        assert!(
            stdout.contains(
                "family=dualshock layout=gamepad-standard key-table=us-game pointer-capture=relative-lock"
            )
        );
        assert!(stdout.contains("delivery=wait-frame"));
        assert!(stdout.contains("game.input.submit pid=77 frame=input-001 ops=3"));
        assert!(stdout.contains("batches=1 token="));
        assert!(stdout.contains("delivery-observed=frame-delivered"));
        assert!(stdout.contains("queue-event queue="));
        assert!(stdout.contains(
            "game.input.status pid=77 device=/dev/input0 driver=/drv/input0 profile=gamepad-first claimed="
        ));
        assert!(stdout.contains("frame=input-001 family=dualshock layout=gamepad-standard key-table=us-game pointer-capture=relative-lock delivery=wait-frame delivery-observed=frame-delivered ops=3 bytes="));
        assert!(stdout.contains("device-queue=2/64 device-submitted=8 device-completed=7"));
        assert!(stdout.contains("driver-queued=1 driver-inflight=0 driver-completed=7"));
        assert!(stdout.contains("game.session.input-queue pid=77 depth=1"));
        assert!(stdout.contains("game.input.next pid=77 frame=input-001 family=dualshock layout=gamepad-standard delivery=wait-frame remaining=0 payload=ngos-input-translate/v1"));
        assert!(stdout.contains("game.input.queue pid=77 depth=0"));
        assert!(stdout.contains("last-status=299"));
    }

    #[test]
    fn native_shell_consumes_unified_game_payload_queue() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\nmkfile-path /games/orbit.frame\nappend-line /games/orbit.frame surface=1280x720\nappend-line /games/orbit.frame frame=orbit-001\nappend-line /games/orbit.frame queue=graphics\nappend-line /games/orbit.frame present-mode=mailbox\nappend-line /games/orbit.frame completion=wait-complete\nappend-line /games/orbit.frame clear=#112233\nappend-line /games/orbit.frame rect=10,20,200,100,#ff8800ff\nmkfile-path /games/orbit.mix\nappend-line /games/orbit.mix rate=48000\nappend-line /games/orbit.mix channels=2\nappend-line /games/orbit.mix stream=orbit-intro\nappend-line /games/orbit.mix route=music\nappend-line /games/orbit.mix latency-mode=interactive\nappend-line /games/orbit.mix spatialization=world-3d\nappend-line /games/orbit.mix completion=wait-drain\nappend-line /games/orbit.mix tone=lead,440,120,0.800,-0.250,sine\nmkfile-path /games/orbit.input\nappend-line /games/orbit.input device=gamepad\nappend-line /games/orbit.input family=dualshock\nappend-line /games/orbit.input frame=input-001\nappend-line /games/orbit.input layout=gamepad-standard\nappend-line /games/orbit.input key-table=us-game\nappend-line /games/orbit.input pointer-capture=relative-lock\nappend-line /games/orbit.input delivery=wait-frame\nappend-line /games/orbit.input button=cross,press\ngame-launch /games/orbit.manifest\ngame-gfx-submit 77 /games/orbit.frame\ngame-audio-submit 77 /games/orbit.mix\ngame-input-submit 77 /games/orbit.input\ngame-next 77\ngame-next 77\ngame-next 77\ngame-next 77\nlast-status\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("game.next pid=77 kind=graphics tag=orbit-001 remaining[gfx=0;audio=1;input=1] payload=ngos-gfx-translate/v1"));
        assert!(stdout.contains("game.next pid=77 kind=audio tag=orbit-intro remaining[gfx=0;audio=0;input=1] payload=ngos-audio-translate/v1"));
        assert!(stdout.contains("game.next pid=77 kind=input tag=input-001 remaining[gfx=0;audio=0;input=0] payload=ngos-input-translate/v1"));
        assert!(stdout.contains("game.next pid=77 depth[gfx=0;audio=0;input=0]"));
        assert!(stdout.contains("last-status=299"));
    }

    #[test]
    fn native_shell_writes_runtime_bootstrap_files_for_game_session() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest arg=--vsync\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-launch /games/orbit.manifest\ngame-status\nproc 77 environ\nproc 77 cmdline\nproc 77 cwd\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(
            stdout.contains(
                "env-file=/compat/orbit/session.env argv-file=/compat/orbit/session.argv channel-file=/compat/orbit/session.chan"
            )
        );
        assert!(stdout.contains("NGOS_GAME_TITLE=Orbit Runner"));
        assert!(stdout.contains("NGOS_GFX_BACKEND=vulkan"));
        assert!(stdout.contains("NGOS_COMPAT_PREFIX=/compat/orbit"));
        assert!(stdout.contains("NGOS_GAME_CHANNEL=/compat/orbit/session.chan"));
        assert!(stdout.contains("NGOS_AUDIO_BACKEND=native-mixer"));
        assert!(stdout.contains("/bin/worker"));
        assert!(stdout.contains("--fullscreen"));
        assert!(stdout.contains("--vsync"));
        assert!(stdout.contains("/games/orbit"));
    }

    #[test]
    fn native_shell_tracks_game_lane_watch_lifecycle() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-launch /games/orbit.manifest\ngame-sessions\ngame-watch-status-all\ngame-watch-wait 77 audio\nlast-status\ngame-watch-start 77 audio\ngame-watch-status 77 audio\ngame-watch-start 77 input\ngame-watch-status-all\ngame-watch-wait 77 audio\ngame-watch-wait 77 input\ngame-watch-stop 77 audio\ngame-watch-stop 77 input\ngame-watch-status-all\ngame-status\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("game.watch.start pid=77 kind=audio queue="));
        assert!(stdout.contains("game.watch.status pid=77 kind=audio queue="));
        assert!(
            stdout.contains("game.session.summary pid=77 slug=orbit-runner title=Orbit Runner")
        );
        assert!(stdout.contains("game.watch.summary pid=77 slug=orbit-runner kind=graphics queue=inactive token=inactive"));
        assert!(stdout.contains("last-status=299"));
        assert!(stdout.contains("queue-event queue="));
        assert!(stdout.contains("game.watch.start pid=77 kind=input queue="));
        assert!(stdout.contains("game.watch.stop pid=77 kind=audio"));
        assert!(stdout.contains("game.watch.stop pid=77 kind=input"));
        assert!(stdout.contains(
            "game.watch.summary pid=77 slug=orbit-runner kind=input queue=inactive token=inactive"
        ));
        assert!(stdout.contains("game.session.watch kind=audio queue=inactive token=inactive"));
        assert!(stdout.contains("game.session.watch kind=input queue=inactive token=inactive"));
    }

    #[test]
    fn native_shell_rejects_duplicate_game_watch_start_without_replacing_active_watch() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-launch /games/orbit.manifest\ngame-watch-start 77 audio\ngame-watch-status 77 audio\ngame-watch-start 77 audio\nlast-status\ngame-watch-status 77 audio\ngame-watch-stop 77 audio\ngame-watch-status 77 audio\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("game.watch.start pid=77 kind=audio queue="));
        assert!(stdout.contains("last-status=298"));
        assert!(stdout.contains("game.watch.stop pid=77 kind=audio"));
        assert!(
            stdout.contains("game.watch.status pid=77 kind=audio queue=inactive token=inactive")
        );
    }

    #[test]
    fn native_shell_rejects_game_watch_start_after_session_stop() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-launch /games/orbit.manifest\ngame-stop 77\ngame-watch-start 77 audio\nlast-status\ngame-watch-status 77 audio\ngame-status\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("last-status=295"));
        assert!(
            stdout.contains("game.watch.status pid=77 kind=audio queue=inactive token=inactive")
        );
        assert!(stdout.contains("game.session pid=77 title=Orbit Runner slug=orbit-runner"));
        assert!(stdout.contains("stopped=true exit="));
    }

    #[test]
    fn native_shell_closes_watch_queue_fds_on_watch_stop_and_session_stop() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-launch /games/orbit.manifest\ngame-watch-start 77 audio\ngame-watch-start 77 input\ngame-watch-stop 77 audio\ngame-stop 77\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let frames = runtime.backend().frames.borrow();
        let close_count = frames
            .iter()
            .filter(|frame| frame.number == SYS_CLOSE)
            .count();
        assert!(close_count >= 2);
    }

    #[test]
    fn native_shell_tracks_multiple_game_sessions_with_distinct_pids() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"mkdir-path /games\nmkdir-path /games/orbit\nmkdir-path /games/comet\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\nmkfile-path /games/comet.manifest\nappend-line /games/comet.manifest title=Comet Arena\nappend-line /games/comet.manifest slug=comet-arena\nappend-line /games/comet.manifest exec=/bin/worker\nappend-line /games/comet.manifest cwd=/games/comet\nappend-line /games/comet.manifest arg=--windowed\nappend-line /games/comet.manifest gfx.backend=vulkan\nappend-line /games/comet.manifest gfx.profile=frame-pace\nappend-line /games/comet.manifest audio.backend=native-mixer\nappend-line /games/comet.manifest audio.profile=arena-mix\nappend-line /games/comet.manifest input.backend=native-input\nappend-line /games/comet.manifest input.profile=kbm-first\nappend-line /games/comet.manifest shim.prefix=/compat/comet\nappend-line /games/comet.manifest shim.saves=/saves/comet\nappend-line /games/comet.manifest shim.cache=/cache/comet\ngame-launch /games/orbit.manifest\ngame-launch /games/comet.manifest\ngame-watch-poll-all\nlast-status\ngame-watch-start 77 audio\ngame-watch-start 78 input\ngame-sessions\ngame-watch-status-all\ngame-watch-poll-all\ngame-status\ngame-stop 77\ngame-stop 78\ngame-sessions\ngame-watch-status-all\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("game.session pid=77 title=Orbit Runner slug=orbit-runner"));
        assert!(stdout.contains("game.session pid=78 title=Comet Arena slug=comet-arena"));
        assert!(stdout.contains("game.watch.start pid=77 kind=audio queue="));
        assert!(stdout.contains("game.watch.start pid=78 kind=input queue="));
        assert!(stdout.contains("last-status=299"));
        assert!(
            stdout.contains("game.session.summary pid=77 slug=orbit-runner title=Orbit Runner")
        );
        assert!(stdout.contains("game.session.summary pid=78 slug=comet-arena title=Comet Arena"));
        assert!(stdout.contains("game.watch.summary pid=77 slug=orbit-runner kind=audio queue="));
        assert!(stdout.contains("game.watch.summary pid=78 slug=comet-arena kind=input queue="));
        assert!(stdout.contains("queue-event queue="));
        assert!(stdout.contains("game.watch.event pid=77 slug=orbit-runner kind=audio queue="));
        assert!(stdout.contains("game.watch.event pid=78 slug=comet-arena kind=input queue="));
        assert!(stdout.contains("game.watch.poll count=2"));
        assert!(
            stdout.contains("game.session pid=77 title=Orbit Runner slug=orbit-runner domain=")
        );
        assert!(stdout.contains("game.session pid=78 title=Comet Arena slug=comet-arena domain="));
        assert!(stdout.contains(
            "game.session.summary pid=77 slug=orbit-runner title=Orbit Runner stopped=false exit=- lanes=3 watches=1"
        ));
        assert!(stdout.contains(
            "game.session.summary pid=78 slug=comet-arena title=Comet Arena stopped=false exit=- lanes=3 watches=1"
        ));
        assert!(stdout.contains(
            "game.session.summary pid=77 slug=orbit-runner title=Orbit Runner stopped=true exit="
        ));
        assert!(stdout.contains(
            "game.session.summary pid=78 slug=comet-arena title=Comet Arena stopped=true exit="
        ));
    }

    #[test]
    fn native_shell_preserves_other_game_session_activity_after_partial_stop() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"mkdir-path /games\nmkdir-path /games/orbit\nmkdir-path /games/comet\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\nmkfile-path /games/comet.manifest\nappend-line /games/comet.manifest title=Comet Arena\nappend-line /games/comet.manifest slug=comet-arena\nappend-line /games/comet.manifest exec=/bin/worker\nappend-line /games/comet.manifest cwd=/games/comet\nappend-line /games/comet.manifest arg=--windowed\nappend-line /games/comet.manifest gfx.backend=vulkan\nappend-line /games/comet.manifest gfx.profile=frame-pace\nappend-line /games/comet.manifest audio.backend=native-mixer\nappend-line /games/comet.manifest audio.profile=arena-mix\nappend-line /games/comet.manifest input.backend=native-input\nappend-line /games/comet.manifest input.profile=kbm-first\nappend-line /games/comet.manifest shim.prefix=/compat/comet\nappend-line /games/comet.manifest shim.saves=/saves/comet\nappend-line /games/comet.manifest shim.cache=/cache/comet\ngame-launch /games/orbit.manifest\ngame-launch /games/comet.manifest\ngame-watch-start 77 audio\ngame-watch-start 78 input\ngame-stop 77\ngame-sessions\ngame-watch-status-all\ngame-watch-poll-all\nlast-status\ngame-status\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains(
            "game.session.summary pid=77 slug=orbit-runner title=Orbit Runner stopped=true exit="
        ));
        assert!(stdout.contains(
            "game.session.summary pid=78 slug=comet-arena title=Comet Arena stopped=false exit=- lanes=3 watches=1"
        ));
        assert!(stdout.contains(
            "game.watch.summary pid=77 slug=orbit-runner kind=audio queue=inactive token=inactive"
        ));
        assert!(stdout.contains("game.watch.summary pid=78 slug=comet-arena kind=input queue="));
        assert!(stdout.contains("game.watch.event pid=78 slug=comet-arena kind=input queue="));
        assert!(stdout.contains("game.watch.poll count=1"));
        assert!(stdout.contains("last-status=0"));
        assert!(stdout.contains("game.session pid=78 title=Comet Arena slug=comet-arena"));
    }

    #[test]
    fn native_shell_rejects_repeated_game_stop_after_session_is_already_stopped() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-launch /games/orbit.manifest\ngame-stop 77\ngame-stop 77\nlast-status\ngame-status\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("last-status=295"));
        assert!(stdout.contains("game.session pid=77 title=Orbit Runner slug=orbit-runner"));
        assert!(stdout.contains("stopped=true exit="));
    }

    #[test]
    fn native_shell_reports_empty_global_watch_poll_after_all_sessions_are_stopped() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"mkdir-path /games\nmkdir-path /games/orbit\nmkdir-path /games/comet\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\nmkfile-path /games/comet.manifest\nappend-line /games/comet.manifest title=Comet Arena\nappend-line /games/comet.manifest slug=comet-arena\nappend-line /games/comet.manifest exec=/bin/worker\nappend-line /games/comet.manifest cwd=/games/comet\nappend-line /games/comet.manifest arg=--windowed\nappend-line /games/comet.manifest gfx.backend=vulkan\nappend-line /games/comet.manifest gfx.profile=frame-pace\nappend-line /games/comet.manifest audio.backend=native-mixer\nappend-line /games/comet.manifest audio.profile=arena-mix\nappend-line /games/comet.manifest input.backend=native-input\nappend-line /games/comet.manifest input.profile=kbm-first\nappend-line /games/comet.manifest shim.prefix=/compat/comet\nappend-line /games/comet.manifest shim.saves=/saves/comet\nappend-line /games/comet.manifest shim.cache=/cache/comet\ngame-launch /games/orbit.manifest\ngame-launch /games/comet.manifest\ngame-watch-start 77 audio\ngame-watch-start 78 input\ngame-stop 77\ngame-stop 78\ngame-watch-status-all\ngame-watch-poll-all\nlast-status\ngame-sessions\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains(
            "game.watch.summary pid=77 slug=orbit-runner kind=audio queue=inactive token=inactive"
        ));
        assert!(stdout.contains(
            "game.watch.summary pid=78 slug=comet-arena kind=input queue=inactive token=inactive"
        ));
        assert!(stdout.contains("last-status=299"));
        assert!(stdout.contains(
            "game.session.summary pid=77 slug=orbit-runner title=Orbit Runner stopped=true exit="
        ));
        assert!(stdout.contains(
            "game.session.summary pid=78 slug=comet-arena title=Comet Arena stopped=true exit="
        ));
    }

    #[test]
    fn native_shell_runs_game_simulate_agent_and_renders_quality_report() {
        let runtime = UserRuntime::new(RecordingBackend::with_stdin(
            b"mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-simulate /games/orbit 2\nexit 0\n",
        ));
        let argv = ["ngos-userland-native"];
        let envp = [
            "NGOS_SESSION=1",
            "NGOS_SESSION_PROTOCOL=kernel-launch",
            "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=/bin/ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
        ];
        let auxv = [
            ngos_user_abi::AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            ngos_user_abi::AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(main(&runtime, &bootstrap), 0);
        let stdout = String::from_utf8(runtime.backend().stdout.borrow().clone()).unwrap();
        assert!(stdout.contains("game-simulate starting slug=orbit-runner frames=2 pid="));
        assert!(stdout.contains("== GAME QUALITY REPORT =="));
        assert!(stdout.contains("title: Orbit Runner"));
        assert!(stdout.contains("frames_submitted: 2"));
        assert!(stdout.contains("quality_score:"));
    }
}
