#![allow(dead_code)]

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
    use kernel_core::{
        CapabilityRights, Handle, KernelRuntime, ObjectHandle, ObjectKind, SchedulerClass,
    };
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
        let mut runtime = KernelRuntime::host_runtime_default();
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
    }
}
