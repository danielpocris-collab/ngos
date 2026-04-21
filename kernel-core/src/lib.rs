//! Kernel core for `ngos`.
//!
//! This crate is the main development foundation for real kernel subsystems in
//! Rust. The host runtime exists to validate `ngos` semantics and
//! architecture incrementally while preserving a path toward a complete,
//! complex operating system with its own identity.
//!
//! Canonical subsystem role:
//! - subsystem: kernel semantic core
//! - owner layer: Layer 1
//! - semantic owner: `kernel-core`
//! - truth path: `boot-x86_64 -> platform-x86_64 -> kernel-core -> user-runtime -> userland-native -> QEMU`
//!
//! Canonical contract families exposed from this crate:
//! - kernel object contracts
//! - process inspection contracts
//! - procfs / observability contracts
//! - syscall contracts
//! - verified-core contracts
//!
//! This crate is allowed to define system truth.
//! Higher layers may transport, classify, inspect, or operate that truth, but
//! they must not redefine it.

#![cfg_attr(target_os = "none", no_std)]

#[macro_use]
extern crate alloc;

mod bus_model;
mod core_objects;
mod descriptor_io_dispatch;
mod descriptor_io_runtime;
mod descriptor_model;
mod descriptor_runtime;
mod device_model;
mod device_runtime;
mod event_queue_runtime;
mod eventing_model;
mod foundation;
mod memory_wait_runtime;
mod native_model;
mod observability;
mod process_model;
mod process_table;
mod process_vm_dispatch;
mod queue_introspection;
mod runtime_core;
mod scheduler;
mod signal_runtime;
mod sleep_queue_runtime;
mod syscall_eventing;
mod syscall_surface;
mod user_launch;
mod user_memory_runtime;
mod user_syscall_runtime;
mod verified_core;
mod vfs_model;
mod vm_model;

pub use bus_model::{BusEndpointInfo, BusEndpointKind, BusPeerInfo};
pub use core_objects::{
    Capability, CapabilityError, CapabilityId, CapabilityRights, CapabilityTable, Handle,
    HandleError, HandleSpace, KernelObjectTable, ObjectError, ObjectHandle,
};
pub use descriptor_model::{
    CloseRangeMode, Descriptor, DescriptorError, DescriptorFlags, DescriptorNamespace, FcntlCmd,
    FcntlResult, FileStatus, FileSystemStatus, FiledescEntry, FiledescShareGroupInfo,
    IoCapabilities, IoError, IoObject, IoPayloadLayoutInfo, IoPayloadSegmentInfo, IoPollEvents,
    IoRegistry, IoState, KinfoFileEntry, ObjectDescriptor, ObjectKind,
};
pub use eventing_model::{
    EventMultiplexerDescriptor, EventMultiplexerFdOp, EventMultiplexerFdWatch,
    EventMultiplexerFlavor, EventMultiplexerMemoryWatch, EventMultiplexerPollRequest,
    EventMultiplexerProcessWatch, EventMultiplexerSignalWatch, EventMultiplexerTimerWatch,
    EventQueue, EventQueueError, EventQueueId, EventQueueMode, EventQueueWaitResult, EventRecord,
    EventSource, EventTimerId, EventWatch, EventWatchBehavior, MemoryWaitDomain,
    MemoryWaitEventKind, MemoryWaitKey, MemoryWordCmpRequeueResult, MemoryWordRequeueResult,
    MemoryWordWaitAnyResult, MemoryWordWaitDomainEntry, MemoryWordWaitEntry, MemoryWordWaitResult,
    NetworkEventInterest, NetworkEventKind, PendingSignalCode, PendingSignalDelivery,
    PendingSignalSender, PendingSignalSource, PendingSignalWaitResult, PendingSignalWaitResume,
    ProcessLifecycleEventKind, ProcessLifecycleInterest, ReadinessInterest, ReadinessRegistration,
    ResourceEventInterest, ResourceEventKind, SignalDisposition, SleepQueueId,
};
pub use foundation::{
    ActiveCpuExtendedStateSlot, BusEndpointId, BusPeerId, CpuExtendedStateHandoff,
    CpuExtendedStateHardwareTelemetry, RuntimePolicy, SchedulerCpuTopologyEntry,
};
pub use native_model::{
    ContractInfo, ContractKind, ContractState, DomainInfo, ResourceClaimResult, ResourceInfo,
    ResourceKind, ResourceReleaseResult,
};
pub use process_model::{
    AddressSpace, AddressSpaceId, AddressSpaceInfo, AddressSpaceRegionInfo,
    AlignedCpuExtendedStateBuffer, AuxiliaryVectorEntry, ExecutableImage, Process,
    ProcessAbiProfile, ProcessError, ProcessId, ProcessInfo, ProcessIntrospection,
    ProcessMemoryRegion, ProcessState, Thread, ThreadCpuExtendedStateImage,
    ThreadCpuExtendedStateProfile, ThreadId, ThreadInfo, ThreadState,
    project_hal_address_space_layout, project_hal_page_mappings,
};
pub use process_table::ProcessTable;
pub use scheduler::{ScheduledProcess, Scheduler, SchedulerClass, SchedulerError};
pub use syscall_surface::*;
pub use user_launch::{UserLaunchArgs, UserLaunchPlan};
pub use verified_core::{VerifiedCoreFamily, VerifiedCoreReport, VerifiedCoreViolation};
pub use vfs_model::{MountPoint, VfsError, VfsNamespace, VfsNode};
pub use vm_model::{
    MemoryAdvice, MemoryTouchStats, VmManager, VmObject, VmObjectKind, VmObjectLayoutInfo,
    VmObjectSegmentInfo, VmPageState,
};

