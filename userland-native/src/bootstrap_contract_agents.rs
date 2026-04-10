use super::*;

pub(crate) fn validate_program_bootstrap(bootstrap: &BootstrapArgs<'_>) -> Result<(), ExitCode> {
    if bootstrap.argc != bootstrap.argv.len() || bootstrap.argc == 0 {
        return Err(1);
    }
    if !image_path_matches_program(bootstrap.argv[0]) {
        return Err(103);
    }
    Ok(())
}

pub(crate) fn parse_and_validate_boot_context(
    bootstrap: &BootstrapArgs<'_>,
) -> Result<ngos_user_abi::bootstrap::BootContext, ExitCode> {
    let boot_flag = bootstrap.has_flag(BOOT_ARG_FLAG);
    if !boot_flag {
        return Err(104);
    }

    let context = parse_boot_context(bootstrap).map_err(|_| 105)?;
    if context.page_size != 4096 {
        return Err(106);
    }
    if context.entry == 0 {
        return Err(107);
    }
    if context.process_name != PROGRAM_NAME {
        return Err(108);
    }
    if !image_path_matches_program(&context.image_path) {
        return Err(109);
    }
    if context.cwd != "/" {
        return Err(110);
    }
    if context.root_mount_path != "/" {
        return Err(111);
    }
    if context.root_mount_name != "rootfs" {
        return Err(112);
    }
    if context.image_base == 0 {
        return Err(113);
    }
    if context.stack_top == 0 {
        return Err(114);
    }
    if context.phdr == 0 {
        return Err(115);
    }
    if context.phent == 0 {
        return Err(116);
    }
    if context.cpu.xsave_managed && context.cpu.save_area_bytes == 0 {
        return Err(117);
    }
    if context.cpu.xsave_managed && context.cpu.xcr0_mask == 0 {
        return Err(118);
    }
    if context.cpu.hardware_provider_available && !context.cpu.xsave_managed {
        return Err(119);
    }
    if context.phnum == 0 {
        return Err(117);
    }
    let framebuffer = context.framebuffer.as_ref().ok_or(118)?;
    if framebuffer.width == 0 {
        return Err(119);
    }
    if framebuffer.height == 0 {
        return Err(120);
    }
    if framebuffer.pitch == 0 {
        return Err(121);
    }
    if framebuffer.bpp == 0 {
        return Err(122);
    }
    if context.memory_region_count == 0 {
        return Err(123);
    }
    if context.usable_memory_bytes == 0 {
        return Err(124);
    }
    if context.module_phys_start == 0 || context.module_phys_end <= context.module_phys_start {
        return Err(125);
    }
    if context.kernel_phys_start == 0 || context.kernel_phys_end <= context.kernel_phys_start {
        return Err(126);
    }
    match context.boot_outcome_policy {
        BootOutcomePolicy::RequireZeroExit | BootOutcomePolicy::AllowAnyExit => {}
    }

    Ok(context)
}
