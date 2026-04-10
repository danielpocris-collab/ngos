//! Canonical subsystem role:
//! - subsystem: user-runtime compat ABI model
//! - owner layer: Layer 2
//! - semantic owner: `user-runtime`
//! - truth path role: canonical compat runtime structures and rules used by
//!   user-mode execution
//!
//! Canonical contract families handled here:
//! - compat handle-table contracts
//! - compat mutex/event/timer contracts
//! - compat path normalization contracts
//!
//! This module may define compat runtime behavior for user mode, but it must
//! remain subordinate to the `ngos` kernel model and must not redefine kernel
//! subsystem ownership.

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use ngos_user_abi::NativeSchedulerClass;

// --- errors ---

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompatAbiError {
    HandleNotFound { id: u32 },
    HandleAlreadyClosed { id: u32 },
    HandleTableFull,
    MutexNotFound { id: u32 },
    MutexAlreadyLocked { id: u32, owner_pid: u64 },
    MutexNotOwner { id: u32, caller_pid: u64 },
    EventNotFound { id: u32 },
    TimerNotFound { id: u32 },
    TimerInvalidInterval { id: u32 },
    ModuleNotFound { id: u32 },
    ModuleAlreadyUnloaded { id: u32 },
    EmptyPath,
    PathTraversal { path: String },
    InvalidPathFormat { path: String },
}

impl CompatAbiError {
    pub fn describe(&self) -> String {
        match self {
            Self::HandleNotFound { id } => format!("handle not found id={id}"),
            Self::HandleAlreadyClosed { id } => format!("handle already closed id={id}"),
            Self::HandleTableFull => String::from("handle table full"),
            Self::MutexNotFound { id } => format!("mutex not found id={id}"),
            Self::MutexAlreadyLocked { id, owner_pid } => {
                format!("mutex already locked id={id} owner={owner_pid}")
            }
            Self::MutexNotOwner { id, caller_pid } => {
                format!("mutex not owned by caller id={id} caller={caller_pid}")
            }
            Self::EventNotFound { id } => format!("event not found id={id}"),
            Self::TimerNotFound { id } => format!("timer not found id={id}"),
            Self::TimerInvalidInterval { id } => format!("timer invalid interval id={id}"),
            Self::ModuleNotFound { id } => format!("module not found id={id}"),
            Self::ModuleAlreadyUnloaded { id } => format!("module already unloaded id={id}"),
            Self::EmptyPath => String::from("path is empty"),
            Self::PathTraversal { path } => format!("path traversal refused path={path}"),
            Self::InvalidPathFormat { path } => format!("invalid path format path={path}"),
        }
    }
}

// --- handle table ---

/// Kind of NGOS object a compat handle wraps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatHandleKind {
    Domain,
    Resource,
    Contract,
    Process,
    EventObject,
    Mutex,
    Timer,
}

