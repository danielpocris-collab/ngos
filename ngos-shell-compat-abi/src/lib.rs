//! Canonical subsystem role:
//! - subsystem: native compat ABI control surface
//! - owner layer: Layer 3
//! - semantic owner: `ngos-shell-compat-abi`
//! - truth path role: operator-facing compat ABI orchestration over canonical
//!   compat runtime contracts
//!
//! Canonical contract families handled here:
//! - compat ABI command contracts
//! - compat handle/event/timer shell-state contracts
//! - compat module registry orchestration contracts
//!
//! This module may orchestrate compat ABI state in userland, but it must not
//! redefine canonical kernel, ABI, or compat subsystem truth from lower
//! layers.

#![no_std]
extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;
use ngos_user_runtime::compat_abi::{
    CompatEvent, CompatHandleKind, CompatHandleTable, CompatModuleRegistry, CompatModuleState,
    CompatMutex, CompatPathFlavor, CompatPathNormalizer, CompatSchedulerMap, CompatTimer,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatAbiCoreSmokeReport {
    pub handle_line: String,
    pub path_line: String,
    pub sched_line: String,
    pub sync_line: String,
    pub timer_line: String,
    pub module_line: String,
    pub refusal_line: String,
    pub recovery_line: String,
}

pub fn build_compat_abi_core_smoke_report() -> Result<CompatAbiCoreSmokeReport, ExitCode> {
    let mut handles = CompatHandleTable::new();
    let handle_id = handles
        .open(CompatHandleKind::Domain, 1001)
        .map_err(|_| 415)?;
    let dup_id = handles.duplicate(handle_id).map_err(|_| 416)?;
    let handle_entry = handles.get(dup_id).map_err(|_| 417)?;
    if handle_entry.object_id != 1001 || handle_entry.kind != CompatHandleKind::Domain {
        return Err(418);
    }
    let handle_line = format!(
        "compat.abi.smoke.handle.success id={} dup={} kind={} object-id={} open={}",
        handle_id,
        dup_id,
        handle_entry.kind.name(),
        handle_entry.object_id,
        handles.open_count()
    );

    let normalizer = CompatPathNormalizer::new("/compat/root");
    let unix_path = normalizer
        .normalize("/games/nova/config.toml", CompatPathFlavor::UnixAbsolute)
        .map_err(|_| 420)?;
    let relative_path = normalizer
        .normalize("profiles/player-one.cfg", CompatPathFlavor::UnixRelative)
        .map_err(|_| 421)?;
    let path_line = format!(
        "compat.abi.smoke.path.success unix={} relative={}",
        unix_path, relative_path
    );

    let win32_class = CompatSchedulerMap::from_win32_priority(15);
    let posix_class = CompatSchedulerMap::from_posix_nice(5);
    let sched_line = format!(
        "compat.abi.smoke.sched.success win32={} posix={}",
        CompatSchedulerMap::class_name(win32_class),
        CompatSchedulerMap::class_name(posix_class)
    );

    let mut mutex = CompatMutex::new(1);
    mutex.try_lock(1000).map_err(|_| 424)?;
    let mut event = CompatEvent::new(1, false);
    event.signal();
    if !event.is_signaled() {
        return Err(425);
    }
    let sync_line = format!(
        "compat.abi.smoke.sync.success mutex-id=1 state={} owner={} event-id=1 event-state={}",
        mutex.state_name(),
        mutex.owner_pid().unwrap_or(0),
        event.state_name()
    );

    let mut timer = CompatTimer::new(1);
    timer.arm_oneshot(42);
    if !timer.tick(42) || timer.fire_count != 1 || timer.armed {
        return Err(428);
    }
    let mut periodic_timer = CompatTimer::new(2);
    periodic_timer.arm_periodic(100, 25).map_err(|_| 429)?;
    if periodic_timer.tick(99)
        || !periodic_timer.tick(100)
        || !periodic_timer.tick(125)
        || periodic_timer.fire_count != 2
        || periodic_timer.due_tick != 150
    {
        return Err(430);
    }
    let timer_line = format!(
        "compat.abi.smoke.timer.success oneshot-id=1 oneshot-fires={} oneshot-state={} periodic-id=2 periodic-fires={} periodic-due={} periodic-state={}",
        timer.fire_count,
        timer.state_name(),
        periodic_timer.fire_count,
        periodic_timer.due_tick,
        periodic_timer.state_name()
    );

    let mut modules = CompatModuleRegistry::new();
    let module_id = modules.load(
        "nova.renderer",
        "/compat/root/modules/nova-renderer.ngm",
        0x400000,
        0x20000,
    );
    let retained_ref_count = modules.retain(module_id).map_err(|_| 432)?;
    let released_ref_count = modules.release(module_id).map_err(|_| 433)?;
    let module = modules.get(module_id).map_err(|_| 434)?;
    let module_line = format!(
        "compat.abi.smoke.module.success id={} name={} path={} base={:#x} size={:#x} state=loaded retain={} release={}",
        module_id,
        module.name,
        module.path,
        module.base,
        module.size,
        retained_ref_count,
        released_ref_count
    );

    let refusal_close = handles.close(999).err().map(|e| e.describe()).ok_or(436)?;
    let refusal_path = normalizer
        .normalize(r"C:\..\secret", CompatPathFlavor::WindowsAbsolute)
        .err()
        .map(|e| e.describe())
        .ok_or(437)?;
    let refusal_sched = CompatSchedulerMap::from_class_name("unknown-xyz")
        .map(CompatSchedulerMap::class_name)
        .unwrap_or("unknown-class");
    let refusal_lock = mutex
        .try_lock(2000)
        .err()
        .map(|e| e.describe())
        .ok_or(438)?;
    let refusal_unlock = mutex.unlock(999).err().map(|e| e.describe()).ok_or(439)?;
    let refusal_timer = periodic_timer
        .arm_periodic(200, 0)
        .err()
        .map(|e| e.describe())
        .ok_or(440)?;
    modules.unload(module_id).map_err(|_| 441)?;
    let refusal_module = modules
        .retain(module_id)
        .err()
        .map(|e| e.describe())
        .ok_or(442)?;
    let refusal_line = format!(
        "compat.abi.smoke.refusal close={} path={} sched={} lock={} unlock={} timer={} module={} outcome=expected",
        refusal_close,
        refusal_path,
        refusal_sched,
        refusal_lock,
        refusal_unlock,
        refusal_timer,
        refusal_module
    );

    handles.close(handle_id).map_err(|_| 444)?;
    handles.close(dup_id).map_err(|_| 445)?;
    mutex.unlock(1000).map_err(|_| 446)?;
    event.reset();
    periodic_timer.cancel();
    let recovery_path = normalizer
        .normalize("restored/session.ok", CompatPathFlavor::UnixRelative)
        .map_err(|_| 447)?;
    let recovery_class = CompatSchedulerMap::from_class_name("background")
        .map(CompatSchedulerMap::class_name)
        .ok_or(448)?;
    let recovery_module_id = modules.load(
        "nova.runtime",
        "/compat/root/modules/nova-runtime.ngm",
        0x420000,
        0x18000,
    );
    let recovery_module = modules.get(recovery_module_id).map_err(|_| 449)?;
    if handles.open_count() != 0
        || mutex.owner_pid().is_some()
        || event.is_signaled()
        || periodic_timer.armed
    {
        return Err(450);
    }
    let recovery_line = format!(
        "compat.abi.smoke.recovery handles-open={} mutex-state={} event-state={} path={} sched={} timer-state={} timer-fires={} module-id={} module-name={} module-state=loaded module-ref-count={} outcome=ok",
        handles.open_count(),
        mutex.state_name(),
        event.state_name(),
        recovery_path,
        recovery_class,
        periodic_timer.state_name(),
        periodic_timer.fire_count,
        recovery_module.id,
        recovery_module.name,
        recovery_module.ref_count
    );

    Ok(CompatAbiCoreSmokeReport {
        handle_line,
        path_line,
        sched_line,
        sync_line,
        timer_line,
        module_line,
        refusal_line,
        recovery_line,
    })
}

fn write_line<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 196)?;
    Ok(())
}

