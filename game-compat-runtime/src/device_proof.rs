use alloc::{format, vec, vec::Vec};

use ngos_audio_translate::{AudioTranslator, ForeignAudioApi, MixScript};
use ngos_gfx_translate::{EncodedFrame, ForeignFrameScript, GfxTranslator, SourceApi};
use ngos_input_translate::{ForeignInputApi, InputScript, InputTranslator};
use ngos_shell_proc::fixed_text_field;
use ngos_user_abi::{
    Errno, ExitCode, NativeDeviceRecord, NativeDeviceRequestRecord, NativeDriverRecord,
    SyscallBackend,
};
use ngos_user_runtime::Runtime;

fn write_line<B: SyscallBackend>(runtime: &Runtime<B>, text: &str) -> Result<(), ExitCode> {
    runtime
        .writev(1, &[text.as_bytes(), b"\n"])
        .map_err(|_| 196)?;
    Ok(())
}

pub fn build_compat_gfx_translated_payload(
    source_api: SourceApi,
    text: &str,
) -> Result<EncodedFrame, ExitCode> {
    let foreign = ForeignFrameScript::parse_for_api(Some(source_api), text).map_err(|_| 339)?;
    let translator = GfxTranslator::new(source_api);
    let script = translator.translate(&foreign).map_err(|_| 340)?;
    Ok(script.encode_translated(
        source_api.translation_label(),
        source_api.name(),
        source_api.translation_label(),
    ))
}