impl CompatHandleKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Domain => "domain",
            Self::Resource => "resource",
            Self::Contract => "contract",
            Self::Process => "process",
            Self::EventObject => "event-object",
            Self::Mutex => "mutex",
            Self::Timer => "timer",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "domain" => Some(Self::Domain),
            "resource" => Some(Self::Resource),
            "contract" => Some(Self::Contract),
            "process" => Some(Self::Process),
            "event-object" => Some(Self::EventObject),
            "mutex" => Some(Self::Mutex),
            "timer" => Some(Self::Timer),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatHandleState {
    Open,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatHandleEntry {
    pub id: u32,
    pub kind: CompatHandleKind,
    pub object_id: u64,
    pub state: CompatHandleState,
    pub ref_count: u32,
}

/// Handle table — maps opaque u32 handle IDs to NGOS object references.
pub struct CompatHandleTable {
    next_id: u32,
    entries: Vec<CompatHandleEntry>,
}

const HANDLE_TABLE_CAPACITY: usize = 1024;

impl CompatHandleTable {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            entries: Vec::new(),
        }
    }

    /// Open a handle to an NGOS object. Returns the new handle ID.
    pub fn open(&mut self, kind: CompatHandleKind, object_id: u64) -> Result<u32, CompatAbiError> {
        if self
            .entries
            .iter()
            .filter(|e| e.state == CompatHandleState::Open)
            .count()
            >= HANDLE_TABLE_CAPACITY
        {
            return Err(CompatAbiError::HandleTableFull);
        }
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.entries.push(CompatHandleEntry {
            id,
            kind,
            object_id,
            state: CompatHandleState::Open,
            ref_count: 1,
        });
        Ok(id)
    }

    /// Close a handle. Errors if not found or already closed.
    pub fn close(&mut self, id: u32) -> Result<(), CompatAbiError> {
        let entry = self
            .entries
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or(CompatAbiError::HandleNotFound { id })?;
        if entry.state == CompatHandleState::Closed {
            return Err(CompatAbiError::HandleAlreadyClosed { id });
        }
        entry.ref_count = entry.ref_count.saturating_sub(1);
        if entry.ref_count == 0 {
            entry.state = CompatHandleState::Closed;
        }
        Ok(())
    }

    /// Duplicate a handle. Returns a new independent handle ID for the same object.
    pub fn duplicate(&mut self, id: u32) -> Result<u32, CompatAbiError> {
        let (kind, object_id) = {
            let entry = self
                .entries
                .iter()
                .find(|e| e.id == id)
                .ok_or(CompatAbiError::HandleNotFound { id })?;
            if entry.state == CompatHandleState::Closed {
                return Err(CompatAbiError::HandleAlreadyClosed { id });
            }
            (entry.kind, entry.object_id)
        };
        self.open(kind, object_id)
    }

    /// Get a handle entry by ID.
    pub fn get(&self, id: u32) -> Result<&CompatHandleEntry, CompatAbiError> {
        let entry = self
            .entries
            .iter()
            .find(|e| e.id == id)
            .ok_or(CompatAbiError::HandleNotFound { id })?;
        if entry.state == CompatHandleState::Closed {
            return Err(CompatAbiError::HandleAlreadyClosed { id });
        }
        Ok(entry)
    }

    pub fn open_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.state == CompatHandleState::Open)
            .count()
    }
}

// --- path normalization ---

/// Input path flavor for normalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatPathFlavor {
    /// `C:\games\foo.exe` — Windows absolute with drive letter
    WindowsAbsolute,
    /// `games\foo.exe` — Windows relative (no drive letter)
    WindowsRelative,
    /// `/usr/bin/foo` — Unix absolute
    UnixAbsolute,
    /// `usr/bin/foo` — Unix relative
    UnixRelative,
    /// `Z:\foo` — Wine drive letter maps to prefix root
    WineDrive,
}

impl CompatPathFlavor {
    pub fn name(self) -> &'static str {
        match self {
            Self::WindowsAbsolute => "windows-absolute",
            Self::WindowsRelative => "windows-relative",
            Self::UnixAbsolute => "unix-absolute",
            Self::UnixRelative => "unix-relative",
            Self::WineDrive => "wine-drive",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "windows-absolute" | "win-abs" => Some(Self::WindowsAbsolute),
            "windows-relative" | "win-rel" => Some(Self::WindowsRelative),
            "unix-absolute" | "unix-abs" => Some(Self::UnixAbsolute),
            "unix-relative" | "unix-rel" => Some(Self::UnixRelative),
            "wine-drive" | "wine" => Some(Self::WineDrive),
            _ => None,
        }
    }
}

/// Normalizes foreign paths to NGOS VFS paths under a compat prefix.
pub struct CompatPathNormalizer {
    /// The NGOS prefix root, e.g. `/compat/prefix`. Must start with `/`.
    pub prefix: String,
}

impl CompatPathNormalizer {
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
        }
    }

    /// Normalize a foreign path to an NGOS path under the prefix.
    ///
    /// Refuses paths that are empty or contain `..` components (traversal).
    pub fn normalize(
        &self,
        path: &str,
        flavor: CompatPathFlavor,
    ) -> Result<String, CompatAbiError> {
        if path.is_empty() {
            return Err(CompatAbiError::EmptyPath);
        }
        if has_parent_traversal(path) {
            return Err(CompatAbiError::PathTraversal {
                path: path.to_string(),
            });
        }

        let relative = match flavor {
            CompatPathFlavor::WindowsAbsolute => {
                // Expect: `X:\rest` — strip drive letter + `:` + separator
                let rest =
                    strip_windows_drive(path).ok_or_else(|| CompatAbiError::InvalidPathFormat {
                        path: path.to_string(),
                    })?;
                backslash_to_slash(rest)
            }
            CompatPathFlavor::WindowsRelative => backslash_to_slash(path),
            CompatPathFlavor::UnixAbsolute => strip_leading_slashes(path),
            CompatPathFlavor::UnixRelative => path.to_string(),
            CompatPathFlavor::WineDrive => {
                // `X:\rest` → strip drive, treat rest as relative to prefix root
                let rest =
                    strip_windows_drive(path).ok_or_else(|| CompatAbiError::InvalidPathFormat {
                        path: path.to_string(),
                    })?;
                backslash_to_slash(rest)
            }
        };

        let normalized = join_prefix_and_relative(&self.prefix, &relative);
        Ok(normalized)
    }
}

