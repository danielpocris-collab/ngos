use super::*;

pub(crate) fn run_boot_desktop<B: SyscallBackend>(
    runtime: &Runtime<B>,
    context: &ngos_user_abi::bootstrap::BootContext,
) -> ExitCode {
    let framebuffer = match &context.framebuffer {
        Some(framebuffer) => framebuffer,
        None => return 118,
    };
    let frame = match build_boot_desktop_frame(framebuffer) {
        Some(frame) => frame,
        None => return 127,
    };
    let encoded = frame.encode("desktop-boot");
    let graphics_domain = match runtime.create_domain(None, "boot-desktop") {
        Ok(domain) => domain,
        Err(_) => return 234,
    };
    let graphics_resource = match runtime.create_resource(
        graphics_domain,
        NativeResourceKind::Device,
        "boot-desktop-gpu",
    ) {
        Ok(resource) => resource,
        Err(_) => return 234,
    };
    if game_apply_resource_policy(runtime, graphics_resource, CompatLaneKind::Graphics).is_err() {
        return 234;
    }
    let graphics_contract = match runtime.create_contract(
        graphics_domain,
        graphics_resource,
        NativeContractKind::Display,
        "boot-desktop-scanout",
    ) {
        Ok(contract) => contract,
        Err(_) => return 234,
    };
    if runtime
        .set_contract_state(graphics_contract, NativeContractState::Active)
        .is_err()
    {
        return 234;
    }
    let claim_outcome = match runtime.claim_resource(graphics_contract) {
        Ok(outcome) => outcome,
        Err(_) => return 234,
    };
    if !matches!(claim_outcome, ResourceClaimOutcome::Acquired { .. }) {
        let _ = write_line(
            runtime,
            "desktop.boot stage=claim queue=default reason=queued",
        );
        return 0;
    }

    if write_line(
        runtime,
        &format!(
            "desktop.boot stage=compose surface={}x{} frame={} queue={} present={} completion={} ops={}",
            frame.width,
            frame.height,
            frame.frame_tag,
            frame.queue,
            frame.present_mode,
            frame.completion,
            frame.ops.len()
        ),
    )
    .is_err()
    {
        return 190;
    }
    let queue_configured =
        ngos_shell_gpu::shell_gpu_queue_capacity(runtime, "/dev/gpu0", 32).is_ok();
    if !queue_configured
        && write_line(
            runtime,
            "desktop.boot stage=queue-config queue=default reason=unsupported",
        )
        .is_err()
    {
        return 190;
    }
    let submitted =
        ngos_shell_gpu::shell_gpu_submit(runtime, "/dev/gpu0", &encoded.payload).is_ok();
    let presented = submitted
        && ngos_shell_gpu::shell_gpu_present(runtime, "/dev/gpu0", &encoded.frame_tag).is_ok();
    if presented {
        if write_line(
            runtime,
            &format!(
                "desktop.boot stage=presented frame={} payload={} framebuffer={}x{}",
                encoded.frame_tag,
                encoded.payload.len(),
                framebuffer.width,
                framebuffer.height
            ),
            )
        .is_err()
        {
            return 190;
        }
    } else if submitted {
        if write_line(
            runtime,
            &format!(
                "desktop.boot stage=submitted frame={} payload={} framebuffer={}x{} present=pending",
                encoded.frame_tag,
                encoded.payload.len(),
                framebuffer.width,
                framebuffer.height
            ),
        )
        .is_err()
        {
            return 190;
        }
    } else if write_line(
        runtime,
        &format!(
            "desktop.boot stage=submit-failed frame={} payload={} framebuffer={}x{} present=pending",
            encoded.frame_tag,
            encoded.payload.len(),
            framebuffer.width,
            framebuffer.height
        ),
    )
    .is_err()
    {
        return 190;
    }
    let _ = runtime.release_claimed_resource(graphics_contract);
    0
}

pub(crate) fn boot_bind_observe_contract_handle<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> Result<usize, ExitCode> {
    let domain = runtime
        .create_domain(None, "boot-vm-observe")
        .map_err(|_| 235)?;
    let resource = runtime
        .create_resource(domain, NativeResourceKind::Namespace, "boot-vm-observe")
        .map_err(|_| 235)?;
    runtime
        .set_resource_contract_policy(resource, NativeResourceContractPolicy::Observe)
        .map_err(|_| 235)?;
    let contract = runtime
        .create_contract(
            domain,
            resource,
            NativeContractKind::Observe,
            "boot-vm-observe",
        )
        .map_err(|_| 235)?;
    runtime.bind_process_contract(contract).map_err(|_| 235)?;
    Ok(contract)
}

pub(crate) fn boot_bind_observe_contract<B: SyscallBackend>(
    runtime: &Runtime<B>,
) -> Result<(), ExitCode> {
    boot_bind_observe_contract_handle(runtime).map(|_| ())
}