pub struct CompatAbiShellState {
    pub handles: CompatHandleTable,
    pub mutexes: Vec<CompatMutex>,
    pub events: Vec<CompatEvent>,
    pub timers: Vec<CompatTimer>,
    pub modules: CompatModuleRegistry,
    pub path_prefix: String,
    next_mutex_id: u32,
    next_event_id: u32,
    next_timer_id: u32,
}

impl CompatAbiShellState {
    pub fn new() -> Self {
        Self {
            handles: CompatHandleTable::new(),
            mutexes: Vec::new(),
            events: Vec::new(),
            timers: Vec::new(),
            modules: CompatModuleRegistry::new(),
            path_prefix: String::from("/compat/prefix"),
            next_mutex_id: 1,
            next_event_id: 1,
            next_timer_id: 1,
        }
    }
}

pub fn try_handle_compat_abi_command<B: SyscallBackend>(
    runtime: &Runtime<B>,
    line: &str,
    state: &mut CompatAbiShellState,
    last_status: &mut ExitCode,
) -> Option<Result<(), ExitCode>> {
    macro_rules! settle {
        ($result:expr) => {{
            *last_status = match $result {
                Ok(()) => 0,
                Err(code) => code,
            };
            Some(Ok(()))
        }};
    }

    if let Some(rest) = line.strip_prefix("compat-handle-open ") {
        return settle!(handle_open(runtime, rest.trim(), state));
    }
    if let Some(rest) = line.strip_prefix("compat-handle-close ") {
        return settle!(handle_close(runtime, rest.trim(), state));
    }
    if let Some(rest) = line.strip_prefix("compat-handle-dup ") {
        return settle!(handle_dup(runtime, rest.trim(), state));
    }
    if let Some(rest) = line.strip_prefix("compat-handle-status ") {
        return settle!(handle_status(runtime, rest.trim(), state));
    }
    if line == "compat-handle-count" {
        return settle!(handle_count(runtime, state));
    }
    if let Some(rest) = line.strip_prefix("compat-path-normalize ") {
        return settle!(path_normalize(runtime, rest.trim(), state));
    }
    if let Some(rest) = line.strip_prefix("compat-path-prefix ") {
        state.path_prefix = rest.trim().to_string();
        *last_status = 0;
        return Some(Ok(()));
    }
    if let Some(rest) = line.strip_prefix("compat-sched-win32 ") {
        return settle!(sched_map_win32(runtime, rest.trim()));
    }
    if let Some(rest) = line.strip_prefix("compat-sched-posix ") {
        return settle!(sched_map_posix(runtime, rest.trim()));
    }
    if let Some(rest) = line.strip_prefix("compat-sched-class ") {
        return settle!(sched_map_class(runtime, rest.trim()));
    }
    if let Some(rest) = line.strip_prefix("compat-mutex-create") {
        let _ = rest;
        return settle!(mutex_create(runtime, state));
    }
    if let Some(rest) = line.strip_prefix("compat-mutex-lock ") {
        return settle!(mutex_lock(runtime, rest.trim(), state));
    }
    if let Some(rest) = line.strip_prefix("compat-mutex-unlock ") {
        return settle!(mutex_unlock(runtime, rest.trim(), state));
    }
    if let Some(rest) = line.strip_prefix("compat-mutex-status ") {
        return settle!(mutex_status(runtime, rest.trim(), state));
    }
    if let Some(rest) = line.strip_prefix("compat-event-create") {
        let auto_reset = rest.trim() == "--auto-reset";
        return settle!(event_create(runtime, auto_reset, state));
    }
    if let Some(rest) = line.strip_prefix("compat-event-signal ") {
        return settle!(event_signal(runtime, rest.trim(), state));
    }
    if let Some(rest) = line.strip_prefix("compat-event-reset ") {
        return settle!(event_reset(runtime, rest.trim(), state));
    }
    if let Some(rest) = line.strip_prefix("compat-event-status ") {
        return settle!(event_status(runtime, rest.trim(), state));
    }
    if line == "compat-timer-create" {
        return settle!(timer_create(runtime, state));
    }
    if let Some(rest) = line.strip_prefix("compat-timer-arm ") {
        return settle!(timer_arm(runtime, rest.trim(), state));
    }
    if let Some(rest) = line.strip_prefix("compat-timer-arm-periodic ") {
        return settle!(timer_arm_periodic(runtime, rest.trim(), state));
    }
    if let Some(rest) = line.strip_prefix("compat-timer-tick ") {
        return settle!(timer_tick(runtime, rest.trim(), state));
    }
    if let Some(rest) = line.strip_prefix("compat-timer-cancel ") {
        return settle!(timer_cancel(runtime, rest.trim(), state));
    }
    if let Some(rest) = line.strip_prefix("compat-timer-status ") {
        return settle!(timer_status(runtime, rest.trim(), state));
    }
    if let Some(rest) = line.strip_prefix("compat-module-load ") {
        return settle!(module_load(runtime, rest.trim(), state));
    }
    if let Some(rest) = line.strip_prefix("compat-module-status ") {
        return settle!(module_status(runtime, rest.trim(), state));
    }
    if let Some(rest) = line.strip_prefix("compat-module-retain ") {
        return settle!(module_retain(runtime, rest.trim(), state));
    }
    if let Some(rest) = line.strip_prefix("compat-module-release ") {
        return settle!(module_release(runtime, rest.trim(), state));
    }
    if let Some(rest) = line.strip_prefix("compat-module-unload ") {
        return settle!(module_unload(runtime, rest.trim(), state));
    }
    None
}

