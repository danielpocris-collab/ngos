#![allow(dead_code)]

use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};

use ngos_user_abi::bootstrap::BootOutcomePolicy;
use ngos_user_abi::{BootSessionReport, BootSessionStage, BootSessionStatus};

use crate::serial;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstUserProcessDisposition {
    Running,
    Exited { code: i32 },
    Faulted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstUserProcessOutcome {
    Running,
    SessionIncomplete,
    Success,
    ExitFailure { code: i32 },
    Faulted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootOutcomeAction {
    Continue,
    HaltSuccess,
    HaltFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirstUserProcessStatus {
    pub started: bool,
    pub main_reached: bool,
    pub exited: bool,
    pub faulted: bool,
    pub exit_code: i32,
    pub syscall_count: u64,
    pub last_syscall: u64,
    pub stdout_write_count: u64,
    pub stderr_write_count: u64,
    pub bytes_written: u64,
    pub last_write_fd: u64,
    pub last_write_len: u64,
    pub boot_reported: bool,
    pub boot_report_count: u64,
    pub boot_report_status: u32,
    pub boot_report_stage: u32,
    pub boot_report_code: i32,
    pub boot_report_detail: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootReportError {
    InvalidStatus,
    InvalidStage,
    DuplicateComplete,
    StageRegression,
}

const fn stage_rank(stage: BootSessionStage) -> u32 {
    match stage {
        BootSessionStage::Bootstrap => 0,
        BootSessionStage::NativeRuntime => 1,
        BootSessionStage::Complete => 2,
    }
}

fn emit_terminal_summary(snapshot: FirstUserProcessStatus) {
    let disposition = match snapshot.disposition() {
        FirstUserProcessDisposition::Running => "running",
        FirstUserProcessDisposition::Exited { .. } => "exited",
        FirstUserProcessDisposition::Faulted => "faulted",
    };
    let outcome = match snapshot.outcome() {
        FirstUserProcessOutcome::Running => "running",
        FirstUserProcessOutcome::SessionIncomplete => "session-incomplete",
        FirstUserProcessOutcome::Success => "success",
        FirstUserProcessOutcome::ExitFailure { .. } => "exit-failure",
        FirstUserProcessOutcome::Faulted => "faulted",
    };
    if matches!(
        snapshot.outcome(),
        FirstUserProcessOutcome::Success | FirstUserProcessOutcome::Running
    ) {
        serial::write_bytes(b"FIRST USER PROCESS REPORT\n");
    } else {
        serial::write_stderr_bytes(b"FIRST USER PROCESS FAILURE\n");
    }
    serial::print(format_args!(
        "ngos/x86_64: first user process report disposition={} outcome={} started={} main={} exit_code={} syscalls={} last_syscall={} stdout_writes={} stderr_writes={} bytes_written={} last_write_fd={} last_write_len={} boot_reported={} boot_report_count={} boot_status={} boot_stage={} boot_code={} boot_detail={}\n",
        disposition,
        outcome,
        snapshot.started,
        snapshot.main_reached,
        snapshot.exit_code,
        snapshot.syscall_count,
        snapshot.last_syscall,
        snapshot.stdout_write_count,
        snapshot.stderr_write_count,
        snapshot.bytes_written,
        snapshot.last_write_fd,
        snapshot.last_write_len,
        snapshot.boot_reported,
        snapshot.boot_report_count,
        snapshot.boot_report_status,
        snapshot.boot_report_stage,
        snapshot.boot_report_code,
        snapshot.boot_report_detail
    ));
    crate::diagnostics::record_user_status(snapshot);
    crate::diagnostics::emit_report();
}

fn emit_compact_failure_summary_once(snapshot: FirstUserProcessStatus) -> bool {
    let failure = !matches!(
        snapshot.outcome(),
        FirstUserProcessOutcome::Running | FirstUserProcessOutcome::Success
    );
    if !failure {
        return false;
    }
    if COMPACT_FAILURE_REPORT_EMITTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return false;
    }
    crate::diagnostics::record_user_status(snapshot);
    crate::diagnostics::emit_report_compact();
    crate::diagnostics::emit_patch_suggestion_compact();
    true
}

static STARTED: AtomicBool = AtomicBool::new(false);
static MAIN_REACHED: AtomicBool = AtomicBool::new(false);
static EXITED: AtomicBool = AtomicBool::new(false);
static FAULTED: AtomicBool = AtomicBool::new(false);
static EXIT_CODE: AtomicI32 = AtomicI32::new(0);
static SYSCALL_COUNT: AtomicU64 = AtomicU64::new(0);
static LAST_SYSCALL: AtomicU64 = AtomicU64::new(0);
static STDOUT_WRITE_COUNT: AtomicU64 = AtomicU64::new(0);
static STDERR_WRITE_COUNT: AtomicU64 = AtomicU64::new(0);
static BYTES_WRITTEN: AtomicU64 = AtomicU64::new(0);
static LAST_WRITE_FD: AtomicU64 = AtomicU64::new(0);
static LAST_WRITE_LEN: AtomicU64 = AtomicU64::new(0);
static FINAL_REPORT_EMITTED: AtomicBool = AtomicBool::new(false);
static COMPACT_FAILURE_REPORT_EMITTED: AtomicBool = AtomicBool::new(false);
static OUTCOME_POLICY_APPLIED: AtomicBool = AtomicBool::new(false);
static CONFIGURED_OUTCOME_POLICY: AtomicU64 =
    AtomicU64::new(BootOutcomePolicy::RequireZeroExit as u64);
static BOOT_REPORTED: AtomicBool = AtomicBool::new(false);
static BOOT_REPORT_COUNT: AtomicU64 = AtomicU64::new(0);
static BOOT_REPORT_STATUS: AtomicU64 = AtomicU64::new(BootSessionStatus::Failure as u64);
static BOOT_REPORT_STAGE: AtomicU64 = AtomicU64::new(BootSessionStage::Bootstrap as u64);
static BOOT_REPORT_CODE: AtomicI32 = AtomicI32::new(0);
static BOOT_REPORT_DETAIL: AtomicU64 = AtomicU64::new(0);

pub fn reset() {
    STARTED.store(false, Ordering::SeqCst);
    MAIN_REACHED.store(false, Ordering::SeqCst);
    EXITED.store(false, Ordering::SeqCst);
    FAULTED.store(false, Ordering::SeqCst);
    EXIT_CODE.store(0, Ordering::SeqCst);
    SYSCALL_COUNT.store(0, Ordering::SeqCst);
    LAST_SYSCALL.store(0, Ordering::SeqCst);
    STDOUT_WRITE_COUNT.store(0, Ordering::SeqCst);
    STDERR_WRITE_COUNT.store(0, Ordering::SeqCst);
    BYTES_WRITTEN.store(0, Ordering::SeqCst);
    LAST_WRITE_FD.store(0, Ordering::SeqCst);
    LAST_WRITE_LEN.store(0, Ordering::SeqCst);
    FINAL_REPORT_EMITTED.store(false, Ordering::SeqCst);
    COMPACT_FAILURE_REPORT_EMITTED.store(false, Ordering::SeqCst);
    OUTCOME_POLICY_APPLIED.store(false, Ordering::SeqCst);
    CONFIGURED_OUTCOME_POLICY.store(BootOutcomePolicy::RequireZeroExit as u64, Ordering::SeqCst);
    BOOT_REPORTED.store(false, Ordering::SeqCst);
    BOOT_REPORT_COUNT.store(0, Ordering::SeqCst);
    BOOT_REPORT_STATUS.store(BootSessionStatus::Failure as u64, Ordering::SeqCst);
    BOOT_REPORT_STAGE.store(BootSessionStage::Bootstrap as u64, Ordering::SeqCst);
    BOOT_REPORT_CODE.store(0, Ordering::SeqCst);
    BOOT_REPORT_DETAIL.store(0, Ordering::SeqCst);
    crate::diagnostics::record_user_status(snapshot());
}

pub fn set_boot_outcome_policy(policy: BootOutcomePolicy) {
    CONFIGURED_OUTCOME_POLICY.store(policy as u64, Ordering::SeqCst);
}

pub fn boot_outcome_policy() -> BootOutcomePolicy {
    match CONFIGURED_OUTCOME_POLICY.load(Ordering::SeqCst) as u32 {
        0 => BootOutcomePolicy::RequireZeroExit,
        1 => BootOutcomePolicy::AllowAnyExit,
        _ => BootOutcomePolicy::RequireZeroExit,
    }
}

pub fn mark_started() {
    STARTED.store(true, Ordering::SeqCst);
    crate::diagnostics::record_user_status(snapshot());
}

pub fn mark_main_reached() {
    MAIN_REACHED.store(true, Ordering::SeqCst);
    crate::diagnostics::record_user_status(snapshot());
}

pub fn mark_exit(code: i32) {
    EXIT_CODE.store(code, Ordering::SeqCst);
    EXITED.store(true, Ordering::SeqCst);
    crate::diagnostics::record_user_status(snapshot());
}

pub fn mark_fault() {
    FAULTED.store(true, Ordering::SeqCst);
    crate::diagnostics::record_user_status(snapshot());
}

pub fn record_syscall(number: u64) {
    LAST_SYSCALL.store(number, Ordering::SeqCst);
    SYSCALL_COUNT.fetch_add(1, Ordering::SeqCst);
    crate::diagnostics::record_user_status(snapshot());
}

pub fn record_write(fd: usize, len: usize) {
    let fd = fd as u64;
    let len = len as u64;
    LAST_WRITE_FD.store(fd, Ordering::SeqCst);
    LAST_WRITE_LEN.store(len, Ordering::SeqCst);
    BYTES_WRITTEN.fetch_add(len, Ordering::SeqCst);
    match fd {
        1 => {
            STDOUT_WRITE_COUNT.fetch_add(1, Ordering::SeqCst);
        }
        2 => {
            STDERR_WRITE_COUNT.fetch_add(1, Ordering::SeqCst);
        }
        _ => {}
    }
    crate::diagnostics::record_user_status(snapshot());
}

pub fn record_boot_report(report: BootSessionReport) -> Result<(), BootReportError> {
    let status =
        BootSessionStatus::from_raw(report.status).ok_or(BootReportError::InvalidStatus)?;
    let stage = BootSessionStage::from_raw(report.stage).ok_or(BootReportError::InvalidStage)?;
    let prior_count = BOOT_REPORT_COUNT.load(Ordering::SeqCst);
    if prior_count != 0 {
        let prior_stage =
            BootSessionStage::from_raw(BOOT_REPORT_STAGE.load(Ordering::SeqCst) as u32)
                .ok_or(BootReportError::InvalidStage)?;
        if prior_stage == BootSessionStage::Complete {
            return Err(BootReportError::DuplicateComplete);
        }
        if stage_rank(stage) < stage_rank(prior_stage) {
            return Err(BootReportError::StageRegression);
        }
    }
    BOOT_REPORTED.store(true, Ordering::SeqCst);
    BOOT_REPORT_COUNT.fetch_add(1, Ordering::SeqCst);
    BOOT_REPORT_STATUS.store(status as u64, Ordering::SeqCst);
    BOOT_REPORT_STAGE.store(stage as u64, Ordering::SeqCst);
    BOOT_REPORT_CODE.store(report.code, Ordering::SeqCst);
    BOOT_REPORT_DETAIL.store(report.detail, Ordering::SeqCst);
    let current = snapshot();
    crate::diagnostics::record_user_status(current);
    if stage == BootSessionStage::Complete && status != BootSessionStatus::Success {
        emit_compact_failure_summary_once(current);
    }
    Ok(())
}

pub fn snapshot() -> FirstUserProcessStatus {
    FirstUserProcessStatus {
        started: STARTED.load(Ordering::SeqCst),
        main_reached: MAIN_REACHED.load(Ordering::SeqCst),
        exited: EXITED.load(Ordering::SeqCst),
        faulted: FAULTED.load(Ordering::SeqCst),
        exit_code: EXIT_CODE.load(Ordering::SeqCst),
        syscall_count: SYSCALL_COUNT.load(Ordering::SeqCst),
        last_syscall: LAST_SYSCALL.load(Ordering::SeqCst),
        stdout_write_count: STDOUT_WRITE_COUNT.load(Ordering::SeqCst),
        stderr_write_count: STDERR_WRITE_COUNT.load(Ordering::SeqCst),
        bytes_written: BYTES_WRITTEN.load(Ordering::SeqCst),
        last_write_fd: LAST_WRITE_FD.load(Ordering::SeqCst),
        last_write_len: LAST_WRITE_LEN.load(Ordering::SeqCst),
        boot_reported: BOOT_REPORTED.load(Ordering::SeqCst),
        boot_report_count: BOOT_REPORT_COUNT.load(Ordering::SeqCst),
        boot_report_status: BOOT_REPORT_STATUS.load(Ordering::SeqCst) as u32,
        boot_report_stage: BOOT_REPORT_STAGE.load(Ordering::SeqCst) as u32,
        boot_report_code: BOOT_REPORT_CODE.load(Ordering::SeqCst),
        boot_report_detail: BOOT_REPORT_DETAIL.load(Ordering::SeqCst),
    }
}

pub fn emit_final_report_if_terminal() -> bool {
    let snapshot = snapshot();
    let terminal = matches!(
        snapshot.disposition(),
        FirstUserProcessDisposition::Exited { .. } | FirstUserProcessDisposition::Faulted
    );
    if !terminal {
        return false;
    }
    if FINAL_REPORT_EMITTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return false;
    }

    emit_compact_failure_summary_once(snapshot);
    emit_terminal_summary(snapshot);
    true
}

pub fn apply_boot_outcome_policy(policy: BootOutcomePolicy) -> BootOutcomeAction {
    let snapshot = snapshot();
    let action = match (policy, snapshot.outcome()) {
        (_, FirstUserProcessOutcome::Running) => BootOutcomeAction::Continue,
        (_, FirstUserProcessOutcome::SessionIncomplete) => BootOutcomeAction::HaltFailure,
        (BootOutcomePolicy::RequireZeroExit, FirstUserProcessOutcome::Success) => {
            BootOutcomeAction::HaltSuccess
        }
        (BootOutcomePolicy::AllowAnyExit, FirstUserProcessOutcome::Success)
        | (BootOutcomePolicy::AllowAnyExit, FirstUserProcessOutcome::ExitFailure { .. }) => {
            BootOutcomeAction::HaltSuccess
        }
        (BootOutcomePolicy::RequireZeroExit, FirstUserProcessOutcome::ExitFailure { .. })
        | (BootOutcomePolicy::RequireZeroExit, FirstUserProcessOutcome::Faulted)
        | (BootOutcomePolicy::AllowAnyExit, FirstUserProcessOutcome::Faulted) => {
            BootOutcomeAction::HaltFailure
        }
    };

    if action == BootOutcomeAction::Continue {
        return action;
    }
    if action == BootOutcomeAction::HaltFailure {
        emit_compact_failure_summary_once(snapshot);
    }
    if OUTCOME_POLICY_APPLIED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return action;
    }

    let outcome = match snapshot.outcome() {
        FirstUserProcessOutcome::Running => "running",
        FirstUserProcessOutcome::SessionIncomplete => "session-incomplete",
        FirstUserProcessOutcome::Success => "success",
        FirstUserProcessOutcome::ExitFailure { .. } => "exit-failure",
        FirstUserProcessOutcome::Faulted => "faulted",
    };
    let action_name = match action {
        BootOutcomeAction::Continue => "continue",
        BootOutcomeAction::HaltSuccess => "halt-success",
        BootOutcomeAction::HaltFailure => "halt-failure",
    };
    let marker = match action {
        BootOutcomeAction::Continue => b'O',
        BootOutcomeAction::HaltSuccess => b'Q',
        BootOutcomeAction::HaltFailure => b'X',
    };
    serial::debug_marker(marker);
    #[cfg(target_os = "none")]
    match action {
        BootOutcomeAction::HaltSuccess => {
            crate::reboot_trace::mark_clean_shutdown();
            crate::framebuffer::status_banner("USER MODE COMPLETE");
        }
        BootOutcomeAction::HaltFailure => crate::framebuffer::alert_banner("USER MODE FAILED"),
        BootOutcomeAction::Continue => {}
    }
    serial::print(format_args!(
        "ngos/x86_64: first user process boot outcome policy={:?} outcome={} action={} exit_code={}\n",
        policy, outcome, action_name, snapshot.exit_code
    ));
    action
}

pub fn apply_configured_boot_outcome_policy() -> BootOutcomeAction {
    apply_boot_outcome_policy(boot_outcome_policy())
}

impl FirstUserProcessStatus {
    pub const fn disposition(self) -> FirstUserProcessDisposition {
        if self.faulted {
            FirstUserProcessDisposition::Faulted
        } else if self.exited {
            FirstUserProcessDisposition::Exited {
                code: self.exit_code,
            }
        } else {
            FirstUserProcessDisposition::Running
        }
    }

    pub const fn outcome(self) -> FirstUserProcessOutcome {
        if self.boot_reported {
            if self.boot_report_stage != BootSessionStage::Complete as u32 {
                return match self.disposition() {
                    FirstUserProcessDisposition::Running => FirstUserProcessOutcome::Running,
                    _ => FirstUserProcessOutcome::SessionIncomplete,
                };
            }
            return match self.boot_report_status {
                x if x == BootSessionStatus::Success as u32 => FirstUserProcessOutcome::Success,
                _ => FirstUserProcessOutcome::ExitFailure {
                    code: self.boot_report_code,
                },
            };
        }
        match self.disposition() {
            FirstUserProcessDisposition::Running => FirstUserProcessOutcome::Running,
            FirstUserProcessDisposition::Faulted => FirstUserProcessOutcome::Faulted,
            FirstUserProcessDisposition::Exited { code: 0 } => FirstUserProcessOutcome::Success,
            FirstUserProcessDisposition::Exited { code } => {
                FirstUserProcessOutcome::ExitFailure { code }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::string::String;
    use std::sync::{Mutex, MutexGuard};

    static TEST_GUARD: Mutex<()> = Mutex::new(());

    struct TestGuards {
        _state: MutexGuard<'static, ()>,
        _io: MutexGuard<'static, ()>,
    }

    fn lock_test_state() -> TestGuards {
        TestGuards {
            _state: TEST_GUARD
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            _io: crate::serial::lock_test_io(),
        }
    }

    #[test]
    fn runtime_status_tracks_first_user_process_lifecycle() {
        let _guard = lock_test_state();
        reset();
        mark_started();
        mark_main_reached();
        record_syscall(1);
        record_syscall(2);
        record_write(1, 24);
        mark_exit(7);

        assert_eq!(
            snapshot(),
            FirstUserProcessStatus {
                started: true,
                main_reached: true,
                exited: true,
                faulted: false,
                exit_code: 7,
                syscall_count: 2,
                last_syscall: 2,
                stdout_write_count: 1,
                stderr_write_count: 0,
                bytes_written: 24,
                last_write_fd: 1,
                last_write_len: 24,
                boot_reported: false,
                boot_report_count: 0,
                boot_report_status: BootSessionStatus::Failure as u32,
                boot_report_stage: BootSessionStage::Bootstrap as u32,
                boot_report_code: 0,
                boot_report_detail: 0,
            }
        );
    }

    #[test]
    fn runtime_status_can_record_fault_without_exit() {
        let _guard = lock_test_state();
        reset();
        mark_started();
        mark_fault();

        let snapshot = snapshot();
        assert!(snapshot.started);
        assert!(snapshot.faulted);
        assert!(!snapshot.exited);
        assert_eq!(snapshot.disposition(), FirstUserProcessDisposition::Faulted);
    }

    #[test]
    fn runtime_status_tracks_stdout_and_stderr_writes() {
        let _guard = lock_test_state();
        reset();
        record_write(1, 11);
        record_write(2, 7);

        let snapshot = snapshot();
        assert_eq!(snapshot.stdout_write_count, 1);
        assert_eq!(snapshot.stderr_write_count, 1);
        assert_eq!(snapshot.bytes_written, 18);
        assert_eq!(snapshot.last_write_fd, 2);
        assert_eq!(snapshot.last_write_len, 7);
    }

    #[test]
    fn runtime_status_disposition_reports_running_and_exit() {
        let _guard = lock_test_state();
        reset();
        assert_eq!(
            snapshot().disposition(),
            FirstUserProcessDisposition::Running
        );

        mark_exit(9);
        assert_eq!(
            snapshot().disposition(),
            FirstUserProcessDisposition::Exited { code: 9 }
        );
    }

    #[test]
    fn final_report_emits_once_for_terminal_state() {
        let _guard = lock_test_state();
        reset();
        mark_exit(5);
        crate::serial::clear_output();

        assert!(emit_final_report_if_terminal());
        assert!(!emit_final_report_if_terminal());
        let output = String::from_utf8(crate::serial::take_output()).expect("utf8");
        let error = String::from_utf8(crate::serial::take_error_output()).expect("utf8");
        assert!(
            output.contains("FIRST USER PROCESS FAILURE")
                || error.contains("FIRST USER PROCESS FAILURE")
        );
        assert!(output.contains("outcome=exit-failure") || error.contains("outcome=exit-failure"));
    }

    #[test]
    fn final_report_does_not_emit_while_running() {
        let _guard = lock_test_state();
        reset();
        mark_started();

        assert!(!emit_final_report_if_terminal());
    }

    #[test]
    fn outcome_distinguishes_success_failure_and_fault() {
        let _guard = lock_test_state();
        reset();
        mark_exit(0);
        assert_eq!(snapshot().outcome(), FirstUserProcessOutcome::Success);

        reset();
        mark_exit(3);
        assert_eq!(
            snapshot().outcome(),
            FirstUserProcessOutcome::ExitFailure { code: 3 }
        );

        reset();
        mark_fault();
        assert_eq!(snapshot().outcome(), FirstUserProcessOutcome::Faulted);
    }

    #[test]
    fn boot_outcome_policy_maps_zero_exit_to_success_halt() {
        let _guard = lock_test_state();
        reset();
        mark_exit(0);
        crate::serial::clear_output();

        assert_eq!(
            apply_boot_outcome_policy(BootOutcomePolicy::RequireZeroExit),
            BootOutcomeAction::HaltSuccess
        );
        assert_eq!(
            apply_boot_outcome_policy(BootOutcomePolicy::RequireZeroExit),
            BootOutcomeAction::HaltSuccess
        );
        let output = String::from_utf8(crate::serial::take_output()).expect("utf8");
        assert!(output.contains("action=halt-success"));
    }

    #[test]
    fn boot_outcome_policy_maps_failure_exit_and_fault_to_failure_halt() {
        let _guard = lock_test_state();
        reset();
        mark_exit(7);
        crate::serial::clear_output();
        assert_eq!(
            apply_boot_outcome_policy(BootOutcomePolicy::RequireZeroExit),
            BootOutcomeAction::HaltFailure
        );
        let output = String::from_utf8(crate::serial::take_output()).expect("utf8");
        assert!(output.contains("action=halt-failure"));

        reset();
        mark_fault();
        crate::serial::clear_output();
        assert_eq!(
            apply_boot_outcome_policy(BootOutcomePolicy::RequireZeroExit),
            BootOutcomeAction::HaltFailure
        );
        let output = String::from_utf8(crate::serial::take_output()).expect("utf8");
        assert!(output.contains("outcome=faulted"));
    }

    #[test]
    fn configured_boot_outcome_policy_can_be_updated() {
        let _guard = lock_test_state();
        reset();
        set_boot_outcome_policy(BootOutcomePolicy::AllowAnyExit);
        assert_eq!(boot_outcome_policy(), BootOutcomePolicy::AllowAnyExit);

        mark_exit(9);
        assert_eq!(
            apply_configured_boot_outcome_policy(),
            BootOutcomeAction::HaltSuccess
        );
    }

    #[test]
    fn boot_report_is_captured_and_drives_outcome() {
        let _guard = lock_test_state();
        reset();
        record_boot_report(BootSessionReport {
            status: BootSessionStatus::Failure as u32,
            stage: BootSessionStage::Complete as u32,
            code: 42,
            reserved: 0,
            detail: 99,
        })
        .unwrap();

        let snapshot = snapshot();
        assert!(snapshot.boot_reported);
        assert_eq!(snapshot.boot_report_count, 1);
        assert_eq!(
            snapshot.boot_report_status,
            BootSessionStatus::Failure as u32
        );
        assert_eq!(
            snapshot.boot_report_stage,
            BootSessionStage::Complete as u32
        );
        assert_eq!(snapshot.boot_report_code, 42);
        assert_eq!(snapshot.boot_report_detail, 99);
        assert_eq!(
            snapshot.outcome(),
            FirstUserProcessOutcome::ExitFailure { code: 42 }
        );
    }

    #[test]
    fn boot_report_progression_is_monotonic() {
        let _guard = lock_test_state();
        reset();
        record_boot_report(BootSessionReport {
            status: BootSessionStatus::Success as u32,
            stage: BootSessionStage::Bootstrap as u32,
            code: 0,
            reserved: 0,
            detail: 1,
        })
        .unwrap();
        record_boot_report(BootSessionReport {
            status: BootSessionStatus::Success as u32,
            stage: BootSessionStage::NativeRuntime as u32,
            code: 0,
            reserved: 0,
            detail: 2,
        })
        .unwrap();
        assert_eq!(
            record_boot_report(BootSessionReport {
                status: BootSessionStatus::Success as u32,
                stage: BootSessionStage::Bootstrap as u32,
                code: 0,
                reserved: 0,
                detail: 3,
            }),
            Err(BootReportError::StageRegression)
        );
    }

    #[test]
    fn duplicate_complete_report_is_rejected() {
        let _guard = lock_test_state();
        reset();
        record_boot_report(BootSessionReport {
            status: BootSessionStatus::Success as u32,
            stage: BootSessionStage::Complete as u32,
            code: 0,
            reserved: 0,
            detail: 7,
        })
        .unwrap();
        assert_eq!(snapshot().outcome(), FirstUserProcessOutcome::Success);
        assert_eq!(
            record_boot_report(BootSessionReport {
                status: BootSessionStatus::Failure as u32,
                stage: BootSessionStage::Complete as u32,
                code: 9,
                reserved: 0,
                detail: 8,
            }),
            Err(BootReportError::DuplicateComplete)
        );
    }

    #[test]
    fn incomplete_boot_session_fails_even_with_zero_exit() {
        let _guard = lock_test_state();
        reset();
        record_boot_report(BootSessionReport {
            status: BootSessionStatus::Success as u32,
            stage: BootSessionStage::NativeRuntime as u32,
            code: 0,
            reserved: 0,
            detail: 11,
        })
        .unwrap();
        mark_exit(0);
        assert_eq!(
            snapshot().outcome(),
            FirstUserProcessOutcome::SessionIncomplete
        );
        assert_eq!(
            apply_boot_outcome_policy(BootOutcomePolicy::RequireZeroExit),
            BootOutcomeAction::HaltFailure
        );
        reset();
        record_boot_report(BootSessionReport {
            status: BootSessionStatus::Success as u32,
            stage: BootSessionStage::NativeRuntime as u32,
            code: 0,
            reserved: 0,
            detail: 11,
        })
        .unwrap();
        mark_exit(0);
        assert_eq!(
            apply_boot_outcome_policy(BootOutcomePolicy::AllowAnyExit),
            BootOutcomeAction::HaltFailure
        );
    }
}
