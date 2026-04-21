use ngos_audio_translate::ForeignAudioApi;
use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::write_line;

pub struct AudioTranslationArgs<'a> {
    pub pid: u64,
    pub api_str: &'a str,
    pub path_str: &'a str,
    pub source_api: ForeignAudioApi,
}

pub fn parse_audio_translation_args<'a, B: SyscallBackend>(
    runtime: &Runtime<B>,
    rest: &'a str,
) -> Result<AudioTranslationArgs<'a>, ExitCode> {
    let mut parts = rest.splitn(3, ' ');
    let pid_str = parts.next().ok_or_else(|| {
        let _ = write_line(
            runtime,
            "usage: game-audio-translate <pid> <api> <mix-script>",
        );
        2i32
    })?;
    let api_str = parts.next().ok_or_else(|| {
        let _ = write_line(
            runtime,
            "usage: game-audio-translate <pid> <api> <mix-script>",
        );
        2i32
    })?;
    let path_str = parts.next().ok_or_else(|| {
        let _ = write_line(
            runtime,
            "usage: game-audio-translate <pid> <api> <mix-script>",
        );
        2i32
    })?;
    let pid = pid_str.trim().parse::<u64>().map_err(|_| {
        let _ = write_line(
            runtime,
            "usage: game-audio-translate <pid> <api> <mix-script>",
        );
        2i32
    })?;
    let source_api = ForeignAudioApi::parse(api_str.trim()).ok_or_else(|| {
        let _ = write_line(
            runtime,
            &alloc::format!(
                "game.audio.translate.refused pid={pid} api={api_str} reason=unknown-api"
            ),
        );
        296i32
    })?;
    Ok(AudioTranslationArgs {
        pid,
        api_str,
        path_str,
        source_api,
    })
}