fn handle_open<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    state: &mut CompatAbiShellState,
) -> Result<(), ExitCode> {
    let mut parts = rest.split_whitespace();
    let kind_str = parts.next().ok_or_else(|| {
        let _ = write_line(runtime, "usage: compat-handle-open <kind> <object-id>");
        2
    })?;
    let object_id_str = parts.next().ok_or_else(|| {
        let _ = write_line(runtime, "usage: compat-handle-open <kind> <object-id>");
        2
    })?;
    let kind = CompatHandleKind::parse(kind_str).ok_or_else(|| {
        let _ = write_line(
            runtime,
            &format!("compat.handle.open.refused kind={kind_str} reason=unknown-kind"),
        );
        300
    })?;
    let object_id = object_id_str.parse::<u64>().map_err(|_| {
        let _ = write_line(
            runtime,
            &format!("compat.handle.open.refused object-id={object_id_str} reason=invalid-id"),
        );
        300
    })?;
    let id = state.handles.open(kind, object_id).map_err(|e| {
        let _ = write_line(
            runtime,
            &format!("compat.handle.open.refused reason={}", e.describe()),
        );
        301
    })?;
    write_line(
        runtime,
        &format!(
            "compat.handle.open id={id} kind={} object-id={object_id} open={}",
            kind.name(),
            state.handles.open_count()
        ),
    )
}