pub fn run_native_compat_graphics_boot_smoke<B: SyscallBackend>(runtime: &Runtime<B>) -> ExitCode {
    let first_payload = match build_compat_gfx_translated_payload(
        SourceApi::DirectX12,
        "surface=640x480\nframe=qemu-compat-001\nqueue=graphics\npresent-mode=mailbox\ncompletion=wait-present\ndx-clear-rtv=000000ff\ndx-gradient-rect=0,0,320,180,112233ff,223344ff,334455ff,445566ff\ndx-present=0,0,640,480\n",
    ) {
        Ok(encoded) => encoded,
        Err(code) => return code,
    };
    let second_payload = match build_compat_gfx_translated_payload(
        SourceApi::OpenGL,
        "surface=640x480\nframe=qemu-compat-002\nqueue=graphics\npresent-mode=fifo\ncompletion=fire-and-forget\ngl-clear=112233ff\ngl-set-clip=0,0,320,240\ngl-clear-clip\ngl-swap-buffers=0,0,640,480\n",
    ) {
        Ok(encoded) => encoded,
        Err(code) => return code,
    };
    let first_payload_text = first_payload.payload.clone();
    let second_payload_text = second_payload.payload.clone();

    let request_id = match runtime.present_gpu_frame("/dev/gpu0", first_payload_text.as_bytes()) {
        Ok(id) => id as u64,
        Err(_) => return 300,
    };
    let driver_fd = match runtime.open_path("/drv/gpu0") {
        Ok(fd) => fd,
        Err(_) => return 331,
    };
    let mut driver_request = [0u8; 2048];
    let driver_request_len = match runtime.read(driver_fd, &mut driver_request) {
        Ok(count) => count,
        Err(_) => return 332,
    };
    if driver_request_len == 0 {
        return 333;
    }
    let mut completion = format!("complete-request:{request_id}\n").into_bytes();
    completion.extend_from_slice(first_payload_text.as_bytes());
    if runtime.write(driver_fd, &completion).is_err() {
        return 334;
    }
    let _ = runtime.close(driver_fd);
    let device = match runtime.inspect_device("/dev/gpu0") {
        Ok(record) => record,
        Err(_) => return 301,
    };
    let driver = match runtime.inspect_driver("/drv/gpu0") {
        Ok(record) => record,
        Err(_) => return 302,
    };
    let display = match runtime.inspect_gpu_display("/dev/gpu0") {
        Ok(record) => record,
        Err(_) => return 303,
    };
    let scanout = match runtime.inspect_gpu_scanout("/dev/gpu0") {
        Ok(record) => record,
        Err(_) => return 304,
    };
    let request = match runtime.inspect_device_request(request_id) {
        Ok(record) => record,
        Err(_) => return 305,
    };
    let mut frame = [0u8; 2048];
    let frame_len = match runtime.read_gpu_scanout_frame("/dev/gpu0", &mut frame) {
        Ok(count) => count,
        Err(_) => return 306,
    };

    if display.present == 0 || display.active_pipes == 0 || display.planned_frames == 0 {
        return 307;
    }
    if scanout.presented_frames == 0 {
        return 308;
    }
    if fixed_text_field(&scanout.last_frame_tag) != "qemu-compat-001" {
        return 309;
    }
    if fixed_text_field(&scanout.last_source_api_name) != "directx12" {
        return 310;
    }
    if fixed_text_field(&scanout.last_translation_label) != "compat-to-vulkan" {
        return 311;
    }
    if request.state != 2 || fixed_text_field(&request.frame_tag) != "qemu-compat-001" {
        return 312;
    }
    if fixed_text_field(&request.source_api_name) != "directx12"
        || fixed_text_field(&request.translation_label) != "compat-to-vulkan"
    {
        return 313;
    }
    if request.response_len == 0 {
        return 314;
    }
    if fixed_text_field(&device.last_terminal_frame_tag) != "qemu-compat-001" {
        return 315;
    }
    if fixed_text_field(&driver.last_terminal_translation_label) != "compat-to-vulkan" {
        return 316;
    }
    if &frame[..frame_len] != first_payload_text.as_bytes() {
        return 317;
    }

    if write_line(
        runtime,
        &format!(
            "compat.gfx.smoke.success request={} frame={} api={} translation={} deep-ops={} display={}x{} pipes={} planned={} presented={} bytes={}",
            request_id,
            fixed_text_field(&request.frame_tag),
            fixed_text_field(&request.source_api_name),
            fixed_text_field(&request.translation_label),
            ngos_shell_gpu::summarize_graphics_deep_ops(&first_payload_text),
            display.last_present_len,
            scanout.last_frame_len,
            display.active_pipes,
            display.planned_frames,
            scanout.presented_frames,
            frame_len
        ),
    )
    .is_err()
    {
        return 318;
    }

    match runtime.inspect_device_request(request_id + 99) {
        Err(Errno::NoEnt) => {}
        _ => return 319,
    }
    if write_line(
        runtime,
        "compat.gfx.smoke.refusal request=missing errno=ENOENT outcome=expected",
    )
    .is_err()
    {
        return 320;
    }

    let recovery_request =
        match runtime.present_gpu_frame("/dev/gpu0", second_payload_text.as_bytes()) {
            Ok(id) => id as u64,
            Err(_) => return 321,
        };
    let recovery_driver_fd = match runtime.open_path("/drv/gpu0") {
        Ok(fd) => fd,
        Err(_) => return 335,
    };
    let mut recovery_driver_request = [0u8; 2048];
    let recovery_driver_request_len =
        match runtime.read(recovery_driver_fd, &mut recovery_driver_request) {
            Ok(count) => count,
            Err(_) => return 336,
        };
    if recovery_driver_request_len == 0 {
        return 337;
    }
    let mut recovery_completion = format!("complete-request:{recovery_request}\n").into_bytes();
    recovery_completion.extend_from_slice(second_payload_text.as_bytes());
    if runtime
        .write(recovery_driver_fd, &recovery_completion)
        .is_err()
    {
        return 338;
    }
    let _ = runtime.close(recovery_driver_fd);
    let recovery_scanout = match runtime.inspect_gpu_scanout("/dev/gpu0") {
        Ok(record) => record,
        Err(_) => return 322,
    };
    let recovery_request_record = match runtime.inspect_device_request(recovery_request) {
        Ok(record) => record,
        Err(_) => return 323,
    };
    let mut recovery_frame = [0u8; 2048];
    let recovery_frame_len = match runtime.read_gpu_scanout_frame("/dev/gpu0", &mut recovery_frame)
    {
        Ok(count) => count,
        Err(_) => return 324,
    };
    if fixed_text_field(&recovery_scanout.last_frame_tag) != "qemu-compat-002" {
        return 325;
    }
    if fixed_text_field(&recovery_scanout.last_source_api_name) != "opengl" {
        return 326;
    }
    if fixed_text_field(&recovery_request_record.translation_label) != "compat-to-vulkan" {
        return 327;
    }
    if &recovery_frame[..recovery_frame_len] != second_payload_text.as_bytes() {
        return 328;
    }
    if write_line(
        runtime,
        &format!(
            "compat.gfx.smoke.recovery request={} frame={} api={} translation={} deep-ops={} presented={} outcome=ok",
            recovery_request,
            fixed_text_field(&recovery_request_record.frame_tag),
            fixed_text_field(&recovery_request_record.source_api_name),
            fixed_text_field(&recovery_request_record.translation_label),
            ngos_shell_gpu::summarize_graphics_deep_ops(&second_payload_text),
            recovery_scanout.presented_frames
        ),
    )
    .is_err()
    {
        return 329;
    }
    if write_line(runtime, "compat-gfx-smoke-ok").is_err() {
        return 330;
    }
    0
}

