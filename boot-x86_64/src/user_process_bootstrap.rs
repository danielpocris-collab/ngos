//! Canonical subsystem role:
//! - subsystem: first-user bootstrap contract assembly
//! - owner layer: Layer 0
//! - semantic owner: `boot-x86_64`
//! - truth path role: assembly of canonical bootstrap env/auxv for the first
//!   native user process
//!
//! Canonical contract families produced here:
//! - boot environment contracts
//! - session bootstrap contracts
//! - first-user CPU/runtime handoff contracts
//!
//! This module may serialize canonical boot facts into the first-user bootstrap
//! contract, but it must not define long-term kernel truth beyond that
//! handoff boundary.

use super::*;

pub(super) struct BootBootstrapInputs {
    pub(super) argv: Vec<String>,
    pub(super) envp: Vec<String>,
    pub(super) auxv: Vec<AuxvEntry>,
    pub(super) boot_outcome_policy: BootOutcomePolicy,
}

pub(super) fn build_bootstrap_inputs(
    boot_info: &platform_x86_64::BootInfo<'_>,
    module: platform_x86_64::BootModule<'_>,
    entry_point: u64,
    image_base: u64,
    stack_top: u64,
    phdr_addr: u64,
    phent_size: u64,
    phnum: u64,
) -> BootBootstrapInputs {
    let argv = vec![String::from(USER_MODULE_NAME), String::from(BOOT_ARG_FLAG)];
    let module_phys_end = module.physical_start.saturating_add(module.len);
    let kernel_phys_end = boot_info
        .kernel_phys_range
        .start
        .saturating_add(boot_info.kernel_phys_range.len);
    let usable_bytes = boot_info
        .memory_regions
        .iter()
        .filter(|region| region.kind == BootMemoryRegionKind::Usable)
        .map(|region| region.len)
        .sum::<u64>();
    let cpu = crate::cpu_runtime_status::snapshot();
    let mut envp = vec![
        String::from(BOOT_ENV_MARKER),
        String::from(SESSION_ENV_MARKER),
        format!(
            "{}{}",
            BOOT_ENV_PROTOCOL_PREFIX,
            boot_protocol_name(boot_info.protocol)
        ),
        format!("{}{}", SESSION_ENV_PROTOCOL_PREFIX, "kernel-launch"),
        format!("{}{}", BOOT_ENV_MODULE_PREFIX, module.name),
        format!("{}{}", BOOT_ENV_MODULE_LEN_PREFIX, module.len),
        format!(
            "{}{:#x}",
            BOOT_ENV_MODULE_PHYS_START_PREFIX, module.physical_start
        ),
        format!("{}{:#x}", BOOT_ENV_MODULE_PHYS_END_PREFIX, module_phys_end),
        format!("{}{}", PROCESS_NAME_ENV_PREFIX, USER_MODULE_NAME),
        format!("{}{}", IMAGE_PATH_ENV_PREFIX, module.name),
        format!("{}{}", CWD_ENV_PREFIX, "/"),
        format!("{}{}", ROOT_MOUNT_PATH_ENV_PREFIX, "/"),
        format!("{}{}", ROOT_MOUNT_NAME_ENV_PREFIX, "rootfs"),
        format!("{}{:#x}", IMAGE_BASE_ENV_PREFIX, image_base),
        format!("{}{:#x}", STACK_TOP_ENV_PREFIX, stack_top),
        format!("{}{:#x}", PHDR_ENV_PREFIX, phdr_addr),
        format!("{}{}", PHENT_ENV_PREFIX, phent_size),
        format!("{}{}", PHNUM_ENV_PREFIX, phnum),
        format!(
            "{}{}",
            MEMORY_REGION_COUNT_ENV_PREFIX,
            boot_info.memory_regions.len()
        ),
        format!("{}{}", USABLE_MEMORY_BYTES_ENV_PREFIX, usable_bytes),
        format!(
            "{}{:#x}",
            PHYSICAL_MEMORY_OFFSET_ENV_PREFIX, boot_info.physical_memory_offset
        ),
        format!(
            "{}{:#x}",
            KERNEL_PHYS_START_ENV_PREFIX, boot_info.kernel_phys_range.start
        ),
        format!("{}{:#x}", KERNEL_PHYS_END_ENV_PREFIX, kernel_phys_end),
        format!(
            "{}{}",
            BOOT_ENV_CPU_XSAVE_PREFIX,
            if cpu.xsave_enabled { 1 } else { 0 }
        ),
        format!("{}{}", BOOT_ENV_CPU_SAVE_AREA_PREFIX, cpu.save_area_bytes),
        format!("{}{:#x}", BOOT_ENV_CPU_XCR0_PREFIX, cpu.xcr0),
        format!(
            "{}{:#x}",
            BOOT_ENV_CPU_BOOT_SEED_PREFIX, cpu.probe_seed_marker
        ),
        format!(
            "{}{}",
            BOOT_ENV_CPU_HW_PROVIDER_PREFIX,
            if cpu.hardware_provider_installed {
                1
            } else {
                0
            }
        ),
    ];
    let boot_outcome_policy = boot_outcome_policy_from_boot_info(boot_info);
    envp.push(format!(
        "{}{}",
        BOOT_ENV_OUTCOME_POLICY_PREFIX,
        boot_outcome_policy_name(boot_outcome_policy)
    ));
    envp.push(format!(
        "{}{}",
        SESSION_ENV_OUTCOME_POLICY_PREFIX,
        boot_outcome_policy_name(boot_outcome_policy)
    ));
    if let Some(command_line) = boot_info.command_line {
        envp.push(format!("{}{}", BOOT_ENV_CMDLINE_PREFIX, command_line));
        if let Some(proof) = boot_proof_from_command_line(command_line) {
            envp.push(format!("{}{}", BOOT_ENV_PROOF_PREFIX, proof));
        }
    }
    if let Some(rsdp) = boot_info.rsdp {
        envp.push(format!("{}{:#x}", RSDP_ENV_PREFIX, rsdp));
    }
    if let Some(framebuffer) = boot_info.framebuffer {
        envp.push(format!("{}1", FRAMEBUFFER_PRESENT_ENV_PREFIX));
        envp.push(format!(
            "{}{}",
            FRAMEBUFFER_WIDTH_ENV_PREFIX, framebuffer.width
        ));
        envp.push(format!(
            "{}{}",
            FRAMEBUFFER_HEIGHT_ENV_PREFIX, framebuffer.height
        ));
        envp.push(format!(
            "{}{}",
            FRAMEBUFFER_PITCH_ENV_PREFIX, framebuffer.pitch
        ));
        envp.push(format!("{}{}", FRAMEBUFFER_BPP_ENV_PREFIX, framebuffer.bpp));
    }
    crate::boot_locator::event(
        crate::boot_locator::BootLocatorStage::User,
        crate::boot_locator::BootLocatorKind::Transition,
        crate::boot_locator::BootLocatorSeverity::Info,
        0x565,
        crate::boot_locator::BootPayloadLabel::Length,
        cpu.save_area_bytes as u64,
        crate::boot_locator::BootPayloadLabel::Status,
        ((cpu.xsave_enabled as u64) << 1) | (cpu.hardware_provider_installed as u64),
    );

    BootBootstrapInputs {
        argv,
        envp,
        boot_outcome_policy,
        auxv: vec![
            AuxvEntry {
                key: AT_PAGESZ,
                value: PAGE_SIZE_4K as usize,
            },
            AuxvEntry {
                key: AT_ENTRY,
                value: entry_point as usize,
            },
        ],
    }
}

