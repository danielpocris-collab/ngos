use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::mem;

use crate::{
    AT_ENTRY, AT_PAGESZ, AuxvEntry, BOOT_ARG_FLAG, BOOT_ENV_CMDLINE_PREFIX, BOOT_ENV_MARKER,
    BOOT_ENV_MODULE_LEN_PREFIX, BOOT_ENV_MODULE_PHYS_END_PREFIX, BOOT_ENV_MODULE_PHYS_START_PREFIX,
    BOOT_ENV_MODULE_PREFIX, BOOT_ENV_OUTCOME_POLICY_PREFIX, BOOT_ENV_PROTOCOL_PREFIX,
    BootstrapArgs, CWD_ENV_PREFIX, FRAMEBUFFER_BPP_ENV_PREFIX, FRAMEBUFFER_HEIGHT_ENV_PREFIX,
    FRAMEBUFFER_PITCH_ENV_PREFIX, FRAMEBUFFER_PRESENT_ENV_PREFIX, FRAMEBUFFER_WIDTH_ENV_PREFIX,
    IMAGE_BASE_ENV_PREFIX, IMAGE_PATH_ENV_PREFIX, KERNEL_PHYS_END_ENV_PREFIX,
    KERNEL_PHYS_START_ENV_PREFIX, MEMORY_REGION_COUNT_ENV_PREFIX, PHDR_ENV_PREFIX,
    PHENT_ENV_PREFIX, PHNUM_ENV_PREFIX, PHYSICAL_MEMORY_OFFSET_ENV_PREFIX, PROCESS_NAME_ENV_PREFIX,
    ROOT_MOUNT_NAME_ENV_PREFIX, ROOT_MOUNT_PATH_ENV_PREFIX, RSDP_ENV_PREFIX, SESSION_ENV_MARKER,
    SESSION_ENV_OUTCOME_POLICY_PREFIX, SESSION_ENV_PROTOCOL_PREFIX, STACK_ALIGNMENT,
    STACK_TOP_ENV_PREFIX, StartFrame, USABLE_MEMORY_BYTES_ENV_PREFIX,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootstrapError {
    StackTooSmall,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootContextError {
    NotBootMode,
    Missing(&'static str),
    InvalidNumber(&'static str),
    InvalidRange(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FramebufferContext {
    pub width: u64,
    pub height: u64,
    pub pitch: u64,
    pub bpp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootContext {
    pub protocol: String,
    pub cmdline: String,
    pub module_name: String,
    pub module_len: u64,
    pub module_phys_start: u64,
    pub module_phys_end: u64,
    pub process_name: String,
    pub image_path: String,
    pub cwd: String,
    pub root_mount_path: String,
    pub root_mount_name: String,
    pub image_base: u64,
    pub stack_top: u64,
    pub phdr: u64,
    pub phent: u64,
    pub phnum: u64,
    pub page_size: u64,
    pub entry: u64,
    pub framebuffer: Option<FramebufferContext>,
    pub memory_region_count: u64,
    pub usable_memory_bytes: u64,
    pub physical_memory_offset: u64,
    pub rsdp: u64,
    pub kernel_phys_start: u64,
    pub kernel_phys_end: u64,
    pub boot_outcome_policy: BootOutcomePolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionContext {
    pub protocol: String,
    pub outcome_policy: BootOutcomePolicy,
    pub process_name: String,
    pub image_path: String,
    pub cwd: String,
    pub root_mount_path: String,
    pub root_mount_name: String,
    pub image_base: u64,
    pub stack_top: u64,
    pub phdr: u64,
    pub phent: u64,
    pub phnum: u64,
    pub page_size: u64,
    pub entry: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootOutcomePolicy {
    RequireZeroExit,
    AllowAnyExit,
}

fn parse_u64(value: &str, field: &'static str) -> Result<u64, BootContextError> {
    let parsed = if let Some(hex) = value.strip_prefix("0x") {
        u64::from_str_radix(hex, 16)
    } else {
        value.parse::<u64>()
    };
    parsed.map_err(|_| BootContextError::InvalidNumber(field))
}

fn required_env<'a>(
    bootstrap: &'a BootstrapArgs<'_>,
    prefix: &'static str,
) -> Result<&'a str, BootContextError> {
    bootstrap
        .env_value(prefix)
        .ok_or(BootContextError::Missing(prefix))
}

fn required_env_u64(
    bootstrap: &BootstrapArgs<'_>,
    prefix: &'static str,
) -> Result<u64, BootContextError> {
    parse_u64(required_env(bootstrap, prefix)?, prefix)
}

fn parse_outcome_policy(
    bootstrap: &BootstrapArgs<'_>,
    prefix: &'static str,
) -> Result<BootOutcomePolicy, BootContextError> {
    match required_env(bootstrap, prefix)? {
        "require-zero-exit" => Ok(BootOutcomePolicy::RequireZeroExit),
        "allow-any-exit" => Ok(BootOutcomePolicy::AllowAnyExit),
        _ => Err(BootContextError::InvalidNumber(prefix)),
    }
}

pub fn parse_boot_context(bootstrap: &BootstrapArgs<'_>) -> Result<BootContext, BootContextError> {
    if !bootstrap.has_env_value(BOOT_ENV_MARKER) || !bootstrap.has_flag(BOOT_ARG_FLAG) {
        return Err(BootContextError::NotBootMode);
    }

    let module_phys_start = required_env_u64(bootstrap, BOOT_ENV_MODULE_PHYS_START_PREFIX)?;
    let module_phys_end = required_env_u64(bootstrap, BOOT_ENV_MODULE_PHYS_END_PREFIX)?;
    if module_phys_end <= module_phys_start {
        return Err(BootContextError::InvalidRange(
            BOOT_ENV_MODULE_PHYS_END_PREFIX,
        ));
    }

    let kernel_phys_start = required_env_u64(bootstrap, KERNEL_PHYS_START_ENV_PREFIX)?;
    let kernel_phys_end = required_env_u64(bootstrap, KERNEL_PHYS_END_ENV_PREFIX)?;
    if kernel_phys_end <= kernel_phys_start {
        return Err(BootContextError::InvalidRange(KERNEL_PHYS_END_ENV_PREFIX));
    }

    let framebuffer = match bootstrap.env_value(FRAMEBUFFER_PRESENT_ENV_PREFIX) {
        Some("1") => Some(FramebufferContext {
            width: required_env_u64(bootstrap, FRAMEBUFFER_WIDTH_ENV_PREFIX)?,
            height: required_env_u64(bootstrap, FRAMEBUFFER_HEIGHT_ENV_PREFIX)?,
            pitch: required_env_u64(bootstrap, FRAMEBUFFER_PITCH_ENV_PREFIX)?,
            bpp: required_env_u64(bootstrap, FRAMEBUFFER_BPP_ENV_PREFIX)?,
        }),
        Some(_) => {
            return Err(BootContextError::InvalidNumber(
                FRAMEBUFFER_PRESENT_ENV_PREFIX,
            ));
        }
        None => None,
    };

    Ok(BootContext {
        protocol: required_env(bootstrap, BOOT_ENV_PROTOCOL_PREFIX)?.into(),
        cmdline: bootstrap
            .env_value(BOOT_ENV_CMDLINE_PREFIX)
            .unwrap_or("")
            .into(),
        module_name: required_env(bootstrap, BOOT_ENV_MODULE_PREFIX)?.into(),
        module_len: required_env_u64(bootstrap, BOOT_ENV_MODULE_LEN_PREFIX)?,
        module_phys_start,
        module_phys_end,
        process_name: required_env(bootstrap, PROCESS_NAME_ENV_PREFIX)?.into(),
        image_path: required_env(bootstrap, IMAGE_PATH_ENV_PREFIX)?.into(),
        cwd: required_env(bootstrap, CWD_ENV_PREFIX)?.into(),
        root_mount_path: required_env(bootstrap, ROOT_MOUNT_PATH_ENV_PREFIX)?.into(),
        root_mount_name: required_env(bootstrap, ROOT_MOUNT_NAME_ENV_PREFIX)?.into(),
        image_base: required_env_u64(bootstrap, IMAGE_BASE_ENV_PREFIX)?,
        stack_top: required_env_u64(bootstrap, STACK_TOP_ENV_PREFIX)?,
        phdr: required_env_u64(bootstrap, PHDR_ENV_PREFIX)?,
        phent: required_env_u64(bootstrap, PHENT_ENV_PREFIX)?,
        phnum: required_env_u64(bootstrap, PHNUM_ENV_PREFIX)?,
        page_size: bootstrap
            .aux_value(AT_PAGESZ)
            .ok_or(BootContextError::Missing("AT_PAGESZ"))? as u64,
        entry: bootstrap
            .aux_value(AT_ENTRY)
            .ok_or(BootContextError::Missing("AT_ENTRY"))? as u64,
        framebuffer,
        memory_region_count: required_env_u64(bootstrap, MEMORY_REGION_COUNT_ENV_PREFIX)?,
        usable_memory_bytes: required_env_u64(bootstrap, USABLE_MEMORY_BYTES_ENV_PREFIX)?,
        physical_memory_offset: required_env_u64(bootstrap, PHYSICAL_MEMORY_OFFSET_ENV_PREFIX)?,
        rsdp: required_env_u64(bootstrap, RSDP_ENV_PREFIX)?,
        kernel_phys_start,
        kernel_phys_end,
        boot_outcome_policy: parse_outcome_policy(bootstrap, BOOT_ENV_OUTCOME_POLICY_PREFIX)?,
    })
}

pub fn parse_session_context(
    bootstrap: &BootstrapArgs<'_>,
) -> Result<SessionContext, BootContextError> {
    if !bootstrap.has_env_value(SESSION_ENV_MARKER) {
        return Err(BootContextError::NotBootMode);
    }

    Ok(SessionContext {
        protocol: required_env(bootstrap, SESSION_ENV_PROTOCOL_PREFIX)?.into(),
        outcome_policy: parse_outcome_policy(bootstrap, SESSION_ENV_OUTCOME_POLICY_PREFIX)?,
        process_name: required_env(bootstrap, PROCESS_NAME_ENV_PREFIX)?.into(),
        image_path: required_env(bootstrap, IMAGE_PATH_ENV_PREFIX)?.into(),
        cwd: required_env(bootstrap, CWD_ENV_PREFIX)?.into(),
        root_mount_path: required_env(bootstrap, ROOT_MOUNT_PATH_ENV_PREFIX)?.into(),
        root_mount_name: required_env(bootstrap, ROOT_MOUNT_NAME_ENV_PREFIX)?.into(),
        image_base: required_env_u64(bootstrap, IMAGE_BASE_ENV_PREFIX)?,
        stack_top: required_env_u64(bootstrap, STACK_TOP_ENV_PREFIX)?,
        phdr: required_env_u64(bootstrap, PHDR_ENV_PREFIX)?,
        phent: required_env_u64(bootstrap, PHENT_ENV_PREFIX)?,
        phnum: required_env_u64(bootstrap, PHNUM_ENV_PREFIX)?,
        page_size: bootstrap
            .aux_value(AT_PAGESZ)
            .ok_or(BootContextError::Missing("AT_PAGESZ"))? as u64,
        entry: bootstrap
            .aux_value(AT_ENTRY)
            .ok_or(BootContextError::Missing("AT_ENTRY"))? as u64,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapStackImage {
    pub stack_base: usize,
    pub stack_top: usize,
    pub bytes: Vec<u8>,
    pub argc: usize,
    pub argv_addrs: Vec<usize>,
    pub envp_addrs: Vec<usize>,
    pub auxv: Vec<AuxvEntry>,
    pub start_frame: StartFrame,
}

fn align_up(value: usize, alignment: usize) -> usize {
    let mask = alignment.saturating_sub(1);
    (value + mask) & !mask
}

fn write_usize(bytes: &mut [u8], offset: usize, value: usize) {
    let word = mem::size_of::<usize>();
    bytes[offset..offset + word].copy_from_slice(&value.to_le_bytes());
}

pub fn build_initial_stack(
    stack_top: usize,
    bootstrap: &BootstrapArgs<'_>,
) -> Result<BootstrapStackImage, BootstrapError> {
    let mut strings = Vec::new();
    let mut argv_offsets = Vec::with_capacity(bootstrap.argv.len());
    let mut envp_offsets = Vec::with_capacity(bootstrap.envp.len());

    for value in bootstrap.argv {
        argv_offsets.push(strings.len());
        strings.extend_from_slice(value.as_bytes());
        strings.push(0);
    }

    for value in bootstrap.envp {
        envp_offsets.push(strings.len());
        strings.extend_from_slice(value.as_bytes());
        strings.push(0);
    }

    let word = mem::size_of::<usize>();
    let pointer_words =
        1 + bootstrap.argv.len() + 1 + bootstrap.envp.len() + 1 + bootstrap.auxv.len() * 2 + 2;
    let pointer_bytes = align_up(pointer_words * word, STACK_ALIGNMENT);
    let total_bytes = align_up(pointer_bytes.saturating_add(strings.len()), STACK_ALIGNMENT);
    if total_bytes > stack_top {
        return Err(BootstrapError::StackTooSmall);
    }

    let stack_base = stack_top - total_bytes;
    let strings_base = stack_base + pointer_bytes;

    let argv_addrs = argv_offsets
        .iter()
        .map(|offset| strings_base + *offset)
        .collect::<Vec<_>>();
    let envp_addrs = envp_offsets
        .iter()
        .map(|offset| strings_base + *offset)
        .collect::<Vec<_>>();

    let argv_ptr = stack_base + word;
    let envp_ptr = stack_base + word * (1 + bootstrap.argv.len() + 1);
    let auxv_ptr = stack_base + word * (1 + bootstrap.argv.len() + 1 + bootstrap.envp.len() + 1);

    let mut bytes = vec![0; total_bytes];
    let mut cursor = 0usize;
    write_usize(&mut bytes, cursor, bootstrap.argc);
    cursor += word;
    for addr in &argv_addrs {
        write_usize(&mut bytes, cursor, *addr);
        cursor += word;
    }
    write_usize(&mut bytes, cursor, 0);
    cursor += word;
    for addr in &envp_addrs {
        write_usize(&mut bytes, cursor, *addr);
        cursor += word;
    }
    write_usize(&mut bytes, cursor, 0);
    cursor += word;
    for aux in bootstrap.auxv {
        write_usize(&mut bytes, cursor, aux.key);
        cursor += word;
        write_usize(&mut bytes, cursor, aux.value);
        cursor += word;
    }
    write_usize(&mut bytes, cursor, 0);
    cursor += word;
    write_usize(&mut bytes, cursor, 0);

    bytes[pointer_bytes..pointer_bytes + strings.len()].copy_from_slice(&strings);

    Ok(BootstrapStackImage {
        stack_base,
        stack_top,
        bytes,
        argc: bootstrap.argc,
        argv_addrs,
        envp_addrs,
        auxv: bootstrap.auxv.to_vec(),
        start_frame: StartFrame {
            argc: bootstrap.argc,
            argv: argv_ptr as *const *const u8,
            envp: envp_ptr as *const *const u8,
            auxv: auxv_ptr as *const AuxvEntry,
            stack_alignment: STACK_ALIGNMENT,
        },
    })
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BOOT_ENV_MARKER, SESSION_ENV_MARKER};

    #[test]
    fn build_initial_stack_emits_start_frame() {
        let bootstrap = BootstrapArgs::new(
            &["prog", "arg1"],
            &["A=1", "B=2"],
            &[AuxvEntry { key: 7, value: 11 }],
        );
        let image = build_initial_stack(0x8000, &bootstrap).unwrap();
        assert_eq!(image.argc, 2);
        assert_eq!(image.argv_addrs.len(), 2);
        assert_eq!(image.envp_addrs.len(), 2);
        assert!(image.stack_base < image.stack_top);
        assert_eq!(image.bytes.len(), image.stack_top - image.stack_base);
        assert!(image.bytes.windows(4).any(|window| window == b"prog"));
        assert_eq!(image.start_frame.argc, 2);
        assert_eq!(image.start_frame.stack_alignment, STACK_ALIGNMENT);
        assert_eq!(image.stack_base % STACK_ALIGNMENT, 0);
        assert_eq!(
            (image.start_frame.argv as usize) % mem::size_of::<usize>(),
            0
        );
        assert_eq!(
            (image.start_frame.envp as usize) % mem::size_of::<usize>(),
            0
        );
        assert_eq!(
            (image.start_frame.auxv as usize) % mem::align_of::<AuxvEntry>(),
            0
        );
    }

    #[test]
    fn parse_boot_context_extracts_structured_contract() {
        let argv = ["ngos-userland-native", BOOT_ARG_FLAG];
        let envp = [
            BOOT_ENV_MARKER,
            "NGOS_BOOT_PROTOCOL=limine",
            "NGOS_BOOT_MODULE=ngos-userland-native",
            "NGOS_BOOT_MODULE_LEN=12288",
            "NGOS_BOOT_MODULE_PHYS_START=0x200000",
            "NGOS_BOOT_MODULE_PHYS_END=0x203000",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
            "NGOS_MEMORY_REGION_COUNT=2",
            "NGOS_USABLE_MEMORY_BYTES=8388608",
            "NGOS_PHYSICAL_MEMORY_OFFSET=0x0",
            "NGOS_RSDP=0xdeadbeef",
            "NGOS_KERNEL_PHYS_START=0x100000",
            "NGOS_KERNEL_PHYS_END=0x101000",
            "NGOS_BOOT_OUTCOME_POLICY=require-zero-exit",
            "NGOS_FRAMEBUFFER_PRESENT=1",
            "NGOS_FRAMEBUFFER_WIDTH=1920",
            "NGOS_FRAMEBUFFER_HEIGHT=1080",
            "NGOS_FRAMEBUFFER_PITCH=7680",
            "NGOS_FRAMEBUFFER_BPP=32",
        ];
        let auxv = [
            AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        let context = parse_boot_context(&bootstrap).unwrap();
        assert_eq!(context.protocol, "limine");
        assert_eq!(context.module_name, "ngos-userland-native");
        assert_eq!(context.module_phys_start, 0x200000);
        assert_eq!(context.module_phys_end, 0x203000);
        assert_eq!(context.page_size, 4096);
        assert_eq!(context.entry, 0x401000);
        assert_eq!(context.kernel_phys_start, 0x100000);
        assert_eq!(context.kernel_phys_end, 0x101000);
        assert_eq!(
            context.boot_outcome_policy,
            BootOutcomePolicy::RequireZeroExit
        );
        assert_eq!(context.framebuffer.unwrap().width, 1920);
    }

    #[test]
    fn parse_boot_context_rejects_invalid_ranges() {
        let argv = ["ngos-userland-native", BOOT_ARG_FLAG];
        let envp = [
            BOOT_ENV_MARKER,
            "NGOS_BOOT_PROTOCOL=limine",
            "NGOS_BOOT_MODULE=ngos-userland-native",
            "NGOS_BOOT_MODULE_LEN=12288",
            "NGOS_BOOT_MODULE_PHYS_START=0x203000",
            "NGOS_BOOT_MODULE_PHYS_END=0x200000",
            "NGOS_PROCESS_NAME=ngos-userland-native",
            "NGOS_IMAGE_PATH=ngos-userland-native",
            "NGOS_CWD=/",
            "NGOS_ROOT_MOUNT_PATH=/",
            "NGOS_ROOT_MOUNT_NAME=rootfs",
            "NGOS_IMAGE_BASE=0x400000",
            "NGOS_STACK_TOP=0x7fffffff0000",
            "NGOS_PHDR=0x40",
            "NGOS_PHENT=56",
            "NGOS_PHNUM=2",
            "NGOS_MEMORY_REGION_COUNT=2",
            "NGOS_USABLE_MEMORY_BYTES=8388608",
            "NGOS_PHYSICAL_MEMORY_OFFSET=0x0",
            "NGOS_RSDP=0xdeadbeef",
            "NGOS_KERNEL_PHYS_START=0x100000",
            "NGOS_KERNEL_PHYS_END=0x101000",
            "NGOS_BOOT_OUTCOME_POLICY=require-zero-exit",
            "NGOS_FRAMEBUFFER_PRESENT=1",
            "NGOS_FRAMEBUFFER_WIDTH=1920",
            "NGOS_FRAMEBUFFER_HEIGHT=1080",
            "NGOS_FRAMEBUFFER_PITCH=7680",
            "NGOS_FRAMEBUFFER_BPP=32",
        ];
        let auxv = [
            AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        assert_eq!(
            parse_boot_context(&bootstrap),
            Err(BootContextError::InvalidRange(
                BOOT_ENV_MODULE_PHYS_END_PREFIX
            ))
        );
    }

    #[test]
    fn parse_session_context_extracts_kernel_launch_contract() {
        let argv = ["ngos-userland-native"];
        let envp = [
            SESSION_ENV_MARKER,
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
            AuxvEntry {
                key: AT_PAGESZ,
                value: 4096,
            },
            AuxvEntry {
                key: AT_ENTRY,
                value: 0x401000,
            },
        ];
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);

        let context = parse_session_context(&bootstrap).unwrap();
        assert_eq!(context.protocol, "kernel-launch");
        assert_eq!(context.outcome_policy, BootOutcomePolicy::RequireZeroExit);
        assert_eq!(context.process_name, "ngos-userland-native");
        assert_eq!(context.image_path, "/bin/ngos-userland-native");
        assert_eq!(context.page_size, 4096);
        assert_eq!(context.entry, 0x401000);
    }
}