fn handle_close<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    state: &mut CompatAbiShellState,
) -> Result<(), ExitCode> {
    let id = parse_u32_arg(runtime, rest, "usage: compat-handle-close <id>")?;
    state.handles.close(id).map_err(|e| {
        let _ = write_line(
            runtime,
            &format!(
                "compat.handle.close.refused id={id} reason={}",
                e.describe()
            ),
        );
        301
    })?;
    write_line(
        runtime,
        &format!(
            "compat.handle.close id={id} open={}",
            state.handles.open_count()
        ),
    )
}

fn handle_dup<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    state: &mut CompatAbiShellState,
) -> Result<(), ExitCode> {
    let id = parse_u32_arg(runtime, rest, "usage: compat-handle-dup <id>")?;
    let new_id = state.handles.duplicate(id).map_err(|e| {
        let _ = write_line(
            runtime,
            &format!("compat.handle.dup.refused id={id} reason={}", e.describe()),
        );
        301
    })?;
    let entry = state.handles.get(new_id).map_err(|_| 301i32)?;
    write_line(
        runtime,
        &format!(
            "compat.handle.dup source={id} new-id={new_id} kind={} object-id={} open={}",
            entry.kind.name(),
            entry.object_id,
            state.handles.open_count()
        ),
    )
}

fn handle_status<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    state: &CompatAbiShellState,
) -> Result<(), ExitCode> {
    let id = parse_u32_arg(runtime, rest, "usage: compat-handle-status <id>")?;
    let entry = state.handles.get(id).map_err(|e| {
        let _ = write_line(
            runtime,
            &format!(
                "compat.handle.status.refused id={id} reason={}",
                e.describe()
            ),
        );
        301
    })?;
    write_line(
        runtime,
        &format!(
            "compat.handle.status id={id} kind={} object-id={} state=open ref-count={}",
            entry.kind.name(),
            entry.object_id,
            entry.ref_count
        ),
    )
}

fn handle_count<B: SyscallBackend>(
    runtime: &Runtime<B>,
    state: &CompatAbiShellState,
) -> Result<(), ExitCode> {
    write_line(
        runtime,
        &format!("compat.handle.count open={}", state.handles.open_count()),
    )
}

fn path_normalize<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    state: &CompatAbiShellState,
) -> Result<(), ExitCode> {
    let mut parts = rest.splitn(2, ' ');
    let flavor_str = parts.next().ok_or_else(|| {
        let _ = write_line(runtime, "usage: compat-path-normalize <flavor> <path>");
        2
    })?;
    let path = parts.next().unwrap_or("").trim();
    let flavor = CompatPathFlavor::parse(flavor_str).ok_or_else(|| {
        let _ = write_line(
            runtime,
            &format!("compat.path.normalize.refused flavor={flavor_str} reason=unknown-flavor"),
        );
        302
    })?;
    let norm = CompatPathNormalizer::new(&state.path_prefix);
    let result = norm.normalize(path, flavor).map_err(|e| {
        let _ = write_line(
            runtime,
            &format!(
                "compat.path.normalize.refused flavor={} path={path} reason={}",
                flavor.name(),
                e.describe()
            ),
        );
        302
    })?;
    write_line(
        runtime,
        &format!(
            "compat.path.normalize flavor={} input={path} output={result}",
            flavor.name()
        ),
    )
}

