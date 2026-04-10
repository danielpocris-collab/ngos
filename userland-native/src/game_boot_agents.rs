use super::*;

#[inline(never)]
pub(crate) fn native_game_smoke_image_path<B: SyscallBackend>(runtime: &Runtime<B>) -> String {
    if runtime.stat_path("/bin/ngos-userland-native").is_ok() {
        return String::from("/bin/ngos-userland-native");
    }
    if runtime.stat_path("/bin/userland-native").is_ok() {
        return String::from("/bin/userland-native");
    }
    String::from("/kernel/ngos-userland-native")
}

#[inline(never)]
pub(crate) fn run_native_game_compat_worker<B: SyscallBackend>(
    runtime: &Runtime<B>,
    bootstrap: &BootstrapArgs<'_>,
) -> ExitCode {
    let channel_path = match bootstrap_env_value(bootstrap, "NGOS_GAME_CHANNEL") {
        Some(path) if !path.is_empty() => path,
        _ => return 341,
    };
    if write_line(
        runtime,
        &format!("game.worker.ready channel={channel_path}"),
    )
    .is_err()
    {
        return 342;
    }
    let fd = match runtime.open_path(channel_path) {
        Ok(fd) => fd,
        Err(_) => return 343,
    };
    let mut payload_count = 0usize;
    let mut buffer = [0u8; 512];
    loop {
        let events = match runtime.poll(fd, POLLIN) {
            Ok(events) => events,
            Err(_) => {
                let _ = runtime.close(fd);
                return 344;
            }
        };
        if events & POLLIN == 0 {
            continue;
        }
        let count = match runtime.read(fd, &mut buffer) {
            Ok(count) => count,
            Err(_) => {
                let _ = runtime.close(fd);
                return 345;
            }
        };
        if count == 0 {
            continue;
        }
        payload_count = payload_count.saturating_add(1);
        if write_line(
            runtime,
            &format!("game.worker.payload count={payload_count} bytes={count}"),
        )
        .is_err()
        {
            let _ = runtime.close(fd);
            return 346;
        }
    }
}