fn strip_windows_drive(path: &str) -> Option<&str> {
    // `X:\rest` or `X:/rest` — drive letter is one ASCII letter
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        let rest = &path[2..];
        let mut index = 0usize;
        let rest_bytes = rest.as_bytes();
        while index < rest_bytes.len() && (rest_bytes[index] == b'\\' || rest_bytes[index] == b'/')
        {
            index += 1;
        }
        Some(&rest[index..])
    } else {
        None
    }
}

fn backslash_to_slash(path: &str) -> String {
    let mut output = String::with_capacity(path.len());
    for byte in path.bytes() {
        if byte == b'\\' {
            output.push('/');
        } else {
            output.push(byte as char);
        }
    }
    output
}

fn strip_leading_slashes(path: &str) -> String {
    let bytes = path.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() && bytes[index] == b'/' {
        index += 1;
    }
    path[index..].to_string()
}

fn has_parent_traversal(path: &str) -> bool {
    let bytes = path.as_bytes();
    let mut start = 0usize;
    let mut index = 0usize;
    while index <= bytes.len() {
        let at_end = index == bytes.len();
        let is_sep = !at_end && (bytes[index] == b'/' || bytes[index] == b'\\');
        if at_end || is_sep {
            let len = index.saturating_sub(start);
            if len == 2 && bytes[start] == b'.' && bytes[start + 1] == b'.' {
                return true;
            }
            start = index.saturating_add(1);
        }
        index += 1;
    }
    false
}

fn join_prefix_and_relative(prefix: &str, relative: &str) -> String {
    let prefix_bytes = prefix.as_bytes();
    let mut end = prefix_bytes.len();
    while end > 0 && prefix_bytes[end - 1] == b'/' {
        end -= 1;
    }
    if relative.is_empty() {
        return prefix[..end].to_string();
    }
    let mut output = String::with_capacity(end + 1 + relative.len());
    output.push_str(&prefix[..end]);
    output.push('/');
    output.push_str(relative);
    output
}

// --- scheduler mapping ---

/// Maps foreign scheduler priorities to NGOS `NativeSchedulerClass`.
pub struct CompatSchedulerMap;

impl CompatSchedulerMap {
    /// Map a Win32 thread priority integer to NGOS scheduler class.
    ///
    /// Win32 range: -15 (IDLE) to +15 (TIME_CRITICAL), 0 = NORMAL.
    pub fn from_win32_priority(priority: i32) -> NativeSchedulerClass {
        match priority {
            p if p >= 15 => NativeSchedulerClass::LatencyCritical,
            p if p >= 2 => NativeSchedulerClass::Interactive,
            p if p >= -1 => NativeSchedulerClass::BestEffort,
            _ => NativeSchedulerClass::Background,
        }
    }

    /// Map a POSIX nice value to NGOS scheduler class.
    ///
    /// POSIX range: -20 (highest) to +19 (lowest).
    pub fn from_posix_nice(nice: i32) -> NativeSchedulerClass {
        match nice {
            n if n <= -10 => NativeSchedulerClass::LatencyCritical,
            n if n <= 0 => NativeSchedulerClass::Interactive,
            n if n <= 10 => NativeSchedulerClass::BestEffort,
            _ => NativeSchedulerClass::Background,
        }
    }

    /// Parse a class name string directly.
    pub fn from_class_name(name: &str) -> Option<NativeSchedulerClass> {
        match name {
            "latency-critical" | "realtime" => Some(NativeSchedulerClass::LatencyCritical),
            "interactive" => Some(NativeSchedulerClass::Interactive),
            "best-effort" | "normal" => Some(NativeSchedulerClass::BestEffort),
            "background" | "idle" => Some(NativeSchedulerClass::Background),
            _ => None,
        }
    }

    pub fn class_name(class: NativeSchedulerClass) -> &'static str {
        match class {
            NativeSchedulerClass::LatencyCritical => "latency-critical",
            NativeSchedulerClass::Interactive => "interactive",
            NativeSchedulerClass::BestEffort => "best-effort",
            NativeSchedulerClass::Background => "background",
        }
    }
}