pub fn encode_compat_audio_payload(
    api: ForeignAudioApi,
    script_text: &str,
) -> Result<Vec<u8>, ExitCode> {
    let script = MixScript::parse(script_text).map_err(|_| 331)?;
    let translator = AudioTranslator::new(api);
    let encoded = translator.translate(&script).map_err(|_| 332)?;
    let payload = format!(
        "{}\nsource-api={}\ntranslation={}",
        encoded.payload,
        api.name(),
        api.translation_label()
    );
    Ok(payload.into_bytes())
}

pub fn parse_driver_request_id(payload: &[u8]) -> Option<u64> {
    let text = core::str::from_utf8(payload).ok()?;
    let header = text.lines().next()?;
    header.strip_prefix("request:")?.parse::<u64>().ok()
}

pub fn resolve_device_request_id<B: SyscallBackend>(
    runtime: &Runtime<B>,
    driver_path: &str,
    driver_payload: &[u8],
) -> Option<u64> {
    parse_driver_request_id(driver_payload).or_else(|| {
        runtime
            .inspect_driver(driver_path)
            .ok()
            .map(|record| record.last_terminal_request_id)
            .filter(|request_id| *request_id != 0)
    })
}

pub fn compat_audio_roundtrip<B: SyscallBackend>(
    runtime: &Runtime<B>,
    payload: &[u8],
) -> Result<
    (
        u64,
        NativeDeviceRecord,
        NativeDriverRecord,
        NativeDeviceRequestRecord,
        Vec<u8>,
    ),
    ExitCode,
> {
    let device_fd = runtime.open_path("/dev/audio0").map_err(|_| 333)?;
    if runtime.write(device_fd, payload).is_err() {
        let _ = runtime.close(device_fd);
        return Err(334);
    }
    let _ = runtime.close(device_fd);

    let driver_fd = runtime.open_path("/drv/audio0").map_err(|_| 335)?;
    let mut driver_request = vec![0u8; 384];
    let driver_request_len = runtime
        .read(driver_fd, &mut driver_request)
        .map_err(|_| 336)?;
    if driver_request_len == 0 {
        let _ = runtime.close(driver_fd);
        return Err(337);
    }
    let request_id = resolve_device_request_id(
        runtime,
        "/drv/audio0",
        &driver_request[..driver_request_len],
    )
    .ok_or(338)?;
    let mut completion = format!("complete-request:{request_id}\n").into_bytes();
    completion.extend_from_slice(payload);
    if runtime.write(driver_fd, &completion).is_err() {
        let _ = runtime.close(driver_fd);
        return Err(339);
    }
    let _ = runtime.close(driver_fd);

    let device = runtime.inspect_device("/dev/audio0").map_err(|_| 340)?;
    let driver = runtime.inspect_driver("/drv/audio0").map_err(|_| 341)?;
    let request = runtime
        .inspect_device_request(request_id)
        .map_err(|_| 342)?;
    let completion_fd = runtime.open_path("/dev/audio0").map_err(|_| 343)?;
    let mut completion_payload = vec![0u8; 384];
    let completion_len = match runtime.read(completion_fd, &mut completion_payload) {
        Ok(count) => count,
        Err(_) => {
            let _ = runtime.close(completion_fd);
            return Err(344);
        }
    };
    let _ = runtime.close(completion_fd);
    completion_payload.truncate(completion_len);
    Ok((request_id, device, driver, request, completion_payload))
}