fn boot_outcome_policy_from_boot_info(
    boot_info: &platform_x86_64::BootInfo<'_>,
) -> BootOutcomePolicy {
    let command_line = match boot_info.command_line {
        Some(value) => value,
        None => return BootOutcomePolicy::RequireZeroExit,
    };
    if command_line
        .split_ascii_whitespace()
        .any(|token| token == "ngos.boot_outcome=allow-any-exit")
    {
        BootOutcomePolicy::AllowAnyExit
    } else {
        BootOutcomePolicy::RequireZeroExit
    }
}

fn boot_proof_from_command_line(command_line: &str) -> Option<&'static str> {
    for token in command_line.split_ascii_whitespace() {
        match token {
            "ngos.boot.proof=vm" => return Some("vm"),
            "ngos.boot.proof=device-runtime" => return Some("device-runtime"),
            "ngos.boot.proof=shell" => return Some("shell"),
            "ngos.boot.proof=scheduler" => return Some("scheduler"),
            "ngos.boot.proof=process-exec" => return Some("process-exec"),
            "ngos.boot.proof=vfs" => return Some("vfs"),
            "ngos.boot.proof=wasm" => return Some("wasm"),
            "ngos.boot.proof=render3d" => return Some("render3d"),
            "ngos.boot.proof=compat-gfx" => return Some("compat-gfx"),
            "ngos.boot.proof=compat-audio" => return Some("compat-audio"),
            "ngos.boot.proof=compat-input" => return Some("compat-input"),
            "ngos.boot.proof=compat-loader" => return Some("compat-loader"),
            "ngos.boot.proof=compat-abi" => return Some("compat-abi"),
            "ngos.boot.proof=compat-foreign" => return Some("compat-foreign"),
            "ngos.boot.proof=bus" => return Some("bus"),
            "ngos.boot.proof=network" => return Some("network"),
            "ngos.boot.proof=network-hardware" => return Some("network-hardware"),
            "ngos.boot.proof=network-hardware-interface" => {
                return Some("network-hardware-interface");
            }
            "ngos.boot.proof=network-hardware-rx" => return Some("network-hardware-rx"),
            "ngos.boot.proof=network-hardware-udp-rx" => return Some("network-hardware-udp-rx"),
            "ngos.boot.proof=network-hardware-tx" => return Some("network-hardware-tx"),
            "ngos.boot.proof=network-hardware-udp-tx" => return Some("network-hardware-udp-tx"),
            "ngos.boot.proof=storage-commit" => return Some("storage-commit"),
            "ngos.boot.proof=storage-recover" => return Some("storage-recover"),
            "ngos.boot.proof=storage-corrupt" => return Some("storage-corrupt"),
            _ => {}
        }
    }
    None
}

