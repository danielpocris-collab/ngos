use ngos_user_abi::{
    Amd64UserEntryRegisters, AuxvEntry, BootstrapArgs, CWD_ENV_PREFIX, IMAGE_BASE_ENV_PREFIX,
    IMAGE_PATH_ENV_PREFIX, PHDR_ENV_PREFIX, PHENT_ENV_PREFIX, PHNUM_ENV_PREFIX,
    PROCESS_NAME_ENV_PREFIX, ROOT_MOUNT_NAME_ENV_PREFIX, ROOT_MOUNT_PATH_ENV_PREFIX,
    SESSION_ENV_MARKER, SESSION_ENV_OUTCOME_POLICY_PREFIX, SESSION_ENV_PROTOCOL_PREFIX,
    STACK_TOP_ENV_PREFIX,
    bootstrap::{BootstrapStackImage, build_initial_stack},
};
use platform_hal::{CachePolicy, MemoryPermissions, PageMapping, VirtualRange};

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserLaunchArgs {
    pub argv: Vec<String>,
    pub envp: Vec<String>,
    pub auxv: Vec<AuxiliaryVectorEntry>,
}

impl UserLaunchArgs {
    fn from_process(runtime: &KernelRuntime, process: &Process) -> Self {
        let mut envp = process.envp().to_vec();
        if !envp.iter().any(|entry| entry == SESSION_ENV_MARKER) {
            envp.push(String::from(SESSION_ENV_MARKER));
        }
        append_metadata_env(&mut envp, SESSION_ENV_PROTOCOL_PREFIX, "kernel-launch");
        append_metadata_env(
            &mut envp,
            SESSION_ENV_OUTCOME_POLICY_PREFIX,
            "require-zero-exit",
        );
        append_metadata_env(&mut envp, PROCESS_NAME_ENV_PREFIX, process.name());
        append_metadata_env(&mut envp, IMAGE_PATH_ENV_PREFIX, process.image_path());
        append_metadata_env(&mut envp, CWD_ENV_PREFIX, process.cwd());
        append_metadata_env(
            &mut envp,
            IMAGE_BASE_ENV_PREFIX,
            &format!("{:#x}", process.executable_image().base_addr),
        );
        append_metadata_env(
            &mut envp,
            STACK_TOP_ENV_PREFIX,
            &format!("{:#x}", process.executable_image().stack_top),
        );
        append_metadata_env(
            &mut envp,
            PHDR_ENV_PREFIX,
            &format!("{:#x}", process.executable_image().phdr_addr),
        );
        append_metadata_env(
            &mut envp,
            PHENT_ENV_PREFIX,
            &process.executable_image().phent_size.to_string(),
        );
        append_metadata_env(
            &mut envp,
            PHNUM_ENV_PREFIX,
            &process.executable_image().phnum.to_string(),
        );
        if let Ok(mount) = runtime.vfs().statfs(process.cwd()) {
            append_metadata_env(&mut envp, ROOT_MOUNT_PATH_ENV_PREFIX, mount.mount_path());
            append_metadata_env(&mut envp, ROOT_MOUNT_NAME_ENV_PREFIX, mount.name());
        }
        Self {
            argv: process.argv().to_vec(),
            envp,
            auxv: process.auxv().to_vec(),
        }
    }

    fn as_bootstrap_args<'a>(
        &'a self,
        argv_refs: &'a [&'a str],
        envp_refs: &'a [&'a str],
        auxv: &'a [AuxvEntry],
    ) -> BootstrapArgs<'a> {
        BootstrapArgs::new(argv_refs, envp_refs, auxv)
    }

    fn build_stack_image(&self, stack_top: usize) -> Result<BootstrapStackImage, RuntimeError> {
        let argv_refs = self
            .argv
            .iter()
            .map(|value| value.as_str())
            .collect::<Vec<_>>();
        let envp_refs = self
            .envp
            .iter()
            .map(|value| value.as_str())
            .collect::<Vec<_>>();
        let auxv = self
            .auxv
            .iter()
            .map(|entry| AuxvEntry {
                key: entry.key as usize,
                value: entry.value as usize,
            })
            .collect::<Vec<_>>();
        let bootstrap = self.as_bootstrap_args(&argv_refs, &envp_refs, &auxv);
        build_initial_stack(stack_top, &bootstrap)
            .map_err(|_| RuntimeError::Process(ProcessError::InvalidMemoryLayout))
    }
}