pub trait HardwareProvider: Send {
    fn submit_gpu_command(&mut self, rpc_id: u32, payload: &[u8]) -> Result<Vec<u8>, HalError>;
    fn allocate_gpu_memory(
        &mut self,
        kind: platform_hal::GpuMemoryKind,
        size: u64,
    ) -> Result<u64, HalError>;
    fn set_primary_gpu_power_state(&mut self, pstate: u32) -> Result<(), HalError>;
    fn start_primary_gpu_media_session(
        &mut self,
        width: u32,
        height: u32,
        bitrate_kbps: u32,
        codec: u32,
    ) -> Result<(), HalError>;
    fn inject_primary_gpu_neural_semantic(&mut self, semantic_label: &str) -> Result<(), HalError>;
    fn commit_primary_gpu_neural_frame(&mut self) -> Result<(), HalError>;
    fn dispatch_primary_gpu_tensor_kernel(&mut self, kernel_id: u32) -> Result<(), HalError>;
    fn gpu_binding_evidence(
        &mut self,
        device: platform_hal::DeviceLocator,
    ) -> Result<Option<platform_hal::GpuBindingEvidence>, HalError>;
    fn primary_gpu_binding_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuBindingEvidence>, HalError>;
    fn primary_gpu_vbios_window(
        &mut self,
    ) -> Result<Option<platform_hal::GpuVbiosWindowEvidence>, HalError>;
    fn primary_gpu_vbios_bytes(&mut self, max_len: usize) -> Result<Vec<u8>, HalError>;
    fn primary_gpu_vbios_image_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuVbiosImageEvidence>, HalError>;
    fn primary_gpu_gsp_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuGspEvidence>, HalError>;
    fn primary_gpu_interrupt_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuInterruptEvidence>, HalError>;
    fn primary_gpu_display_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuDisplayEvidence>, HalError>;
    fn primary_gpu_power_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuPowerEvidence>, HalError>;
    fn primary_gpu_media_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuMediaEvidence>, HalError>;
    fn primary_gpu_neural_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuNeuralEvidence>, HalError>;
    fn primary_gpu_tensor_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuTensorEvidence>, HalError>;

    fn save_cpu_extended_state(
        &mut self,
        _owner_pid: ProcessId,
        _owner_tid: ThreadId,
        _image: &mut ThreadCpuExtendedStateImage,
    ) -> Result<(), HalError> {
        Err(HalError::Unsupported)
    }

    fn restore_cpu_extended_state(
        &mut self,
        _owner_pid: ProcessId,
        _owner_tid: ThreadId,
        _image: &ThreadCpuExtendedStateImage,
    ) -> Result<(), HalError> {
        Err(HalError::Unsupported)
    }
}