pub fn run_native_compat_audio_boot_smoke<B: SyscallBackend>(runtime: &Runtime<B>) -> ExitCode {
    let first_payload = match encode_compat_audio_payload(
        ForeignAudioApi::XAudio2,
        "rate=48000\nchannels=2\nstream=qemu-audio-001\nroute=music\nlatency-mode=interactive\nspatialization=world-3d\ncompletion=wait-drain\ntone=lead,440,120,0.800,-0.250,sine\n",
    ) {
        Ok(payload) => payload,
        Err(code) => return code,
    };
    let second_payload = match encode_compat_audio_payload(
        ForeignAudioApi::WebAudio,
        "rate=48000\nchannels=2\nstream=qemu-audio-002\nroute=effects\nlatency-mode=buffered\nspatialization=stereo\ncompletion=wait-batch\nclip=ambience,hangar-loop,2,0.650,0.100\n",
    ) {
        Ok(payload) => payload,
        Err(code) => return code,
    };

    let (request_id, device, driver, _request, completion_payload) =
        match compat_audio_roundtrip(runtime, &first_payload) {
            Ok(roundtrip) => roundtrip,
            Err(code) => return code,
        };

    if driver.last_terminal_state != 2 {
        return 344;
    }
    if fixed_text_field(&device.last_terminal_frame_tag) != "qemu-audio-001" {
        return 345;
    }
    if fixed_text_field(&device.last_terminal_source_api_name) != "xaudio2" {
        return 346;
    }
    if fixed_text_field(&driver.last_terminal_translation_label) != "compat-to-mixer" {
        return 347;
    }
    if completion_payload.as_slice() != first_payload.as_slice() {
        return 348;
    }
    if write_line(
        runtime,
        &format!(
            "compat.audio.smoke.success request={} stream={} api={} translation={} device-queue={}/{} driver-completed={} bytes={}",
            request_id,
            fixed_text_field(&device.last_terminal_frame_tag),
            fixed_text_field(&device.last_terminal_source_api_name),
            fixed_text_field(&driver.last_terminal_translation_label),
            device.queue_depth,
            device.queue_capacity,
            driver.completed_requests,
            completion_payload.len()
        ),
    )
    .is_err()
    {
        return 349;
    }

    match runtime.inspect_device_request(request_id + 99) {
        Err(Errno::NoEnt) => {}
        _ => return 350,
    }
    if write_line(
        runtime,
        "compat.audio.smoke.refusal request=missing errno=ENOENT outcome=expected",
    )
    .is_err()
    {
        return 351;
    }

    let (
        recovery_request_id,
        recovery_device,
        recovery_driver,
        _recovery_request,
        recovery_payload,
    ) = match compat_audio_roundtrip(runtime, &second_payload) {
        Ok(roundtrip) => roundtrip,
        Err(code) => return code + 19,
    };

    if fixed_text_field(&recovery_device.last_terminal_frame_tag) != "qemu-audio-002" {
        return 364;
    }
    if fixed_text_field(&recovery_device.last_terminal_source_api_name) != "webaudio" {
        return 365;
    }
    if fixed_text_field(&recovery_driver.last_terminal_translation_label) != "native-mixer" {
        return 366;
    }
    if recovery_payload.as_slice() != second_payload.as_slice() {
        return 367;
    }
    if write_line(
        runtime,
        &format!(
            "compat.audio.smoke.recovery request={} stream={} api={} translation={} completed={} outcome=ok",
            recovery_request_id,
            fixed_text_field(&recovery_device.last_terminal_frame_tag),
            fixed_text_field(&recovery_device.last_terminal_source_api_name),
            fixed_text_field(&recovery_driver.last_terminal_translation_label),
            recovery_device.completed_requests
        ),
    )
    .is_err()
    {
        return 368;
    }
    if write_line(runtime, "compat-audio-smoke-ok").is_err() {
        return 369;
    }
    0
}

pub fn encode_compat_input_payload(
    api: ForeignInputApi,
    script_text: &str,
) -> Result<Vec<u8>, ExitCode> {
    let script = InputScript::parse(script_text).map_err(|_| 370)?;
    let translator = InputTranslator::new(api);
    let encoded = translator.translate(&script).map_err(|_| 371)?;
    let payload = format!(
        "{}\nsource-api={}\ntranslation={}",
        encoded.payload,
        api.name(),
        api.translation_label()
    );
    Ok(payload.into_bytes())
}

pub fn compat_input_roundtrip<B: SyscallBackend>(
    runtime: &Runtime<B>,
    payload: &[u8],
) -> Result<
    (
        u64,
        NativeDeviceRecord,
        NativeDriverRecord,
        NativeDeviceRequestRecord,
        Vec<u8>,
    ),
    ExitCode,
> {
    let device_fd = runtime.open_path("/dev/input0").map_err(|_| 372)?;
    if runtime.write(device_fd, payload).is_err() {
        let _ = runtime.close(device_fd);
        return Err(373);
    }
    let _ = runtime.close(device_fd);

    let driver_fd = runtime.open_path("/drv/input0").map_err(|_| 374)?;
    let mut driver_request = vec![0u8; 384];
    let driver_request_len = runtime
        .read(driver_fd, &mut driver_request)
        .map_err(|_| 375)?;
    if driver_request_len == 0 {
        let _ = runtime.close(driver_fd);
        return Err(376);
    }
    let request_id = resolve_device_request_id(
        runtime,
        "/drv/input0",
        &driver_request[..driver_request_len],
    )
    .ok_or(377)?;
    let mut completion = format!("complete-request:{request_id}\n").into_bytes();
    completion.extend_from_slice(payload);
    if runtime.write(driver_fd, &completion).is_err() {
        let _ = runtime.close(driver_fd);
        return Err(378);
    }
    let _ = runtime.close(driver_fd);

    let device = runtime.inspect_device("/dev/input0").map_err(|_| 379)?;
    let driver = runtime.inspect_driver("/drv/input0").map_err(|_| 380)?;
    let request = runtime
        .inspect_device_request(request_id)
        .map_err(|_| 381)?;
    let completion_fd = runtime.open_path("/dev/input0").map_err(|_| 382)?;
    let mut completion_payload = vec![0u8; 384];
    let completion_len = match runtime.read(completion_fd, &mut completion_payload) {
        Ok(count) => count,
        Err(_) => {
            let _ = runtime.close(completion_fd);
            return Err(383);
        }
    };
    let _ = runtime.close(completion_fd);
    completion_payload.truncate(completion_len);
    Ok((request_id, device, driver, request, completion_payload))
}

