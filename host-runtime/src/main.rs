mod backend;
mod report;
mod session;

use session::build_native_session_report;

fn main() {
    let native = build_native_session_report();
    print!("{}", native.render());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::HostRuntimeKernelBackend;
    use crate::session::{
        HOST_SCRATCH_LEN, build_native_host_test_fixture_and_configure,
        build_native_session_report_with_script,
        build_native_session_report_with_script_and_configure,
    };
    use kernel_core::{
        CapabilityRights, ContractKind, Descriptor, DeviceRequestState, Handle, KernelRuntime,
        ObjectHandle, ObjectKind, ResourceContractPolicy, ResourceKind, ResourceReleaseResult,
        SchedulerClass,
    };
    use ngos_platform_hal::{
        BarKind, DeviceIdentity, DeviceLocator, DevicePlatform, DmaCoherency, DmaConstraints,
        DmaDirection, DmaOwnership, GpuPlatform, HalError, MmioCachePolicy, MmioPermissions,
    };
    use ngos_platform_x86_64::device_platform::PciAddress;
    use ngos_platform_x86_64::{
        DEFAULT_DIRECT_MAP_BASE, SyntheticPciConfigBackend, X86_64DevicePlatform,
        X86_64DevicePlatformConfig,
    };
    use user_abi::{
        AuxvEntry, BootstrapArgs, NativeContractKind, NativeContractState,
        NativeResourceArbitrationPolicy, NativeResourceContractPolicy,
        NativeResourceGovernanceMode, NativeResourceIssuerPolicy, NativeResourceKind,
        NativeResourceState,
    };
    use user_runtime::Runtime as UserRuntime;

    fn report_for_script(script: &str) -> crate::report::HostRuntimeNativeSessionReport {
        build_native_session_report_with_script(script.as_bytes())
    }

    fn run_shell_script_direct(script: &str) -> (i32, String) {
        let fixture = build_native_host_test_fixture_and_configure(|_| {});
        let mut runtime = fixture.runtime;
        let app = fixture.app;
        runtime.seed_standard_input(app, script.as_bytes()).unwrap();
        let scratch = fixture.scratch;
        let launch = runtime.prepare_user_launch(app).unwrap();
        let argv = launch
            .bootstrap
            .argv
            .iter()
            .map(|value| value.as_str())
            .collect::<Vec<_>>();
        let envp = launch
            .bootstrap
            .envp
            .iter()
            .map(|value| value.as_str())
            .collect::<Vec<_>>();
        let auxv = launch
            .bootstrap
            .auxv
            .iter()
            .map(|entry| AuxvEntry {
                key: entry.key as usize,
                value: entry.value as usize,
            })
            .collect::<Vec<_>>();
        let backend =
            HostRuntimeKernelBackend::new(runtime, app, scratch, HOST_SCRATCH_LEN as usize);
        let user_runtime = UserRuntime::new(backend);
        let bootstrap = BootstrapArgs::new(&argv, &envp, &auxv);
        let exit_code = userland_native::main(&user_runtime, &bootstrap);
        let mut runtime = user_runtime.backend().runtime_mut();
        let stdout = String::from_utf8(
            runtime
                .inspect_io(app, Descriptor::new(1))
                .unwrap()
                .payload()
                .to_vec(),
        )
        .unwrap();
        runtime.exit(app, exit_code).unwrap();
        (exit_code, stdout)
    }

    fn report_for_script_with_runtime<F>(
        script: &str,
        configure_runtime: F,
    ) -> crate::report::HostRuntimeNativeSessionReport
    where
        F: FnOnce(&mut KernelRuntime),
    {
        build_native_session_report_with_script_and_configure(script.as_bytes(), configure_runtime)
    }

    fn configured_game_user_runtime() -> (
        UserRuntime<HostRuntimeKernelBackend>,
        kernel_core::ProcessId,
    ) {
        let fixture = build_native_host_test_fixture_and_configure(|_| {});
        let app = fixture.app;
        let backend = HostRuntimeKernelBackend::new(
            fixture.runtime,
            app,
            fixture.scratch,
            HOST_SCRATCH_LEN as usize,
        );
        let user_runtime = UserRuntime::new(backend);
        user_runtime.mkdir_path("/games").unwrap();
        user_runtime.mkdir_path("/games/orbit").unwrap();
        (user_runtime, app)
    }

    fn write_session_file(
        user_runtime: &UserRuntime<HostRuntimeKernelBackend>,
        path: &str,
        value: &[u8],
    ) {
        let fd = user_runtime.open_path(path).unwrap();
        user_runtime.write(fd, value).unwrap();
        user_runtime.close(fd).unwrap();
    }

    fn prepare_game_session_bootstrap_io(user_runtime: &UserRuntime<HostRuntimeKernelBackend>) {
        user_runtime.mkdir_path("/compat").unwrap();
        user_runtime.mkdir_path("/compat/orbit").unwrap();
        user_runtime.mkdir_path("/saves").unwrap();
        user_runtime.mkdir_path("/saves/orbit").unwrap();
        user_runtime.mkdir_path("/cache").unwrap();
        user_runtime.mkdir_path("/cache/orbit").unwrap();
        user_runtime
            .mkfile_path("/compat/orbit/session.env")
            .unwrap();
        user_runtime
            .mkfile_path("/compat/orbit/session.argv")
            .unwrap();
        user_runtime
            .mkchan_path("/compat/orbit/session.chan")
            .unwrap();
        write_session_file(
            user_runtime,
            "/compat/orbit/session.env",
            b"NGOS_GAME_CHANNEL=/compat/orbit/session.chan",
        );
        write_session_file(user_runtime, "/compat/orbit/session.argv", b"/bin/worker");
    }

    fn prepare_game_session_resources(
        user_runtime: &UserRuntime<HostRuntimeKernelBackend>,
    ) -> ((usize, usize), (usize, usize), (usize, usize)) {
        let domain = user_runtime
            .create_domain(None, "compat-game-orbit-runner")
            .unwrap();

        let gfx = user_runtime
            .create_resource(domain, NativeResourceKind::Surface, "orbit-runner-gfx")
            .unwrap();
        user_runtime
            .set_resource_arbitration_policy(gfx, NativeResourceArbitrationPolicy::Fifo)
            .unwrap();
        user_runtime
            .set_resource_governance_mode(gfx, NativeResourceGovernanceMode::ExclusiveLease)
            .unwrap();
        user_runtime
            .set_resource_contract_policy(gfx, NativeResourceContractPolicy::Display)
            .unwrap();
        user_runtime
            .set_resource_issuer_policy(gfx, NativeResourceIssuerPolicy::CreatorOnly)
            .unwrap();
        let gfx_contract = user_runtime
            .create_contract(
                domain,
                gfx,
                NativeContractKind::Display,
                "frame-pace-display",
            )
            .unwrap();
        user_runtime
            .set_contract_state(gfx_contract, NativeContractState::Active)
            .unwrap();
        user_runtime.acquire_resource(gfx_contract).unwrap();

        let audio = user_runtime
            .create_resource(domain, NativeResourceKind::Channel, "orbit-runner-audio")
            .unwrap();
        user_runtime
            .set_resource_arbitration_policy(audio, NativeResourceArbitrationPolicy::Fifo)
            .unwrap();
        user_runtime
            .set_resource_governance_mode(audio, NativeResourceGovernanceMode::Queueing)
            .unwrap();
        user_runtime
            .set_resource_contract_policy(audio, NativeResourceContractPolicy::Io)
            .unwrap();
        user_runtime
            .set_resource_issuer_policy(audio, NativeResourceIssuerPolicy::CreatorOnly)
            .unwrap();
        let audio_contract = user_runtime
            .create_contract(domain, audio, NativeContractKind::Io, "spatial-mix-mix")
            .unwrap();
        user_runtime
            .set_contract_state(audio_contract, NativeContractState::Active)
            .unwrap();
        user_runtime.acquire_resource(audio_contract).unwrap();

        let input = user_runtime
            .create_resource(domain, NativeResourceKind::Device, "orbit-runner-input")
            .unwrap();
        user_runtime
            .set_resource_arbitration_policy(input, NativeResourceArbitrationPolicy::Fifo)
            .unwrap();
        user_runtime
            .set_resource_governance_mode(input, NativeResourceGovernanceMode::ExclusiveLease)
            .unwrap();
        user_runtime
            .set_resource_contract_policy(input, NativeResourceContractPolicy::Observe)
            .unwrap();
        user_runtime
            .set_resource_issuer_policy(input, NativeResourceIssuerPolicy::CreatorOnly)
            .unwrap();
        let input_contract = user_runtime
            .create_contract(
                domain,
                input,
                NativeContractKind::Observe,
                "gamepad-first-capture",
            )
            .unwrap();
        user_runtime
            .set_contract_state(input_contract, NativeContractState::Active)
            .unwrap();
        user_runtime.acquire_resource(input_contract).unwrap();

        (
            (gfx, gfx_contract),
            (audio, audio_contract),
            (input, input_contract),
        )
    }

    fn cleanup_game_session_resources(
        user_runtime: &UserRuntime<HostRuntimeKernelBackend>,
        resources: &[(usize, usize)],
    ) {
        for &(_, contract) in resources {
            user_runtime.release_resource(contract).unwrap();
        }
        for &(resource, contract) in resources {
            user_runtime
                .set_contract_state(contract, NativeContractState::Suspended)
                .unwrap();
            user_runtime
                .set_resource_state(resource, NativeResourceState::Suspended)
                .unwrap();
        }
    }

    fn sample_platform_identity(base_class: u8, sub_class: u8, interface: u8) -> DeviceIdentity {
        DeviceIdentity {
            vendor_id: 0x8086,
            device_id: 0x100e,
            subsystem_vendor_id: 0,
            subsystem_device_id: 0,
            revision_id: 1,
            base_class,
            sub_class,
            programming_interface: interface,
        }
    }

    fn sample_gpu_platform() -> X86_64DevicePlatform<SyntheticPciConfigBackend> {
        let mut backend = SyntheticPciConfigBackend::new();
        let gpu = PciAddress {
            segment: 0,
            bus: 0,
            device: 3,
            function: 0,
        };
        backend.define_device(
            gpu,
            sample_platform_identity(0x03, 0x00, 0x00),
            0x1234,
            0x1111,
            false,
            9,
            1,
        );
        backend.define_bar(gpu, 0, 0xfebe_0000, 0xffff_f000);
        X86_64DevicePlatform::new(backend, X86_64DevicePlatformConfig::default())
    }

    fn sample_nvidia_gpu_platform() -> (
        X86_64DevicePlatform<SyntheticPciConfigBackend>,
        DeviceLocator,
    ) {
        let mut backend = SyntheticPciConfigBackend::new();
        let address = PciAddress {
            segment: 0,
            bus: 0,
            device: 5,
            function: 0,
        };
        backend.define_device(
            address,
            DeviceIdentity {
                vendor_id: 0x10de,
                device_id: 0x2d04,
                subsystem_vendor_id: 0x10de,
                subsystem_device_id: 0x0001,
                revision_id: 1,
                base_class: 0x03,
                sub_class: 0,
                programming_interface: 0,
            },
            0x10de,
            0x0001,
            false,
            9,
            1,
        );
        backend.define_bar(address, 0, 0xfec0_0000, 0xffff_f000);
        backend.define_bar(address, 1, 0xd000_0000, 0xf000_0000);
        backend.define_capability(address, 0x50, 0x0003_0011, 0x00);
        backend.define_config_dword(address, 0x30, 0x00c0_0001);
        let mut rom = vec![0; 0x400];
        rom[0..8].copy_from_slice(&[0x55, 0xaa, 0x4e, 0x56, 0x49, 0x44, 0x49, 0x41]);
        rom[0x40..0x44].copy_from_slice(b"NVFW");
        rom[0x120..0x124].copy_from_slice(b"PCIR");
        rom[0x124..0x126].copy_from_slice(&0x10deu16.to_le_bytes());
        rom[0x126..0x128].copy_from_slice(&0x2d04u16.to_le_bytes());
        rom[0x1c0..0x1da].copy_from_slice(b"NVIDIA GeForce RTX 5060 Ti");
        rom[0x220..0x22e].copy_from_slice(b"P14N:506T301FB");
        rom[0x280..0x296].copy_from_slice(b"Version 98.06.1F.00.DC");
        rom[0x320..0x323].copy_from_slice(b"BIT");
        backend.define_rom(0x00c0_0000, &rom);
        let mut platform =
            X86_64DevicePlatform::new(backend, X86_64DevicePlatformConfig::default());
        let locator = platform.enumerate_devices().unwrap().remove(0).locator;
        (platform, locator)
    }

    #[test]
    fn native_session_report_completes_trivial_shell() {
        let report = report_for_script("exit 0\n");

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(report.stdout.contains("ngos shell"));
    }

    #[test]
    fn direct_shell_game_launch_exit_runs_without_report_prelude() {
        let (exit_code, stdout) = run_shell_script_direct(
            "mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-launch /games/orbit.manifest\nexit 0\n",
        );

        assert_eq!(exit_code, 0, "{stdout}");
        assert!(stdout.starts_with("ngos shell\n"), "{stdout}");
        assert!(stdout.contains("game.launched pid="), "{stdout}");
        assert!(stdout.contains("game.session pid="), "{stdout}");
        assert!(!stdout.contains("== chronoscope =="), "{stdout}");
    }

    #[test]
    fn native_session_report_can_spawn_kill_and_reap_worker_process() {
        let report = report_for_script(
            "spawn-path worker /bin/worker\nkill $LAST_PID 15\nreap $LAST_PID\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains("process-spawned pid="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("signal-sent pid="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("process-reaped pid="),
            "{}",
            report.render()
        );
    }

    #[test]
    fn configured_spawn_process_can_launch_signal_and_reap_worker_directly() {
        let (user_runtime, _) = configured_game_user_runtime();

        let pid = user_runtime
            .spawn_configured_process(
                "game-orbit-runner",
                "/bin/worker",
                "/games/orbit",
                &["/bin/worker", "--fullscreen"],
                &["NGOS_GAME_CHANNEL=/compat/orbit/session.chan"],
            )
            .unwrap();
        user_runtime.send_signal(pid, 15).unwrap();
        let exit_code = user_runtime.reap_process(pid).unwrap();

        assert_eq!(pid, 3);
        assert_eq!(exit_code, 143);
    }

    #[test]
    fn configured_game_session_launch_primitives_can_spawn_and_cleanup_exactly() {
        let (user_runtime, _) = configured_game_user_runtime();
        prepare_game_session_bootstrap_io(&user_runtime);
        let ((gfx, gfx_contract), (audio, audio_contract), (input, input_contract)) =
            prepare_game_session_resources(&user_runtime);

        user_runtime.chdir_path("/games/orbit").unwrap();
        let pid = user_runtime
            .spawn_configured_process(
                "game-orbit-runner",
                "/bin/worker",
                "/games/orbit",
                &["/bin/worker"],
                &["NGOS_GAME_CHANNEL=/compat/orbit/session.chan"],
            )
            .unwrap();
        user_runtime.send_signal(pid, 15).unwrap();
        let exit_code = user_runtime.reap_process(pid).unwrap();
        cleanup_game_session_resources(
            &user_runtime,
            &[
                (gfx, gfx_contract),
                (audio, audio_contract),
                (input, input_contract),
            ],
        );

        assert_eq!(pid, 3);
        assert_eq!(exit_code, 143);
    }

    #[test]
    fn configured_game_session_exact_bootstrap_io_can_spawn_and_cleanup() {
        let (user_runtime, _) = configured_game_user_runtime();
        prepare_game_session_bootstrap_io(&user_runtime);
        let ((gfx, gfx_contract), (audio, audio_contract), (input, input_contract)) =
            prepare_game_session_resources(&user_runtime);

        let pid = user_runtime
            .spawn_configured_process(
                "game-orbit-runner",
                "/bin/worker",
                "/games/orbit",
                &["/bin/worker"],
                &["NGOS_GAME_CHANNEL=/compat/orbit/session.chan"],
            )
            .unwrap();
        user_runtime.send_signal(pid, 15).unwrap();
        let exit_code = user_runtime.reap_process(pid).unwrap();
        cleanup_game_session_resources(
            &user_runtime,
            &[
                (gfx, gfx_contract),
                (audio, audio_contract),
                (input, input_contract),
            ],
        );

        assert_eq!(pid, 3);
        assert_eq!(exit_code, 143);
    }

    #[test]
    fn native_session_report_configures_graphics_game_resource_family() {
        let report = report_for_script(
            "mkdomain game\nmkresource 2 surface gfx0\nmkcontract 2 2 display scanout\nresource-governance 2 exclusive-lease\nresource-contract-policy 2 display\nresource-issuer-policy 2 creator-only\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("resource-governance-updated id=2 mode=exclusive-lease"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("resource-contract-policy-updated id=2 policy=display"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_configures_audio_game_resource_family() {
        let report = report_for_script(
            "mkdomain game\nmkresource 2 channel audio0\nmkcontract 2 2 io mix\nresource-governance 2 queueing\nresource-contract-policy 2 io\nresource-issuer-policy 2 creator-only\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("resource-governance-updated id=2 mode=queueing"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("resource-contract-policy-updated id=2 policy=io"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_configures_input_game_resource_family() {
        let report = report_for_script(
            "mkdomain game\nmkresource 2 device input0\nmkcontract 2 2 observe controls\nresource-governance 2 exclusive-lease\nresource-contract-policy 2 observe\nresource-issuer-policy 2 creator-only\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("resource-governance-updated id=2 mode=exclusive-lease"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("resource-contract-policy-updated id=2 policy=observe"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_can_bootstrap_and_spawn_game_worker_without_game_resources() {
        let report = report_for_script(
            "mkdir-path /games\nmkdir-path /games/orbit\nmkdir-path /compat\nmkdir-path /compat/orbit\nmkdir-path /saves\nmkdir-path /saves/orbit\nmkdir-path /cache\nmkdir-path /cache/orbit\nmkfile-path /compat/orbit/session.env\nwrite-file /compat/orbit/session.env NGOS_GAME_CHANNEL=/compat/orbit/session.chan\nmkfile-path /compat/orbit/session.argv\nwrite-file /compat/orbit/session.argv /bin/worker\nmkchan-path /compat/orbit/session.chan\ncd /games/orbit\nspawn-path game-orbit-runner /bin/worker\nkill $LAST_PID 15\nreap $LAST_PID\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains("process-spawned pid="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("signal-sent pid="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("process-reaped pid="),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_can_prepare_game_resources_then_spawn_and_reap_worker() {
        let report = report_for_script(
            "mkdir-path /games\nmkdir-path /games/orbit\nmkdir-path /compat\nmkdir-path /compat/orbit\nmkdir-path /saves\nmkdir-path /saves/orbit\nmkdir-path /cache\nmkdir-path /cache/orbit\nmkfile-path /compat/orbit/session.env\nwrite-file /compat/orbit/session.env NGOS_GAME_CHANNEL=/compat/orbit/session.chan\nmkfile-path /compat/orbit/session.argv\nwrite-file /compat/orbit/session.argv /bin/worker\nmkchan-path /compat/orbit/session.chan\nmkdomain compat-game-orbit-runner\nmkresource 2 surface orbit-runner-gfx\nresource-governance 2 exclusive-lease\nresource-contract-policy 2 display\nresource-issuer-policy 2 creator-only\nmkcontract 2 2 display frame-pace-display\ncontract-state 2 active\nclaim 2\nmkresource 2 channel orbit-runner-audio\nresource-governance 3 queueing\nresource-contract-policy 3 io\nresource-issuer-policy 3 creator-only\nmkcontract 2 3 io spatial-mix-mix\ncontract-state 3 active\nclaim 3\nmkresource 2 device orbit-runner-input\nresource-governance 4 exclusive-lease\nresource-contract-policy 4 observe\nresource-issuer-policy 4 creator-only\nmkcontract 2 4 observe gamepad-first-capture\ncontract-state 4 active\nclaim 4\ncd /games/orbit\nspawn-path game-orbit-runner /bin/worker\nkill $LAST_PID 15\nreap $LAST_PID\nreleaseclaim 2\nreleaseclaim 3\nreleaseclaim 4\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains("process-spawned pid="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("process-reaped pid="),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_can_cleanup_game_resources_after_worker_reap() {
        let report = report_for_script(
            "mkdir-path /games\nmkdir-path /games/orbit\nmkdir-path /compat\nmkdir-path /compat/orbit\nmkdir-path /saves\nmkdir-path /saves/orbit\nmkdir-path /cache\nmkdir-path /cache/orbit\nmkfile-path /compat/orbit/session.env\nwrite-file /compat/orbit/session.env NGOS_GAME_CHANNEL=/compat/orbit/session.chan\nmkfile-path /compat/orbit/session.argv\nwrite-file /compat/orbit/session.argv /bin/worker\nmkchan-path /compat/orbit/session.chan\nmkdomain compat-game-orbit-runner\nmkresource 2 surface orbit-runner-gfx\nresource-policy 2 fifo\nresource-governance 2 exclusive-lease\nresource-contract-policy 2 display\nresource-issuer-policy 2 creator-only\nmkcontract 2 2 display frame-pace-display\ncontract-state 2 active\nclaim 2\nmkresource 2 channel orbit-runner-audio\nresource-policy 3 fifo\nresource-governance 3 queueing\nresource-contract-policy 3 io\nresource-issuer-policy 3 creator-only\nmkcontract 2 3 io spatial-mix-mix\ncontract-state 3 active\nclaim 3\nmkresource 2 device orbit-runner-input\nresource-policy 4 fifo\nresource-governance 4 exclusive-lease\nresource-contract-policy 4 observe\nresource-issuer-policy 4 creator-only\nmkcontract 2 4 observe gamepad-first-capture\ncontract-state 4 active\nclaim 4\ncd /games/orbit\nspawn-path game-orbit-runner /bin/worker\nkill $LAST_PID 15\nreap $LAST_PID\nreleaseclaim 2\nreleaseclaim 3\nreleaseclaim 4\ncontract-state 2 suspended\ncontract-state 3 suspended\ncontract-state 4 suspended\nresource-state 2 suspended\nresource-state 3 suspended\nresource-state 4 suspended\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains("process-reaped pid="),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("resource-state-updated id=4 state=suspended"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn host_runtime_can_exit_shell_process_after_game_worker_reap_and_cleanup() {
        let (user_runtime, app) = configured_game_user_runtime();
        let ((gfx, gfx_contract), (audio, audio_contract), (input, input_contract)) =
            prepare_game_session_resources(&user_runtime);

        let pid = user_runtime
            .spawn_configured_process(
                "game-orbit-runner",
                "/bin/worker",
                "/games/orbit",
                &["/bin/worker"],
                &["NGOS_GAME_CHANNEL=/compat/orbit/session.chan"],
            )
            .unwrap();
        user_runtime.send_signal(pid, 15).unwrap();
        let exit_code = user_runtime.reap_process(pid).unwrap();
        cleanup_game_session_resources(
            &user_runtime,
            &[
                (gfx, gfx_contract),
                (audio, audio_contract),
                (input, input_contract),
            ],
        );

        let mut runtime = user_runtime.backend().runtime_mut();
        runtime.exit(app, 0).unwrap();

        assert_eq!(pid, 3);
        assert_eq!(exit_code, 143);
        assert_eq!(
            runtime.process_info(app).unwrap().state,
            kernel_core::ProcessState::Exited
        );
    }

    #[test]
    fn native_session_report_can_launch_and_stop_game_session_by_last_pid() {
        let report = report_for_script(
            "mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-launch /games/orbit.manifest\ngame-stop $LAST_PID\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains("game.session pid="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("stopped=true"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_can_render_game_manifest_without_launching() {
        let report = report_for_script(
            "mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-manifest /games/orbit.manifest\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains(
                "game.manifest path=/games/orbit.manifest title=Orbit Runner slug=orbit-runner"
            ),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_can_render_game_plan_without_launching() {
        let report = report_for_script(
            "mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-plan /games/orbit.manifest\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("game.plan domain=compat-game-orbit-runner process=game-orbit-runner cwd=/games/orbit exec=/bin/worker"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_can_append_and_read_manifest_file_text() {
        let report = report_for_script(
            "mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\ncat-file /games/orbit.manifest\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("file-written path=/games/orbit.manifest bytes=36"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("append-line-ok path=/games/orbit.manifest line=2"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("title=Orbit Runner"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("slug=orbit-runner"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_completes_process_and_procfs_commands() {
        let report = report_for_script(
            "session\nps\nprocess-info 2\nproc 2 maps\ncat /proc/2/status\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0);
        assert!(report.stdout.contains("ngos shell"));
        assert!(report.stdout.contains("pid=2 name=ngos-userland-native"));
        assert!(report.stdout.contains("SchedulerClass:\t"));
    }

    #[test]
    fn native_session_report_completes_ps_command() {
        let report = report_for_script("ps\nexit 0\n");

        assert_eq!(report.exit_code, 0);
        assert!(report.stdout.contains("pid=2 name=ngos-userland-native"));
    }

    #[test]
    fn native_session_report_exposes_gpu_binding_evidence_for_initialized_nvidia_provider() {
        let report =
            report_for_script_with_runtime("gpu-evidence /dev/gpu0\nexit 0\n", |runtime| {
                let (mut platform, locator) = sample_nvidia_gpu_platform();
                platform.setup_gpu_agent(locator).unwrap();
                runtime.install_hardware_provider(Box::new(platform));
            });

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(report.stdout.contains("gpu-binding device=/dev/gpu0"));
        assert!(report.stdout.contains("architecture=Blackwell"));
        assert!(report.stdout.contains("product=NVIDIA GeForce RTX 5060 Ti"));
        assert!(report.stdout.contains("die=GB206"));
        assert!(report.stdout.contains("bus-interface=PCIe x8 5.0 @ x8 4.0"));
        assert!(report.stdout.contains("inf-section=Section048"));
        assert!(report.stdout.contains("kernel-service=nvlddmkm"));
        assert!(report.stdout.contains("vbios=98.06.1f.00.dc"));
        assert!(report.stdout.contains("part=2D04-300-A1"));
        assert!(report.stdout.contains("resizable-bar=1"));
        assert!(report.stdout.contains("display-engine-confirmed=0"));
        assert!(report.stdout.contains("msi-source=nv_msiSupport_addreg"));
        assert!(report.stdout.contains("msi-limit=1"));
    }

    #[test]
    fn native_session_report_exposes_gpu_vbios_window_and_header_for_initialized_nvidia_provider() {
        let report = report_for_script_with_runtime("gpu-vbios /dev/gpu0\nexit 0\n", |runtime| {
            let (mut platform, locator) = sample_nvidia_gpu_platform();
            platform.setup_gpu_agent(locator).unwrap();
            runtime.install_hardware_provider(Box::new(platform));
        });

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(report.stdout.contains("gpu-vbios device=/dev/gpu0"));
        assert!(report.stdout.contains("enabled=1"));
        assert!(report.stdout.contains("rom-bar=0x00c00001"));
        assert!(report.stdout.contains("physical-base=0xc00000"));
        assert!(report.stdout.contains("vendor=0x10de"));
        assert!(report.stdout.contains("device=0x2d04"));
        assert!(report.stdout.contains("pcir=0x"));
        assert!(report.stdout.contains("board=NVIDIA GeForce RTX 5060 Ti"));
        assert!(report.stdout.contains("code=P14N:506T301FB"));
        assert!(report.stdout.contains("version=Version 98.06.1F.00.DC"));
        assert!(report.stdout.contains("header=55:aa:4e:56:49:44:49:41"));
    }

    #[test]
    fn native_session_report_reports_gpu_vbios_unavailable_without_initialized_provider() {
        let report = report_for_script("gpu-vbios /dev/gpu0\nexit 0\n");

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-vbios device=/dev/gpu0 status=unavailable")
        );
    }

    #[test]
    fn native_session_report_exposes_gpu_gsp_loopback_status_for_initialized_nvidia_provider() {
        let report = report_for_script_with_runtime(
            "gpu-gsp /dev/gpu0\ngpu-buffer-create 32\ngpu-buffer-write 1 0 draw:hardware\ngpu-submit-buffer /dev/gpu0 1\ngpu-gsp /dev/gpu0\nexit 0\n",
            |runtime| {
                let (mut platform, locator) = sample_nvidia_gpu_platform();
                platform.setup_gpu_agent(locator).unwrap();
                runtime.install_hardware_provider(Box::new(platform));
            },
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(report.stdout.contains("gpu-gsp device=/dev/gpu0 ready=1 completions=0 failures=0 firmware-known=0 firmware-version=N/A blackwell-blob=0"));
        assert!(report.stdout.contains("blobs=gsp_ga10x.bin,gsp_tu10x.bin"));
        assert!(report.stdout.contains("gpu-gsp device=/dev/gpu0 ready=1 completions=1 failures=0 firmware-known=0 firmware-version=N/A blackwell-blob=0"));
    }

    #[test]
    fn native_session_report_reports_gpu_gsp_unavailable_without_initialized_provider() {
        let report = report_for_script("gpu-gsp /dev/gpu0\nexit 0\n");

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-gsp device=/dev/gpu0 status=unavailable")
        );
    }

    #[test]
    fn native_session_report_exposes_gpu_interrupt_delivery_for_initialized_nvidia_provider() {
        let report = report_for_script_with_runtime(
            "gpu-irq /dev/gpu0\ngpu-buffer-create 32\ngpu-buffer-write 1 0 draw:hardware\ngpu-submit-buffer /dev/gpu0 1\ngpu-irq /dev/gpu0\nexit 0\n",
            |runtime| {
                let (mut platform, locator) = sample_nvidia_gpu_platform();
                platform.setup_gpu_agent(locator).unwrap();
                runtime.install_hardware_provider(Box::new(platform));
            },
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(report.stdout.contains("gpu-irq device=/dev/gpu0"));
        assert!(report.stdout.contains("delivered=1"));
        assert!(report.stdout.contains("msi-supported=1"));
        assert!(report.stdout.contains("message-limit=1"));
        assert!(report.stdout.contains("windows-max=9"));
        assert!(report.stdout.contains("hardware-confirmed=0"));
    }

    #[test]
    fn native_session_report_reports_gpu_interrupt_unavailable_without_initialized_provider() {
        let report = report_for_script("gpu-irq /dev/gpu0\nexit 0\n");

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-irq device=/dev/gpu0 status=unavailable")
        );
    }

    #[test]
    fn native_session_report_exposes_gpu_display_plan_for_initialized_nvidia_provider() {
        let report = report_for_script_with_runtime(
            "gpu-display /dev/gpu0\ngpu-buffer-create 32\ngpu-buffer-write 1 0 draw:hardware\ngpu-submit-buffer /dev/gpu0 1\ngpu-display /dev/gpu0\nexit 0\n",
            |runtime| {
                let (mut platform, locator) = sample_nvidia_gpu_platform();
                platform.setup_gpu_agent(locator).unwrap();
                runtime.install_hardware_provider(Box::new(platform));
            },
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-display device=/dev/gpu0 pipes=1 planned=0")
        );
        assert!(
            report
                .stdout
                .contains("gpu-display device=/dev/gpu0 pipes=1 planned=1")
        );
        assert!(report.stdout.contains("hardware-confirmed=0"));
    }

    #[test]
    fn native_session_report_reports_gpu_display_unavailable_without_initialized_provider() {
        let report = report_for_script("gpu-display /dev/gpu0\nexit 0\n");

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-display device=/dev/gpu0 status=unavailable")
        );
    }

    #[test]
    fn native_session_report_exposes_gpu_display_without_submit_for_initialized_nvidia_provider() {
        let report = report_for_script_with_runtime("gpu-display /dev/gpu0\nexit 0\n", |runtime| {
            let (mut platform, locator) = sample_nvidia_gpu_platform();
            platform.setup_gpu_agent(locator).unwrap();
            runtime.install_hardware_provider(Box::new(platform));
        });

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-display device=/dev/gpu0 pipes=1 planned=0")
        );
        assert!(report.stdout.contains("hardware-confirmed=0"));
    }

    #[test]
    fn native_session_report_exposes_gpu_power_evidence_for_initialized_nvidia_provider() {
        let report = report_for_script_with_runtime("gpu-power /dev/gpu0\nexit 0\n", |runtime| {
            let (mut platform, locator) = sample_nvidia_gpu_platform();
            platform.setup_gpu_agent(locator).unwrap();
            runtime.install_hardware_provider(Box::new(platform));
        });

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(report.stdout.contains("gpu-power device=/dev/gpu0"));
        assert!(report.stdout.contains("pstate=P8"));
        assert!(report.stdout.contains("graphics-mhz="));
        assert!(report.stdout.contains("memory-mhz="));
        assert!(report.stdout.contains("boost-mhz="));
        assert!(report.stdout.contains("hardware-confirmed=0"));
    }

    #[test]
    fn native_session_report_reports_gpu_power_unavailable_without_initialized_provider() {
        let report = report_for_script("gpu-power /dev/gpu0\nexit 0\n");

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-power device=/dev/gpu0 status=unavailable")
        );
    }

    #[test]
    fn native_session_report_sets_gpu_power_state_for_initialized_nvidia_provider() {
        let report = report_for_script_with_runtime(
            "gpu-power /dev/gpu0\ngpu-power-set /dev/gpu0 P0\ngpu-power /dev/gpu0\nexit 0\n",
            |runtime| {
                let (mut platform, locator) = sample_nvidia_gpu_platform();
                platform.setup_gpu_agent(locator).unwrap();
                runtime.install_hardware_provider(Box::new(platform));
            },
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-power device=/dev/gpu0 pstate=P8")
        );
        assert!(report.stdout.contains("graphics-mhz=1200"));
        assert!(report.stdout.contains("memory-mhz=900"));
        assert!(report.stdout.contains("boost-mhz=1500"));
        assert!(
            report
                .stdout
                .contains("gpu-power-set device=/dev/gpu0 state=P0 status=ok")
        );
        assert!(
            report
                .stdout
                .contains("gpu-power device=/dev/gpu0 pstate=P0")
        );
        assert!(report.stdout.contains("graphics-mhz=2407"));
        assert!(report.stdout.contains("memory-mhz=1750"));
        assert!(report.stdout.contains("boost-mhz=2602"));
    }

    #[test]
    fn native_session_report_refuses_invalid_gpu_power_state_requests() {
        let report = report_for_script_with_runtime(
            "gpu-power-set /dev/gpu0 P3\ngpu-power /dev/gpu0\nexit 0\n",
            |runtime| {
                let (mut platform, locator) = sample_nvidia_gpu_platform();
                platform.setup_gpu_agent(locator).unwrap();
                runtime.install_hardware_provider(Box::new(platform));
            },
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-power-set device=/dev/gpu0 state=P3 status=invalid")
        );
        assert!(
            report
                .stdout
                .contains("gpu-power device=/dev/gpu0 pstate=P8")
        );
    }

    #[test]
    fn native_session_report_restores_gpu_power_state_after_multiple_transitions() {
        let report = report_for_script_with_runtime(
            "gpu-power /dev/gpu0\ngpu-power-set /dev/gpu0 P0\ngpu-power /dev/gpu0\ngpu-power-set /dev/gpu0 P12\ngpu-power /dev/gpu0\nexit 0\n",
            |runtime| {
                let (mut platform, locator) = sample_nvidia_gpu_platform();
                platform.setup_gpu_agent(locator).unwrap();
                runtime.install_hardware_provider(Box::new(platform));
            },
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-power device=/dev/gpu0 pstate=P8")
        );
        assert!(report.stdout.contains("graphics-mhz=1200"));
        assert!(report.stdout.contains("memory-mhz=900"));
        assert!(report.stdout.contains("boost-mhz=1500"));
        assert!(
            report
                .stdout
                .contains("gpu-power-set device=/dev/gpu0 state=P0 status=ok")
        );
        assert!(
            report
                .stdout
                .contains("gpu-power device=/dev/gpu0 pstate=P0")
        );
        assert!(report.stdout.contains("graphics-mhz=2407"));
        assert!(report.stdout.contains("memory-mhz=1750"));
        assert!(report.stdout.contains("boost-mhz=2602"));
        assert!(
            report
                .stdout
                .contains("gpu-power-set device=/dev/gpu0 state=P12 status=ok")
        );
        assert!(
            report
                .stdout
                .contains("gpu-power device=/dev/gpu0 pstate=P12")
        );
        assert!(report.stdout.contains("graphics-mhz=300"));
        assert!(report.stdout.contains("memory-mhz=405"));
        assert!(report.stdout.contains("boost-mhz=600"));
    }

    #[test]
    fn native_session_report_exposes_gpu_media_state_after_start_for_initialized_nvidia_provider() {
        let report = report_for_script_with_runtime(
            "gpu-media /dev/gpu0\ngpu-media-start /dev/gpu0 1920 1080 12000 av1\ngpu-media /dev/gpu0\nexit 0\n",
            |runtime| {
                let (mut platform, locator) = sample_nvidia_gpu_platform();
                platform.setup_gpu_agent(locator).unwrap();
                runtime.install_hardware_provider(Box::new(platform));
            },
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(report.stdout.contains("gpu-media device=/dev/gpu0 sessions=0 codec=0 width=0 height=0 bitrate-kbps=0 hardware-confirmed=0"));
        assert!(report.stdout.contains("gpu-media-start device=/dev/gpu0 width=1920 height=1080 bitrate-kbps=12000 codec=av1 status=ok"));
        assert!(report.stdout.contains("gpu-media device=/dev/gpu0 sessions=1 codec=2 width=1920 height=1080 bitrate-kbps=12000 hardware-confirmed=0"));
    }

    #[test]
    fn native_session_report_refuses_invalid_gpu_media_requests() {
        let report = report_for_script_with_runtime(
            "gpu-media-start /dev/gpu0 0 1080 12000 av1\ngpu-media-start /dev/gpu0 1920 1080 12000 vp9\ngpu-media /dev/gpu0\nexit 0\n",
            |runtime| {
                let (mut platform, locator) = sample_nvidia_gpu_platform();
                platform.setup_gpu_agent(locator).unwrap();
                runtime.install_hardware_provider(Box::new(platform));
            },
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert_eq!(
            report
                .stdout
                .matches("gpu-media-start device=/dev/gpu0 status=invalid")
                .count(),
            2
        );
        assert!(report.stdout.contains("gpu-media device=/dev/gpu0 sessions=0 codec=0 width=0 height=0 bitrate-kbps=0 hardware-confirmed=0"));
    }

    #[test]
    fn native_session_report_exposes_gpu_neural_state_after_inject_and_commit() {
        let report = report_for_script_with_runtime(
            "gpu-neural /dev/gpu0\ngpu-neural-inject /dev/gpu0 enemy vehicle\ngpu-neural /dev/gpu0\ngpu-neural-commit /dev/gpu0\ngpu-neural /dev/gpu0\nexit 0\n",
            |runtime| {
                let (mut platform, locator) = sample_nvidia_gpu_platform();
                platform.setup_gpu_agent(locator).unwrap();
                runtime.install_hardware_provider(Box::new(platform));
            },
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(report.stdout.contains("gpu-neural device=/dev/gpu0 model-loaded=0 semantics=0 committed=0 hardware-confirmed=0"));
        assert!(
            report
                .stdout
                .contains("gpu-neural-inject device=/dev/gpu0 semantic=enemy vehicle status=ok")
        );
        assert!(report.stdout.contains("gpu-neural device=/dev/gpu0 model-loaded=1 semantics=1 committed=0 hardware-confirmed=0"));
        assert!(
            report
                .stdout
                .contains("gpu-neural-commit device=/dev/gpu0 status=ok")
        );
        assert!(report.stdout.contains("gpu-neural device=/dev/gpu0 model-loaded=1 semantics=1 committed=1 hardware-confirmed=0"));
    }

    #[test]
    fn native_session_report_refuses_invalid_gpu_neural_requests() {
        let report = report_for_script_with_runtime(
            "gpu-neural-inject /dev/gpu0   \ngpu-neural /dev/gpu0\nexit 0\n",
            |runtime| {
                let (mut platform, locator) = sample_nvidia_gpu_platform();
                platform.setup_gpu_agent(locator).unwrap();
                runtime.install_hardware_provider(Box::new(platform));
            },
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-neural-inject device=/dev/gpu0 status=invalid")
        );
        assert!(report.stdout.contains("gpu-neural device=/dev/gpu0 model-loaded=0 semantics=0 committed=0 hardware-confirmed=0"));
    }

    #[test]
    fn native_session_report_exposes_gpu_tensor_state_after_dispatch() {
        let report = report_for_script_with_runtime(
            "gpu-tensor /dev/gpu0\ngpu-tensor-dispatch /dev/gpu0 77\ngpu-tensor /dev/gpu0\nexit 0\n",
            |runtime| {
                let (mut platform, locator) = sample_nvidia_gpu_platform();
                platform.setup_gpu_agent(locator).unwrap();
                runtime.install_hardware_provider(Box::new(platform));
            },
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(report.stdout.contains(
            "gpu-tensor device=/dev/gpu0 active-jobs=0 last-kernel=0 hardware-confirmed=0"
        ));
        assert!(
            report
                .stdout
                .contains("gpu-tensor-dispatch device=/dev/gpu0 kernel=77 status=ok")
        );
        assert!(report.stdout.contains(
            "gpu-tensor device=/dev/gpu0 active-jobs=1 last-kernel=77 hardware-confirmed=0"
        ));
    }

    #[test]
    fn native_session_report_refuses_invalid_gpu_tensor_requests() {
        let report = report_for_script_with_runtime(
            "gpu-tensor-dispatch /dev/gpu0 0\ngpu-tensor /dev/gpu0\nexit 0\n",
            |runtime| {
                let (mut platform, locator) = sample_nvidia_gpu_platform();
                platform.setup_gpu_agent(locator).unwrap();
                runtime.install_hardware_provider(Box::new(platform));
            },
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-tensor-dispatch device=/dev/gpu0 status=invalid")
        );
        assert!(report.stdout.contains(
            "gpu-tensor device=/dev/gpu0 active-jobs=0 last-kernel=0 hardware-confirmed=0"
        ));
    }

    #[test]
    fn native_session_report_completes_vm_reclaim_and_heap_flow() {
        let report = report_for_script(
            "proc 2 vmobjects\nvm-load-word 2 $VM_FILE_ADDR\nvm-advise 2 $VM_FILE_ADDR 4096 dontneed\nproc 2 vmobjects\nvm-load-word 2 $VM_FILE_ADDR\nproc 2 vmobjects\nproc 2 maps\nvm-brk 2 $VM_HEAP_GROW\nproc 2 maps\nvm-brk 2 $VM_HEAP_SHRINK\nproc 2 maps\nvm-probe-brk 2 $VM_HEAP_INVALID\nproc 2 vmepisodes\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(report.stdout.contains("vm-load-word pid=2 addr="));
        assert!(report.stdout.contains("vm-advise pid=2 addr="));
        assert!(report.stdout.contains("vm-brk pid=2 end="));
        assert!(report.stdout.contains("vm-probe-brk pid=2 requested="));
        assert!(report.stdout.contains("outcome=error"));
        assert!(report.stdout.contains("[heap]"));
        assert!(report.stdout.contains("resident=0"));
        assert!(report.vm_episodes.entries.iter().flatten().any(|entry| {
            matches!(entry.kind, crate::report::VmEpisodeKind::HeapPath)
                && entry.grew
                && entry.shrank
        }));
        assert!(report.vm_episodes.entries.iter().flatten().any(|entry| {
            matches!(entry.kind, crate::report::VmEpisodeKind::ReclaimPath)
                && entry.evicted
                && entry.restored
        }));
    }

    #[test]
    fn native_session_report_completes_vm_map_anon_flow() {
        let report = report_for_script(
            "vm-probe-map-anon 2 4096 rw- shell-map-gap\nproc 2 maps\nvm-probe-map-anon 2 0 rw- shell-map-gap-invalid\nproc 2 vmepisodes\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains("vm-probe-map-anon pid=2 start="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("outcome=mapped label=shell-map-gap"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("shell-map-gap"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("vm-probe-map-anon pid=2 len=0 outcome=error errno=Inval label=shell-map-gap-invalid"),
            "{}",
            report.render()
        );
        assert!(
            report
                .vm_agents
                .entries
                .iter()
                .flatten()
                .any(|entry| { matches!(entry.agent, kernel_core::VmAgentKind::MapAgent) })
        );
        assert!(report.vm_episodes.entries.iter().flatten().any(|entry| {
            matches!(entry.kind, crate::report::VmEpisodeKind::MapPath) && entry.mapped_kind == 0
        }));
    }

    #[test]
    fn native_session_report_completes_vm_shadow_chain_and_file_persistence_flow() {
        let report = report_for_script(
            "echo shared-value=$VM_SHARED_VALUE\necho shared-restored=$VM_SHARED_RESTORED\necho grandchild-depth=$VM_PROBE_GRANDCHILD_DEPTH\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains("shared-value=287454020"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("shared-restored=287454020"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("grandchild-depth=2"),
            "{}",
            report.render()
        );
        assert!(report.vm_episodes.entries.iter().flatten().any(|entry| {
            matches!(entry.kind, crate::report::VmEpisodeKind::FaultPath) && entry.cow
        }));
        assert!(report.vm_episodes.entries.iter().flatten().any(|entry| {
            matches!(entry.kind, crate::report::VmEpisodeKind::ReclaimPath)
                && entry.evicted
                && entry.restored
        }));
    }

    #[test]
    fn native_session_report_completes_vm_live_shared_multi_process_flow() {
        let report = report_for_script(
            "echo shared-live-value=$VM_SHARED_LIVE_VALUE\necho shared-live-owners=$VM_SHARED_LIVE_OWNERS\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains("shared-live-value=1432778632"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("shared-live-owners=3"),
            "{}",
            report.render()
        );
        assert!(report.vm_episodes.entries.iter().flatten().any(|entry| {
            matches!(entry.kind, crate::report::VmEpisodeKind::FaultPath)
                && !entry.cow
                && entry.synced
        }));
    }

    #[test]
    fn native_session_report_completes_vm_advanced_fault_and_cow_flow() {
        let report = report_for_script(
            "echo cow-shadow-before=$VM_COW_SHADOW_BEFORE\necho cow-shadow-after=$VM_COW_SHADOW_AFTER\necho cow-faults=$VM_COW_COW_FAULTS\necho split-read-faults=$VM_SPLIT_READ_FAULTS\necho split-write-faults=$VM_SPLIT_WRITE_FAULTS\necho split-total-faults=$VM_SPLIT_TOTAL_FAULTS\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains("cow-shadow-before=0"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("cow-shadow-after=1"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("cow-faults=1"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("split-read-faults=2"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("split-write-faults=1"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("split-total-faults=3"),
            "{}",
            report.render()
        );
        assert!(report.vm_episodes.entries.iter().flatten().any(|entry| {
            matches!(entry.kind, crate::report::VmEpisodeKind::FaultPath) && entry.cow
        }));
    }

    #[test]
    fn native_session_report_completes_vm_layout_and_region_flow() {
        let report = report_for_script(
            "echo offset-segments=$VM_OFFSET_SEGMENTS\necho offset-first=$VM_OFFSET_FIRST\necho offset-second=$VM_OFFSET_SECOND\necho read-resident=$VM_READ_RESIDENT\necho read-dirty=$VM_READ_DIRTY\necho read-accessed=$VM_READ_ACCESSED\necho read-readfaults=$VM_READ_READFAULTS\necho read-writefaults=$VM_READ_WRITEFAULTS\necho mprotect-faults=$VM_MPROTECT_FAULTS\necho mprotect-dirty=$VM_MPROTECT_DIRTY\necho range-split-count=$VM_RANGE_SPLIT_COUNT\necho range-coalesced-count=$VM_RANGE_COALESCED_COUNT\necho range-dirty-after-sync=$VM_RANGE_DIRTY_AFTER_SYNC\necho range-faults=$VM_RANGE_FAULTS\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains("offset-segments=2"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("offset-first=12288"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("offset-second=16384"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("read-resident=1"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("read-dirty=0"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("read-accessed=1"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("read-readfaults=1"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("read-writefaults=0"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("mprotect-faults=0"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("mprotect-dirty=0"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("range-split-count=3"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("range-coalesced-count=1"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("range-dirty-after-sync=0"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("range-faults=1"),
            "{}",
            report.render()
        );
        assert!(report.vm_episodes.entries.iter().flatten().any(|entry| {
            matches!(entry.kind, crate::report::VmEpisodeKind::RegionPath) && entry.protected
        }));
        assert!(report.vm_episodes.entries.iter().flatten().any(|entry| {
            matches!(entry.kind, crate::report::VmEpisodeKind::FaultPath)
                && !entry.cow
                && entry.touched
                && entry.synced
        }));
        assert!(report.vm_episodes.entries.iter().flatten().any(|entry| {
            matches!(entry.kind, crate::report::VmEpisodeKind::ReclaimPath) && entry.advised
        }));
    }

    #[test]
    fn native_session_report_completes_vm_memory_contract_gate_flow() {
        let report = report_for_script(
            "echo vm-contract-pid=$VM_CONTRACT_PID\necho vm-contract-id=$VM_CONTRACT_ID\necho vm-contract-allowed-map=$VM_CONTRACT_ALLOWED_MAP\necho vm-contract-blocked-state=$VM_CONTRACT_BLOCKED_STATE\nprocess-info $VM_CONTRACT_PID\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains("vm-contract-pid="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("vm-contract-id="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("vm-contract-allowed-map="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("vm-contract-blocked-state=1"),
            "{}",
            report.render()
        );
        assert!(
            report
                .resource_agents
                .entries
                .iter()
                .flatten()
                .any(|entry| {
                    matches!(
                        entry.agent,
                        kernel_core::ResourceAgentKind::ContractStateTransitionAgent
                    )
                })
        );
        assert!(report.vm_agents.entries.iter().flatten().any(|entry| {
            matches!(
                entry.agent,
                kernel_core::VmAgentKind::MapAgent | kernel_core::VmAgentKind::PolicyBlockAgent
            )
        }));
        assert!(report.vm_episodes.entries.iter().flatten().any(|entry| {
            matches!(entry.kind, crate::report::VmEpisodeKind::PolicyPath)
                && entry.policy_blocked
                && entry.policy_state == 1
        }));
    }

    #[test]
    fn native_session_report_completes_vm_pressure_flow() {
        let report = report_for_script(
            "proc 2 vmobjects\nvm-pressure 2 2\nproc 2 vmobjects\nvm-load-word 2 $VM_PRESSURE_A\nproc 2 vmobjects\nproc 2 vmdecisions\nproc 2 vmepisodes\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("vm-pressure pid=2 target-pages=2 reclaimed-pages="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("vm-load-word pid=2 addr="),
            "{}",
            report.render()
        );
        assert!(
            report
                .vm_agents
                .entries
                .iter()
                .flatten()
                .any(|entry| matches!(entry.agent, kernel_core::VmAgentKind::PressureVictimAgent)),
            "{}",
            report.render()
        );
        assert!(report.vm_episodes.entries.iter().flatten().any(|entry| {
            matches!(entry.kind, crate::report::VmEpisodeKind::ReclaimPath)
                && entry.evicted
                && entry.restored
        }));
    }

    #[test]
    fn native_session_report_completes_vm_composed_policy_pressure_cow_flow() {
        let report = report_for_script(
            "echo pressure-global-reclaimed=$VM_PRESSURE_GLOBAL_RECLAIMED\necho pressure-global-victims=$VM_PRESSURE_GLOBAL_VICTIMS\necho pressure-global-policy-blocks=$VM_PRESSURE_GLOBAL_POLICY_BLOCKS\necho pressure-global-cow-events=$VM_PRESSURE_GLOBAL_COW_EVENTS\necho shared-live-restored=$VM_SHARED_LIVE_RESTORED\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains("pressure-global-reclaimed="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("pressure-global-victims="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("pressure-global-policy-blocks=1"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("pressure-global-cow-events="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("shared-live-restored=1432778632"),
            "{}",
            report.render()
        );
        assert!(
            report
                .vm_agents
                .entries
                .iter()
                .flatten()
                .any(|entry| matches!(entry.agent, kernel_core::VmAgentKind::PolicyBlockAgent)),
            "{}",
            report.render()
        );
        assert!(
            report
                .vm_agents
                .entries
                .iter()
                .flatten()
                .any(|entry| matches!(entry.agent, kernel_core::VmAgentKind::PressureVictimAgent)),
            "{}",
            report.render()
        );
        assert!(report.vm_episodes.entries.iter().flatten().any(|entry| {
            matches!(entry.kind, crate::report::VmEpisodeKind::FaultPath) && entry.cow
        }));
        assert!(report.vm_episodes.entries.iter().flatten().any(|entry| {
            matches!(entry.kind, crate::report::VmEpisodeKind::PolicyPath)
                && entry.policy_blocked
                && entry.policy_state == 1
        }));
        assert!(report.vm_episodes.entries.iter().flatten().any(|entry| {
            matches!(entry.kind, crate::report::VmEpisodeKind::ReclaimPath)
                && entry.evicted
                && entry.restored
        }));
    }

    #[test]
    fn native_session_report_completes_process_info_command() {
        let report = report_for_script("process-info 2\nexit 0\n");

        assert_eq!(report.exit_code, 0);
        assert!(report.stdout.contains("pid=2 name=ngos-userland-native"));
    }

    #[test]
    fn native_session_report_completes_procfs_cat_command() {
        let report = report_for_script("cat /proc/2/status\nexit 0\n");

        assert_eq!(report.exit_code, 0);
        assert!(report.stdout.contains("SchedulerClass:\t"));
    }

    #[test]
    fn native_session_report_completes_file_and_vfs_commands() {
        let report = report_for_script(
            "open-path /etc/motd\nreadlink-path /motd\ncat-file /etc/motd\nmkdir-path /shell-tmp\ncd /shell-tmp\nmkfile-path note\nwrite-file note shell-note\nappend-file note -extra\ncat-file note\nlist-path .\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0);
        assert!(report.stdout.contains("opened path=/etc/motd"));
        assert!(report.stdout.contains("ngos host motd"));
        assert!(report.stdout.contains("directory-created path=/shell-tmp"));
    }

    #[test]
    fn native_session_report_renders_storage_device_and_driver_info() {
        let report = report_for_script("device /dev/storage0\ndriver /drv/storage0\nexit 0\n");

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(report.stdout.contains("device path=/dev/storage0"));
        assert!(report.stdout.contains("block-size=512"));
        assert!(report.stdout.contains("capacity-bytes=134217728"));
        assert!(report.stdout.contains("driver path=/drv/storage0"));
        assert!(report.stdout.contains("bound-devices=1"));
    }

    #[test]
    fn native_session_report_completes_storage_request_and_completion_flow() {
        let report = report_for_script(
            "blk-read /dev/storage0 0 1\npoll-path /drv/storage0 read\ndriver-read /drv/storage0\nwrite-file /drv/storage0 sector0:eb58904d5357494e\npoll-path /dev/storage0 read\ncat-file /dev/storage0\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("blk-read device=/dev/storage0 sector=0 sectors=1 block-size=512")
        );
        assert!(
            report
                .stdout
                .contains("poll path=/drv/storage0 interest=0x1 ready=0x1")
        );
        assert!(report.stdout.contains("request:"));
        assert!(report.stdout.contains("device=/dev/storage0"));
        assert!(report.stdout.contains(
            "block-request path=/drv/storage0 op=read sector=0 sectors=1 block-size=512"
        ));
        assert!(
            report
                .stdout
                .contains("file-written path=/drv/storage0 bytes=24")
        );
        assert!(
            report
                .stdout
                .contains("poll path=/dev/storage0 interest=0x1 ready=0x1")
        );
        assert!(report.stdout.contains("sector0:eb58904d5357494e"));
    }

    #[test]
    fn native_session_report_completes_networking_commands() {
        let report = report_for_script(
            "mksock-path /run/net0.sock\nnet-config /dev/net0 10.1.0.2 255.255.255.0 10.1.0.1\nnet-admin /dev/net0 1500 4 4 2 up promisc\nudp-bind /run/net0.sock /dev/net0 4000 0.0.0.0 0\nqueue-create kqueue\nnet-watch $LAST_QUEUE_FD /dev/net0 700 /run/net0.sock\nnet-sendto /run/net0.sock 10.1.0.9 5000 hello-host\nnet-sendto /run/net0.sock 10.1.0.10 5001 host-peer2\npoll-path /drv/net0 read\nnet-driver-read /drv/net0\nnet-driver-read /drv/net0\nnet-complete /drv/net0 2\nqueue-wait $LAST_QUEUE_FD\nnet-inject-udp /drv/net0 10.1.0.9 5000 10.1.0.99 4000 host-reply\nnet-inject-udp /drv/net0 10.1.0.10 5001 10.1.0.77 4000 host-peer2-reply\nqueue-wait $LAST_QUEUE_FD\npoll-path /run/net0.sock read\nnet-recvfrom /run/net0.sock\nnet-recvfrom /run/net0.sock\nnet-link /dev/net0 down\nqueue-wait $LAST_QUEUE_FD\nnet-unwatch $LAST_QUEUE_FD /dev/net0 700 /run/net0.sock\nnetif /dev/net0\nnetsock /run/net0.sock\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(report.stdout.contains("socket-created path=/run/net0.sock"));
        assert!(
            report
                .stdout
                .contains("netif-configured path=/dev/net0 addr=10.1.0.2")
        );
        assert!(report.stdout.contains("netif-admin path=/dev/net0"));
        assert!(
            report
                .stdout
                .contains("udp-bound socket=/run/net0.sock device=/dev/net0")
        );
        assert!(report.stdout.contains("queue-created fd="));
        assert!(report.stdout.contains("mode=kqueue"));
        assert!(report.stdout.contains("net-watch queue="));
        assert!(
            report
                .stdout
                .contains("device=/dev/net0 socket=/run/net0.sock token=700")
        );
        assert!(
            report
                .stdout
                .contains("net-sendto socket=/run/net0.sock remote=10.1.0.9:5000")
        );
        assert!(
            report
                .stdout
                .contains("net-sendto socket=/run/net0.sock remote=10.1.0.10:5001")
        );
        assert!(report.stdout.contains("queue-event queue="));
        assert!(report.stdout.contains("token=700"));
        assert!(report.stdout.contains("source=network"));
        assert!(
            report
                .stdout
                .contains("poll path=/drv/net0 interest=0x1 ready=0x1")
        );
        assert!(
            report
                .stdout
                .contains("net-complete driver=/drv/net0 completed=2")
        );
        assert!(
            report
                .stdout
                .contains("net-inject driver=/drv/net0 src=10.1.0.9:5000 dst=10.1.0.99:4000")
        );
        assert!(
            report
                .stdout
                .contains("poll path=/run/net0.sock interest=0x1 ready=0x1")
        );
        assert!(report.stdout.contains(
            "net-recvfrom socket=/run/net0.sock remote=10.1.0.9:5000 bytes=10 payload=host-reply"
        ));
        assert!(
            report
                .stdout
                .contains("net-recvfrom socket=/run/net0.sock remote=10.1.0.10:5001 bytes=16 payload=host-peer2-reply")
        );
        assert!(
            report
                .stdout
                .contains("netif-link path=/dev/net0 state=down")
        );
        assert!(report.stdout.contains("net-unwatch queue="));
        assert!(
            report
                .stdout
                .contains("device=/dev/net0 socket=/run/net0.sock token=700")
        );
        assert!(
            report
                .stdout
                .contains("netif path=/dev/net0 admin=up link=down promisc=on")
        );
        assert!(report.stdout.contains(
            "netsock path=/run/net0.sock local=10.1.0.2:4000 remote=0.0.0.0:0 connected=no"
        ));
    }

    #[test]
    fn native_session_report_detects_invoke_on_suspended_contract() {
        let report = report_for_script(
            "mkdomain render\nmkresource 2 device lease0\nmkcontract 2 1 display scanout\ncontract-state 1 suspended\ninvoke 1\n",
        );

        assert_eq!(report.exit_code, 208, "{}", report.render());
        assert!(report.stdout.contains("domain-created id=2 name=render"));
    }

    #[test]
    fn native_session_report_detects_invoke_after_resource_retire() {
        let report = report_for_script(
            "mkdomain render\nmkresource 2 device lease0\nmkcontract 2 1 display scanout\nresource-state 1 retired\ncontracts\ninvoke 1\n",
        );

        assert_eq!(report.exit_code, 208, "{}", report.render());
        assert!(report.stdout.contains("domain-created id=2 name=render"));
    }

    #[test]
    fn native_session_report_completes_gpu_submit_and_completion_flow() {
        let report = report_for_script(
            "device /dev/gpu0\ndriver /drv/gpu0\nmkdomain render\nmkresource 2 device gpu0\nresource-governance 2 exclusive-lease\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\ngpu-probe-submit /dev/gpu0 draw:triangle\nclaim 2\ngpu-submit /dev/gpu0 draw:triangle\ndriver /drv/gpu0\ngpu-driver-read /drv/gpu0\ngpu-complete /drv/gpu0 fence:7\ndevice /dev/gpu0\ngpu-read /dev/gpu0\nreleaseclaim 2\ngpu-probe-submit /dev/gpu0 draw:triangle\ncontracts\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("device path=/dev/gpu0 class=graphics")
        );
        assert!(
            report
                .stdout
                .contains("resource-created id=2 domain=2 kind=device name=gpu0")
        );
        assert!(
            report
                .stdout
                .contains("contract-created id=2 domain=2 resource=2 kind=display label=scanout")
        );
        assert!(
            report
                .stdout
                .contains("gpu-probe-submit device=/dev/gpu0 bytes=13 outcome=error")
        );
        assert!(
            report
                .stdout
                .contains("claim-acquired contract=2 resource=2")
        );
        assert!(
            report
                .stdout
                .contains("gpu-submit device=/dev/gpu0 bytes=13 payload=draw:triangle")
        );
        assert!(report.stdout.contains("gpu-driver-read driver=/drv/gpu0 outcome=request header=request:1 kind=Write device=/dev/gpu0 opcode=None payload=draw:triangle"));
        assert!(
            report
                .stdout
                .contains("gpu-complete driver=/drv/gpu0 bytes=7 payload=fence:7")
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=7 payload=fence:7")
        );
        assert!(
            report
                .stdout
                .contains("claim-released contract=2 resource=2")
        );
        assert_eq!(
            report
                .stdout
                .matches("gpu-probe-submit device=/dev/gpu0 bytes=13 outcome=error")
                .count(),
            2
        );
        assert!(
            report
                .stdout
                .contains("device path=/dev/gpu0 class=graphics state=1")
        );
        assert!(
            report
                .stdout
                .contains("driver path=/drv/gpu0 state=1 bound-devices=1")
        );
    }

    #[test]
    fn native_session_report_detects_gpu_submit_errors_for_unbound_and_empty_driver_paths() {
        let report = report_for_script(
            "gpu-probe-submit /dev/gpu-unbound draw:triangle\ngpu-driver-read /drv/gpu0\ngpu-probe-complete /drv/gpu0 fence:empty\ngpu-read /dev/gpu0\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-probe-submit device=/dev/gpu-unbound bytes=13 outcome=error")
        );
        assert!(
            report
                .stdout
                .contains("gpu-driver-read driver=/drv/gpu0 outcome=empty")
        );
        assert!(
            report
                .stdout
                .contains("gpu-probe-complete driver=/drv/gpu0 bytes=11 outcome=error")
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 outcome=empty")
        );
    }

    #[test]
    fn native_session_report_completes_gpu_present_flow() {
        let report = report_for_script(
            "device /dev/gpu0\ndriver /drv/gpu0\nmkdomain render\nmkresource 2 device gpu0\nresource-governance 2 exclusive-lease\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\ngpu-probe-present /dev/gpu0 frame:boot\nclaim 2\ngpu-present /dev/gpu0 frame:boot\ngpu-driver-read /drv/gpu0\ngpu-complete /drv/gpu0 present:ok\ngpu-read /dev/gpu0\nreleaseclaim 2\ngpu-probe-present /dev/gpu0 frame:boot\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(report.stdout.contains(
            "gpu-probe-present device=/dev/gpu0 opcode=0x47500001 outcome=error frame=frame:boot"
        ));
        assert!(
            report
                .stdout
                .contains("claim-acquired contract=2 resource=2")
        );
        assert!(report.stdout.contains(
            "gpu-present device=/dev/gpu0 opcode=0x47500001 response=0x47500000 frame=frame:boot"
        ));
        assert!(
            report
                .stdout
                .contains("gpu-driver-read driver=/drv/gpu0 outcome=request header=request:1 kind=Control device=/dev/gpu0 opcode=Some(")
        );
        assert!(
            report
                .stdout
                .contains("gpu-complete driver=/drv/gpu0 bytes=10 payload=present:ok")
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=10 payload=present:ok")
        );
        assert!(
            report
                .stdout
                .contains("claim-released contract=2 resource=2")
        );
        assert_eq!(
            report
                .stdout
                .matches("gpu-probe-present device=/dev/gpu0 opcode=0x47500001 outcome=error frame=frame:boot")
                .count(),
            2
        );
    }

    #[test]
    fn native_session_report_completes_gpu_readiness_flow() {
        let report = report_for_script(
            "mkdomain render\nmkresource 2 device gpu0\nresource-governance 2 exclusive-lease\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\nfd-watch /drv/gpu0 read\nfd-watch /dev/gpu0 read\nfd-ready\nclaim 2\ngpu-submit /dev/gpu0 draw:ready\nfd-ready\ngpu-driver-read /drv/gpu0\nfd-ready\ngpu-complete /drv/gpu0 fence:11\nfd-ready\ngpu-read /dev/gpu0\nfd-ready\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains("fd-watch fd="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("fd-ready count=0"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-submit device=/dev/gpu0 bytes=10 payload=draw:ready"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("fd-ready owner=2 fd="),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-driver-read driver=/drv/gpu0 outcome=request"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-complete driver=/drv/gpu0 bytes=8 payload=fence:11"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=8 payload=fence:11"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.matches("fd-ready count=0").count() >= 2,
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_completes_gpu_queue_backpressure_flow() {
        let report = report_for_script(
            "device /dev/gpu0\ndriver /drv/gpu0\nmkdomain render\nmkresource 2 device gpu0\nresource-governance 2 exclusive-lease\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\ngpu-queue-capacity /dev/gpu0 2\nclaim 2\ngpu-submit /dev/gpu0 draw:a\ngpu-present /dev/gpu0 frame:b\ndevice /dev/gpu0\ngpu-probe-submit /dev/gpu0 draw:c\ndriver /drv/gpu0\ngpu-driver-read /drv/gpu0\ngpu-complete /drv/gpu0 fence:a\ngpu-probe-submit /dev/gpu0 draw:c\ndriver /drv/gpu0\ngpu-driver-read /drv/gpu0\ngpu-complete /drv/gpu0 present:b\ngpu-driver-read /drv/gpu0\ngpu-complete /drv/gpu0 fence:c\ngpu-read /dev/gpu0\ngpu-read /dev/gpu0\ngpu-read /dev/gpu0\ndevice /dev/gpu0\ndriver /drv/gpu0\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-queue-capacity device=/dev/gpu0 queue-capacity=2"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "device path=/dev/gpu0 class=graphics state=1 queue-depth=2 queue-capacity=2"
            ),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-probe-submit device=/dev/gpu0 bytes=6 outcome=error"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "driver path=/drv/gpu0 state=1 bound-devices=1 queued=2 inflight=0 completed=0"
            ),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-driver-read driver=/drv/gpu0 outcome=request header=request:1 kind=Write device=/dev/gpu0 opcode=None payload=draw:a"
            ),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-probe-submit device=/dev/gpu0 bytes=6 outcome=submitted payload=draw:c"
            ),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-driver-read driver=/drv/gpu0 outcome=request header=request:2 kind=Control device=/dev/gpu0 opcode=Some("
            ),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-driver-read driver=/drv/gpu0 outcome=request header=request:3 kind=Write device=/dev/gpu0 opcode=None payload=draw:c"
            ),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=7 payload=fence:a"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=9 payload=present:b"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=7 payload=fence:c"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "device path=/dev/gpu0 class=graphics state=1 queue-depth=0 queue-capacity=2"
            ),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("submitted=3 completed=3"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "driver path=/drv/gpu0 state=1 bound-devices=1 queued=0 inflight=0 completed=3"
            ),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_prioritizes_gpu_present_over_queued_writes() {
        let report = report_for_script(
            "mkdomain render\nmkresource 2 device gpu0\nresource-governance 2 exclusive-lease\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\ngpu-queue-capacity /dev/gpu0 4\nclaim 2\ngpu-submit /dev/gpu0 draw:a\ngpu-submit /dev/gpu0 draw:b\ngpu-present /dev/gpu0 frame:p\ngpu-driver-read /drv/gpu0\ngpu-complete /drv/gpu0 fence:a\ngpu-driver-read /drv/gpu0\ngpu-complete /drv/gpu0 present:p\ngpu-driver-read /drv/gpu0\ngpu-complete /drv/gpu0 fence:b\ngpu-read /dev/gpu0\ngpu-read /dev/gpu0\ngpu-read /dev/gpu0\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains(
                "gpu-driver-read driver=/drv/gpu0 outcome=request header=request:1 kind=Write device=/dev/gpu0 opcode=None payload=draw:a"
            ),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-driver-read driver=/drv/gpu0 outcome=request header=request:3 kind=Control device=/dev/gpu0 opcode=Some("
            ),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-driver-read driver=/drv/gpu0 outcome=request header=request:2 kind=Write device=/dev/gpu0 opcode=None payload=draw:b"
            ),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=7 payload=fence:a"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=9 payload=present:p"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=7 payload=fence:b"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_reserves_gpu_queue_slot_for_present_control() {
        let report = report_for_script(
            "mkdomain render\nmkresource 2 device gpu0\nresource-governance 2 exclusive-lease\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\ngpu-queue-capacity /dev/gpu0 2\nclaim 2\ngpu-submit /dev/gpu0 draw:a\ngpu-probe-submit /dev/gpu0 draw:b\ngpu-present /dev/gpu0 frame:r\ngpu-driver-read /drv/gpu0\ngpu-complete /drv/gpu0 fence:a\ngpu-driver-read /drv/gpu0\ngpu-complete /drv/gpu0 present:r\ngpu-read /dev/gpu0\ngpu-read /dev/gpu0\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-submit device=/dev/gpu0 bytes=6 payload=draw:a"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-probe-submit device=/dev/gpu0 bytes=6 outcome=error"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-present device=/dev/gpu0 opcode=0x47500001"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-driver-read driver=/drv/gpu0 outcome=request header=request:1 kind=Write device=/dev/gpu0 opcode=None payload=draw:a"
            ),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-driver-read driver=/drv/gpu0 outcome=request header=request:2 kind=Control device=/dev/gpu0 opcode=Some("
            ),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=7 payload=fence:a"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=9 payload=present:r"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_releases_reserved_gpu_control_slot_after_failed_present() {
        let report = report_for_script(
            "mkdomain render\nmkresource 2 device gpu0\nresource-governance 2 exclusive-lease\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\ngpu-queue-capacity /dev/gpu0 2\nclaim 2\ngpu-submit /dev/gpu0 draw:a\ngpu-present /dev/gpu0 frame:f\ngpu-driver-read /drv/gpu0\ngpu-fail-request /drv/gpu0 2 error:present\ngpu-probe-submit /dev/gpu0 draw:b\ngpu-driver-read /drv/gpu0\ngpu-complete /drv/gpu0 fence:a\ngpu-driver-read /drv/gpu0\ngpu-complete /drv/gpu0 fence:b\ngpu-read /dev/gpu0\ngpu-read /dev/gpu0\ngpu-read /dev/gpu0\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains(
                "gpu-fail-request driver=/drv/gpu0 request=2 bytes=13 payload=error:present"
            ),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-probe-submit device=/dev/gpu0 bytes=6 outcome=submitted payload=draw:b"
            ),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-driver-read driver=/drv/gpu0 outcome=request header=request:2 kind=Control device=/dev/gpu0 opcode=Some("
            ),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-driver-read driver=/drv/gpu0 outcome=request header=request:1 kind=Write device=/dev/gpu0 opcode=None payload=draw:a"
            ),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-driver-read driver=/drv/gpu0 outcome=request header=request:3 kind=Write device=/dev/gpu0 opcode=None payload=draw:b"
            ),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=13 payload=error:present"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=7 payload=fence:a"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=7 payload=fence:b"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_observes_gpu_control_reserve_state_transitions() {
        let report = report_for_script(
            "mkdomain render\nmkresource 2 device gpu0\nresource-governance 2 exclusive-lease\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\ngpu-queue-capacity /dev/gpu0 2\nclaim 2\ndevice /dev/gpu0\ngpu-submit /dev/gpu0 draw:a\ndevice /dev/gpu0\ngpu-present /dev/gpu0 frame:f\ngpu-driver-read /drv/gpu0\ngpu-fail-request /drv/gpu0 2 error:present\ndevice /dev/gpu0\ngpu-driver-read /drv/gpu0\ngpu-complete /drv/gpu0 fence:a\ndevice /dev/gpu0\ngpu-submit /dev/gpu0 draw:b\ndevice /dev/gpu0\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains(
                "device path=/dev/gpu0 class=graphics state=1 queue-depth=0 queue-capacity=2 control-reserve=armed"
            ),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "device path=/dev/gpu0 class=graphics state=1 queue-depth=1 queue-capacity=2 control-reserve=armed"
            ),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "device path=/dev/gpu0 class=graphics state=1 queue-depth=1 queue-capacity=2 control-reserve=released"
            ),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "device path=/dev/gpu0 class=graphics state=1 queue-depth=0 queue-capacity=2 control-reserve=armed"
            ),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-submit device=/dev/gpu0 bytes=6 payload=draw:b"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "device path=/dev/gpu0 class=graphics state=1 queue-depth=1 queue-capacity=2 control-reserve=armed"
            ),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_completes_gpu_event_queue_flow() {
        let report = report_for_script(
            "mkdomain render\nmkresource 2 device gpu0\nresource-governance 2 exclusive-lease\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\nclaim 2\nqueue-create kqueue\ngpu-watch $LAST_QUEUE_FD /dev/gpu0 901\ngpu-submit /dev/gpu0 draw:event\nqueue-wait $LAST_QUEUE_FD\ngpu-driver-read /drv/gpu0\ngpu-complete /drv/gpu0 fence:event\nqueue-wait $LAST_QUEUE_FD\ngpu-read /dev/gpu0\ngpu-unwatch $LAST_QUEUE_FD /dev/gpu0 901\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains("gpu-watch queue="),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-submit device=/dev/gpu0 bytes=10 payload=draw:event"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("queue-event queue="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("source=graphics device="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("kind=submitted"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("kind=completed"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("kind=drained"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-complete driver=/drv/gpu0 bytes=11 payload=fence:event"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=11 payload=fence:event"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("gpu-unwatch queue="),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_completes_gpu_request_inspection_flow() {
        let report = report_for_script(
            "mkdomain render\nmkresource 2 device gpu0\nresource-governance 2 exclusive-lease\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\nclaim 2\ngpu-submit /dev/gpu0 draw:req\ngpu-request 1\ngpu-driver-read /drv/gpu0\ngpu-request 1\ngpu-complete /drv/gpu0 fence:req\ngpu-request 1\ngpu-read /dev/gpu0\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-request id=1 issuer=2 kind=write state=queued"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-request id=1 issuer=2 kind=write state=inflight"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-request id=1 issuer=2 kind=write state=completed"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("gpu-request id=1")
                && report.stdout.contains("payload=8")
                && report.stdout.contains("response=9"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("opcode=0x00000000"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-complete driver=/drv/gpu0 bytes=9 payload=fence:req"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=9 payload=fence:req"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_completes_gpu_lease_handoff_and_event_flow() {
        let report = report_for_script(
            "mkdomain render\nmkresource 2 device gpu0\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\nmkcontract 2 2 display mirror\ngpu-lease-watch 2 900\nclaim 2\nclaim 3\ngpu-lease-wait $LAST_QUEUE_FD\nreleaseclaim 2\ngpu-lease-wait $LAST_QUEUE_FD\ngpu-submit /dev/gpu0 draw:mirror\ngpu-driver-read /drv/gpu0\ngpu-complete /drv/gpu0 fence:9\ngpu-read /dev/gpu0\nreleaseclaim 3\ngpu-probe-submit /dev/gpu0 draw:mirror\ngpu-lease-unwatch $LAST_QUEUE_FD 2 900\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(report.stdout.contains("gpu-lease-watch queue="));
        assert!(
            report
                .stdout
                .contains("claim-acquired contract=2 resource=2")
        );
        assert!(
            report
                .stdout
                .contains("claim-queued contract=3 resource=2 holder=2 position=1")
        );
        assert!(report.stdout.contains("gpu-lease-event queue="));
        assert!(
            report
                .stdout
                .contains("token=900 resource=2 contract=3 kind=queued")
        );
        assert!(
            report
                .stdout
                .contains("token=900 resource=2 contract=3 kind=handed-off")
        );
        assert!(
            report
                .stdout
                .contains("gpu-submit device=/dev/gpu0 bytes=11 payload=draw:mirror")
        );
        assert!(
            report
                .stdout
                .contains("gpu-driver-read driver=/drv/gpu0 outcome=request")
        );
        assert!(report.stdout.contains("payload=draw:mirror"));
        assert!(
            report
                .stdout
                .contains("gpu-complete driver=/drv/gpu0 bytes=7 payload=fence:9")
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=7 payload=fence:9")
        );
        assert!(
            report
                .stdout
                .contains("claim-released contract=3 resource=2")
        );
        assert!(
            report
                .stdout
                .contains("gpu-probe-submit device=/dev/gpu0 bytes=11 outcome=error")
        );
        assert!(report.stdout.contains("gpu-lease-unwatch queue="));
    }

    #[test]
    fn native_session_report_completes_gpu_graphics_lease_event_flow() {
        let report = report_for_script(
            "mkdomain render\nmkresource 2 device gpu0\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\nmkcontract 2 2 display mirror\nclaim 2\nqueue-create kqueue\ngpu-watch $LAST_QUEUE_FD /dev/gpu0 902\nclaim 3\nreleaseclaim 2\nqueue-wait $LAST_QUEUE_FD\ngpu-present /dev/gpu0 frame:handoff\ngpu-driver-read /drv/gpu0\ngpu-complete /drv/gpu0 present:handoff\ngpu-read /dev/gpu0\ngpu-unwatch $LAST_QUEUE_FD /dev/gpu0 902\nreleaseclaim 3\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains("gpu-watch queue="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("source=graphics device="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("contract=2 kind=lease-released"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("contract=3 kind=lease-acquired"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-present device=/dev/gpu0 opcode=0x47500001"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-complete driver=/drv/gpu0 bytes=15 payload=present:handoff"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=15 payload=present:handoff"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_completes_gpu_request_cancellation_on_lease_loss_flow() {
        let report = report_for_script(
            "mkdomain render\nmkresource 2 device gpu0\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\nclaim 2\nqueue-create kqueue\ngpu-watch $LAST_QUEUE_FD /dev/gpu0 903\ngpu-submit /dev/gpu0 draw:drop\ngpu-driver-read /drv/gpu0\ngpu-request 1\nreleaseclaim 2\nqueue-wait $LAST_QUEUE_FD\ngpu-request 1\ngpu-driver-read /drv/gpu0\ngpu-read /dev/gpu0\ngpu-unwatch $LAST_QUEUE_FD /dev/gpu0 903\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-request id=1 issuer=2 kind=write state=inflight"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-request id=1 issuer=2 kind=write state=canceled"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("kind=canceled"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("contract=2 kind=lease-released"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-driver-read driver=/drv/gpu0 outcome=empty"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 outcome=empty"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_rejects_stale_gpu_completion_request_ids() {
        let report = report_for_script(
            "mkdomain render\nmkresource 2 device gpu0\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\nmkcontract 2 2 display mirror\nclaim 2\ngpu-submit /dev/gpu0 draw:stale\ngpu-driver-read /drv/gpu0\nreleaseclaim 2\nclaim 3\ngpu-submit /dev/gpu0 draw:new\ngpu-driver-read /drv/gpu0\ngpu-probe-complete-request /drv/gpu0 1 stale:done\ngpu-request 1\ngpu-request 2\ngpu-complete-request /drv/gpu0 2 fresh:done\ngpu-request 2\ngpu-read /dev/gpu0\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains(
                "gpu-probe-complete-request driver=/drv/gpu0 request=1 bytes=10 outcome=error"
            ),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-request id=1 issuer=2 kind=write state=canceled"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-request id=2 issuer=2 kind=write state=inflight"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-complete-request driver=/drv/gpu0 request=2 bytes=10 payload=fresh:done"
            ),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-request id=2 issuer=2 kind=write state=completed"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=10 payload=fresh:done"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_completes_gpu_driver_reset_recovery_flow() {
        let report = report_for_script(
            "mkdomain render\nmkresource 2 device gpu0\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\nclaim 2\ngpu-submit /dev/gpu0 draw:reset\ngpu-driver-read /drv/gpu0\ngpu-request 1\ngpu-driver-reset /drv/gpu0\ngpu-request 1\ndriver /drv/gpu0\ndevice /dev/gpu0\ngpu-submit /dev/gpu0 draw:after\ngpu-driver-read /drv/gpu0\ngpu-complete-request /drv/gpu0 2 fence:after\ngpu-request 2\ngpu-read /dev/gpu0\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-request id=1 issuer=2 kind=write state=inflight"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-driver-reset driver=/drv/gpu0 opcode=0x47501001 canceled=1"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-request id=1 issuer=2 kind=write state=canceled"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("driver path=/drv/gpu0 state=1 bound-devices=1 queued=0 inflight=0"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("device path=/dev/gpu0 class=graphics state=1 queue-depth=0"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-submit device=/dev/gpu0 bytes=10 payload=draw:after"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-complete-request driver=/drv/gpu0 request=2 bytes=11 payload=fence:after"
            ),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-request id=2 issuer=2 kind=write state=completed"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=11 payload=fence:after"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_completes_gpu_fault_and_recovery_event_flow() {
        let report = report_for_script(
            "mkdomain render\nmkresource 2 device gpu0\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\nclaim 2\nqueue-create kqueue\ngpu-watch $LAST_QUEUE_FD /dev/gpu0 904\ngpu-submit /dev/gpu0 draw:fault\ngpu-driver-read /drv/gpu0\ngpu-driver-reset /drv/gpu0\nqueue-wait $LAST_QUEUE_FD\ndriver /drv/gpu0\ndevice /dev/gpu0\ngpu-unwatch $LAST_QUEUE_FD /dev/gpu0 904\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains("kind=faulted"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("kind=recovered"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-driver-reset driver=/drv/gpu0 opcode=0x47501001 canceled=1"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("driver path=/drv/gpu0 state=1 bound-devices=1 queued=0 inflight=0"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("device path=/dev/gpu0 class=graphics state=1 queue-depth=0"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_completes_gpu_failed_request_flow() {
        let report = report_for_script(
            "mkdomain render\nmkresource 2 device gpu0\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\nclaim 2\nqueue-create kqueue\ngpu-watch $LAST_QUEUE_FD /dev/gpu0 905\ngpu-submit /dev/gpu0 draw:fail\ngpu-driver-read /drv/gpu0\ngpu-fail-request /drv/gpu0 1 error:shader\nqueue-wait $LAST_QUEUE_FD\ngpu-request 1\ngpu-read /dev/gpu0\ngpu-unwatch $LAST_QUEUE_FD /dev/gpu0 905\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains(
                "gpu-fail-request driver=/drv/gpu0 request=1 bytes=12 payload=error:shader"
            ),
            "{}",
            report.render()
        );
        assert!(report.stdout.contains("kind=failed"), "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-request id=1 issuer=2 kind=write state=failed"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=12 payload=error:shader"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_completes_gpu_explicit_cancel_flow() {
        let report = report_for_script(
            "mkdomain render\nmkresource 2 device gpu0\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\ngpu-queue-capacity /dev/gpu0 2\nclaim 2\nqueue-create kqueue\ngpu-watch $LAST_QUEUE_FD /dev/gpu0 906\ngpu-submit /dev/gpu0 draw:a\ngpu-present /dev/gpu0 frame:cancel\ngpu-request 2\ngpu-cancel-request /drv/gpu0 2 abort:present\nqueue-wait $LAST_QUEUE_FD\ngpu-request 2\ngpu-probe-submit /dev/gpu0 draw:b\ngpu-driver-read /drv/gpu0\ngpu-complete /drv/gpu0 fence:a\ngpu-driver-read /drv/gpu0\ngpu-complete /drv/gpu0 fence:b\ngpu-read /dev/gpu0\ngpu-read /dev/gpu0\ngpu-read /dev/gpu0\ngpu-unwatch $LAST_QUEUE_FD /dev/gpu0 906\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-request id=2 issuer=2 kind=control state=queued"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-cancel-request driver=/drv/gpu0 request=2 bytes=13 payload=abort:present"
            ),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("kind=canceled"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-request id=2 issuer=2 kind=control state=canceled"),
            "{}",
            report.render()
        );
        assert!(report.stdout.contains("response=13"), "{}", report.render());
        assert!(
            report.stdout.contains(
                "gpu-probe-submit device=/dev/gpu0 bytes=6 outcome=submitted payload=draw:b"
            ),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-driver-read driver=/drv/gpu0 outcome=request header=request:1 kind=Write device=/dev/gpu0 opcode=None payload=draw:a"
            ),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-driver-read driver=/drv/gpu0 outcome=request header=request:3 kind=Write device=/dev/gpu0 opcode=None payload=draw:b"
            ),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=7 payload=fence:a"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=7 payload=fence:b"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 outcome=empty"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_completes_gpu_driver_retire_flow() {
        let report = report_for_script(
            "mkdomain render\nmkresource 2 device gpu0\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\nclaim 2\ngpu-submit /dev/gpu0 draw:retire\ngpu-driver-read /drv/gpu0\ngpu-request 1\ngpu-driver-retire /drv/gpu0\ngpu-request 1\ndriver /drv/gpu0\ndevice /dev/gpu0\ngpu-probe-submit /dev/gpu0 draw:after\ngpu-probe-present /dev/gpu0 frame:after\ngpu-probe-driver-reset /drv/gpu0\ngpu-probe-driver-retire /drv/gpu0\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-request id=1 issuer=2 kind=write state=inflight"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-driver-retire driver=/drv/gpu0 opcode=0x47501002 canceled=1"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-request id=1 issuer=2 kind=write state=canceled"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("driver path=/drv/gpu0 state=3"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("device path=/dev/gpu0 class=graphics state=3"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-probe-submit device=/dev/gpu0 bytes=10 outcome=error"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-probe-present device=/dev/gpu0 opcode=0x47500001 outcome=error frame=frame:after"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-probe-driver-reset driver=/drv/gpu0 opcode=0x47501001 outcome=error"
            ),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-probe-driver-retire driver=/drv/gpu0 opcode=0x47501002 outcome=error"
            ),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_completes_gpu_retired_event_flow() {
        let report = report_for_script(
            "mkdomain render\nmkresource 2 device gpu0\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\nclaim 2\nqueue-create kqueue\ngpu-watch $LAST_QUEUE_FD /dev/gpu0 907\ngpu-submit /dev/gpu0 draw:retire-event\ngpu-driver-read /drv/gpu0\ngpu-driver-retire /drv/gpu0\nqueue-wait $LAST_QUEUE_FD\ngpu-unwatch $LAST_QUEUE_FD /dev/gpu0 907\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-driver-retire driver=/drv/gpu0 opcode=0x47501002 canceled=1"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("kind=retired"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("kind=canceled"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_completes_gpu_unbind_and_rebind_flow() {
        let report = report_for_script(
            "mkdomain render\nmkresource 2 device gpu0\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\nclaim 2\ngpu-submit /dev/gpu0 draw:busy\ngpu-probe-driver-unbind /dev/gpu0\ngpu-driver-read /drv/gpu0\ngpu-complete /drv/gpu0 fence:busy\ngpu-read /dev/gpu0\ngpu-driver-unbind /dev/gpu0\ndevice /dev/gpu0\ndriver /drv/gpu0\ngpu-probe-submit /dev/gpu0 draw:after-unbind\ngpu-driver-bind /dev/gpu0 /drv/gpu1\ndriver /drv/gpu1\ngpu-submit /dev/gpu0 draw:rebound\ngpu-driver-read /drv/gpu1\ngpu-complete /drv/gpu1 fence:rebound\ngpu-read /dev/gpu0\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-probe-driver-unbind device=/dev/gpu0 outcome=error"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("gpu-driver-unbind device=/dev/gpu0"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("device path=/dev/gpu0 class=graphics state=0"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("driver path=/drv/gpu0 state=0 bound-devices=0"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-probe-submit device=/dev/gpu0 bytes=17 outcome=error"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-driver-bind device=/dev/gpu0 driver=/drv/gpu1"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("driver path=/drv/gpu1 state=1 bound-devices=1"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-driver-read driver=/drv/gpu1 outcome=request header=request:2 kind=Write device=/dev/gpu0 opcode=None payload=draw:rebound"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-complete driver=/drv/gpu1 bytes=13 payload=fence:rebound"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=13 payload=fence:rebound"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_rejects_gpu_rebind_after_retire() {
        let report = report_for_script(
            "mkdomain render\nmkresource 2 device gpu0\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\nclaim 2\ngpu-driver-retire /drv/gpu0\ngpu-probe-driver-unbind /dev/gpu0\ngpu-probe-driver-bind /dev/gpu0 /drv/gpu1\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-driver-retire driver=/drv/gpu0 opcode=0x47501002 canceled=0"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-probe-driver-unbind device=/dev/gpu0 outcome=error"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-probe-driver-bind device=/dev/gpu0 driver=/drv/gpu1 outcome=error"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_tracks_gpu_requests_across_distinct_process_holders() {
        let mut runtime = KernelRuntime::host_runtime_default();
        let owner = runtime
            .spawn_process("gpu-owner", None, SchedulerClass::Interactive)
            .unwrap();
        let worker = runtime
            .spawn_process("gpu-worker", None, SchedulerClass::Interactive)
            .unwrap();
        let root = runtime
            .grant_capability(
                owner,
                ObjectHandle::new(Handle::new(35_000), 0),
                CapabilityRights::READ | CapabilityRights::WRITE | CapabilityRights::DUPLICATE,
                "root",
            )
            .unwrap();

        runtime
            .create_vfs_node("/", ObjectKind::Directory, root)
            .unwrap();
        runtime
            .create_vfs_node("/dev", ObjectKind::Directory, root)
            .unwrap();
        runtime
            .create_vfs_node("/drv", ObjectKind::Directory, root)
            .unwrap();
        runtime
            .create_vfs_node("/dev/gpu0", ObjectKind::Device, root)
            .unwrap();
        runtime
            .create_vfs_node("/drv/gpu0", ObjectKind::Driver, root)
            .unwrap();
        runtime
            .bind_device_to_driver("/dev/gpu0", "/drv/gpu0")
            .unwrap();

        let domain = runtime.create_domain(owner, None, "display").unwrap();
        let resource = runtime
            .create_resource(owner, domain, ResourceKind::Device, "gpu0")
            .unwrap();
        runtime
            .set_resource_contract_policy(resource, ResourceContractPolicy::Display)
            .unwrap();
        let primary = runtime
            .create_contract(owner, domain, resource, ContractKind::Display, "scanout")
            .unwrap();
        let mirror = runtime
            .create_contract(worker, domain, resource, ContractKind::Display, "mirror")
            .unwrap();

        runtime.claim_resource_via_contract(primary).unwrap();
        runtime.claim_resource_via_contract(mirror).unwrap();

        let owner_device_fd = runtime.open_path(owner, "/dev/gpu0").unwrap();
        let worker_device_fd = runtime.open_path(worker, "/dev/gpu0").unwrap();
        let driver_fd = runtime.open_path(owner, "/drv/gpu0").unwrap();

        assert_eq!(
            runtime
                .write_io(owner, owner_device_fd, b"draw:owner")
                .unwrap(),
            10
        );
        let owner_header =
            String::from_utf8(runtime.read_io(owner, driver_fd, 128).unwrap()).unwrap();
        assert!(owner_header.contains("request:1"), "{owner_header}");
        assert!(owner_header.contains("draw:owner"), "{owner_header}");
        let owner_request = runtime.device_request_info(1).unwrap();
        assert_eq!(owner_request.issuer, owner);

        assert!(matches!(
            runtime.release_claimed_resource_via_contract(primary).unwrap(),
            ResourceReleaseResult::HandedOff {
                resource: released,
                contract: new_holder,
                ..
            } if released == resource && new_holder == mirror
        ));

        let canceled_owner_request = runtime.device_request_info(1).unwrap();
        assert_eq!(canceled_owner_request.issuer, owner);
        assert_eq!(canceled_owner_request.state, DeviceRequestState::Canceled);

        assert_eq!(
            runtime
                .write_io(worker, worker_device_fd, b"draw:worker")
                .unwrap(),
            11
        );
        let worker_header =
            String::from_utf8(runtime.read_io(owner, driver_fd, 128).unwrap()).unwrap();
        assert!(worker_header.contains("request:2"), "{worker_header}");
        assert!(worker_header.contains("draw:worker"), "{worker_header}");
        let worker_request = runtime.device_request_info(2).unwrap();
        assert_eq!(worker_request.issuer, worker);
    }

    #[test]
    fn native_session_report_completes_gpu_bar_mmio_claim_map_unmap_flow() {
        let mut platform = sample_gpu_platform();
        let devices = platform.enumerate_devices().unwrap();
        let gpu = devices
            .iter()
            .find(|device| device.identity.base_class == 0x03)
            .unwrap();
        let bar = gpu
            .bars
            .iter()
            .find(|bar| matches!(bar.kind, BarKind::Memory32 | BarKind::Memory64))
            .unwrap();

        let region = platform.claim_bar(gpu.locator, bar.id).unwrap();
        let mapping = platform
            .map_mmio(
                region,
                MmioPermissions::read_write(),
                MmioCachePolicy::Uncacheable,
            )
            .unwrap();

        assert_eq!(mapping.region, region);
        assert_eq!(mapping.physical_base, bar.base);
        assert_eq!(mapping.virtual_base, DEFAULT_DIRECT_MAP_BASE + bar.base);
        assert_eq!(mapping.len, bar.size);
        assert_eq!(platform.release_bar(region), Err(HalError::DmaBusy));

        platform.unmap_mmio(mapping.id).unwrap();
        platform.release_bar(region).unwrap();
    }

    #[test]
    fn native_session_report_rejects_double_gpu_bar_claim_and_stale_unmap() {
        let mut platform = sample_gpu_platform();
        let devices = platform.enumerate_devices().unwrap();
        let gpu = devices
            .iter()
            .find(|device| device.identity.base_class == 0x03)
            .unwrap();
        let bar = gpu
            .bars
            .iter()
            .find(|bar| matches!(bar.kind, BarKind::Memory32 | BarKind::Memory64))
            .unwrap();

        let region = platform.claim_bar(gpu.locator, bar.id).unwrap();
        let mapping = platform
            .map_mmio(
                region,
                MmioPermissions::read_write(),
                MmioCachePolicy::Uncacheable,
            )
            .unwrap();

        assert_eq!(
            platform.claim_bar(gpu.locator, bar.id),
            Err(HalError::BarAlreadyClaimed)
        );
        platform.unmap_mmio(mapping.id).unwrap();
        assert_eq!(
            platform.unmap_mmio(mapping.id),
            Err(HalError::InvalidMmioMapping)
        );
        platform.release_bar(region).unwrap();
    }

    #[test]
    fn native_session_report_completes_gpu_dma_prepare_complete_release_flow() {
        let mut platform = sample_gpu_platform();
        let buffer = platform
            .allocate_dma(
                0x3000,
                DmaDirection::Bidirectional,
                DmaCoherency::Coherent,
                DmaConstraints::platform_default(),
            )
            .unwrap();

        assert_eq!(buffer.ownership, DmaOwnership::Cpu);
        platform.prepare_dma_for_device(buffer.id).unwrap();
        assert_eq!(
            platform.prepare_dma_for_device(buffer.id),
            Err(HalError::DmaBusy)
        );
        platform.complete_dma_from_device(buffer.id).unwrap();
        platform.release_dma(buffer.id).unwrap();

        let recycled = platform
            .allocate_dma(
                0x3000,
                DmaDirection::ToDevice,
                DmaCoherency::Coherent,
                DmaConstraints::platform_default(),
            )
            .unwrap();
        assert_eq!(recycled.ownership, DmaOwnership::Cpu);
        assert_eq!(buffer.device_address, recycled.device_address);
    }

    #[test]
    fn native_session_report_enforces_gpu_dma_constraints_and_stale_release_refusal() {
        let mut platform = sample_gpu_platform();
        let constrained = platform
            .allocate_dma(
                0x1000,
                DmaDirection::FromDevice,
                DmaCoherency::NonCoherent,
                DmaConstraints {
                    alignment: 0x2000,
                    max_address_bits: 29,
                    segment_boundary: u64::MAX,
                    contiguous: true,
                },
            )
            .unwrap();

        assert!(constrained.device_address.is_multiple_of(0x2000));
        assert!(constrained.device_address < (1u64 << 29));
        platform.release_dma(constrained.id).unwrap();
        assert_eq!(
            platform.release_dma(constrained.id),
            Err(HalError::InvalidDmaBuffer)
        );
    }

    #[test]
    fn native_session_report_completes_gpu_interrupt_claim_enable_dispatch_ack_flow() {
        let mut platform = sample_gpu_platform();
        let devices = platform.enumerate_devices().unwrap();
        let gpu = devices
            .iter()
            .find(|device| device.identity.base_class == 0x03)
            .unwrap();

        let (handle, route) = platform.claim_interrupt(gpu.locator, 0).unwrap();
        platform.enable_interrupt(handle).unwrap();
        let event = platform
            .dispatch_interrupt_vector(route.vector)
            .unwrap()
            .unwrap();
        assert_eq!(event.handle, handle);
        assert_eq!(event.route, route);
        assert_eq!(platform.pending_interrupts(), &[handle]);
        platform.acknowledge_interrupt(handle).unwrap();
        assert!(platform.pending_interrupts().is_empty());
    }

    #[test]
    fn native_session_report_rejects_gpu_interrupt_double_claim_and_stale_ack() {
        let mut platform = sample_gpu_platform();
        let devices = platform.enumerate_devices().unwrap();
        let gpu = devices
            .iter()
            .find(|device| device.identity.base_class == 0x03)
            .unwrap();

        let (handle, route) = platform.claim_interrupt(gpu.locator, 0).unwrap();
        assert_eq!(
            platform.claim_interrupt(gpu.locator, 0),
            Err(HalError::InterruptAlreadyClaimed)
        );
        assert_eq!(
            platform.dispatch_interrupt_vector(route.vector).unwrap(),
            None
        );
        platform.enable_interrupt(handle).unwrap();
        let event = platform.dispatch_interrupt_vector(route.vector).unwrap();
        assert!(event.is_some());
        platform.acknowledge_interrupt(handle).unwrap();
    }

    #[test]
    fn native_session_report_completes_gpu_buffer_object_submit_flow() {
        let report = report_for_script(
            "mkdomain render\nmkresource 2 device gpu0\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\nclaim 2\ngpu-buffer-create 32\ngpu-buffer-write 1 0 draw:buffer\ngpu-buffer 1\ngpu-submit-buffer /dev/gpu0 1\ngpu-request 1\ngpu-driver-read /drv/gpu0\ngpu-complete-request /drv/gpu0 1 fence:buffer\ngpu-request 1\ngpu-read /dev/gpu0\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains("gpu-buffer-create id=1 length=32"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-buffer-write id=1 offset=0 bytes=11 payload=draw:buffer"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-buffer id=1 owner=2 length=32 used=11"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-submit-buffer device=/dev/gpu0 buffer=1 submitted=1"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-request id=1 issuer=2 kind=write state=queued opcode=0x00000000 buffer=1"
            ),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-driver-read driver=/drv/gpu0 outcome=request header=request:1 kind=Write device=/dev/gpu0 opcode=None buffer=1 payload=draw:buffer"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-complete-request driver=/drv/gpu0 request=1 bytes=12 payload=fence:buffer"
            ),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-request id=1 issuer=2 kind=write state=completed opcode=0x00000000 buffer=1"
            ),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=12 payload=fence:buffer"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_observes_hardware_gpu_buffer_submit_via_platform_x86_64() {
        let report = report_for_script_with_runtime(
            "mkdomain render\nmkresource 2 device gpu0\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\nclaim 2\ngpu-buffer-create 32\ngpu-buffer-write 1 0 draw:hardware\ngpu-submit-buffer /dev/gpu0 1\ngpu-driver-read /drv/gpu0\ngpu-scanout /dev/gpu0\nexit 0\n",
            |runtime| {
                let (mut platform, locator) = sample_nvidia_gpu_platform();
                platform.setup_gpu_agent(locator).unwrap();
                runtime.install_hardware_provider(Box::new(platform));
            },
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-submit-buffer device=/dev/gpu0 buffer=1 submitted=13"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-driver-read driver=/drv/gpu0 outcome=empty"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-scanout device=/dev/gpu0 presented=1 last-frame-bytes=13 frame=draw:hardware"
            ),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_observes_hardware_gpu_present_via_platform_x86_64() {
        let report = report_for_script_with_runtime(
            "mkdomain render\nmkresource 2 device gpu0\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\nclaim 2\ngpu-present /dev/gpu0 frame:hardware\ngpu-driver-read /drv/gpu0\ngpu-scanout /dev/gpu0\nexit 0\n",
            |runtime| {
                let (mut platform, locator) = sample_nvidia_gpu_platform();
                platform.setup_gpu_agent(locator).unwrap();
                runtime.install_hardware_provider(Box::new(platform));
            },
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-present device=/dev/gpu0 opcode=0x47500001 response=0x47500000 frame=frame:hardware"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-driver-read driver=/drv/gpu0 outcome=empty"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-scanout device=/dev/gpu0 presented=1 last-frame-bytes=14 frame=frame:hardware"
            ),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_completes_gpu_scanout_present_flow() {
        let report = report_for_script(
            "mkdomain render\nmkresource 2 device gpu0\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\nclaim 2\ngpu-present /dev/gpu0 frame:scanout\ngpu-driver-read /drv/gpu0\ngpu-complete-request /drv/gpu0 1 frame:scanout\ngpu-scanout /dev/gpu0\ngpu-read /dev/gpu0\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-present device=/dev/gpu0 opcode=0x47500001 response=0x47500000 frame=frame:scanout"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-driver-read driver=/drv/gpu0 outcome=request header=request:1 kind=Control device=/dev/gpu0 opcode=Some(1196425217) payload="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-complete-request driver=/drv/gpu0 request=1 bytes=13 payload=frame:scanout"
            ),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "gpu-scanout device=/dev/gpu0 presented=1 last-frame-bytes=13 frame=frame:scanout"
            ),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-read device=/dev/gpu0 bytes=13 payload=frame:scanout"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_observes_gpu_performance_counters() {
        let report = report_for_script(
            "mkdomain render\nmkresource 2 device gpu0\nresource-contract-policy 2 display\nmkcontract 2 2 display scanout\nclaim 2\ngpu-submit /dev/gpu0 draw:perf\ngpu-request 1\ngpu-driver-read /drv/gpu0\ngpu-request 1\ngpu-complete-request /drv/gpu0 1 fence:perf\ngpu-request 1\ngpu-perf /dev/gpu0\ndevice /dev/gpu0\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains(
                "gpu-request id=1 issuer=2 kind=write state=queued opcode=0x00000000 buffer=0 payload=9 response=0 submitted="
            ),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(" started=0 completed=0"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-request id=1 issuer=2 kind=write state=inflight"),
            "{}",
            report.render()
        );
        assert!(report.stdout.contains(" started="), "{}", report.render());
        assert!(
            report
                .stdout
                .contains("gpu-request id=1 issuer=2 kind=write state=completed"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("gpu-perf device=/dev/gpu0 submitted=1 completed=1 total-latency="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("max-latency=") && report.stdout.contains("total-queue-wait="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains(
                "device path=/dev/gpu0 class=graphics state=1 queue-depth=0 queue-capacity=64 control-reserve=armed submitted=1 completed=1 total-latency="
            ),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_runs_kernel_launched_shell_through_real_syscall_path() {
        let report = build_native_session_report();

        assert_eq!(report.exit_code, 0);
        assert!(report.session_reported);
        assert_eq!(report.session_report_count, 3);
        assert_eq!(report.session_status, 0);
        assert_eq!(report.session_stage, 2);
        assert_eq!(report.session_code, report.exit_code);
        assert!(report.domain_count >= 1);
        assert!(report.resource_count >= 1);
        assert!(report.contract_count >= 1);
        assert_eq!(report.stdout_bytes, report.stdout.len());
        let rendered = report.render();
        assert!(rendered.contains("== chronoscope =="));
        assert!(rendered.contains("== resource-agents =="));
        assert!(rendered.contains("== vm-agents =="));
        assert!(rendered.contains("cause:"));
        assert!(rendered.contains("responsible:"));
        assert!(rendered.contains("last_writer:"));
        assert!(rendered.contains("rewind:"));
        assert!(rendered.contains("divergence:"));
        assert!(rendered.contains("propagation:"));
        assert!(rendered.contains("trust:"));
        assert!(rendered.contains("replay:"));
        assert!(rendered.contains("confidence:"));
        assert!(rendered.contains("decisions:"));
        assert!(report.stdout.contains("ngos shell"));
        assert!(rendered.contains("agent=fault-classifier"));
        assert!(rendered.contains("agent=page-touch"));
        assert!(rendered.contains("session: reported=true"));
    }

    #[test]
    fn native_session_report_completes_game_stack_runtime_flow() {
        let report = report_for_script(
            "mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest arg=--fullscreen\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\nmkfile-path /games/orbit.frame\nappend-line /games/orbit.frame surface=1280x720\nappend-line /games/orbit.frame frame=orbit-001\nappend-line /games/orbit.frame queue=graphics\nappend-line /games/orbit.frame present-mode=mailbox\nappend-line /games/orbit.frame completion=wait-complete\nappend-line /games/orbit.frame clear=#112233\nappend-line /games/orbit.frame rect=10,20,200,100,#ff8800ff\nmkfile-path /games/orbit.mix\nappend-line /games/orbit.mix rate=48000\nappend-line /games/orbit.mix channels=2\nappend-line /games/orbit.mix stream=orbit-intro\nappend-line /games/orbit.mix route=music\nappend-line /games/orbit.mix latency-mode=interactive\nappend-line /games/orbit.mix spatialization=world-3d\nappend-line /games/orbit.mix completion=fire-and-forget\nappend-line /games/orbit.mix tone=lead,440,120,0.800,-0.250,sine\nmkfile-path /games/orbit.input\nappend-line /games/orbit.input device=gamepad\nappend-line /games/orbit.input family=dualshock\nappend-line /games/orbit.input frame=input-001\nappend-line /games/orbit.input layout=gamepad-standard\nappend-line /games/orbit.input key-table=us-game\nappend-line /games/orbit.input pointer-capture=relative-lock\nappend-line /games/orbit.input delivery=immediate\nappend-line /games/orbit.input button=cross,press\ngame-launch /games/orbit.manifest\ngame-gfx-submit $LAST_PID /games/orbit.frame\ngame-audio-submit $LAST_PID /games/orbit.mix\ngame-input-submit $LAST_PID /games/orbit.input\nproc $LAST_PID environ\ncat-file /compat/orbit/session.chan\ngame-status\ngame-stop $LAST_PID\ngame-audio-submit $LAST_PID /games/orbit.mix\nlast-status\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains("game.session pid="),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("channel-file=/compat/orbit/session.chan"),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("NGOS_GAME_CHANNEL=/compat/orbit/session.chan"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("kind=graphics tag=orbit-001"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("kind=audio tag=orbit-intro"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("kind=input tag=input-001"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("game.session.runtime-channel pid="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("stopped=true exit="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("last-status=295"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_completes_game_audio_lane_runtime_flow() {
        let report = report_for_script(
            "mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\nmkfile-path /games/orbit.mix\nappend-line /games/orbit.mix rate=48000\nappend-line /games/orbit.mix channels=2\nappend-line /games/orbit.mix stream=orbit-intro\nappend-line /games/orbit.mix route=music\nappend-line /games/orbit.mix latency-mode=interactive\nappend-line /games/orbit.mix spatialization=world-3d\nappend-line /games/orbit.mix completion=fire-and-forget\nappend-line /games/orbit.mix tone=lead,440,120,0.800,-0.250,sine\ngame-launch /games/orbit.manifest\ngame-audio-submit $LAST_PID /games/orbit.mix\ngame-audio-status $LAST_PID\ndriver /drv/audio0\ndevice /dev/audio0\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains("game.audio.submit pid="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("completion-observed=submitted"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("game.audio.status pid="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("device-submitted="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("driver-queued="),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("device path=/dev/audio0 class=audio"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("driver path=/drv/audio0 state=1"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_completes_game_input_lane_runtime_flow() {
        let report = report_for_script(
            "mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\nmkfile-path /games/orbit.input\nappend-line /games/orbit.input device=gamepad\nappend-line /games/orbit.input family=dualshock\nappend-line /games/orbit.input frame=input-001\nappend-line /games/orbit.input layout=gamepad-standard\nappend-line /games/orbit.input key-table=us-game\nappend-line /games/orbit.input pointer-capture=relative-lock\nappend-line /games/orbit.input delivery=immediate\nappend-line /games/orbit.input button=cross,press\ngame-launch /games/orbit.manifest\ngame-input-submit $LAST_PID /games/orbit.input\ngame-input-status $LAST_PID\ndriver /drv/input0\ndevice /dev/input0\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains("game.input.submit pid="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("delivery-observed=submitted"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("game.input.status pid="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("device-submitted="),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("driver-queued="),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("device path=/dev/input0 class=input"),
            "{}",
            report.render()
        );
        assert!(
            report.stdout.contains("driver path=/drv/input0 state=1"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_completes_game_stack_launch_only() {
        let report = report_for_script(
            "mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-launch /games/orbit.manifest\nproc $LAST_PID environ\ngame-status\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains("game.session pid="),
            "{}",
            report.render()
        );
        assert!(
            report
                .stdout
                .contains("NGOS_GAME_CHANNEL=/compat/orbit/session.chan"),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_completes_game_stack_launch_exit_only() {
        let report = report_for_script(
            "mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-launch /games/orbit.manifest\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains("game.session pid="),
            "{}",
            report.render()
        );
    }

    #[test]
    fn native_session_report_completes_game_stack_launch_and_status_only() {
        let report = report_for_script(
            "mkdir-path /games\nmkdir-path /games/orbit\nmkfile-path /games/orbit.manifest\nappend-line /games/orbit.manifest title=Orbit Runner\nappend-line /games/orbit.manifest slug=orbit-runner\nappend-line /games/orbit.manifest exec=/bin/worker\nappend-line /games/orbit.manifest cwd=/games/orbit\nappend-line /games/orbit.manifest gfx.backend=vulkan\nappend-line /games/orbit.manifest gfx.profile=frame-pace\nappend-line /games/orbit.manifest audio.backend=native-mixer\nappend-line /games/orbit.manifest audio.profile=spatial-mix\nappend-line /games/orbit.manifest input.backend=native-input\nappend-line /games/orbit.manifest input.profile=gamepad-first\nappend-line /games/orbit.manifest shim.prefix=/compat/orbit\nappend-line /games/orbit.manifest shim.saves=/saves/orbit\nappend-line /games/orbit.manifest shim.cache=/cache/orbit\ngame-launch /games/orbit.manifest\ngame-status\nexit 0\n",
        );

        assert_eq!(report.exit_code, 0, "{}", report.render());
        assert!(
            report.stdout.contains("game.session pid="),
            "{}",
            report.render()
        );
    }
}