fn boot_outcome_policy_name(policy: BootOutcomePolicy) -> &'static str {
    match policy {
        BootOutcomePolicy::RequireZeroExit => "require-zero-exit",
        BootOutcomePolicy::AllowAnyExit => "allow-any-exit",
    }
}

#[cfg(test)]
mod tests {
    use super::boot_proof_from_command_line;

    #[test]
    fn parses_network_hardware_proof_from_command_line() {
        assert_eq!(
            boot_proof_from_command_line("console=ttyS0 ngos.boot.proof=network-hardware"),
            Some("network-hardware")
        );
    }

    #[test]
    fn parses_network_hardware_tx_proof_from_command_line() {
        assert_eq!(
            boot_proof_from_command_line("console=ttyS0 ngos.boot.proof=network-hardware-tx"),
            Some("network-hardware-tx")
        );
    }

    #[test]
    fn parses_network_hardware_rx_proof_from_command_line() {
        assert_eq!(
            boot_proof_from_command_line("console=ttyS0 ngos.boot.proof=network-hardware-rx"),
            Some("network-hardware-rx")
        );
    }

    #[test]
    fn parses_network_hardware_udp_rx_proof_from_command_line() {
        assert_eq!(
            boot_proof_from_command_line("console=ttyS0 ngos.boot.proof=network-hardware-udp-rx"),
            Some("network-hardware-udp-rx")
        );
    }

    #[test]
    fn parses_network_hardware_interface_proof_from_command_line() {
        assert_eq!(
            boot_proof_from_command_line(
                "console=ttyS0 ngos.boot.proof=network-hardware-interface"
            ),
            Some("network-hardware-interface")
        );
    }

    #[test]
    fn parses_network_hardware_udp_tx_proof_from_command_line() {
        assert_eq!(
            boot_proof_from_command_line("console=ttyS0 ngos.boot.proof=network-hardware-udp-tx"),
            Some("network-hardware-udp-tx")
        );
    }
}