impl<T> HardwareProvider for T
where
    T: platform_hal::GpuPlatform + Send,
{
    fn submit_gpu_command(&mut self, rpc_id: u32, payload: &[u8]) -> Result<Vec<u8>, HalError> {
        platform_hal::GpuPlatform::submit_gpu_command(self, rpc_id, payload)
    }

    fn allocate_gpu_memory(
        &mut self,
        kind: platform_hal::GpuMemoryKind,
        size: u64,
    ) -> Result<u64, HalError> {
        platform_hal::GpuPlatform::allocate_gpu_memory(self, kind, size)
    }

    fn set_primary_gpu_power_state(&mut self, pstate: u32) -> Result<(), HalError> {
        platform_hal::GpuPlatform::set_primary_gpu_power_state(self, pstate)
    }

    fn start_primary_gpu_media_session(
        &mut self,
        width: u32,
        height: u32,
        bitrate_kbps: u32,
        codec: u32,
    ) -> Result<(), HalError> {
        platform_hal::GpuPlatform::start_primary_gpu_media_session(
            self,
            width,
            height,
            bitrate_kbps,
            codec,
        )
    }

    fn inject_primary_gpu_neural_semantic(&mut self, semantic_label: &str) -> Result<(), HalError> {
        platform_hal::GpuPlatform::inject_primary_gpu_neural_semantic(self, semantic_label)
    }

    fn commit_primary_gpu_neural_frame(&mut self) -> Result<(), HalError> {
        platform_hal::GpuPlatform::commit_primary_gpu_neural_frame(self)
    }

    fn dispatch_primary_gpu_tensor_kernel(&mut self, kernel_id: u32) -> Result<(), HalError> {
        platform_hal::GpuPlatform::dispatch_primary_gpu_tensor_kernel(self, kernel_id)
    }

    fn gpu_binding_evidence(
        &mut self,
        device: platform_hal::DeviceLocator,
    ) -> Result<Option<platform_hal::GpuBindingEvidence>, HalError> {
        platform_hal::GpuPlatform::gpu_binding_evidence(self, device)
    }

    fn primary_gpu_binding_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuBindingEvidence>, HalError> {
        platform_hal::GpuPlatform::primary_gpu_binding_evidence(self)
    }

    fn primary_gpu_vbios_window(
        &mut self,
    ) -> Result<Option<platform_hal::GpuVbiosWindowEvidence>, HalError> {
        platform_hal::GpuPlatform::primary_gpu_vbios_window(self)
    }

    fn primary_gpu_vbios_bytes(&mut self, max_len: usize) -> Result<Vec<u8>, HalError> {
        platform_hal::GpuPlatform::primary_gpu_vbios_bytes(self, max_len)
    }

    fn primary_gpu_vbios_image_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuVbiosImageEvidence>, HalError> {
        platform_hal::GpuPlatform::primary_gpu_vbios_image_evidence(self)
    }

    fn primary_gpu_gsp_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuGspEvidence>, HalError> {
        platform_hal::GpuPlatform::primary_gpu_gsp_evidence(self)
    }

    fn primary_gpu_interrupt_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuInterruptEvidence>, HalError> {
        platform_hal::GpuPlatform::primary_gpu_interrupt_evidence(self)
    }

    fn primary_gpu_display_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuDisplayEvidence>, HalError> {
        platform_hal::GpuPlatform::primary_gpu_display_evidence(self)
    }

    fn primary_gpu_power_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuPowerEvidence>, HalError> {
        platform_hal::GpuPlatform::primary_gpu_power_evidence(self)
    }

    fn primary_gpu_media_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuMediaEvidence>, HalError> {
        platform_hal::GpuPlatform::primary_gpu_media_evidence(self)
    }

    fn primary_gpu_neural_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuNeuralEvidence>, HalError> {
        platform_hal::GpuPlatform::primary_gpu_neural_evidence(self)
    }

    fn primary_gpu_tensor_evidence(
        &mut self,
    ) -> Result<Option<platform_hal::GpuTensorEvidence>, HalError> {
        platform_hal::GpuPlatform::primary_gpu_tensor_evidence(self)
    }
}

pub(crate) use alloc::borrow::ToOwned;
pub(crate) use alloc::boxed::Box;
use alloc::collections::BTreeMap;
pub(crate) use alloc::format;
pub(crate) use alloc::string::{String, ToString};
pub(crate) use alloc::vec;
pub(crate) use alloc::vec::Vec;
use core::fmt::Write;
use native_model::{ContractTable, DomainTable, ResourceTable};
use process_model::{default_auxiliary_vector, default_memory_map, executable_image_from_status};

pub(crate) use descriptor_model::{
    FiledescShareGroup, QueueDescriptorTarget, filedesc_kind_code, io_capabilities_for_kind,
    kinfo_status, match_interest, socket_domain_for_kind, socket_protocol_for_kind,
    socket_type_for_kind,
};
pub(crate) use eventing_model::{
    EventQueueWaiter, EventTimerRegistration, KernelEvent, MemoryWaitEventRegistration,
    MemoryWaiter, NetworkEventRegistration, ProcessEventRegistration, ResourceEventRegistration,
    RuntimeSleepQueue, SIGNAL_WAIT_CHANNEL, SignalEventRegistration, default_signal_disposition,
    event_queue_descriptor_name, sleep_queue_descriptor_name,
};
pub use ngos_core_util::SleepWaitResult;
use ngos_core_util::{
    BufRing, BufferError, KernelBuffer, KernelUio, PctrieMap, Range, RangeSet, ScatterGatherList,
    SleepQueue, SleepQueueError, TaskQueue, TaskQueueError, UioDirection,
};
use platform_hal::{
    AddressSpaceId as HalAddressSpaceId, AddressSpaceLayout as HalAddressSpaceLayout,
    AddressSpaceManager, Architecture, CachePolicy, HalError, MemoryPermissions, PageMapping,
};
pub(crate) use syscall_surface::{map_runtime_io_error, memory_advice_code, proc_state_code};
pub(crate) use vfs_model::normalize_path;
#[cfg(test)]
pub(crate) use vm_model::{IO_PAYLOAD_SEGMENT_BASE, IO_PAYLOAD_SEGMENT_BYTES};
pub(crate) use vm_model::{
    align_up, child_path, compose_labeled_name, copy_payload_slice, inferred_vm_object_kind,
    initial_dirty_pages, initial_resident_pages, normalize_vm_object_name, paged_payload_layout,
    path_prefix, scheduler_queue_snapshot,
};

pub use foundation::*;

pub(crate) use runtime_core::DeferredRuntimeTask;
pub use runtime_core::KernelRuntime;

#[cfg(test)]
mod tests;