fn sched_map_win32<B: SyscallBackend>(runtime: &Runtime<B>, rest: &str) -> Result<(), ExitCode> {
    let priority = rest.parse::<i32>().map_err(|_| {
        let _ = write_line(runtime, "usage: compat-sched-win32 <priority>");
        2
    })?;
    let class = CompatSchedulerMap::from_win32_priority(priority);
    write_line(
        runtime,
        &format!(
            "compat.sched.map source=win32 priority={priority} class={}",
            CompatSchedulerMap::class_name(class)
        ),
    )
}

fn sched_map_posix<B: SyscallBackend>(runtime: &Runtime<B>, rest: &str) -> Result<(), ExitCode> {
    let nice = rest.parse::<i32>().map_err(|_| {
        let _ = write_line(runtime, "usage: compat-sched-posix <nice>");
        2
    })?;
    let class = CompatSchedulerMap::from_posix_nice(nice);
    write_line(
        runtime,
        &format!(
            "compat.sched.map source=posix nice={nice} class={}",
            CompatSchedulerMap::class_name(class)
        ),
    )
}

fn sched_map_class<B: SyscallBackend>(runtime: &Runtime<B>, rest: &str) -> Result<(), ExitCode> {
    match CompatSchedulerMap::from_class_name(rest) {
        Some(class) => write_line(
            runtime,
            &format!(
                "compat.sched.map source=class name={rest} class={}",
                CompatSchedulerMap::class_name(class)
            ),
        ),
        None => {
            let _ = write_line(
                runtime,
                &format!("compat.sched.map.refused name={rest} reason=unknown-class"),
            );
            Err(303)
        }
    }
}

fn mutex_create<B: SyscallBackend>(
    runtime: &Runtime<B>,
    state: &mut CompatAbiShellState,
) -> Result<(), ExitCode> {
    let id = state.next_mutex_id;
    state.next_mutex_id = state.next_mutex_id.wrapping_add(1).max(1);
    state.mutexes.push(CompatMutex::new(id));
    write_line(
        runtime,
        &format!("compat.mutex.create id={id} state=unlocked"),
    )
}

fn mutex_lock<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    state: &mut CompatAbiShellState,
) -> Result<(), ExitCode> {
    let mut parts = rest.split_whitespace();
    let id = parse_u32_from(parts.next(), runtime, "usage: compat-mutex-lock <id> <pid>")?;
    let pid = parse_u64_from(parts.next(), runtime, "usage: compat-mutex-lock <id> <pid>")?;
    let mutex = state
        .mutexes
        .iter_mut()
        .find(|m| m.id == id)
        .ok_or_else(|| {
            let _ = write_line(
                runtime,
                &format!("compat.mutex.lock.refused id={id} reason=not-found"),
            );
            303
        })?;
    mutex.try_lock(pid).map_err(|e| {
        let _ = write_line(
            runtime,
            &format!(
                "compat.mutex.lock.refused id={id} pid={pid} reason={}",
                e.describe()
            ),
        );
        303
    })?;
    write_line(
        runtime,
        &format!(
            "compat.mutex.lock id={id} pid={pid} state={}",
            mutex.state_name()
        ),
    )
}

fn mutex_unlock<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    state: &mut CompatAbiShellState,
) -> Result<(), ExitCode> {
    let mut parts = rest.split_whitespace();
    let id = parse_u32_from(
        parts.next(),
        runtime,
        "usage: compat-mutex-unlock <id> <pid>",
    )?;
    let pid = parse_u64_from(
        parts.next(),
        runtime,
        "usage: compat-mutex-unlock <id> <pid>",
    )?;
    let mutex = state
        .mutexes
        .iter_mut()
        .find(|m| m.id == id)
        .ok_or_else(|| {
            let _ = write_line(
                runtime,
                &format!("compat.mutex.unlock.refused id={id} reason=not-found"),
            );
            303
        })?;
    mutex.unlock(pid).map_err(|e| {
        let _ = write_line(
            runtime,
            &format!(
                "compat.mutex.unlock.refused id={id} pid={pid} reason={}",
                e.describe()
            ),
        );
        303
    })?;
    write_line(
        runtime,
        &format!(
            "compat.mutex.unlock id={id} pid={pid} state={}",
            mutex.state_name()
        ),
    )
}

