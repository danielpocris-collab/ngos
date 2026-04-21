#![cfg_attr(not(target_os = "none"), allow(dead_code))]

//! Canonical subsystem role:
//! - subsystem: boot SMP bring-up
//! - owner layer: Layer 0
//! - semantic owner: `boot-x86_64`
//! - truth path role: boot-stage AP discovery and startup mechanics for the
//!   real x86 path
//!
//! Canonical contract families handled here:
//! - AP bootstrap contracts
//! - SMP bring-up contracts
//! - AP mailbox/counter bootstrap contracts
//!
//! This module may bring up secondary processors at boot, but it must not
//! redefine the higher-level scheduler or runtime ownership model.

use alloc::vec::Vec;
use core::arch::asm;
use core::mem::size_of;
use core::ptr;

use platform_x86_64::{
    ApicTopologyInfo, BootInfo, EarlyFrameAllocator, PAGE_SIZE_4K, apic_topology,
};

use crate::boot_locator::{
    self, BootLocatorKind, BootLocatorSeverity, BootLocatorStage, BootPayloadLabel,
};

pub const AP_TRAMPOLINE_LEN: u32 = 0x1_000;
pub const AP_MAILBOX_LEN: u32 = 0x1_000;
pub const AP_COUNTERS_LEN: u32 = 0x1_000;
pub const AP_JOB_RING_LEN: u32 = 0x1_000;
pub const AP_STACK_BYTES: u64 = 4 * PAGE_SIZE_4K;
const AP_LOW_MEMORY_LIMIT: u64 = 0x10_0000;
const AP_BOOTSTRAP_WINDOW_BYTES: u64 = AP_TRAMPOLINE_LEN as u64
    + AP_MAILBOX_LEN as u64
    + AP_COUNTERS_LEN as u64
    + AP_JOB_RING_LEN as u64;
const AP_BOOTSTRAP_WINDOW_FRAMES: usize = (AP_BOOTSTRAP_WINDOW_BYTES / PAGE_SIZE_4K) as usize;

pub const JOB_RING_CAPACITY: usize = 16;
pub const AP_MAILBOX_CAPACITY: usize = 64;
pub const AP_JOB_COMMAND_STARTUP: u32 = 1;
pub const AP_STATE_OFFLINE: u32 = 0;
pub const AP_STATE_DISPATCHED: u32 = 1;
pub const AP_STATE_ONLINE: u32 = 2;
pub const AP_RENDEZVOUS_DESCRIPTOR_OFFSET: usize = 0x400;
pub const AP_TRAMPOLINE_CODE_LIMIT: usize = 0x3c0;

#[allow(dead_code)]
const APIC_REG_ID: u32 = 0x20;
const APIC_REG_SPURIOUS: u32 = 0xF0;
const APIC_REG_ICR_LOW: u32 = 0x300;
const APIC_REG_ICR_HIGH: u32 = 0x310;
const APIC_SPURIOUS_ENABLE: u32 = 1 << 8;
const APIC_DELIVERY_STATUS_PENDING: u32 = 1 << 12;
const APIC_DELIVERY_MODE_INIT: u32 = 0b101 << 8;
const APIC_DELIVERY_MODE_STARTUP: u32 = 0b110 << 8;
const APIC_LEVEL_ASSERT: u32 = 1 << 14;
const APIC_TRIGGER_LEVEL: u32 = 1 << 15;
const APIC_DESTINATION_PHYSICAL: u32 = 0;
const APIC_INIT_COMMAND: u32 =
    APIC_DELIVERY_MODE_INIT | APIC_LEVEL_ASSERT | APIC_TRIGGER_LEVEL | APIC_DESTINATION_PHYSICAL;

