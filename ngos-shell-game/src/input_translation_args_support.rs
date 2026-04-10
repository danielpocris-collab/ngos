use ngos_input_translate::ForeignInputApi;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::write_line;

pub struct InputTranslationArgs<'a> {
    pub pid: u64,
    pub api_str: &'a str,
    pub path_str: &'a str,
    pub source_api: ForeignInputApi,
}

pub fn parse_input_translation_args<'a, B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &'a str,
) -> Result<InputTranslationArgs<'a>, ExitCode> {
    let mut parts = rest.splitn(3, ' ');
    let pid_str = parts.next().ok_or_else(|| {
        let _ = write_line(
            runtime,
            "usage: game-input-translate <pid> <api> <input-script>",
        );
        2i32
    })?;
    let api_str = parts.next().ok_or_else(|| {
        let _ = write_line(
            runtime,
            "usage: game-input-translate <pid> <api> <input-script>",
        );
        2i32
    })?;
    let path_str = parts.next().ok_or_else(|| {
        let _ = write_line(
            runtime,
            "usage: game-input-translate <pid> <api> <input-script>",
        );
        2i32
    })?;
    let pid = pid_str.trim().parse::<u64>().map_err(|_| {
        let _ = write_line(
            runtime,
            "usage: game-input-translate <pid> <api> <input-script>",
        );
        2i32
    })?;
    let source_api = ForeignInputApi::parse(api_str.trim()).ok_or_else(|| {
        let _ = write_line(
            runtime,
            &alloc::format!(
                "game.input.translate.refused pid={pid} api={api_str} reason=unknown-api"
            ),
        );
        297i32
    })?;
    Ok(InputTranslationArgs {
        pid,
        api_str,
        path_str,
        source_api,
    })
}
