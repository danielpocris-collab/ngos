#![allow(dead_code)]

//! Canonical subsystem role:
//! - subsystem: boot-to-runtime bridge
//! - owner layer: Layer 0 to Layer 1 transition
//! - semantic owner: `boot-x86_64`
//! - truth path role: bridge from boot facts and handoff state into
//!   `kernel-core` runtime installation
//!
//! Canonical contract families handled here:
//! - kernel launch handoff contracts
//! - CPU handoff installation contracts
//! - first-user transition contracts
//!
//! This module may translate and install boot-produced state into runtime
//! structures, but it must not invent a replacement semantic model for
//! `kernel-core` or `user-abi`.

use platform_x86_64::user_mode::{
    UserAddressSpaceMapper, UserModeError, UserModeLaunchPlan, install_user_mode_address_space,
};

use crate::boot_locator::{
    self, BootLocatorKind, BootLocatorSeverity, BootLocatorStage, BootPayloadLabel,
};

#[cfg(not(target_os = "none"))]
pub fn launch_plan_from_kernel(plan: &kernel_core::UserLaunchPlan) -> UserModeLaunchPlan {
    UserModeLaunchPlan {
        registers: plan.registers,
        image_mappings: plan.image_mappings.clone(),
        stack_mapping: plan.stack_mapping,
        stack_bytes: plan.stack_image.bytes.clone(),
    }
}

#[cfg(not(target_os = "none"))]
pub fn host_kernel_runtime_with_boot_cpu_profile() -> kernel_core::KernelRuntime {
    let mut policy = kernel_core::RuntimePolicy::host_runtime_default();
    crate::cpu_handoff::apply_boot_cpu_extended_state_handoff(&mut policy);
    let mut runtime = kernel_core::KernelRuntime::new(policy);
    crate::cpu_hardware_provider::BootCpuHardwareProvider::install_into_runtime(&mut runtime);
    runtime
}