#[cfg(target_os = "none")]
unsafe extern "C" {
    static __ngos_x86_64_ap_trampoline_start: u8;
    static __ngos_x86_64_ap_trampoline_end: u8;
    static __ngos_x86_64_ap_pm_entry_ptr: u8;
    static __ngos_x86_64_ap_pm_stack_ptr: u8;
    static __ngos_x86_64_ap_rendezvous_cr3_ptr: u8;
    static __ngos_x86_64_ap_gdt16_base_ptr: u8;
    static __ngos_x86_64_ap_gdt64_linear_ptr: u8;
    static __ngos_x86_64_ap_gdt64_base_ptr: u8;
    static __ngos_x86_64_ap_lm_entry_ptr: u8;
    static __ngos_x86_64_ap_lm_stack_ptr: u8;
    static __ngos_x86_64_ap_rendezvous_ptr: u8;
    static __ngos_x86_64_ap_rust_entry_ptr: u8;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct ApMailboxEntry {
    pub apic_id: u32,
    pub processor_uid: u32,
    pub stack_top: u64,
    pub entry_point: u64,
    pub page_table_root: u64,
    pub flags: u32,
    pub reserved: u32,
}

impl ApMailboxEntry {
    #[allow(dead_code)]
    pub const fn empty() -> Self {
        Self {
            apic_id: 0,
            processor_uid: 0,
            stack_top: 0,
            entry_point: 0,
            page_table_root: 0,
            flags: 0,
            reserved: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct ApJobPacket {
    pub apic_id: u32,
    pub command: u32,
    pub target_cr3: u64,
    pub target_rip: u64,
    pub target_rsp: u64,
    pub argument0: u64,
    pub argument1: u64,
}

impl ApJobPacket {
    pub const fn empty() -> Self {
        Self {
            apic_id: 0,
            command: 0,
            target_cr3: 0,
            target_rip: 0,
            target_rsp: 0,
            argument0: 0,
            argument1: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct ApJobRing {
    pub head: u32,
    pub tail: u32,
    pub capacity: u32,
    pub reserved: u32,
    pub entries: [ApJobPacket; JOB_RING_CAPACITY],
}

impl ApJobRing {
    pub const fn empty() -> Self {
        Self {
            head: 0,
            tail: 0,
            capacity: JOB_RING_CAPACITY as u32,
            reserved: 0,
            entries: [ApJobPacket::empty(); JOB_RING_CAPACITY],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct ApCounterEntry {
    pub apic_id: u32,
    pub state: u32,
    pub generation: u32,
    pub jobs_completed: u32,
}

impl ApCounterEntry {
    #[allow(dead_code)]
    pub const fn empty() -> Self {
        Self {
            apic_id: 0,
            state: AP_STATE_OFFLINE,
            generation: 0,
            jobs_completed: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct ApRendezvousDescriptor {
    pub signature: u64,
    pub version: u32,
    pub trampoline_base: u32,
    pub bootstrap_cr3: u64,
    pub physical_memory_offset: u64,
    pub local_apic_address: u64,
    pub mailbox_base: u64,
    pub counters_base: u64,
    pub job_ring_base: u64,
    pub ap_entry_point: u64,
}

impl ApRendezvousDescriptor {
    pub const SIGNATURE: u64 = 0x534f_474e_5f41_5052;
    pub const VERSION: u32 = 1;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmpBootstrapLayout {
    pub trampoline_base: u64,
    pub trampoline_len: u32,
    pub mailbox_base: u64,
    pub mailbox_len: u32,
    pub counters_base: u64,
    pub counters_len: u32,
    pub job_ring_base: u64,
    pub job_ring_len: u32,
}

impl SmpBootstrapLayout {
    pub const fn from_trampoline_base(trampoline_base: u64) -> Self {
        Self {
            trampoline_base,
            trampoline_len: AP_TRAMPOLINE_LEN,
            mailbox_base: trampoline_base + AP_TRAMPOLINE_LEN as u64,
            mailbox_len: AP_MAILBOX_LEN,
            counters_base: trampoline_base + AP_TRAMPOLINE_LEN as u64 + AP_MAILBOX_LEN as u64,
            counters_len: AP_COUNTERS_LEN,
            job_ring_base: trampoline_base
                + AP_TRAMPOLINE_LEN as u64
                + AP_MAILBOX_LEN as u64
                + AP_COUNTERS_LEN as u64,
            job_ring_len: AP_JOB_RING_LEN,
        }
    }

    #[cfg(test)]
    pub const fn end(self) -> u64 {
        self.job_ring_base + self.job_ring_len as u64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmpProcessorTarget {
    pub apic_id: u32,
    pub processor_uid: u32,
    pub stack_run_start: u64,
    pub stack_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedSmpBootstrap {
    pub bootstrap_apic_id: u32,
    pub local_apic_address: u64,
    pub layout: SmpBootstrapLayout,
    pub targets: Vec<SmpProcessorTarget>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApJobConsumerState {
    pub apic_id: u32,
    pub command: u32,
    pub target_cr3: u64,
    pub target_rip: u64,
    pub target_rsp: u64,
    pub argument0: u64,
    pub argument1: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApEntryResult {
    pub apic_id: u32,
    pub command: u32,
    pub target_cr3: u64,
    pub target_rip: u64,
    pub target_rsp: u64,
    pub argument0: u64,
    pub argument1: u64,
    pub online_count: usize,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmpLaunchReport {
    pub init_ipis_sent: usize,
    pub startup_ipis_sent: usize,
    pub job_packets_enqueued: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmpOnlineReport {
    pub ap_targets: usize,
    pub ap_online: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmpLaunchError {
    MissingLocalApic,
    InvalidTrampolineBase,
    JobRingFull,
}

pub fn prepare_bootstrap<const N: usize>(
    boot_info: &BootInfo<'_>,
    frame_allocator: &mut EarlyFrameAllocator<N>,
    bootstrap_page_table_root: u64,
    ap_entry_point: u64,
) -> Result<Option<PreparedSmpBootstrap>, platform_x86_64::FrameAllocatorError> {
    let bootstrap_apic_id = bootstrap_apic_id();
    boot_locator::event(
        BootLocatorStage::Smp,
        BootLocatorKind::Transition,
        BootLocatorSeverity::Info,
        0x700,
        BootPayloadLabel::Status,
        bootstrap_apic_id as u64,
        BootPayloadLabel::None,
        0,
    );
    let topology = match apic_topology(boot_info, bootstrap_apic_id) {
        Some(topology) => topology,
        None => return Ok(None),
    };
    prepare_bootstrap_with_topology(
        boot_info,
        frame_allocator,
        bootstrap_page_table_root,
        ap_entry_point,
        bootstrap_apic_id,
        Some(topology),
    )
}

fn prepare_bootstrap_with_topology<const N: usize>(
    boot_info: &BootInfo<'_>,
    frame_allocator: &mut EarlyFrameAllocator<N>,
    bootstrap_page_table_root: u64,
    ap_entry_point: u64,
    bootstrap_apic_id: u32,
    topology: Option<ApicTopologyInfo>,
) -> Result<Option<PreparedSmpBootstrap>, platform_x86_64::FrameAllocatorError> {
    let topology = match topology {
        Some(topology) => topology,
        None => return Ok(None),
    };
    boot_locator::event(
        BootLocatorStage::Smp,
        BootLocatorKind::Contract,
        BootLocatorSeverity::Info,
        0x710,
        BootPayloadLabel::Count,
        topology.processors.len() as u64,
        BootPayloadLabel::Address,
        topology.local_apic_address,
    );
    let low_memory_run = match frame_allocator
        .allocate_frames_under(AP_BOOTSTRAP_WINDOW_FRAMES, AP_LOW_MEMORY_LIMIT)
    {
        Ok(run) => run,
        Err(platform_x86_64::FrameAllocatorError::OutOfMemory { .. }) => {
            boot_locator::event(
                BootLocatorStage::Smp,
                BootLocatorKind::Fault,
                BootLocatorSeverity::Warn,
                0x711,
                BootPayloadLabel::Address,
                AP_LOW_MEMORY_LIMIT,
                BootPayloadLabel::Length,
                AP_BOOTSTRAP_WINDOW_BYTES,
            );
            return Ok(None);
        }
        Err(error) => return Err(error),
    };
    let layout = SmpBootstrapLayout::from_trampoline_base(low_memory_run.start);
    let mut targets = Vec::new();
    for processor in topology
        .processors
        .iter()
        .filter(|processor| processor.enabled && !processor.is_bootstrap)
    {
        let stack_frames = usize::try_from(AP_STACK_BYTES / PAGE_SIZE_4K).unwrap_or(4);
        let stack_run = frame_allocator.allocate_frames(stack_frames)?;
        targets.push(SmpProcessorTarget {
            apic_id: processor.apic_id,
            processor_uid: processor.processor_uid,
            stack_run_start: stack_run.start,
            stack_bytes: stack_run.len_bytes(),
        });
    }

    if targets.is_empty() {
        return Ok(None);
    }
    boot_locator::event(
        BootLocatorStage::Smp,
        BootLocatorKind::Contract,
        BootLocatorSeverity::Info,
        0x720,
        BootPayloadLabel::Count,
        targets.len() as u64,
        BootPayloadLabel::Length,
        AP_STACK_BYTES,
    );
    boot_locator::event(
        BootLocatorStage::Smp,
        BootLocatorKind::Contract,
        BootLocatorSeverity::Info,
        0x721,
        BootPayloadLabel::Address,
        layout.trampoline_base,
        BootPayloadLabel::Length,
        low_memory_run.len_bytes(),
    );

    initialize_low_memory_structures(
        boot_info,
        &layout,
        &targets,
        bootstrap_page_table_root,
        ap_entry_point,
        topology.local_apic_address,
    );
    #[cfg(target_os = "none")]
    let uptime = crate::timer::boot_uptime_micros();
    #[cfg(not(target_os = "none"))]
    let uptime = None;
    crate::diagnostics::record_boot_stage(
        crate::diagnostics::BootTraceStage::SmpTopologyReady,
        uptime,
        targets.len() as u64,
    );
    crate::diagnostics::record_memory_window(
        0x53544d50,
        layout.trampoline_base,
        layout.trampoline_len as u64,
        targets.len() as u64,
    );
    crate::diagnostics::record_memory_window(
        0x534d424d,
        layout.mailbox_base,
        layout.mailbox_len as u64,
        AP_MAILBOX_CAPACITY as u64,
    );
    crate::diagnostics::record_memory_window(
        0x534d4a52,
        layout.job_ring_base,
        layout.job_ring_len as u64,
        JOB_RING_CAPACITY as u64,
    );

    Ok(Some(PreparedSmpBootstrap {
        bootstrap_apic_id,
        local_apic_address: topology.local_apic_address,
        layout,
        targets,
    }))
}

#[allow(dead_code)]
pub fn dispatch_startup_ipis(
    boot_info: &BootInfo<'_>,
    prepared: &PreparedSmpBootstrap,
) -> Result<SmpLaunchReport, SmpLaunchError> {
    if prepared.local_apic_address == 0 {
        return Err(SmpLaunchError::MissingLocalApic);
    }
    if prepared.layout.trampoline_base & 0xfff != 0 {
        return Err(SmpLaunchError::InvalidTrampolineBase);
    }
    let startup_vector = ((prepared.layout.trampoline_base >> 12) & 0xff) as u8;
    let mut local_apic = boot_local_apic(boot_info, prepared.local_apic_address);
    dispatch_startup_ipis_with_access(&mut local_apic, boot_info, prepared, startup_vector)
}

pub fn bootstrap_apic_id() -> u32 {
    #[cfg(target_arch = "x86_64")]
    {
        let signature = core::arch::x86_64::__cpuid(1);
        (signature.ebx >> 24) & 0xff
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        0
    }
}

fn initialize_low_memory_structures(
    boot_info: &BootInfo<'_>,
    layout: &SmpBootstrapLayout,
    targets: &[SmpProcessorTarget],
    bootstrap_page_table_root: u64,
    ap_entry_point: u64,
    local_apic_address: u64,
) {
    let hhdm = boot_info.physical_memory_offset;
    let mailbox_ptr = layout.mailbox_base.saturating_add(hhdm) as *mut ApMailboxEntry;
    let counters_ptr = layout.counters_base.saturating_add(hhdm) as *mut ApCounterEntry;
    let ring_ptr = layout.job_ring_base.saturating_add(hhdm) as *mut ApJobRing;
    let trampoline_ptr = layout.trampoline_base.saturating_add(hhdm) as *mut u8;

    unsafe {
        ptr::write_bytes(trampoline_ptr, 0, layout.trampoline_len as usize);
        ptr::write_bytes(
            mailbox_ptr.cast::<u8>(),
            0,
            AP_MAILBOX_CAPACITY * size_of::<ApMailboxEntry>(),
        );
        ptr::write_bytes(counters_ptr.cast::<u8>(), 0, layout.counters_len as usize);
        ptr::write(ring_ptr, ApJobRing::empty());
        ptr::write(
            trampoline_ptr.add(AP_RENDEZVOUS_DESCRIPTOR_OFFSET) as *mut ApRendezvousDescriptor,
            ApRendezvousDescriptor {
                signature: ApRendezvousDescriptor::SIGNATURE,
                version: ApRendezvousDescriptor::VERSION,
                trampoline_base: layout.trampoline_base as u32,
                bootstrap_cr3: bootstrap_page_table_root,
                physical_memory_offset: hhdm,
                local_apic_address,
                mailbox_base: layout.mailbox_base,
                counters_base: layout.counters_base,
                job_ring_base: layout.job_ring_base,
                ap_entry_point,
            },
        );
        install_ap_trampoline_code(
            trampoline_ptr,
            layout.trampoline_base,
            layout.trampoline_len as usize,
        );
    }

    for (index, target) in targets.iter().enumerate().take(AP_MAILBOX_CAPACITY) {
        let stack_top = target.stack_run_start.saturating_add(target.stack_bytes);
        unsafe {
            ptr::write(
                mailbox_ptr.add(index),
                ApMailboxEntry {
                    apic_id: target.apic_id,
                    processor_uid: target.processor_uid,
                    stack_top,
                    entry_point: ap_entry_point,
                    page_table_root: bootstrap_page_table_root,
                    flags: 1,
                    reserved: 0,
                },
            );
            ptr::write(
                counters_ptr.add(index),
                ApCounterEntry {
                    apic_id: target.apic_id,
                    state: AP_STATE_OFFLINE,
                    generation: 0,
                    jobs_completed: 0,
                },
            );
        }
    }
}

#[cfg(target_os = "none")]
unsafe fn install_ap_trampoline_code(
    destination: *mut u8,
    trampoline_base: u64,
    trampoline_len: usize,
) {
    let start = ptr::addr_of!(__ngos_x86_64_ap_trampoline_start);
    let end = ptr::addr_of!(__ngos_x86_64_ap_trampoline_end);
    let pm_entry_ptr = ptr::addr_of!(__ngos_x86_64_ap_pm_entry_ptr);
    let pm_stack_ptr = ptr::addr_of!(__ngos_x86_64_ap_pm_stack_ptr);
    let rendezvous_cr3_ptr = ptr::addr_of!(__ngos_x86_64_ap_rendezvous_cr3_ptr);
    let gdt16_base_ptr = ptr::addr_of!(__ngos_x86_64_ap_gdt16_base_ptr);
    let gdt64_linear_ptr = ptr::addr_of!(__ngos_x86_64_ap_gdt64_linear_ptr);
    let gdt64_base_ptr = ptr::addr_of!(__ngos_x86_64_ap_gdt64_base_ptr);
    let lm_entry_ptr = ptr::addr_of!(__ngos_x86_64_ap_lm_entry_ptr);
    let lm_stack_ptr = ptr::addr_of!(__ngos_x86_64_ap_lm_stack_ptr);
    let rendezvous_ptr = ptr::addr_of!(__ngos_x86_64_ap_rendezvous_ptr);
    let rust_entry_ptr = ptr::addr_of!(__ngos_x86_64_ap_rust_entry_ptr);
    let len = end as usize - start as usize;
    let copy_len = len.min(AP_TRAMPOLINE_CODE_LIMIT).min(trampoline_len);

    unsafe fn patch_u32(
        destination: *mut u8,
        image_start: *const u8,
        field: *const u8,
        copy_len: usize,
        value: u32,
    ) {
        let patch_offset = field as usize - image_start as usize;
        if patch_offset + size_of::<u32>() <= copy_len {
            unsafe {
                ptr::write_unaligned(destination.add(patch_offset) as *mut u32, value);
            }
        }
    }

    unsafe fn patch_u64(
        destination: *mut u8,
        image_start: *const u8,
        field: *const u8,
        copy_len: usize,
        value: u64,
    ) {
        let patch_offset = field as usize - image_start as usize;
        if patch_offset + size_of::<u64>() <= copy_len {
            unsafe {
                ptr::write_unaligned(destination.add(patch_offset) as *mut u64, value);
            }
        }
    }

    let pm_entry = trampoline_base.saturating_add(0x40);
    let temp_stack = trampoline_base.saturating_add(0xff0);
    let rendezvous = trampoline_base.saturating_add(AP_RENDEZVOUS_DESCRIPTOR_OFFSET as u64);
    let gdt_base = trampoline_base.saturating_add(0x280);
    let long_mode_entry = trampoline_base.saturating_add(0x100);
    unsafe {
        ptr::copy_nonoverlapping(start, destination, copy_len);
        patch_u32(destination, start, pm_entry_ptr, copy_len, pm_entry as u32);
        patch_u32(
            destination,
            start,
            pm_stack_ptr,
            copy_len,
            temp_stack as u32,
        );
        patch_u32(
            destination,
            start,
            rendezvous_cr3_ptr,
            copy_len,
            rendezvous.saturating_add(16) as u32,
        );
        patch_u32(
            destination,
            start,
            gdt16_base_ptr,
            copy_len,
            gdt_base as u32,
        );
        patch_u32(
            destination,
            start,
            gdt64_linear_ptr,
            copy_len,
            trampoline_base.saturating_add(0x2b0) as u32,
        );
        patch_u64(destination, start, gdt64_base_ptr, copy_len, gdt_base);
        patch_u32(
            destination,
            start,
            lm_entry_ptr,
            copy_len,
            long_mode_entry as u32,
        );
        patch_u64(destination, start, lm_stack_ptr, copy_len, temp_stack);
        patch_u64(destination, start, rendezvous_ptr, copy_len, rendezvous);
        patch_u64(
            destination,
            start,
            rust_entry_ptr,
            copy_len,
            x86_64_ap_long_mode_entry as *const () as u64,
        );
    }
}

#[cfg(not(target_os = "none"))]
unsafe fn install_ap_trampoline_code(
    destination: *mut u8,
    _trampoline_base: u64,
    trampoline_len: usize,
) {
    const HOST_TEST_TRAMPOLINE_IMAGE: &[u8] = &[
        0xfa, 0xfc, 0x0f, 0x01, 0x16, 0x00, 0x84, 0xea, 0x00, 0x80, 0x08, 0x00,
    ];
    let copy_len = HOST_TEST_TRAMPOLINE_IMAGE
        .len()
        .min(AP_TRAMPOLINE_CODE_LIMIT)
        .min(trampoline_len);
    unsafe {
        ptr::copy_nonoverlapping(HOST_TEST_TRAMPOLINE_IMAGE.as_ptr(), destination, copy_len);
    }
}

fn dispatch_startup_ipis_with_access<A: LocalApicAccess>(
    local_apic: &mut A,
    boot_info: &BootInfo<'_>,
    prepared: &PreparedSmpBootstrap,
    startup_vector: u8,
) -> Result<SmpLaunchReport, SmpLaunchError> {
    local_apic.enable_spurious_vector(0xff);
    let mut report = SmpLaunchReport {
        init_ipis_sent: 0,
        startup_ipis_sent: 0,
        job_packets_enqueued: 0,
    };

    let hhdm = boot_info.physical_memory_offset;
    let mailbox_ptr = prepared.layout.mailbox_base.saturating_add(hhdm) as *const ApMailboxEntry;
    let ring_ptr = prepared.layout.job_ring_base.saturating_add(hhdm) as *mut ApJobRing;
    let counters_ptr = prepared.layout.counters_base.saturating_add(hhdm) as *mut ApCounterEntry;

    for (index, target) in prepared.targets.iter().enumerate() {
        let mailbox = unsafe { &*mailbox_ptr.add(index) };
        enqueue_startup_job(unsafe { &mut *ring_ptr }, mailbox)?;
        unsafe {
            let counter = &mut *counters_ptr.add(index);
            counter.state = AP_STATE_DISPATCHED;
            counter.generation = counter.generation.saturating_add(1);
        }
        report.job_packets_enqueued += 1;

        local_apic.send_init_ipi(target.apic_id);
        report.init_ipis_sent += 1;

        local_apic.send_startup_ipi(target.apic_id, startup_vector);
        local_apic.send_startup_ipi(target.apic_id, startup_vector);
        report.startup_ipis_sent += 2;
    }

    Ok(report)
}

fn enqueue_startup_job(
    ring: &mut ApJobRing,
    mailbox: &ApMailboxEntry,
) -> Result<(), SmpLaunchError> {
    let head = ring.head as usize;
    let tail = ring.tail as usize;
    let next_tail = (tail + 1) % ring.capacity as usize;
    if next_tail == head {
        return Err(SmpLaunchError::JobRingFull);
    }
    ring.entries[tail] = ApJobPacket {
        apic_id: mailbox.apic_id,
        command: AP_JOB_COMMAND_STARTUP,
        target_cr3: mailbox.page_table_root,
        target_rip: mailbox.entry_point,
        target_rsp: mailbox.stack_top,
        argument0: mailbox.processor_uid as u64,
        argument1: 0,
    };
    ring.tail = next_tail as u32;
    Ok(())
}

pub fn consume_next_job(
    boot_info: &BootInfo<'_>,
    layout: &SmpBootstrapLayout,
    apic_id: u32,
) -> Option<ApJobConsumerState> {
    let hhdm = boot_info.physical_memory_offset;
    let ring = unsafe { &mut *((layout.job_ring_base + hhdm) as *mut ApJobRing) };
    let head = ring.head as usize;
    let tail = ring.tail as usize;
    if head == tail {
        return None;
    }
    let packet = ring.entries[head];
    if packet.apic_id != apic_id {
        return None;
    }
    ring.head = ((head + 1) % ring.capacity as usize) as u32;
    Some(ApJobConsumerState {
        apic_id: packet.apic_id,
        command: packet.command,
        target_cr3: packet.target_cr3,
        target_rip: packet.target_rip,
        target_rsp: packet.target_rsp,
        argument0: packet.argument0,
        argument1: packet.argument1,
    })
}

pub fn mark_ap_online(boot_info: &BootInfo<'_>, layout: &SmpBootstrapLayout, apic_id: u32) -> bool {
    let hhdm = boot_info.physical_memory_offset;
    let counters = (layout.counters_base + hhdm) as *mut ApCounterEntry;
    for index in 0..AP_MAILBOX_CAPACITY {
        let counter = unsafe { &mut *counters.add(index) };
        if counter.apic_id == apic_id {
            counter.state = AP_STATE_ONLINE;
            counter.jobs_completed = counter.jobs_completed.saturating_add(1);
            return true;
        }
    }
    false
}

pub fn online_ap_count(boot_info: &BootInfo<'_>, layout: &SmpBootstrapLayout) -> usize {
    let hhdm = boot_info.physical_memory_offset;
    let counters = (layout.counters_base + hhdm) as *const ApCounterEntry;
    let mut online = 0usize;
    for index in 0..AP_MAILBOX_CAPACITY {
        let counter = unsafe { &*counters.add(index) };
        if counter.apic_id != 0 && counter.state == AP_STATE_ONLINE {
            online += 1;
        }
    }
    online
}

pub fn ap_entry_from_rendezvous(
    descriptor: &ApRendezvousDescriptor,
    apic_id: u32,
) -> Option<ApEntryResult> {
    if descriptor.signature != ApRendezvousDescriptor::SIGNATURE
        || descriptor.version != ApRendezvousDescriptor::VERSION
    {
        return None;
    }
    let boot_info = BootInfo {
        protocol: platform_x86_64::BootProtocol::LoaderDefined,
        command_line: None,
        rsdp: None,
        memory_regions: &[],
        modules: &[],
        framebuffer: None,
        physical_memory_offset: descriptor.physical_memory_offset,
        kernel_phys_range: platform_x86_64::BootMemoryRegion {
            start: 0,
            len: 0,
            kind: platform_x86_64::BootMemoryRegionKind::KernelImage,
        },
    };
    let layout = SmpBootstrapLayout::from_trampoline_base(u64::from(descriptor.trampoline_base));
    let job = consume_next_job(&boot_info, &layout, apic_id)?;
    if !mark_ap_online(&boot_info, &layout, apic_id) {
        return None;
    }
    Some(ApEntryResult {
        apic_id,
        command: job.command,
        target_cr3: job.target_cr3,
        target_rip: job.target_rip,
        target_rsp: job.target_rsp,
        argument0: job.argument0,
        argument1: job.argument1,
        online_count: online_ap_count(&boot_info, &layout),
    })
}

pub fn bring_up_secondary_processors(
    boot_info: &BootInfo<'_>,
    prepared: &PreparedSmpBootstrap,
    wait_spins_per_ap: usize,
) -> Result<SmpOnlineReport, SmpLaunchError> {
    boot_locator::event(
        BootLocatorStage::Smp,
        BootLocatorKind::Transition,
        BootLocatorSeverity::Info,
        0x730,
        BootPayloadLabel::Count,
        prepared.targets.len() as u64,
        BootPayloadLabel::Value,
        wait_spins_per_ap as u64,
    );
    if prepared.local_apic_address == 0 {
        return Err(SmpLaunchError::MissingLocalApic);
    }
    if prepared.layout.trampoline_base & 0xfff != 0 {
        return Err(SmpLaunchError::InvalidTrampolineBase);
    }

    let startup_vector = ((prepared.layout.trampoline_base >> 12) & 0xff) as u8;
    let mut local_apic = boot_local_apic(boot_info, prepared.local_apic_address);
    local_apic.enable_spurious_vector(0xff);

    let hhdm = boot_info.physical_memory_offset;
    let mailbox_ptr = prepared.layout.mailbox_base.saturating_add(hhdm) as *const ApMailboxEntry;
    let ring_ptr = prepared.layout.job_ring_base.saturating_add(hhdm) as *mut ApJobRing;
    let counters_ptr = prepared.layout.counters_base.saturating_add(hhdm) as *mut ApCounterEntry;

    let mut online = 0usize;
    for (index, target) in prepared.targets.iter().enumerate() {
        let mailbox = unsafe { &*mailbox_ptr.add(index) };
        enqueue_startup_job(unsafe { &mut *ring_ptr }, mailbox)?;
        unsafe {
            let counter = &mut *counters_ptr.add(index);
            counter.state = AP_STATE_DISPATCHED;
            counter.generation = counter.generation.saturating_add(1);
        }

        local_apic.send_init_ipi(target.apic_id);
        local_apic.send_startup_ipi(target.apic_id, startup_vector);
        local_apic.send_startup_ipi(target.apic_id, startup_vector);
        boot_locator::event(
            BootLocatorStage::Smp,
            BootLocatorKind::Transition,
            BootLocatorSeverity::Info,
            0x740,
            BootPayloadLabel::Status,
            target.apic_id as u64,
            BootPayloadLabel::Value,
            startup_vector as u64,
        );

        if wait_for_ap_online(counters_ptr, index, target.apic_id, wait_spins_per_ap) {
            online += 1;
        }
    }

    Ok(SmpOnlineReport {
        ap_targets: prepared.targets.len(),
        ap_online: online,
    })
}

fn wait_for_ap_online(
    counters_ptr: *mut ApCounterEntry,
    index: usize,
    apic_id: u32,
    wait_spins: usize,
) -> bool {
    boot_locator::event(
        BootLocatorStage::Smp,
        BootLocatorKind::Transition,
        BootLocatorSeverity::Info,
        0x750,
        BootPayloadLabel::Status,
        apic_id as u64,
        BootPayloadLabel::Value,
        wait_spins as u64,
    );
    for _ in 0..wait_spins {
        let counter = unsafe { &*counters_ptr.add(index) };
        if counter.apic_id == apic_id && counter.state == AP_STATE_ONLINE {
            return true;
        }
        core::hint::spin_loop();
    }
    false
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `descriptor` must point to a valid rendezvous descriptor prepared by the
/// bootstrap CPU and remain valid for the full AP bring-up sequence.
pub unsafe extern "C" fn x86_64_ap_long_mode_entry(descriptor: *const ApRendezvousDescriptor) -> ! {
    let descriptor = unsafe { &*descriptor };
    let apic_id = bootstrap_apic_id();
    let result = ap_entry_from_rendezvous(descriptor, apic_id).unwrap_or_else(|| {
        loop {
            unsafe {
                core::arch::asm!("cli", "hlt", options(nomem, nostack, preserves_flags));
            }
        }
    });

    #[cfg(target_os = "none")]
    let uptime = crate::timer::boot_uptime_micros();
    #[cfg(not(target_os = "none"))]
    let uptime = None;
    crate::diagnostics::record_boot_stage(
        crate::diagnostics::BootTraceStage::SecondaryCpuOnline,
        uptime,
        u64::from(result.apic_id),
    );
    crate::diagnostics::record_transition(
        0x41504f4e,
        u64::from(result.apic_id),
        result.target_rip,
        result.target_rsp,
    );

    unsafe {
        core::arch::asm!(
            "mov rsp, {stack_top}",
            "mov rdi, {apic_id}",
            "mov rsi, {arg0}",
            "mov rdx, {descriptor}",
            "jmp {entry}",
            stack_top = in(reg) result.target_rsp,
            apic_id = in(reg) u64::from(result.apic_id),
            arg0 = in(reg) result.argument0,
            descriptor = in(reg) descriptor as *const _ as u64,
            entry = in(reg) result.target_rip,
            options(noreturn)
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn x86_64_secondary_cpu_main(
    apic_id: u64,
    processor_uid: u64,
    descriptor: *const ApRendezvousDescriptor,
) -> ! {
    crate::diagnostics::record_transition(0x41504d41, apic_id, processor_uid, descriptor as u64);
    crate::serial::print(format_args!(
        "ngos/x86_64: ap online apic_id={} processor_uid={}\n",
        apic_id, processor_uid
    ));
    loop {
        unsafe {
            core::arch::asm!("cli", "hlt", options(nomem, nostack, preserves_flags));
        }
    }
}

trait LocalApicAccess {
    fn read(&self, offset: u32) -> u32;
    fn write(&mut self, offset: u32, value: u32);

    #[allow(dead_code)]
    fn local_apic_id(&self) -> u32 {
        self.read(APIC_REG_ID) >> 24
    }

    fn enable_spurious_vector(&mut self, vector: u8) {
        self.write(APIC_REG_SPURIOUS, APIC_SPURIOUS_ENABLE | u32::from(vector));
    }

    fn wait_for_idle(&self) {
        let mut spins = 0usize;
        while (self.read(APIC_REG_ICR_LOW) & APIC_DELIVERY_STATUS_PENDING) != 0 && spins < 1_000_000
        {
            core::hint::spin_loop();
            spins += 1;
        }
    }

    fn send_init_ipi(&mut self, apic_id: u32) {
        self.wait_for_idle();
        self.write(APIC_REG_ICR_HIGH, apic_id << 24);
        self.write(APIC_REG_ICR_LOW, APIC_INIT_COMMAND);
        self.wait_for_idle();
    }

    fn send_startup_ipi(&mut self, apic_id: u32, startup_vector: u8) {
        self.wait_for_idle();
        self.write(APIC_REG_ICR_HIGH, apic_id << 24);
        self.write(
            APIC_REG_ICR_LOW,
            APIC_DELIVERY_MODE_STARTUP | APIC_DESTINATION_PHYSICAL | u32::from(startup_vector),
        );
        self.wait_for_idle();
    }
}

struct MmioLocalApic {
    base: *mut u8,
}

impl MmioLocalApic {
    fn new(base: *mut u8) -> Self {
        Self { base }
    }
}

impl LocalApicAccess for MmioLocalApic {
    fn read(&self, offset: u32) -> u32 {
        unsafe { core::ptr::read_volatile(self.base.add(offset as usize) as *const u32) }
    }

    fn write(&mut self, offset: u32, value: u32) {
        unsafe {
            core::ptr::write_volatile(self.base.add(offset as usize) as *mut u32, value);
        }
    }
}

struct X2ApicLocalApic;

impl X2ApicLocalApic {
    const MSR_BASE: u32 = 0x800;
    const ICR_MSR: u32 = 0x830;

    fn msr_for_offset(offset: u32) -> u32 {
        Self::MSR_BASE + (offset >> 4)
    }

    fn read_msr(msr: u32) -> u64 {
        let low: u32;
        let high: u32;
        unsafe {
            asm!(
                "rdmsr",
                in("ecx") msr,
                out("eax") low,
                out("edx") high,
                options(nomem, nostack, preserves_flags)
            );
        }
        ((high as u64) << 32) | (low as u64)
    }

    fn write_msr(msr: u32, value: u64) {
        unsafe {
            asm!(
                "wrmsr",
                in("ecx") msr,
                in("eax") value as u32,
                in("edx") (value >> 32) as u32,
                options(nomem, nostack, preserves_flags)
            );
        }
    }

    fn write_icr(&mut self, apic_id: u32, value: u32) {
        let icr = ((apic_id as u64) << 32) | u64::from(value);
        Self::write_msr(Self::ICR_MSR, icr);
    }
}

impl LocalApicAccess for X2ApicLocalApic {
    fn read(&self, offset: u32) -> u32 {
        Self::read_msr(Self::msr_for_offset(offset)) as u32
    }

    fn write(&mut self, offset: u32, value: u32) {
        Self::write_msr(Self::msr_for_offset(offset), u64::from(value));
    }

    fn send_init_ipi(&mut self, apic_id: u32) {
        self.wait_for_idle();
        self.write_icr(apic_id, APIC_INIT_COMMAND);
        self.wait_for_idle();
    }

    fn send_startup_ipi(&mut self, apic_id: u32, startup_vector: u8) {
        self.wait_for_idle();
        self.write_icr(
            apic_id,
            APIC_DELIVERY_MODE_STARTUP | APIC_DESTINATION_PHYSICAL | u32::from(startup_vector),
        );
        self.wait_for_idle();
    }
}

enum BootLocalApic {
    Mmio(MmioLocalApic),
    X2(X2ApicLocalApic),
}

impl LocalApicAccess for BootLocalApic {
    fn read(&self, offset: u32) -> u32 {
        match self {
            Self::Mmio(local) => local.read(offset),
            Self::X2(local) => local.read(offset),
        }
    }

    fn write(&mut self, offset: u32, value: u32) {
        match self {
            Self::Mmio(local) => local.write(offset, value),
            Self::X2(local) => local.write(offset, value),
        }
    }

    fn send_init_ipi(&mut self, apic_id: u32) {
        match self {
            Self::Mmio(local) => local.send_init_ipi(apic_id),
            Self::X2(local) => local.send_init_ipi(apic_id),
        }
    }

    fn send_startup_ipi(&mut self, apic_id: u32, startup_vector: u8) {
        match self {
            Self::Mmio(local) => local.send_startup_ipi(apic_id, startup_vector),
            Self::X2(local) => local.send_startup_ipi(apic_id, startup_vector),
        }
    }
}

fn boot_local_apic(boot_info: &BootInfo<'_>, local_apic_address: u64) -> BootLocalApic {
    let mode = crate::cpu_apic::enable_preferred_local_apic_mode();
    crate::cpu_runtime_status::record_local_apic_mode(mode as u32);
    match mode {
        crate::cpu_apic::LocalApicMode::X2Apic => BootLocalApic::X2(X2ApicLocalApic),
        crate::cpu_apic::LocalApicMode::XApic => BootLocalApic::Mmio(MmioLocalApic::new(
            local_apic_address.saturating_add(boot_info.physical_memory_offset) as *mut u8,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Mutex, OnceLock};

    use platform_x86_64::{BootMemoryRegion, BootMemoryRegionKind, BootProtocol};

    fn smp_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[derive(Default)]
    struct LoggingLocalApic {
        icr_low: u32,
        writes: VecDeque<(u32, u32)>,
    }

    impl LocalApicAccess for LoggingLocalApic {
        fn read(&self, offset: u32) -> u32 {
            match offset {
                APIC_REG_ICR_LOW => self.icr_low,
                APIC_REG_ID => 0,
                _ => 0,
            }
        }

        fn write(&mut self, offset: u32, value: u32) {
            if offset == APIC_REG_ICR_LOW {
                self.icr_low = value & !APIC_DELIVERY_STATUS_PENDING;
            }
            self.writes.push_back((offset, value));
        }
    }

    #[test]
    fn bootstrap_layout_is_derived_from_trampoline_base() {
        let _guard = smp_test_lock().lock().unwrap();
        let layout = SmpBootstrapLayout::from_trampoline_base(0x80_000);
        assert_eq!(layout.trampoline_base, 0x80_000);
        assert_eq!(layout.mailbox_base, 0x81_000);
        assert_eq!(layout.job_ring_len, AP_JOB_RING_LEN);
        assert_eq!(layout.end(), 0x84_000);
    }

    #[test]
    fn prepare_bootstrap_requires_allocatable_low_memory_window() {
        let _guard = smp_test_lock().lock().unwrap();
        let regions = [BootMemoryRegion {
            start: 0x100000,
            len: 0x200000,
            kind: BootMemoryRegionKind::Usable,
        }];
        let boot_info = BootInfo {
            protocol: BootProtocol::Limine,
            command_line: None,
            rsdp: None,
            memory_regions: &regions,
            modules: &[],
            framebuffer: None,
            physical_memory_offset: 0,
            kernel_phys_range: BootMemoryRegion {
                start: 0,
                len: 0x1000,
                kind: BootMemoryRegionKind::KernelImage,
            },
        };
        let mut allocator = EarlyFrameAllocator::<8>::new();
        allocator.add_usable_region(0x100000, 0x200000).unwrap();

        let prepared = prepare_bootstrap(&boot_info, &mut allocator, 0x2000, 0x3000)
            .expect("bootstrap preparation should not fail");

        assert!(prepared.is_none());
    }

    #[test]
    fn prepare_bootstrap_allocates_dynamic_low_memory_layout() {
        let _guard = smp_test_lock().lock().unwrap();
        let mut backing = [0u8; 0x40_000];
        let backing_base = backing.as_mut_ptr() as usize as u64;
        let physical_memory_offset = backing_base.saturating_sub(0x9_000);
        let regions = [
            BootMemoryRegion {
                start: 0x9000,
                len: AP_BOOTSTRAP_WINDOW_BYTES,
                kind: platform_x86_64::BootMemoryRegionKind::Usable,
            },
            BootMemoryRegion {
                start: 0x100000,
                len: 0x200000,
                kind: platform_x86_64::BootMemoryRegionKind::Usable,
            },
        ];
        let processors = ApicTopologyInfo {
            local_apic_address: 0xfee0_0000,
            processors: alloc::vec![
                platform_x86_64::ProcessorTopologyEntry {
                    processor_uid: 1,
                    apic_id: bootstrap_apic_id(),
                    enabled: true,
                    online_capable: true,
                    is_bootstrap: true,
                },
                platform_x86_64::ProcessorTopologyEntry {
                    processor_uid: 2,
                    apic_id: bootstrap_apic_id().wrapping_add(1),
                    enabled: true,
                    online_capable: true,
                    is_bootstrap: false,
                },
            ],
            io_apics: vec![],
            interrupt_overrides: vec![],
        };
        let boot_info = BootInfo {
            protocol: BootProtocol::LoaderDefined,
            command_line: None,
            rsdp: None,
            memory_regions: &regions,
            modules: &[],
            framebuffer: None,
            physical_memory_offset,
            kernel_phys_range: BootMemoryRegion {
                start: 0,
                len: 0x1000,
                kind: platform_x86_64::BootMemoryRegionKind::KernelImage,
            },
        };
        let mut allocator = EarlyFrameAllocator::<8>::new();
        allocator
            .add_usable_region(0x9000, AP_BOOTSTRAP_WINDOW_BYTES)
            .unwrap();
        allocator.add_usable_region(0x100000, 0x200000).unwrap();

        let prepared = prepare_bootstrap_with_topology(
            &boot_info,
            &mut allocator,
            0x2000,
            0x3000,
            bootstrap_apic_id(),
            Some(processors),
        )
        .expect("bootstrap preparation should not fail")
        .expect("bootstrap should prepare one AP");

        assert_eq!(
            prepared.layout,
            SmpBootstrapLayout::from_trampoline_base(0x9000)
        );
        assert_eq!(
            allocator.stats().allocated_frames,
            AP_BOOTSTRAP_WINDOW_FRAMES as u64 + 4
        );
    }

    #[test]
    fn dispatch_startup_ipis_enqueues_jobs_and_sends_init_sipi_sequence() {
        let _guard = smp_test_lock().lock().unwrap();
        let mut backing = [0u8; 0x40_000];
        let backing_base = backing.as_mut_ptr() as usize as u64;
        let layout = SmpBootstrapLayout::from_trampoline_base(0x8_000);
        let physical_memory_offset = backing_base.saturating_sub(layout.trampoline_base);
        let boot_info = BootInfo {
            protocol: BootProtocol::Limine,
            command_line: None,
            rsdp: None,
            memory_regions: &[],
            modules: &[],
            framebuffer: None,
            physical_memory_offset,
            kernel_phys_range: BootMemoryRegion {
                start: 0,
                len: 0x1000,
                kind: BootMemoryRegionKind::KernelImage,
            },
        };
        let prepared = PreparedSmpBootstrap {
            bootstrap_apic_id: 1,
            local_apic_address: 0xfee0_0000,
            layout,
            targets: vec![SmpProcessorTarget {
                apic_id: 3,
                processor_uid: 7,
                stack_run_start: 0x120000,
                stack_bytes: AP_STACK_BYTES,
            }],
        };
        initialize_low_memory_structures(
            &boot_info,
            &prepared.layout,
            &prepared.targets,
            0x2000,
            0x8000,
            prepared.local_apic_address,
        );
        let mut local_apic = LoggingLocalApic::default();

        let report =
            dispatch_startup_ipis_with_access(&mut local_apic, &boot_info, &prepared, 0x08)
                .expect("startup ipis should dispatch");

        assert_eq!(report.init_ipis_sent, 1);
        assert_eq!(report.startup_ipis_sent, 2);
        assert_eq!(report.job_packets_enqueued, 1);

        let ring = unsafe {
            &*((prepared.layout.job_ring_base + boot_info.physical_memory_offset)
                as *const ApJobRing)
        };
        assert_eq!(ring.tail, 1);
        assert_eq!(ring.entries[0].command, AP_JOB_COMMAND_STARTUP);
        assert_eq!(ring.entries[0].apic_id, 3);
        assert_eq!(ring.entries[0].target_cr3, 0x2000);
        assert_eq!(ring.entries[0].target_rip, 0x8000);

        let writes: Vec<(u32, u32)> = local_apic.writes.into_iter().collect();
        assert_eq!(writes[0], (APIC_REG_SPURIOUS, APIC_SPURIOUS_ENABLE | 0xff));
        assert_eq!(writes[1], (APIC_REG_ICR_HIGH, 3 << 24));
        assert_eq!(writes[2], (APIC_REG_ICR_LOW, APIC_INIT_COMMAND));
        assert_eq!(writes[3], (APIC_REG_ICR_HIGH, 3 << 24));
        assert_eq!(
            writes[4],
            (
                APIC_REG_ICR_LOW,
                APIC_DELIVERY_MODE_STARTUP | APIC_DESTINATION_PHYSICAL | 0x08
            )
        );
        assert_eq!(writes[5], (APIC_REG_ICR_HIGH, 3 << 24));
        assert_eq!(
            writes[6],
            (
                APIC_REG_ICR_LOW,
                APIC_DELIVERY_MODE_STARTUP | APIC_DESTINATION_PHYSICAL | 0x08
            )
        );
    }

    #[test]
    fn ap_handshake_consumes_job_and_marks_counter_online() {
        let _guard = smp_test_lock().lock().unwrap();
        let mut backing = [0u8; 0x40_000];
        let backing_base = backing.as_mut_ptr() as usize as u64;
        let layout = SmpBootstrapLayout::from_trampoline_base(0x8_000);
        let physical_memory_offset = backing_base.saturating_sub(layout.trampoline_base);
        let boot_info = BootInfo {
            protocol: BootProtocol::Limine,
            command_line: None,
            rsdp: None,
            memory_regions: &[],
            modules: &[],
            framebuffer: None,
            physical_memory_offset,
            kernel_phys_range: BootMemoryRegion {
                start: 0,
                len: 0x1000,
                kind: BootMemoryRegionKind::KernelImage,
            },
        };
        let prepared = PreparedSmpBootstrap {
            bootstrap_apic_id: 1,
            local_apic_address: 0xfee0_0000,
            layout,
            targets: vec![SmpProcessorTarget {
                apic_id: 5,
                processor_uid: 9,
                stack_run_start: 0x180000,
                stack_bytes: AP_STACK_BYTES,
            }],
        };
        initialize_low_memory_structures(
            &boot_info,
            &prepared.layout,
            &prepared.targets,
            0x4000,
            0x9000,
            prepared.local_apic_address,
        );
        let mut local_apic = LoggingLocalApic::default();
        dispatch_startup_ipis_with_access(&mut local_apic, &boot_info, &prepared, 0x08)
            .expect("dispatch should succeed");

        let consumed = consume_next_job(&boot_info, &prepared.layout, 5)
            .expect("ap should observe startup job");
        assert_eq!(consumed.command, AP_JOB_COMMAND_STARTUP);
        assert_eq!(consumed.target_cr3, 0x4000);
        assert_eq!(consumed.target_rip, 0x9000);
        assert_eq!(consumed.target_rsp, 0x180000 + AP_STACK_BYTES);

        assert!(mark_ap_online(&boot_info, &prepared.layout, 5));
        assert_eq!(online_ap_count(&boot_info, &prepared.layout), 1);
    }

    #[test]
    fn rendezvous_descriptor_is_installed_in_trampoline_region() {
        let _guard = smp_test_lock().lock().unwrap();
        let mut backing = [0u8; 0x40_000];
        let backing_base = backing.as_mut_ptr() as usize as u64;
        let layout = SmpBootstrapLayout::from_trampoline_base(0x8_000);
        let physical_memory_offset = backing_base.saturating_sub(layout.trampoline_base);
        let boot_info = BootInfo {
            protocol: BootProtocol::Limine,
            command_line: None,
            rsdp: None,
            memory_regions: &[],
            modules: &[],
            framebuffer: None,
            physical_memory_offset,
            kernel_phys_range: BootMemoryRegion {
                start: 0,
                len: 0x1000,
                kind: BootMemoryRegionKind::KernelImage,
            },
        };
        let targets = [SmpProcessorTarget {
            apic_id: 2,
            processor_uid: 3,
            stack_run_start: 0x200000,
            stack_bytes: AP_STACK_BYTES,
        }];
        initialize_low_memory_structures(
            &boot_info,
            &layout,
            &targets,
            0x1234_5000,
            0x8000,
            0xfee0_0000,
        );

        let descriptor = unsafe {
            &*((layout.trampoline_base
                + physical_memory_offset
                + AP_RENDEZVOUS_DESCRIPTOR_OFFSET as u64)
                as *const ApRendezvousDescriptor)
        };
        assert_eq!(descriptor.signature, ApRendezvousDescriptor::SIGNATURE);
        assert_eq!(descriptor.bootstrap_cr3, 0x1234_5000);
        assert_eq!(descriptor.local_apic_address, 0xfee0_0000);
        assert_eq!(descriptor.mailbox_base, layout.mailbox_base);
        assert_eq!(descriptor.job_ring_base, layout.job_ring_base);
        assert_eq!(descriptor.ap_entry_point, 0x8000);
        let trampoline = unsafe {
            core::slice::from_raw_parts(
                (layout.trampoline_base + physical_memory_offset) as *const u8,
                AP_TRAMPOLINE_CODE_LIMIT,
            )
        };
        assert!(trampoline.iter().any(|byte| *byte != 0));
    }

    #[test]
    fn ap_entry_from_rendezvous_consumes_job_and_reports_online_state() {
        let _guard = smp_test_lock().lock().unwrap();
        let mut backing = [0u8; 0x40_000];
        let backing_base = backing.as_mut_ptr() as usize as u64;
        let layout = SmpBootstrapLayout::from_trampoline_base(0x8_000);
        let physical_memory_offset = backing_base.saturating_sub(layout.trampoline_base);
        let boot_info = BootInfo {
            protocol: BootProtocol::Limine,
            command_line: None,
            rsdp: None,
            memory_regions: &[],
            modules: &[],
            framebuffer: None,
            physical_memory_offset,
            kernel_phys_range: BootMemoryRegion {
                start: 0,
                len: 0x1000,
                kind: BootMemoryRegionKind::KernelImage,
            },
        };
        let prepared = PreparedSmpBootstrap {
            bootstrap_apic_id: 1,
            local_apic_address: 0xfee0_0000,
            layout,
            targets: vec![SmpProcessorTarget {
                apic_id: 6,
                processor_uid: 11,
                stack_run_start: 0x210000,
                stack_bytes: AP_STACK_BYTES,
            }],
        };
        initialize_low_memory_structures(
            &boot_info,
            &prepared.layout,
            &prepared.targets,
            0x6000,
            0xA000,
            prepared.local_apic_address,
        );
        let mut local_apic = LoggingLocalApic::default();
        dispatch_startup_ipis_with_access(&mut local_apic, &boot_info, &prepared, 0x08)
            .expect("dispatch should succeed");

        let descriptor = unsafe {
            &*((prepared.layout.trampoline_base
                + physical_memory_offset
                + AP_RENDEZVOUS_DESCRIPTOR_OFFSET as u64)
                as *const ApRendezvousDescriptor)
        };
        let result = ap_entry_from_rendezvous(descriptor, 6).expect("ap entry should succeed");
        assert_eq!(result.command, AP_JOB_COMMAND_STARTUP);
        assert_eq!(result.target_cr3, 0x6000);
        assert_eq!(result.target_rip, 0xA000);
        assert_eq!(result.target_rsp, 0x210000 + AP_STACK_BYTES);
        assert_eq!(result.online_count, 1);
    }
}