// --- sync primitives ---

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompatMutexState {
    Unlocked,
    Locked { owner_pid: u64 },
    Contended { owner_pid: u64, waiters: u32 },
}

/// A compatibility mutex — state tracked at the ABI layer.
/// Actual blocking is done via NGOS memory-wait syscalls at the runtime level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatMutex {
    pub id: u32,
    pub state: CompatMutexState,
}

impl CompatMutex {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            state: CompatMutexState::Unlocked,
        }
    }

    /// Try to acquire the mutex for `caller_pid`.
    /// Fails immediately if already locked — no blocking.
    pub fn try_lock(&mut self, caller_pid: u64) -> Result<(), CompatAbiError> {
        match &self.state {
            CompatMutexState::Unlocked => {
                self.state = CompatMutexState::Locked {
                    owner_pid: caller_pid,
                };
                Ok(())
            }
            CompatMutexState::Locked { owner_pid } => {
                let owner = *owner_pid;
                self.state = CompatMutexState::Contended {
                    owner_pid: owner,
                    waiters: 1,
                };
                Err(CompatAbiError::MutexAlreadyLocked {
                    id: self.id,
                    owner_pid: owner,
                })
            }
            CompatMutexState::Contended { owner_pid, waiters } => {
                let owner = *owner_pid;
                let new_waiters = waiters.saturating_add(1);
                self.state = CompatMutexState::Contended {
                    owner_pid: owner,
                    waiters: new_waiters,
                };
                Err(CompatAbiError::MutexAlreadyLocked {
                    id: self.id,
                    owner_pid: owner,
                })
            }
        }
    }

    /// Release the mutex. Fails if `caller_pid` is not the current owner.
    pub fn unlock(&mut self, caller_pid: u64) -> Result<(), CompatAbiError> {
        match &self.state {
            CompatMutexState::Locked { owner_pid } if *owner_pid == caller_pid => {
                self.state = CompatMutexState::Unlocked;
                Ok(())
            }
            CompatMutexState::Contended { owner_pid, .. } if *owner_pid == caller_pid => {
                self.state = CompatMutexState::Unlocked;
                Ok(())
            }
            CompatMutexState::Locked { .. } | CompatMutexState::Contended { .. } => {
                Err(CompatAbiError::MutexNotOwner {
                    id: self.id,
                    caller_pid,
                })
            }
            CompatMutexState::Unlocked => {
                // Unlocking an already-unlocked mutex: no-op (idempotent)
                Ok(())
            }
        }
    }

    pub fn is_locked(&self) -> bool {
        !matches!(self.state, CompatMutexState::Unlocked)
    }

    pub fn owner_pid(&self) -> Option<u64> {
        match &self.state {
            CompatMutexState::Locked { owner_pid }
            | CompatMutexState::Contended { owner_pid, .. } => Some(*owner_pid),
            CompatMutexState::Unlocked => None,
        }
    }

    pub fn state_name(&self) -> &'static str {
        match &self.state {
            CompatMutexState::Unlocked => "unlocked",
            CompatMutexState::Locked { .. } => "locked",
            CompatMutexState::Contended { .. } => "contended",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatEventState {
    Signaled,
    Unsignaled,
}

/// A compatibility event object — manual-reset or auto-reset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatEvent {
    pub id: u32,
    pub auto_reset: bool,
    pub state: CompatEventState,
}

impl CompatEvent {
    pub fn new(id: u32, auto_reset: bool) -> Self {
        Self {
            id,
            auto_reset,
            state: CompatEventState::Unsignaled,
        }
    }

    /// Signal the event.
    pub fn signal(&mut self) {
        self.state = CompatEventState::Signaled;
    }

    /// Reset the event to unsignaled.
    pub fn reset(&mut self) {
        self.state = CompatEventState::Unsignaled;
    }

    /// Check if signaled. If auto-reset, resets after being observed.
    pub fn is_signaled(&mut self) -> bool {
        let signaled = self.state == CompatEventState::Signaled;
        if signaled && self.auto_reset {
            self.state = CompatEventState::Unsignaled;
        }
        signaled
    }

    pub fn state_name(&self) -> &'static str {
        match self.state {
            CompatEventState::Signaled => "signaled",
            CompatEventState::Unsignaled => "unsignaled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatTimerMode {
    OneShot,
    Periodic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatTimer {
    pub id: u32,
    pub armed: bool,
    pub due_tick: u64,
    pub interval_tick: u64,
    pub fire_count: u32,
    pub mode: CompatTimerMode,
}

impl CompatTimer {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            armed: false,
            due_tick: 0,
            interval_tick: 0,
            fire_count: 0,
            mode: CompatTimerMode::OneShot,
        }
    }

