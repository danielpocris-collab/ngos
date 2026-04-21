//! Canonical subsystem role:
//! - subsystem: boot CPU runtime status
//! - owner layer: Layer 0
//! - semantic owner: `boot-x86_64`
//! - truth path role: boot-time CPU bring-up and provider installation status
//!   snapshot
//!
//! Canonical contract families produced here:
//! - CPU bring-up status contracts
//! - CPU feature activation status contracts
//! - CPU hardware-provider install status contracts
//!
//! This module may expose authoritative boot-stage CPU status, but that truth
//! remains bounded to the boot/handoff phase and must not replace long-term
//! runtime ownership in `kernel-core`.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
#[cfg(test)]
use std::sync::{Mutex, MutexGuard, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuRuntimeStatus {
    pub sse_ready: bool,
    pub xsave_enabled: bool,
    pub save_area_bytes: u32,
    pub fsgsbase_enabled: bool,
    pub pcid_enabled: bool,
    pub invpcid_available: bool,
    pub pku_enabled: bool,
    pub smep_enabled: bool,
    pub smap_enabled: bool,
    pub umip_enabled: bool,
    pub xcr0: u64,
    pub probe_attempted: bool,
    pub probe_saved: bool,
    pub probe_restored: bool,
    pub probe_required_bytes: u32,
    pub probe_refusal_code: u32,
    pub probe_seed_marker: u64,
    pub tlb_flush_count: u64,
    pub last_tlb_flush_method: u32,
    pub local_apic_mode: u32,
    pub hardware_provider_installed: bool,
    pub hardware_provider_skipped: bool,
    pub hardware_provider_install_attempts: u64,
    pub hardware_provider_refusal_code: u32,
}

static SSE_READY: AtomicBool = AtomicBool::new(false);
static XSAVE_ENABLED: AtomicBool = AtomicBool::new(false);
static SAVE_AREA_BYTES: AtomicU64 = AtomicU64::new(0);
static FSGSBASE_ENABLED: AtomicBool = AtomicBool::new(false);
static PCID_ENABLED: AtomicBool = AtomicBool::new(false);
static INVPCID_AVAILABLE: AtomicBool = AtomicBool::new(false);
static PKU_ENABLED: AtomicBool = AtomicBool::new(false);
static SMEP_ENABLED: AtomicBool = AtomicBool::new(false);
static SMAP_ENABLED: AtomicBool = AtomicBool::new(false);
static UMIP_ENABLED: AtomicBool = AtomicBool::new(false);
static XCR0: AtomicU64 = AtomicU64::new(0);
static PROBE_ATTEMPTED: AtomicBool = AtomicBool::new(false);
static PROBE_SAVED: AtomicBool = AtomicBool::new(false);
static PROBE_RESTORED: AtomicBool = AtomicBool::new(false);
static PROBE_REQUIRED_BYTES: AtomicU64 = AtomicU64::new(0);
static PROBE_REFUSAL_CODE: AtomicU64 = AtomicU64::new(0);
static PROBE_SEED_MARKER: AtomicU64 = AtomicU64::new(0);
static TLB_FLUSH_COUNT: AtomicU64 = AtomicU64::new(0);
static LAST_TLB_FLUSH_METHOD: AtomicU64 = AtomicU64::new(0);
static LOCAL_APIC_MODE: AtomicU64 = AtomicU64::new(0);
static HARDWARE_PROVIDER_INSTALLED: AtomicBool = AtomicBool::new(false);
static HARDWARE_PROVIDER_SKIPPED: AtomicBool = AtomicBool::new(false);
static HARDWARE_PROVIDER_INSTALL_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static HARDWARE_PROVIDER_REFUSAL_CODE: AtomicU64 = AtomicU64::new(0);

pub fn reset() {
    SSE_READY.store(false, Ordering::SeqCst);
    XSAVE_ENABLED.store(false, Ordering::SeqCst);
    SAVE_AREA_BYTES.store(0, Ordering::SeqCst);
    FSGSBASE_ENABLED.store(false, Ordering::SeqCst);
    PCID_ENABLED.store(false, Ordering::SeqCst);
    INVPCID_AVAILABLE.store(false, Ordering::SeqCst);
    PKU_ENABLED.store(false, Ordering::SeqCst);
    SMEP_ENABLED.store(false, Ordering::SeqCst);
    SMAP_ENABLED.store(false, Ordering::SeqCst);
    UMIP_ENABLED.store(false, Ordering::SeqCst);
    XCR0.store(0, Ordering::SeqCst);
    PROBE_ATTEMPTED.store(false, Ordering::SeqCst);
    PROBE_SAVED.store(false, Ordering::SeqCst);
    PROBE_RESTORED.store(false, Ordering::SeqCst);
    PROBE_REQUIRED_BYTES.store(0, Ordering::SeqCst);
    PROBE_REFUSAL_CODE.store(0, Ordering::SeqCst);
    PROBE_SEED_MARKER.store(0, Ordering::SeqCst);
    TLB_FLUSH_COUNT.store(0, Ordering::SeqCst);
    LAST_TLB_FLUSH_METHOD.store(0, Ordering::SeqCst);
    LOCAL_APIC_MODE.store(0, Ordering::SeqCst);
    HARDWARE_PROVIDER_INSTALLED.store(false, Ordering::SeqCst);
    HARDWARE_PROVIDER_SKIPPED.store(false, Ordering::SeqCst);
    HARDWARE_PROVIDER_INSTALL_ATTEMPTS.store(0, Ordering::SeqCst);
    HARDWARE_PROVIDER_REFUSAL_CODE.store(0, Ordering::SeqCst);
}

#[allow(dead_code)]
pub fn record(
    sse_ready: bool,
    xsave_enabled: bool,
    save_area_bytes: u32,
    fsgsbase_enabled: bool,
    pcid_enabled: bool,
    invpcid_available: bool,
    pku_enabled: bool,
    smep_enabled: bool,
    smap_enabled: bool,
    umip_enabled: bool,
    xcr0: u64,
) {
    SSE_READY.store(sse_ready, Ordering::SeqCst);
    XSAVE_ENABLED.store(xsave_enabled, Ordering::SeqCst);
    SAVE_AREA_BYTES.store(save_area_bytes as u64, Ordering::SeqCst);
    FSGSBASE_ENABLED.store(fsgsbase_enabled, Ordering::SeqCst);
    PCID_ENABLED.store(pcid_enabled, Ordering::SeqCst);
    INVPCID_AVAILABLE.store(invpcid_available, Ordering::SeqCst);
    PKU_ENABLED.store(pku_enabled, Ordering::SeqCst);
    SMEP_ENABLED.store(smep_enabled, Ordering::SeqCst);
    SMAP_ENABLED.store(smap_enabled, Ordering::SeqCst);
    UMIP_ENABLED.store(umip_enabled, Ordering::SeqCst);
    XCR0.store(xcr0, Ordering::SeqCst);
}

#[allow(dead_code)]
pub fn record_probe(
    attempted: bool,
    saved: bool,
    restored: bool,
    required_bytes: u32,
    refusal_code: u32,
    seed_marker: u64,
) {
    PROBE_ATTEMPTED.store(attempted, Ordering::SeqCst);
    PROBE_SAVED.store(saved, Ordering::SeqCst);
    PROBE_RESTORED.store(restored, Ordering::SeqCst);
    PROBE_REQUIRED_BYTES.store(required_bytes as u64, Ordering::SeqCst);
    PROBE_REFUSAL_CODE.store(refusal_code as u64, Ordering::SeqCst);
    PROBE_SEED_MARKER.store(seed_marker, Ordering::SeqCst);
}

#[allow(dead_code)]
pub fn record_tlb_flush(method: u32) {
    TLB_FLUSH_COUNT.fetch_add(1, Ordering::SeqCst);
    LAST_TLB_FLUSH_METHOD.store(method as u64, Ordering::SeqCst);
}

#[allow(dead_code)]
pub fn record_local_apic_mode(mode: u32) {
    LOCAL_APIC_MODE.store(mode as u64, Ordering::SeqCst);
}

#[allow(dead_code)]
pub fn record_hardware_provider_install(installed: bool, skipped: bool, refusal_code: u32) {
    HARDWARE_PROVIDER_INSTALL_ATTEMPTS.fetch_add(1, Ordering::SeqCst);
    HARDWARE_PROVIDER_INSTALLED.store(installed, Ordering::SeqCst);
    HARDWARE_PROVIDER_SKIPPED.store(skipped, Ordering::SeqCst);
    HARDWARE_PROVIDER_REFUSAL_CODE.store(refusal_code as u64, Ordering::SeqCst);
}

pub fn snapshot() -> CpuRuntimeStatus {
    CpuRuntimeStatus {
        sse_ready: SSE_READY.load(Ordering::SeqCst),
        xsave_enabled: XSAVE_ENABLED.load(Ordering::SeqCst),
        save_area_bytes: SAVE_AREA_BYTES.load(Ordering::SeqCst) as u32,
        fsgsbase_enabled: FSGSBASE_ENABLED.load(Ordering::SeqCst),
        pcid_enabled: PCID_ENABLED.load(Ordering::SeqCst),
        invpcid_available: INVPCID_AVAILABLE.load(Ordering::SeqCst),
        pku_enabled: PKU_ENABLED.load(Ordering::SeqCst),
        smep_enabled: SMEP_ENABLED.load(Ordering::SeqCst),
        smap_enabled: SMAP_ENABLED.load(Ordering::SeqCst),
        umip_enabled: UMIP_ENABLED.load(Ordering::SeqCst),
        xcr0: XCR0.load(Ordering::SeqCst),
        probe_attempted: PROBE_ATTEMPTED.load(Ordering::SeqCst),
        probe_saved: PROBE_SAVED.load(Ordering::SeqCst),
        probe_restored: PROBE_RESTORED.load(Ordering::SeqCst),
        probe_required_bytes: PROBE_REQUIRED_BYTES.load(Ordering::SeqCst) as u32,
        probe_refusal_code: PROBE_REFUSAL_CODE.load(Ordering::SeqCst) as u32,
        probe_seed_marker: PROBE_SEED_MARKER.load(Ordering::SeqCst),
        tlb_flush_count: TLB_FLUSH_COUNT.load(Ordering::SeqCst),
        last_tlb_flush_method: LAST_TLB_FLUSH_METHOD.load(Ordering::SeqCst) as u32,
        local_apic_mode: LOCAL_APIC_MODE.load(Ordering::SeqCst) as u32,
        hardware_provider_installed: HARDWARE_PROVIDER_INSTALLED.load(Ordering::SeqCst),
        hardware_provider_skipped: HARDWARE_PROVIDER_SKIPPED.load(Ordering::SeqCst),
        hardware_provider_install_attempts: HARDWARE_PROVIDER_INSTALL_ATTEMPTS
            .load(Ordering::SeqCst),
        hardware_provider_refusal_code: HARDWARE_PROVIDER_REFUSAL_CODE.load(Ordering::SeqCst)
            as u32,
    }
}

#[cfg(test)]
pub(crate) fn lock_shared_test_state() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_runtime_status_tracks_extended_state_snapshot() {
        let _guard = lock_shared_test_state();
        reset();
        let initial = snapshot();
        assert!(!initial.xsave_enabled);
        assert_eq!(initial.xcr0, 0);

        record(
            true, true, 4096, true, true, true, true, true, true, true, 0xe7,
        );

        let current = snapshot();
        assert!(current.sse_ready);
        assert!(current.xsave_enabled);
        assert_eq!(current.save_area_bytes, 4096);
        assert!(current.fsgsbase_enabled);
        assert!(current.pcid_enabled);
        assert!(current.invpcid_available);
        assert!(current.pku_enabled);
        assert!(current.smep_enabled);
        assert!(current.smap_enabled);
        assert!(current.umip_enabled);
        assert_eq!(current.xcr0, 0xe7);
        assert!(!current.probe_attempted);
        assert_eq!(current.tlb_flush_count, 0);
        assert_eq!(current.local_apic_mode, 0);
        assert!(!current.hardware_provider_installed);
        assert!(!current.hardware_provider_skipped);
        assert_eq!(current.hardware_provider_install_attempts, 0);
        assert_eq!(current.hardware_provider_refusal_code, 0);
    }

    #[test]
    fn cpu_runtime_status_tracks_probe_outcome() {
        let _guard = lock_shared_test_state();
        reset();
        record_probe(true, true, true, 4096, 0, 0xfeed_beef);

        let current = snapshot();
        assert!(current.probe_attempted);
        assert!(current.probe_saved);
        assert!(current.probe_restored);
        assert_eq!(current.probe_required_bytes, 4096);
        assert_eq!(current.probe_refusal_code, 0);
        assert_eq!(current.probe_seed_marker, 0xfeed_beef);
    }

    #[test]
    fn cpu_runtime_status_tracks_tlb_flush_method() {
        let _guard = lock_shared_test_state();
        reset();
        record_tlb_flush(2);
        record_tlb_flush(1);

        let current = snapshot();
        assert_eq!(current.tlb_flush_count, 2);
        assert_eq!(current.last_tlb_flush_method, 1);
    }

    #[test]
    fn cpu_runtime_status_tracks_local_apic_mode() {
        let _guard = lock_shared_test_state();
        reset();
        record_local_apic_mode(2);
        assert_eq!(snapshot().local_apic_mode, 2);
    }

    #[test]
    fn cpu_runtime_status_tracks_hardware_provider_installation() {
        let _guard = lock_shared_test_state();
        reset();
        record_hardware_provider_install(true, false, 0);
        let current = snapshot();
        assert!(current.hardware_provider_installed);
        assert!(!current.hardware_provider_skipped);
        assert_eq!(current.hardware_provider_install_attempts, 1);
        assert_eq!(current.hardware_provider_refusal_code, 0);

        record_hardware_provider_install(false, true, 1);
        let current = snapshot();
        assert!(!current.hardware_provider_installed);
        assert!(current.hardware_provider_skipped);
        assert_eq!(current.hardware_provider_install_attempts, 2);
        assert_eq!(current.hardware_provider_refusal_code, 1);
    }
}