fn append_metadata_env(envp: &mut Vec<String>, prefix: &str, value: &str) {
    if envp.iter().any(|entry| entry.starts_with(prefix)) {
        return;
    }
    envp.push(format!("{prefix}{value}"));
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserLaunchPlan {
    pub pid: ProcessId,
    pub executable_image: ExecutableImage,
    pub bootstrap: UserLaunchArgs,
    pub image_mappings: Vec<PageMapping>,
    pub stack_image: BootstrapStackImage,
    pub stack_range: VirtualRange,
    pub stack_mapping: PageMapping,
    pub registers: Amd64UserEntryRegisters,
}

fn user_stack_mapping(stack_image: &BootstrapStackImage) -> Result<PageMapping, RuntimeError> {
    let len = align_up(stack_image.bytes.len() as u64, 0x1000)
        .ok_or(ProcessError::InvalidMemoryLayout)?;
    Ok(PageMapping {
        vaddr: stack_image.stack_base as u64,
        paddr: 0,
        len,
        perms: MemoryPermissions::read_write(),
        cache: CachePolicy::WriteBack,
        user: true,
    })
}

pub(crate) fn prepare_user_launch(
    runtime: &KernelRuntime,
    pid: ProcessId,
) -> Result<UserLaunchPlan, RuntimeError> {
    let process = runtime.processes.get(pid)?;
    let address_space = runtime.processes.get_process_address_space(pid)?;
    let executable_image = process.executable_image().clone();
    let bootstrap = UserLaunchArgs::from_process(runtime, process);
    let stack_image = bootstrap.build_stack_image(executable_image.stack_top as usize)?;
    let stack_mapping = user_stack_mapping(&stack_image)?;
    let image_mappings = address_space
        .memory_map()
        .iter()
        .filter(|region| region.label.trim() != "[stack]")
        .map(|region| {
            let perms = MemoryPermissions {
                read: region.readable,
                write: region.writable,
                execute: region.executable,
            };
            PageMapping {
                vaddr: region.start,
                paddr: 0,
                len: region.end.saturating_sub(region.start),
                perms,
                cache: CachePolicy::WriteBack,
                user: true,
            }
        })
        .collect::<Vec<_>>();
    let stack_range = VirtualRange {
        vaddr: stack_image.stack_base as u64,
        len: stack_mapping.len,
    };
    let registers = Amd64UserEntryRegisters::from_start_frame(
        executable_image.entry_point as usize,
        stack_image.stack_top,
        stack_image.start_frame,
    );

    Ok(UserLaunchPlan {
        pid,
        executable_image,
        bootstrap,
        image_mappings,
        stack_range,
        stack_image,
        stack_mapping,
        registers,
    })
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use ngos_user_abi::{AMD64_USER_CODE_SELECTOR, AMD64_USER_STACK_SELECTOR};

    #[test]
    fn launch_plan_derives_stack_mapping_and_registers() {
        let mut runtime = KernelRuntime::host_runtime_default();
        let pid = runtime
            .spawn_process("bootstrap", None, SchedulerClass::LatencyCritical)
            .unwrap();
        let root = runtime
            .grant_capability(
                pid,
                ObjectHandle::new(Handle::new(7_400), 0),
                CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
                "root",
            )
            .unwrap();
        let bin = runtime
            .grant_capability(
                pid,
                ObjectHandle::new(Handle::new(7_401), 0),
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
            .create_vfs_node("/srv", ObjectKind::Directory, root)
            .unwrap();
        runtime
            .create_vfs_node("/srv/app", ObjectKind::Directory, root)
            .unwrap();
        runtime
            .create_vfs_node("/bin/app", ObjectKind::File, bin)
            .unwrap();
        runtime
            .exec_process(
                pid,
                String::from("/bin/app"),
                vec![String::from("app"), String::from("--flag")],
                vec![String::from("USER=test")],
            )
            .unwrap();
        runtime.set_process_cwd(pid, "/srv/app").unwrap();

        let plan = prepare_user_launch(&runtime, pid).unwrap();
        assert_eq!(plan.pid, pid);
        assert_eq!(plan.bootstrap.argv.len(), 2);
        assert!(
            plan.bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_SESSION=1")
        );
        assert!(
            plan.bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_SESSION_PROTOCOL=kernel-launch")
        );
        assert!(
            plan.bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_PROCESS_NAME=app")
        );
        assert!(
            plan.bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_SESSION_OUTCOME_POLICY=require-zero-exit")
        );
        assert!(
            plan.bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_IMAGE_PATH=/bin/app")
        );
        assert!(
            plan.bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_CWD=/srv/app")
        );
        assert!(
            plan.bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_ROOT_MOUNT_PATH=/")
        );
        assert!(
            plan.bootstrap
                .envp
                .iter()
                .any(|entry| entry == "NGOS_ROOT_MOUNT_NAME=rootfs")
        );
        assert!(
            plan.bootstrap
                .envp
                .iter()
                .any(|entry| entry.starts_with("NGOS_IMAGE_BASE=0x"))
        );
        assert!(
            plan.bootstrap
                .envp
                .iter()
                .any(|entry| entry.starts_with("NGOS_STACK_TOP=0x"))
        );
        assert!(
            plan.bootstrap
                .envp
                .iter()
                .any(|entry| entry.starts_with("NGOS_PHDR=0x"))
        );
        assert!(
            plan.bootstrap
                .envp
                .iter()
                .any(|entry| entry.starts_with("NGOS_PHENT="))
        );
        assert!(
            plan.bootstrap
                .envp
                .iter()
                .any(|entry| entry.starts_with("NGOS_PHNUM="))
        );
        assert!(!plan.image_mappings.is_empty());
        assert!(plan.image_mappings.iter().all(|mapping| mapping.user));
        assert_eq!(plan.stack_image.start_frame.argc, 2);
        assert!(plan.stack_mapping.user);
        assert!(plan.stack_mapping.perms.read);
        assert!(plan.stack_mapping.perms.write);
        assert_eq!(
            plan.registers.rip,
            plan.executable_image.entry_point as usize
        );
        assert_eq!(plan.registers.rsp, plan.stack_image.stack_top);
        assert_eq!(plan.registers.cs, AMD64_USER_CODE_SELECTOR);
        assert_eq!(plan.registers.ss, AMD64_USER_STACK_SELECTOR);
        assert_eq!(plan.registers.rdi, 2);
        assert_eq!(
            plan.registers.rsi,
            plan.stack_image.start_frame.argv as usize
        );
        assert_eq!(
            plan.registers.rdx,
            plan.stack_image.start_frame.envp as usize
        );
    }
}
