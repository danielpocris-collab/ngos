use ngos_audio_translate::MixScript;
use ngos_gfx_translate::FrameScript;
use ngos_input_translate::InputScript;
use ngos_shell_vfs::shell_read_file_text;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

pub fn game_load_frame_script<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<FrameScript, ExitCode> {
    let text = shell_read_file_text(runtime, path)?;
    FrameScript::parse(&text).map_err(|_| 291)
}

pub fn game_load_mix_script<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<MixScript, ExitCode> {
    let text = shell_read_file_text(runtime, path)?;
    MixScript::parse(&text).map_err(|_| 292)
}

pub fn game_load_input_script<B: SyscallBackend>(
    runtime: &Runtime<B>,
    path: &str,
) -> Result<InputScript, ExitCode> {
    let text = shell_read_file_text(runtime, path)?;
    InputScript::parse(&text).map_err(|_| 296)
}