fn mutex_status<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    state: &CompatAbiShellState,
) -> Result<(), ExitCode> {
    let id = parse_u32_arg(runtime, rest, "usage: compat-mutex-status <id>")?;
    let mutex = state.mutexes.iter().find(|m| m.id == id).ok_or_else(|| {
        let _ = write_line(
            runtime,
            &format!("compat.mutex.status.refused id={id} reason=not-found"),
        );
        303
    })?;
    let owner = mutex
        .owner_pid()
        .map(|p| format!("{p}"))
        .unwrap_or_else(|| String::from("-"));
    write_line(
        runtime,
        &format!(
            "compat.mutex.status id={id} state={} owner={owner}",
            mutex.state_name()
        ),
    )
}

fn event_create<B: SyscallBackend>(
    runtime: &Runtime<B>,
    auto_reset: bool,
    state: &mut CompatAbiShellState,
) -> Result<(), ExitCode> {
    let id = state.next_event_id;
    state.next_event_id = state.next_event_id.wrapping_add(1).max(1);
    state.events.push(CompatEvent::new(id, auto_reset));
    write_line(
        runtime,
        &format!("compat.event.create id={id} auto-reset={auto_reset} state=unsignaled"),
    )
}

fn event_signal<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    state: &mut CompatAbiShellState,
) -> Result<(), ExitCode> {
    let id = parse_u32_arg(runtime, rest, "usage: compat-event-signal <id>")?;
    let ev = state
        .events
        .iter_mut()
        .find(|e| e.id == id)
        .ok_or_else(|| {
            let _ = write_line(
                runtime,
                &format!("compat.event.signal.refused id={id} reason=not-found"),
            );
            304
        })?;
    ev.signal();
    write_line(
        runtime,
        &format!("compat.event.signal id={id} state={}", ev.state_name()),
    )
}

fn event_reset<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    state: &mut CompatAbiShellState,
) -> Result<(), ExitCode> {
    let id = parse_u32_arg(runtime, rest, "usage: compat-event-reset <id>")?;
    let ev = state
        .events
        .iter_mut()
        .find(|e| e.id == id)
        .ok_or_else(|| {
            let _ = write_line(
                runtime,
                &format!("compat.event.reset.refused id={id} reason=not-found"),
            );
            304
        })?;
    ev.reset();
    write_line(
        runtime,
        &format!("compat.event.reset id={id} state={}", ev.state_name()),
    )
}

fn event_status<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    state: &CompatAbiShellState,
) -> Result<(), ExitCode> {
    let id = parse_u32_arg(runtime, rest, "usage: compat-event-status <id>")?;
    let ev = state.events.iter().find(|e| e.id == id).ok_or_else(|| {
        let _ = write_line(
            runtime,
            &format!("compat.event.status.refused id={id} reason=not-found"),
        );
        304
    })?;
    write_line(
        runtime,
        &format!(
            "compat.event.status id={id} state={} auto-reset={}",
            ev.state_name(),
            ev.auto_reset
        ),
    )
}

fn timer_create<B: SyscallBackend>(
    runtime: &Runtime<B>,
    state: &mut CompatAbiShellState,
) -> Result<(), ExitCode> {
    let id = state.next_timer_id;
    state.next_timer_id = state.next_timer_id.wrapping_add(1).max(1);
    state.timers.push(CompatTimer::new(id));
    write_line(
        runtime,
        &format!("compat.timer.create id={id} state=idle mode=oneshot"),
    )
}

fn timer_arm<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    state: &mut CompatAbiShellState,
) -> Result<(), ExitCode> {
    let mut parts = rest.split_whitespace();
    let id = parse_u32_from(
        parts.next(),
        runtime,
        "usage: compat-timer-arm <id> <due-tick>",
    )?;
    let due_tick = parse_u64_from(
        parts.next(),
        runtime,
        "usage: compat-timer-arm <id> <due-tick>",
    )?;
    let timer = state
        .timers
        .iter_mut()
        .find(|timer| timer.id == id)
        .ok_or_else(|| {
            let _ = write_line(
                runtime,
                &format!("compat.timer.arm.refused id={id} reason=not-found"),
            );
            305
        })?;
    timer.arm_oneshot(due_tick);
    write_line(
        runtime,
        &format!(
            "compat.timer.arm id={id} due={} mode={} state={}",
            timer.due_tick,
            timer.mode_name(),
            timer.state_name()
        ),
    )
}