    pub fn arm_oneshot(&mut self, due_tick: u64) {
        self.armed = true;
        self.due_tick = due_tick;
        self.interval_tick = 0;
        self.mode = CompatTimerMode::OneShot;
    }

    pub fn arm_periodic(
        &mut self,
        start_tick: u64,
        interval_tick: u64,
    ) -> Result<(), CompatAbiError> {
        if interval_tick == 0 {
            return Err(CompatAbiError::TimerInvalidInterval { id: self.id });
        }
        self.armed = true;
        self.due_tick = start_tick;
        self.interval_tick = interval_tick;
        self.mode = CompatTimerMode::Periodic;
        Ok(())
    }

    pub fn cancel(&mut self) {
        self.armed = false;
    }

    pub fn tick(&mut self, now_tick: u64) -> bool {
        if !self.armed || now_tick < self.due_tick {
            return false;
        }
        self.fire_count = self.fire_count.saturating_add(1);
        match self.mode {
            CompatTimerMode::OneShot => self.armed = false,
            CompatTimerMode::Periodic => {
                self.due_tick = self.due_tick.saturating_add(self.interval_tick);
            }
        }
        true
    }

    pub fn mode_name(&self) -> &'static str {
        match self.mode {
            CompatTimerMode::OneShot => "oneshot",
            CompatTimerMode::Periodic => "periodic",
        }
    }

    pub fn state_name(&self) -> &'static str {
        if self.armed { "armed" } else { "idle" }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatModuleState {
    Loaded,
    Unloaded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatModuleRecord {
    pub id: u32,
    pub name: String,
    pub path: String,
    pub base: u64,
    pub size: u64,
    pub state: CompatModuleState,
    pub ref_count: u32,
}

pub struct CompatModuleRegistry {
    next_id: u32,
    modules: Vec<CompatModuleRecord>,
}

impl CompatModuleRegistry {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            modules: Vec::new(),
        }
    }

    pub fn load(&mut self, name: &str, path: &str, base: u64, size: u64) -> u32 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.modules.push(CompatModuleRecord {
            id,
            name: name.to_string(),
            path: path.to_string(),
            base,
            size,
            state: CompatModuleState::Loaded,
            ref_count: 1,
        });
        id
    }

    pub fn get(&self, id: u32) -> Result<&CompatModuleRecord, CompatAbiError> {
        self.modules
            .iter()
            .find(|module| module.id == id)
            .ok_or(CompatAbiError::ModuleNotFound { id })
    }

    pub fn get_mut(&mut self, id: u32) -> Result<&mut CompatModuleRecord, CompatAbiError> {
        self.modules
            .iter_mut()
            .find(|module| module.id == id)
            .ok_or(CompatAbiError::ModuleNotFound { id })
    }

    pub fn retain(&mut self, id: u32) -> Result<u32, CompatAbiError> {
        let module = self.get_mut(id)?;
        if module.state == CompatModuleState::Unloaded {
            return Err(CompatAbiError::ModuleAlreadyUnloaded { id });
        }
        module.ref_count = module.ref_count.saturating_add(1);
        Ok(module.ref_count)
    }

    pub fn release(&mut self, id: u32) -> Result<u32, CompatAbiError> {
        let module = self.get_mut(id)?;
        if module.state == CompatModuleState::Unloaded {
            return Err(CompatAbiError::ModuleAlreadyUnloaded { id });
        }
        module.ref_count = module.ref_count.saturating_sub(1);
        Ok(module.ref_count)
    }

    pub fn unload(&mut self, id: u32) -> Result<(), CompatAbiError> {
        let module = self.get_mut(id)?;
        if module.state == CompatModuleState::Unloaded {
            return Err(CompatAbiError::ModuleAlreadyUnloaded { id });
        }
        module.state = CompatModuleState::Unloaded;
        module.ref_count = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- handle table ---

    #[test]
    fn handle_table_open_and_get() {
        let mut table = CompatHandleTable::new();
        let id = table.open(CompatHandleKind::Process, 42).unwrap();
        assert!(id >= 1);
        let entry = table.get(id).unwrap();
        assert_eq!(entry.kind, CompatHandleKind::Process);
        assert_eq!(entry.object_id, 42);
        assert_eq!(entry.state, CompatHandleState::Open);
        assert_eq!(entry.ref_count, 1);
        assert_eq!(table.open_count(), 1);
    }

    #[test]
    fn handle_table_close_marks_closed() {
        let mut table = CompatHandleTable::new();
        let id = table.open(CompatHandleKind::Domain, 1).unwrap();
        table.close(id).unwrap();
        assert_eq!(table.open_count(), 0);
        let err = table.get(id).unwrap_err();
        assert_eq!(err, CompatAbiError::HandleAlreadyClosed { id });
    }

    #[test]
    fn handle_table_close_twice_is_error() {
        let mut table = CompatHandleTable::new();
        let id = table.open(CompatHandleKind::Contract, 99).unwrap();
        table.close(id).unwrap();
        let err = table.close(id).unwrap_err();
        assert_eq!(err, CompatAbiError::HandleAlreadyClosed { id });
    }

    #[test]
    fn handle_table_duplicate_creates_independent_handle() {
        let mut table = CompatHandleTable::new();
        let id = table.open(CompatHandleKind::Resource, 10).unwrap();
        let dup_id = table.duplicate(id).unwrap();
        assert_ne!(id, dup_id);
        // both independent — same object_id, different handle IDs
        assert_eq!(table.get(id).unwrap().object_id, 10);
        assert_eq!(table.get(dup_id).unwrap().object_id, 10);
        assert_eq!(table.get(dup_id).unwrap().kind, CompatHandleKind::Resource);
        assert_eq!(table.open_count(), 2);
        // close dup — original unaffected
        table.close(dup_id).unwrap();
        assert_eq!(table.open_count(), 1);
        assert!(table.get(id).is_ok());
        table.close(id).unwrap();
        assert_eq!(table.open_count(), 0);
    }

    #[test]
    fn handle_table_get_missing_is_error() {
        let table = CompatHandleTable::new();
        let err = table.get(999).unwrap_err();
        assert_eq!(err, CompatAbiError::HandleNotFound { id: 999 });
    }

    #[test]
    fn handle_table_all_kinds_open() {
        let mut table = CompatHandleTable::new();
        for kind in [
            CompatHandleKind::Domain,
            CompatHandleKind::Resource,
            CompatHandleKind::Contract,
            CompatHandleKind::Process,
            CompatHandleKind::EventObject,
            CompatHandleKind::Mutex,
            CompatHandleKind::Timer,
        ] {
            let id = table.open(kind, 1).unwrap();
            assert_eq!(table.get(id).unwrap().kind, kind);
        }
        assert_eq!(table.open_count(), 7);
    }

    // --- path normalization ---

    #[test]
    fn normalize_windows_absolute_path() {
        let norm = CompatPathNormalizer::new("/compat/prefix");
        let result = norm
            .normalize("C:\\games\\foo.exe", CompatPathFlavor::WindowsAbsolute)
            .unwrap();
        assert_eq!(result, "/compat/prefix/games/foo.exe");
    }

    #[test]
    fn normalize_windows_relative_path() {
        let norm = CompatPathNormalizer::new("/compat/prefix");
        let result = norm
            .normalize("games\\orbit\\run.exe", CompatPathFlavor::WindowsRelative)
            .unwrap();
        assert_eq!(result, "/compat/prefix/games/orbit/run.exe");
    }

    #[test]
    fn normalize_unix_absolute_path() {
        let norm = CompatPathNormalizer::new("/compat/prefix");
        let result = norm
            .normalize("/usr/bin/foo", CompatPathFlavor::UnixAbsolute)
            .unwrap();
        assert_eq!(result, "/compat/prefix/usr/bin/foo");
    }

    #[test]
    fn normalize_unix_relative_path() {
        let norm = CompatPathNormalizer::new("/compat/prefix");
        let result = norm
            .normalize("usr/bin/foo", CompatPathFlavor::UnixRelative)
            .unwrap();
        assert_eq!(result, "/compat/prefix/usr/bin/foo");
    }

    #[test]
    fn normalize_wine_drive_path() {
        let norm = CompatPathNormalizer::new("/compat/prefix");
        let result = norm
            .normalize("Z:\\Program Files\\game.exe", CompatPathFlavor::WineDrive)
            .unwrap();
        assert_eq!(result, "/compat/prefix/Program Files/game.exe");
    }

    #[test]
    fn normalize_refuses_empty_path() {
        let norm = CompatPathNormalizer::new("/compat/prefix");
        let err = norm
            .normalize("", CompatPathFlavor::UnixAbsolute)
            .unwrap_err();
        assert_eq!(err, CompatAbiError::EmptyPath);
    }

    #[test]
    fn normalize_refuses_traversal() {
        let norm = CompatPathNormalizer::new("/compat/prefix");
        let err = norm
            .normalize("/etc/../passwd", CompatPathFlavor::UnixAbsolute)
            .unwrap_err();
        assert_eq!(
            err,
            CompatAbiError::PathTraversal {
                path: String::from("/etc/../passwd")
            }
        );
    }

    #[test]
    fn normalize_refuses_windows_absolute_without_drive() {
        let norm = CompatPathNormalizer::new("/compat/prefix");
        let err = norm
            .normalize("\\no-drive\\path", CompatPathFlavor::WindowsAbsolute)
            .unwrap_err();
        assert!(matches!(err, CompatAbiError::InvalidPathFormat { .. }));
    }

    // --- scheduler mapping ---

    #[test]
    fn scheduler_map_win32_priorities() {
        assert_eq!(
            CompatSchedulerMap::from_win32_priority(15),
            NativeSchedulerClass::LatencyCritical
        );
        assert_eq!(
            CompatSchedulerMap::from_win32_priority(2),
            NativeSchedulerClass::Interactive
        );
        assert_eq!(
            CompatSchedulerMap::from_win32_priority(0),
            NativeSchedulerClass::BestEffort
        );
        assert_eq!(
            CompatSchedulerMap::from_win32_priority(-1),
            NativeSchedulerClass::BestEffort
        );
        assert_eq!(
            CompatSchedulerMap::from_win32_priority(-15),
            NativeSchedulerClass::Background
        );
    }

    #[test]
    fn scheduler_map_posix_nice() {
        assert_eq!(
            CompatSchedulerMap::from_posix_nice(-20),
            NativeSchedulerClass::LatencyCritical
        );
        assert_eq!(
            CompatSchedulerMap::from_posix_nice(-10),
            NativeSchedulerClass::LatencyCritical
        );
        assert_eq!(
            CompatSchedulerMap::from_posix_nice(-5),
            NativeSchedulerClass::Interactive
        );
        assert_eq!(
            CompatSchedulerMap::from_posix_nice(0),
            NativeSchedulerClass::Interactive
        );
        assert_eq!(
            CompatSchedulerMap::from_posix_nice(5),
            NativeSchedulerClass::BestEffort
        );
        assert_eq!(
            CompatSchedulerMap::from_posix_nice(10),
            NativeSchedulerClass::BestEffort
        );
        assert_eq!(
            CompatSchedulerMap::from_posix_nice(19),
            NativeSchedulerClass::Background
        );
    }

    #[test]
    fn scheduler_map_class_name() {
        assert_eq!(
            CompatSchedulerMap::from_class_name("latency-critical"),
            Some(NativeSchedulerClass::LatencyCritical)
        );
        assert_eq!(
            CompatSchedulerMap::from_class_name("realtime"),
            Some(NativeSchedulerClass::LatencyCritical)
        );
        assert_eq!(
            CompatSchedulerMap::from_class_name("interactive"),
            Some(NativeSchedulerClass::Interactive)
        );
        assert_eq!(
            CompatSchedulerMap::from_class_name("normal"),
            Some(NativeSchedulerClass::BestEffort)
        );
        assert_eq!(
            CompatSchedulerMap::from_class_name("idle"),
            Some(NativeSchedulerClass::Background)
        );
        assert_eq!(CompatSchedulerMap::from_class_name("unknown"), None);
    }

    #[test]
    fn scheduler_class_names_roundtrip() {
        for (class, name) in [
            (NativeSchedulerClass::LatencyCritical, "latency-critical"),
            (NativeSchedulerClass::Interactive, "interactive"),
            (NativeSchedulerClass::BestEffort, "best-effort"),
            (NativeSchedulerClass::Background, "background"),
        ] {
            assert_eq!(CompatSchedulerMap::class_name(class), name);
        }
    }

    // --- mutex ---

    #[test]
    fn mutex_lock_and_unlock() {
        let mut m = CompatMutex::new(1);
        assert!(!m.is_locked());
        m.try_lock(100).unwrap();
        assert!(m.is_locked());
        assert_eq!(m.owner_pid(), Some(100));
        assert_eq!(m.state_name(), "locked");
        m.unlock(100).unwrap();
        assert!(!m.is_locked());
    }

    #[test]
    fn mutex_double_lock_is_error() {
        let mut m = CompatMutex::new(2);
        m.try_lock(100).unwrap();
        let err = m.try_lock(200).unwrap_err();
        assert_eq!(
            err,
            CompatAbiError::MutexAlreadyLocked {
                id: 2,
                owner_pid: 100
            }
        );
        assert!(err.describe().contains("owner=100"));
    }

    #[test]
    fn mutex_unlock_wrong_owner_is_error() {
        let mut m = CompatMutex::new(3);
        m.try_lock(100).unwrap();
        let err = m.unlock(200).unwrap_err();
        assert_eq!(
            err,
            CompatAbiError::MutexNotOwner {
                id: 3,
                caller_pid: 200
            }
        );
    }

    #[test]
    fn mutex_unlock_unlocked_is_noop() {
        let mut m = CompatMutex::new(4);
        m.unlock(100).unwrap();
        assert!(!m.is_locked());
    }

    #[test]
    fn mutex_contended_state_tracks_waiters() {
        let mut m = CompatMutex::new(5);
        m.try_lock(10).unwrap();
        // second caller fails, transitions to Contended
        let err = m.try_lock(20).unwrap_err();
        assert!(matches!(err, CompatAbiError::MutexAlreadyLocked { .. }));
        assert_eq!(m.state_name(), "contended");
        // owner unlocks — state goes back to Unlocked
        m.unlock(10).unwrap();
        assert!(!m.is_locked());
    }

    // --- event ---

    #[test]
    fn event_manual_reset_signal_and_reset() {
        let mut ev = CompatEvent::new(1, false);
        assert_eq!(ev.state_name(), "unsignaled");
        ev.signal();
        assert_eq!(ev.state_name(), "signaled");
        assert!(ev.is_signaled());
        // manual-reset: still signaled after check
        assert!(ev.is_signaled());
        ev.reset();
        assert!(!ev.is_signaled());
    }

    #[test]
    fn event_auto_reset_clears_on_observe() {
        let mut ev = CompatEvent::new(2, true);
        ev.signal();
        assert!(ev.is_signaled());
        // auto-reset: second check returns false
        assert!(!ev.is_signaled());
    }

    #[test]
    fn event_reset_unsignaled_is_noop() {
        let mut ev = CompatEvent::new(3, false);
        ev.reset();
        assert!(!ev.is_signaled());
    }

    #[test]
    fn timer_oneshot_fires_once_and_disarms() {
        let mut timer = CompatTimer::new(1);
        timer.arm_oneshot(10);
        assert!(!timer.tick(9));
        assert!(timer.tick(10));
        assert!(!timer.armed);
        assert_eq!(timer.fire_count, 1);
    }

    #[test]
    fn timer_periodic_rearms_and_tracks_fire_count() {
        let mut timer = CompatTimer::new(2);
        timer.arm_periodic(5, 3).unwrap();
        assert!(timer.tick(5));
        assert_eq!(timer.due_tick, 8);
        assert!(timer.tick(8));
        assert_eq!(timer.fire_count, 2);
        assert!(timer.armed);
    }

    #[test]
    fn timer_periodic_refuses_zero_interval() {
        let mut timer = CompatTimer::new(3);
        let err = timer.arm_periodic(10, 0).unwrap_err();
        assert_eq!(err, CompatAbiError::TimerInvalidInterval { id: 3 });
    }

    #[test]
    fn module_registry_load_retain_release_and_unload() {
        let mut modules = CompatModuleRegistry::new();
        let id = modules.load("d3d11.dll", "/compat/dlls/d3d11.dll", 0x1800_0000, 0x4000);
        assert_eq!(id, 1);
        assert_eq!(modules.retain(id).unwrap(), 2);
        assert_eq!(modules.release(id).unwrap(), 1);
        modules.unload(id).unwrap();
        let module = modules.get(id).unwrap();
        assert_eq!(module.state, CompatModuleState::Unloaded);
        assert_eq!(module.ref_count, 0);
    }

    #[test]
    fn module_registry_refuses_operations_after_unload() {
        let mut modules = CompatModuleRegistry::new();
        let id = modules.load(
            "kernel32.dll",
            "/compat/dlls/kernel32.dll",
            0x1810_0000,
            0x8000,
        );
        modules.unload(id).unwrap();
        assert_eq!(
            modules.retain(id).unwrap_err(),
            CompatAbiError::ModuleAlreadyUnloaded { id }
        );
        assert_eq!(
            modules.release(id).unwrap_err(),
            CompatAbiError::ModuleAlreadyUnloaded { id }
        );
    }
}