pub fn run_native_compat_input_boot_smoke<B: SyscallBackend>(runtime: &Runtime<B>) -> ExitCode {
    let first_payload = match encode_compat_input_payload(
        ForeignInputApi::XInput,
        "device=gamepad\nfamily=xbox\nframe=qemu-input-001\nlayout=gamepad-standard\nkey-table=us-game\npointer-capture=relative-lock\ndelivery=wait-frame\nbutton=a,press\naxis=left-x,750\npointer=4,-2\n",
    ) {
        Ok(payload) => payload,
        Err(code) => return code,
    };
    let second_payload = match encode_compat_input_payload(
        ForeignInputApi::Evdev,
        "device=mouse\nfamily=evdev-mouse\nframe=qemu-input-002\nlayout=pointer-standard\nkey-table=us\npointer-capture=absolute\ndelivery=immediate\npointer=0,0\n",
    ) {
        Ok(payload) => payload,
        Err(code) => return code,
    };

    let (request_id, device, driver, _request, completion_payload) =
        match compat_input_roundtrip(runtime, &first_payload) {
            Ok(roundtrip) => roundtrip,
            Err(code) => return code,
        };

    if driver.last_terminal_state != 2 {
        return 383;
    }
    if fixed_text_field(&device.last_terminal_frame_tag) != "qemu-input-001" {
        return 384;
    }
    if fixed_text_field(&device.last_terminal_source_api_name) != "xinput" {
        return 385;
    }
    if fixed_text_field(&driver.last_terminal_translation_label) != "compat-to-input" {
        return 386;
    }
    if completion_payload.as_slice() != first_payload.as_slice() {
        return 387;
    }
    if write_line(
        runtime,
        &format!(
            "compat.input.smoke.success request={} frame={} api={} translation={} device-queue={}/{} driver-completed={} bytes={}",
            request_id,
            fixed_text_field(&device.last_terminal_frame_tag),
            fixed_text_field(&device.last_terminal_source_api_name),
            fixed_text_field(&driver.last_terminal_translation_label),
            device.queue_depth,
            device.queue_capacity,
            driver.completed_requests,
            completion_payload.len()
        ),
    )
    .is_err()
    {
        return 388;
    }

    match runtime.inspect_device_request(request_id + 99) {
        Err(Errno::NoEnt) => {}
        _ => return 389,
    }
    if write_line(
        runtime,
        "compat.input.smoke.refusal request=missing errno=ENOENT outcome=expected",
    )
    .is_err()
    {
        return 390;
    }

    let (
        recovery_request_id,
        recovery_device,
        recovery_driver,
        _recovery_request,
        recovery_payload,
    ) = match compat_input_roundtrip(runtime, &second_payload) {
        Ok(roundtrip) => roundtrip,
        Err(code) => return code + 19,
    };

    if fixed_text_field(&recovery_device.last_terminal_frame_tag) != "qemu-input-002" {
        return 393;
    }
    if fixed_text_field(&recovery_device.last_terminal_source_api_name) != "evdev" {
        return 394;
    }
    if fixed_text_field(&recovery_driver.last_terminal_translation_label) != "native-input" {
        return 395;
    }
    if recovery_payload.as_slice() != second_payload.as_slice() {
        return 396;
    }
    if write_line(
        runtime,
        &format!(
            "compat.input.smoke.recovery request={} frame={} api={} translation={} completed={} outcome=ok",
            recovery_request_id,
            fixed_text_field(&recovery_device.last_terminal_frame_tag),
            fixed_text_field(&recovery_device.last_terminal_source_api_name),
            fixed_text_field(&recovery_driver.last_terminal_translation_label),
            recovery_device.completed_requests
        ),
    )
    .is_err()
    {
        return 397;
    }
    if write_line(runtime, "compat-input-smoke-ok").is_err() {
        return 398;
    }
    0
}