fn timer_arm_periodic<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    state: &mut CompatAbiShellState,
) -> Result<(), ExitCode> {
    let mut parts = rest.split_whitespace();
    let id = parse_u32_from(
        parts.next(),
        runtime,
        "usage: compat-timer-arm-periodic <id> <start-tick> <interval>",
    )?;
    let start_tick = parse_u64_from(
        parts.next(),
        runtime,
        "usage: compat-timer-arm-periodic <id> <start-tick> <interval>",
    )?;
    let interval = parse_u64_from(
        parts.next(),
        runtime,
        "usage: compat-timer-arm-periodic <id> <start-tick> <interval>",
    )?;
    let timer = state
        .timers
        .iter_mut()
        .find(|timer| timer.id == id)
        .ok_or_else(|| {
            let _ = write_line(
                runtime,
                &format!("compat.timer.arm-periodic.refused id={id} reason=not-found"),
            );
            305
        })?;
    timer.arm_periodic(start_tick, interval).map_err(|e| {
        let _ = write_line(
            runtime,
            &format!(
                "compat.timer.arm-periodic.refused id={id} reason={}",
                e.describe()
            ),
        );
        305
    })?;
    write_line(
        runtime,
        &format!(
            "compat.timer.arm-periodic id={id} due={} interval={} mode={} state={}",
            timer.due_tick,
            timer.interval_tick,
            timer.mode_name(),
            timer.state_name()
        ),
    )
}

fn timer_tick<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    state: &mut CompatAbiShellState,
) -> Result<(), ExitCode> {
    let mut parts = rest.split_whitespace();
    let id = parse_u32_from(
        parts.next(),
        runtime,
        "usage: compat-timer-tick <id> <now-tick>",
    )?;
    let now_tick = parse_u64_from(
        parts.next(),
        runtime,
        "usage: compat-timer-tick <id> <now-tick>",
    )?;
    let timer = state
        .timers
        .iter_mut()
        .find(|timer| timer.id == id)
        .ok_or_else(|| {
            let _ = write_line(
                runtime,
                &format!("compat.timer.tick.refused id={id} reason=not-found"),
            );
            305
        })?;
    let fired = timer.tick(now_tick);
    write_line(
        runtime,
        &format!(
            "compat.timer.tick id={id} now={} fired={} due={} state={} fires={}",
            now_tick,
            fired,
            timer.due_tick,
            timer.state_name(),
            timer.fire_count
        ),
    )
}

fn timer_cancel<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    state: &mut CompatAbiShellState,
) -> Result<(), ExitCode> {
    let id = parse_u32_arg(runtime, rest, "usage: compat-timer-cancel <id>")?;
    let timer = state
        .timers
        .iter_mut()
        .find(|timer| timer.id == id)
        .ok_or_else(|| {
            let _ = write_line(
                runtime,
                &format!("compat.timer.cancel.refused id={id} reason=not-found"),
            );
            305
        })?;
    timer.cancel();
    write_line(
        runtime,
        &format!("compat.timer.cancel id={id} state={}", timer.state_name()),
    )
}

fn timer_status<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    state: &CompatAbiShellState,
) -> Result<(), ExitCode> {
    let id = parse_u32_arg(runtime, rest, "usage: compat-timer-status <id>")?;
    let timer = state
        .timers
        .iter()
        .find(|timer| timer.id == id)
        .ok_or_else(|| {
            let _ = write_line(
                runtime,
                &format!("compat.timer.status.refused id={id} reason=not-found"),
            );
            305
        })?;
    write_line(
        runtime,
        &format!(
            "compat.timer.status id={id} state={} mode={} due={} interval={} fires={}",
            timer.state_name(),
            timer.mode_name(),
            timer.due_tick,
            timer.interval_tick,
            timer.fire_count
        ),
    )
}

fn module_load<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    state: &mut CompatAbiShellState,
) -> Result<(), ExitCode> {
    let mut parts = rest.split_whitespace();
    let Some(name) = parts.next() else {
        let _ = write_line(
            runtime,
            "usage: compat-module-load <name> <path> <base> <size>",
        );
        return Err(2);
    };
    let Some(path) = parts.next() else {
        let _ = write_line(
            runtime,
            "usage: compat-module-load <name> <path> <base> <size>",
        );
        return Err(2);
    };
    let base = parse_u64_from(
        parts.next(),
        runtime,
        "usage: compat-module-load <name> <path> <base> <size>",
    )?;
    let size = parse_u64_from(
        parts.next(),
        runtime,
        "usage: compat-module-load <name> <path> <base> <size>",
    )?;
    let id = state.modules.load(name, path, base, size);
    write_line(
        runtime,
        &format!(
            "compat.module.load id={id} name={name} path={path} base={base:#x} size={size:#x} state=loaded ref-count=1",
        ),
    )
}