#[inline(never)]
pub(crate) fn run_native_game_stack_smoke<B: SyscallBackend>(runtime: &Runtime<B>) -> ExitCode {
    debug_break(0x4e47_4f53_4753_3130, 0);
    let executable_path = native_game_smoke_image_path(runtime);
    debug_break(0x4e47_4f53_4753_3131, 0);
    let manifest_text = format!(
        "title=Orbit Runner\nslug=orbit-runner\nexec={executable_path}\ncwd=/\narg={COMPAT_WORKER_ARG}\ngfx.backend=vulkan\ngfx.profile=frame-pace\naudio.backend=native-mixer\naudio.profile=spatial-mix\ninput.backend=native-input\ninput.profile=gamepad-first\nshim.prefix=/\nshim.saves=/\nshim.cache=/\n"
    );
    let manifest = match GameCompatManifest::parse(&manifest_text) {
        Ok(manifest) => manifest,
        Err(_) => return 348,
    };
    if game_render_manifest(runtime, "/orbit.manifest", &manifest).is_err() {
        return 349;
    }
    if game_render_plan(runtime, &manifest.session_plan()).is_err() {
        return 350;
    }
    debug_break(0x4e47_4f53_4753_3132, 0);
    let mut cwd = String::from("/");
    let mut session = match game_launch_session(runtime, &mut cwd, &manifest) {
        Ok(session) => session,
        Err(code) => return code,
    };
    debug_break(0x4e47_4f53_4753_3133, session.pid);
    if game_render_session(runtime, &session).is_err() {
        return 351;
    }
    let env_text = match shell_read_file_text(runtime, &session.runtime_env_path) {
        Ok(text) => text,
        Err(code) => return code,
    };
    debug_break(0x4e47_4f53_4753_3134, env_text.len() as u64);
    if !bytes_contain_all_markers(
        env_text.as_bytes(),
        &[&format!(
            "NGOS_GAME_CHANNEL={}",
            session.runtime_channel_path
        )],
    ) {
        let _ = write_line(
            runtime,
            &format!(
                "game.smoke.env-mismatch path={} expected=NGOS_GAME_CHANNEL={}",
                session.runtime_env_path, session.runtime_channel_path
            ),
        );
        return 354;
    }
    debug_break(0x4e47_4f53_4753_3135, 0);
    let argv_text = match shell_read_file_text(runtime, &session.runtime_argv_path) {
        Ok(text) => text,
        Err(code) => return code,
    };
    debug_break(0x4e47_4f53_4753_3136, argv_text.len() as u64);
    if !bytes_contain_all_markers(
        argv_text.as_bytes(),
        &[executable_path.as_str(), COMPAT_WORKER_ARG],
    ) {
        return 352;
    }
    let frame_script = match FrameScript::parse(
        "surface=1280x720\nframe=orbit-001\nqueue=graphics\npresent-mode=mailbox\ncompletion=fire-and-forget\nclear=#112233\nrect=10,20,200,100,#ff8800ff\n",
    ) {
        Ok(script) => script,
        Err(_) => return 353,
    };
    let encoded_frame = match game_encode_frame(&session, &frame_script) {
        Ok(encoded) => encoded,
        Err(code) => return code,
    };
    debug_break(0x4e47_4f53_4753_3137, encoded_frame.payload.len() as u64);
    if game_submit_frame(runtime, &mut session, &encoded_frame).is_err() {
        return 365;
    }
    let mix_script = match MixScript::parse(
        "rate=48000\nchannels=2\nstream=orbit-intro\nroute=music\nlatency-mode=interactive\nspatialization=world-3d\ncompletion=fire-and-forget\ntone=lead,440,120,0.800,-0.250,sine\n",
    ) {
        Ok(script) => script,
        Err(_) => return 355,
    };
    let encoded_mix = match game_encode_mix(&session, &mix_script) {
        Ok(encoded) => encoded,
        Err(code) => return code,
    };
    debug_break(0x4e47_4f53_4753_3138, encoded_mix.payload.len() as u64);
    if game_submit_mix(runtime, &mut session, &encoded_mix).is_err() {
        return 356;
    }
    let input_script = match InputScript::parse(
        "device=gamepad\nfamily=dualshock\nframe=input-001\nlayout=gamepad-standard\nkey-table=us-game\npointer-capture=relative-lock\ndelivery=immediate\nbutton=cross,press\n",
    ) {
        Ok(script) => script,
        Err(_) => return 357,
    };
    let encoded_input = match game_encode_input(&session, &input_script) {
        Ok(encoded) => encoded,
        Err(code) => return code,
    };
    debug_break(0x4e47_4f53_4753_3139, encoded_input.payload.len() as u64);
    if game_submit_input(runtime, &mut session, &encoded_input).is_err() {
        return 358;
    }
    debug_break(0x4e47_4f53_4753_3140, 0);
    if game_render_session_summary(runtime, &session).is_err() {
        return 359;
    }
    debug_break(0x4e47_4f53_4753_3141, 0);
    let channel_text = match shell_read_file_text(runtime, &session.runtime_channel_path) {
        Ok(text) => text,
        Err(code) => return code,
    };
    debug_break(0x4e47_4f53_4753_3142, channel_text.len() as u64);
    if !bytes_contain_all_markers(
        channel_text.as_bytes(),
        &[
            "kind=graphics tag=orbit-001",
            "kind=audio tag=orbit-intro",
            "kind=input tag=input-001",
        ],
    ) {
        return 360;
    }
    debug_break(0x4e47_4f53_4753_3143, 0);
    if game_stop_session(runtime, &mut session).is_err() {
        return 361;
    }
    debug_break(
        0x4e47_4f53_4753_3144,
        session.exit_code.unwrap_or_default() as u64,
    );
    match game_submit_frame(runtime, &mut session, &encoded_frame) {
        Err(295) => {}
        _ => return 362,
    }
    if game_render_session(runtime, &session).is_err()
        || game_render_session_summary(runtime, &session).is_err()
    {
        return 363;
    }
    if !session.stopped || session.exit_code.is_none() {
        return 364;
    }
    0
}