pub fn install_first_user_process<M: UserAddressSpaceMapper>(
    mapper: &mut M,
    plan: &UserModeLaunchPlan,
) -> Result<(), UserModeError> {
    boot_locator::event(
        BootLocatorStage::User,
        BootLocatorKind::Transition,
        BootLocatorSeverity::Info,
        0x570,
        BootPayloadLabel::Count,
        plan.image_mappings.len() as u64,
        BootPayloadLabel::Length,
        plan.stack_mapping.len,
    );
    install_user_mode_address_space(mapper, plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_core::{CapabilityRights, Handle, ObjectHandle, ObjectKind, SchedulerClass};
    use platform_hal::PageMapping;

    #[derive(Default)]
    struct RecordingMapper {
        mappings: Vec<(PageMapping, Option<Vec<u8>>)>,
        activated: bool,
    }

    impl UserAddressSpaceMapper for RecordingMapper {
        fn map_user_pages(
            &mut self,
            mapping: PageMapping,
            initial_bytes: Option<&[u8]>,
        ) -> Result<(), UserModeError> {
            self.mappings
                .push((mapping, initial_bytes.map(|bytes| bytes.to_vec())));
            Ok(())
        }

        fn activate_user_address_space(&mut self) -> Result<(), UserModeError> {
            self.activated = true;
            Ok(())
        }
    }

    #[test]
    fn bridge_converts_kernel_launch_plan_and_installs_mappings() {
        let _guard = crate::cpu_runtime_status::lock_shared_test_state();
        crate::boot_locator::reset();
        crate::cpu_runtime_status::reset();
        crate::cpu_runtime_status::record(
            true, true, 4096, true, true, true, true, true, true, true, 0xe7,
        );
        crate::cpu_runtime_status::record_probe(true, true, true, 4096, 0, 0x1234_5678);
        let mut runtime = host_kernel_runtime_with_boot_cpu_profile();
        let pid = runtime
            .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
            .unwrap();
        let root = runtime
            .grant_capability(
                pid,
                ObjectHandle::new(Handle::new(9_001), 0),
                CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
                "root",
            )
            .unwrap();
        let bin = runtime
            .grant_capability(
                pid,
                ObjectHandle::new(Handle::new(9_002), 0),
                CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
                "bin",
            )
            .unwrap();
        runtime
            .create_vfs_node("/", ObjectKind::Directory, root)
            .unwrap();
        runtime
            .create_vfs_node("/bin", ObjectKind::Directory, root)
            .unwrap();
        runtime
            .create_vfs_node("/bin/userland-native", ObjectKind::File, bin)
            .unwrap();
        runtime
            .exec_process(
                pid,
                "/bin/userland-native",
                vec![String::from("userland-native")],
                vec![String::from("USERLAND=1")],
            )
            .unwrap();
        let kernel_plan = runtime.prepare_user_launch(pid).unwrap();
        let arch_plan = launch_plan_from_kernel(&kernel_plan);
        let mut mapper = RecordingMapper::default();
        install_first_user_process(&mut mapper, &arch_plan).unwrap();

        assert!(!arch_plan.image_mappings.is_empty());
        assert_eq!(mapper.mappings.len(), arch_plan.image_mappings.len() + 1);
        assert!(mapper.mappings.last().unwrap().1.is_some());
        assert!(mapper.activated);
        let threads = runtime.thread_infos(pid).unwrap();
        assert_eq!(threads.len(), 1);
        assert!(threads[0].cpu_extended_state.xsave_managed);
        assert_eq!(threads[0].cpu_extended_state.save_area_bytes, 4096);
        assert_eq!(threads[0].cpu_extended_state.xcr0_mask, 0xe7);
        assert!(threads[0].cpu_extended_state.boot_probed);
        assert_eq!(threads[0].cpu_extended_state.boot_seed_marker, 0x1234_5678);
    }

    #[test]
    fn bridge_installs_boot_cpu_hardware_provider_for_runtime_switches() {
        let _guard = crate::cpu_runtime_status::lock_shared_test_state();
        crate::boot_locator::reset();
        crate::cpu_runtime_status::reset();
        crate::cpu_runtime_status::record(
            true, true, 512, true, true, true, true, true, true, true, 0x3,
        );
        crate::cpu_runtime_status::record_probe(true, true, true, 512, 0, 0x9abc_def0);
        let mut runtime = host_kernel_runtime_with_boot_cpu_profile();
        let init = runtime
            .spawn_process("bridge-init", None, SchedulerClass::BestEffort)
            .unwrap();
        let shell = runtime
            .spawn_process("bridge-shell", Some(init), SchedulerClass::Interactive)
            .unwrap();

        let first = runtime.tick().unwrap();
        runtime.block_running().unwrap();
        let second = runtime.tick().unwrap();
        assert_ne!(first.tid, second.tid);

        let telemetry = runtime.cpu_extended_state_hardware_telemetry();
        assert_eq!(telemetry.save_count, 1);
        assert_eq!(telemetry.restore_count, 2);
        assert_eq!(telemetry.fallback_count, 0);
        assert_eq!(telemetry.last_saved_tid, Some(first.tid));
        assert_eq!(telemetry.last_restored_tid, Some(second.tid));
        let boot_cpu = crate::cpu_runtime_status::snapshot();
        assert!(boot_cpu.hardware_provider_installed);
        assert!(!boot_cpu.hardware_provider_skipped);
        assert_eq!(boot_cpu.hardware_provider_install_attempts, 1);
        assert_eq!(boot_cpu.hardware_provider_refusal_code, 0);
        let user_status = crate::user_runtime_status::snapshot();
        assert!(user_status.cpu_hw_provider_installed);
        assert!(!user_status.cpu_hw_provider_skipped);
        assert_eq!(user_status.cpu_hw_provider_attempts, 1);
        assert_eq!(user_status.cpu_hw_provider_refusal_code, 0);
        let locator = crate::boot_locator::snapshot();
        assert_eq!(locator.stage, crate::boot_locator::BootLocatorStage::User);
        assert_eq!(locator.checkpoint, 0x571);
        assert_eq!(locator.payload0, 1);
        assert_eq!(locator.payload1, 512);

        let cpu = String::from_utf8(runtime.read_procfs_path("/proc/system/cpu").unwrap()).unwrap();
        assert!(cpu.contains("hardware-saves:\t1"));
        assert!(cpu.contains("hardware-restores:\t2"));
        assert!(cpu.contains("hardware-fallbacks:\t0"));

        let shell_threads = runtime.thread_infos(shell).unwrap();
        let init_threads = runtime.thread_infos(init).unwrap();
        assert!(
            shell_threads[0].cpu_extended_state.save_count >= 1
                || init_threads[0].cpu_extended_state.save_count >= 1
        );
    }

    #[test]
    fn bridge_skips_boot_cpu_hardware_provider_when_xsave_is_unavailable() {
        let _guard = crate::cpu_runtime_status::lock_shared_test_state();
        crate::boot_locator::reset();
        crate::cpu_runtime_status::reset();
        crate::cpu_runtime_status::record(
            true, false, 0, true, true, true, true, true, true, true, 0,
        );
        crate::cpu_runtime_status::record_probe(false, false, false, 0, 1, 0);
        let mut runtime = host_kernel_runtime_with_boot_cpu_profile();
        let init = runtime
            .spawn_process("bridge-no-xsave-init", None, SchedulerClass::BestEffort)
            .unwrap();
        let shell = runtime
            .spawn_process(
                "bridge-no-xsave-shell",
                Some(init),
                SchedulerClass::Interactive,
            )
            .unwrap();

        let first = runtime.tick().unwrap();
        runtime.block_running().unwrap();
        let second = runtime.tick().unwrap();
        assert_ne!(first.tid, second.tid);

        let telemetry = runtime.cpu_extended_state_hardware_telemetry();
        assert_eq!(telemetry.save_count, 0);
        assert_eq!(telemetry.restore_count, 0);
        assert_eq!(telemetry.fallback_count, 0);
        assert_eq!(telemetry.last_error, None);
        let boot_cpu = crate::cpu_runtime_status::snapshot();
        assert!(!boot_cpu.hardware_provider_installed);
        assert!(boot_cpu.hardware_provider_skipped);
        assert_eq!(boot_cpu.hardware_provider_install_attempts, 1);
        assert_eq!(
            boot_cpu.hardware_provider_refusal_code,
            crate::cpu_hardware_provider::BootCpuHardwareProviderRefusal::XsaveDisabled as u32
        );
        let user_status = crate::user_runtime_status::snapshot();
        assert!(!user_status.cpu_hw_provider_installed);
        assert!(user_status.cpu_hw_provider_skipped);
        assert_eq!(user_status.cpu_hw_provider_attempts, 1);
        assert_eq!(
            user_status.cpu_hw_provider_refusal_code,
            crate::cpu_hardware_provider::BootCpuHardwareProviderRefusal::XsaveDisabled as u32
        );
        let locator = crate::boot_locator::snapshot();
        assert_eq!(locator.stage, crate::boot_locator::BootLocatorStage::User);
        assert_eq!(locator.checkpoint, 0x572);
        assert_eq!(
            locator.payload0,
            crate::cpu_hardware_provider::BootCpuHardwareProviderRefusal::XsaveDisabled as u64
        );
        assert_eq!(locator.payload1, 0);

        let cpu = String::from_utf8(runtime.read_procfs_path("/proc/system/cpu").unwrap()).unwrap();
        assert!(cpu.contains("hardware-saves:\t0"));
        assert!(cpu.contains("hardware-restores:\t0"));
        assert!(cpu.contains("hardware-fallbacks:\t0"));

        let shell_threads = runtime.thread_infos(shell).unwrap();
        let init_threads = runtime.thread_infos(init).unwrap();
        assert!(!shell_threads[0].cpu_extended_state.xsave_managed);
        assert!(!init_threads[0].cpu_extended_state.xsave_managed);
    }

    #[test]
    fn bridge_records_save_area_too_large_refusal_for_boot_cpu_provider() {
        let _guard = crate::cpu_runtime_status::lock_shared_test_state();
        crate::boot_locator::reset();
        crate::cpu_runtime_status::reset();
        crate::cpu_runtime_status::record(
            true,
            true,
            (16 * 1024 + 64) as u32,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            0x3,
        );
        crate::cpu_runtime_status::record_probe(false, false, false, (16 * 1024 + 64) as u32, 2, 0);
        let runtime = host_kernel_runtime_with_boot_cpu_profile();
        let telemetry = runtime.cpu_extended_state_hardware_telemetry();
        assert_eq!(telemetry.save_count, 0);
        assert_eq!(telemetry.restore_count, 0);
        assert_eq!(telemetry.fallback_count, 0);

        let boot_cpu = crate::cpu_runtime_status::snapshot();
        assert!(!boot_cpu.hardware_provider_installed);
        assert!(boot_cpu.hardware_provider_skipped);
        assert_eq!(boot_cpu.hardware_provider_install_attempts, 1);
        assert_eq!(
            boot_cpu.hardware_provider_refusal_code,
            crate::cpu_hardware_provider::BootCpuHardwareProviderRefusal::SaveAreaTooLarge as u32
        );
        let user_status = crate::user_runtime_status::snapshot();
        assert_eq!(
            user_status.cpu_hw_provider_refusal_code,
            crate::cpu_hardware_provider::BootCpuHardwareProviderRefusal::SaveAreaTooLarge as u32
        );
        let locator = crate::boot_locator::snapshot();
        assert_eq!(locator.checkpoint, 0x572);
        assert_eq!(
            locator.payload0,
            crate::cpu_hardware_provider::BootCpuHardwareProviderRefusal::SaveAreaTooLarge as u64
        );
        assert_eq!(locator.payload1, (16 * 1024 + 64) as u64);
    }

    #[test]
    fn bridge_records_save_area_unavailable_refusal_for_boot_cpu_provider() {
        let _guard = crate::cpu_runtime_status::lock_shared_test_state();
        crate::boot_locator::reset();
        crate::cpu_runtime_status::reset();
        crate::cpu_runtime_status::record(
            true, true, 0, true, true, true, true, true, true, true, 0x3,
        );
        crate::cpu_runtime_status::record_probe(false, false, false, 0, 3, 0);
        let runtime = host_kernel_runtime_with_boot_cpu_profile();
        let telemetry = runtime.cpu_extended_state_hardware_telemetry();
        assert_eq!(telemetry.save_count, 0);
        assert_eq!(telemetry.restore_count, 0);
        assert_eq!(telemetry.fallback_count, 0);

        let boot_cpu = crate::cpu_runtime_status::snapshot();
        assert!(!boot_cpu.hardware_provider_installed);
        assert!(boot_cpu.hardware_provider_skipped);
        assert_eq!(boot_cpu.hardware_provider_install_attempts, 1);
        assert_eq!(
            boot_cpu.hardware_provider_refusal_code,
            crate::cpu_hardware_provider::BootCpuHardwareProviderRefusal::SaveAreaUnavailable
                as u32
        );
        let user_status = crate::user_runtime_status::snapshot();
        assert_eq!(
            user_status.cpu_hw_provider_refusal_code,
            crate::cpu_hardware_provider::BootCpuHardwareProviderRefusal::SaveAreaUnavailable
                as u32
        );
        let locator = crate::boot_locator::snapshot();
        assert_eq!(locator.checkpoint, 0x572);
        assert_eq!(
            locator.payload0,
            crate::cpu_hardware_provider::BootCpuHardwareProviderRefusal::SaveAreaUnavailable
                as u64
        );
        assert_eq!(locator.payload1, 0);
    }

    #[test]
    fn bridge_records_xcr0_unavailable_refusal_for_boot_cpu_provider() {
        let _guard = crate::cpu_runtime_status::lock_shared_test_state();
        crate::boot_locator::reset();
        crate::cpu_runtime_status::reset();
        crate::cpu_runtime_status::record(
            true, true, 512, true, true, true, true, true, true, true, 0,
        );
        crate::cpu_runtime_status::record_probe(false, false, false, 512, 4, 0);
        let runtime = host_kernel_runtime_with_boot_cpu_profile();
        let telemetry = runtime.cpu_extended_state_hardware_telemetry();
        assert_eq!(telemetry.save_count, 0);
        assert_eq!(telemetry.restore_count, 0);
        assert_eq!(telemetry.fallback_count, 0);

        let boot_cpu = crate::cpu_runtime_status::snapshot();
        assert!(!boot_cpu.hardware_provider_installed);
        assert!(boot_cpu.hardware_provider_skipped);
        assert_eq!(boot_cpu.hardware_provider_install_attempts, 1);
        assert_eq!(
            boot_cpu.hardware_provider_refusal_code,
            crate::cpu_hardware_provider::BootCpuHardwareProviderRefusal::Xcr0Unavailable as u32
        );
        let user_status = crate::user_runtime_status::snapshot();
        assert_eq!(
            user_status.cpu_hw_provider_refusal_code,
            crate::cpu_hardware_provider::BootCpuHardwareProviderRefusal::Xcr0Unavailable as u32
        );
        let locator = crate::boot_locator::snapshot();
        assert_eq!(locator.checkpoint, 0x572);
        assert_eq!(
            locator.payload0,
            crate::cpu_hardware_provider::BootCpuHardwareProviderRefusal::Xcr0Unavailable as u64
        );
        assert_eq!(locator.payload1, 512);
    }
}