fn module_status<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    state: &CompatAbiShellState,
) -> Result<(), ExitCode> {
    let id = parse_u32_arg(runtime, rest, "usage: compat-module-status <id>")?;
    let module = state.modules.get(id).map_err(|e| {
        let _ = write_line(
            runtime,
            &format!(
                "compat.module.status.refused id={id} reason={}",
                e.describe()
            ),
        );
        306
    })?;
    let state_name = match module.state {
        CompatModuleState::Loaded => "loaded",
        CompatModuleState::Unloaded => "unloaded",
    };
    write_line(
        runtime,
        &format!(
            "compat.module.status id={id} name={} path={} base={:#x} size={:#x} state={} ref-count={}",
            module.name, module.path, module.base, module.size, state_name, module.ref_count
        ),
    )
}

fn module_retain<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    state: &mut CompatAbiShellState,
) -> Result<(), ExitCode> {
    let id = parse_u32_arg(runtime, rest, "usage: compat-module-retain <id>")?;
    let ref_count = state.modules.retain(id).map_err(|e| {
        let _ = write_line(
            runtime,
            &format!(
                "compat.module.retain.refused id={id} reason={}",
                e.describe()
            ),
        );
        306
    })?;
    write_line(
        runtime,
        &format!("compat.module.retain id={id} ref-count={ref_count}"),
    )
}

fn module_release<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    state: &mut CompatAbiShellState,
) -> Result<(), ExitCode> {
    let id = parse_u32_arg(runtime, rest, "usage: compat-module-release <id>")?;
    let ref_count = state.modules.release(id).map_err(|e| {
        let _ = write_line(
            runtime,
            &format!(
                "compat.module.release.refused id={id} reason={}",
                e.describe()
            ),
        );
        306
    })?;
    write_line(
        runtime,
        &format!("compat.module.release id={id} ref-count={ref_count}"),
    )
}

fn module_unload<B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &str,
    state: &mut CompatAbiShellState,
) -> Result<(), ExitCode> {
    let id = parse_u32_arg(runtime, rest, "usage: compat-module-unload <id>")?;
    state.modules.unload(id).map_err(|e| {
        let _ = write_line(
            runtime,
            &format!(
                "compat.module.unload.refused id={id} reason={}",
                e.describe()
            ),
        );
        306
    })?;
    write_line(
        runtime,
        &format!("compat.module.unload id={id} state=unloaded ref-count=0"),
    )
}

fn parse_u32_arg<B: SyscallBackend>(
    runtime: &Runtime<B>,
    s: &str,
    usage: &str,
) -> Result<u32, ExitCode> {
    s.trim().parse::<u32>().map_err(|_| {
        let _ = write_line(runtime, usage);
        2
    })
}

fn parse_u32_from<B: SyscallBackend>(
    token: Option<&str>,
    runtime: &Runtime<B>,
    usage: &str,
) -> Result<u32, ExitCode> {
    token.and_then(|s| s.parse::<u32>().ok()).ok_or_else(|| {
        let _ = write_line(runtime, usage);
        2
    })
}

fn parse_u64_from<B: SyscallBackend>(
    token: Option<&str>,
    runtime: &Runtime<B>,
    usage: &str,
) -> Result<u64, ExitCode> {
    token.and_then(|s| s.parse::<u64>().ok()).ok_or_else(|| {
        let _ = write_line(runtime, usage);
        2
    })
}

#[cfg(test)]
mod tests {
    use super::build_compat_abi_core_smoke_report;

    #[test]
    fn compat_abi_core_smoke_report_emits_success_refusal_and_recovery_markers() {
        let report = build_compat_abi_core_smoke_report().unwrap();

        assert!(
            report
                .handle_line
                .contains("compat.abi.smoke.handle.success")
        );
        assert!(report.path_line.contains("compat.abi.smoke.path.success"));
        assert!(report.sched_line.contains("compat.abi.smoke.sched.success"));
        assert!(report.sync_line.contains("compat.abi.smoke.sync.success"));
        assert!(report.timer_line.contains("compat.abi.smoke.timer.success"));
        assert!(
            report
                .module_line
                .contains("compat.abi.smoke.module.success")
        );
        assert!(report.refusal_line.contains("compat.abi.smoke.refusal"));
        assert!(report.recovery_line.contains("compat.abi.smoke.recovery"));
    }
}
