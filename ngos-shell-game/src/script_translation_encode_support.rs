use ngos_audio_translate::{EncodedMix, MixScript};
use ngos_gfx_translate::{EncodedFrame, FrameScript};
use ngos_input_translate::{EncodedInput, InputScript};
use ngos_user_abi::ExitCode;

use crate::GameCompatSession;

pub fn game_encode_frame(
    session: &GameCompatSession,
    script: &FrameScript,
) -> Result<EncodedFrame, ExitCode> {
    script.validate().map_err(|_| 291)?;
    Ok(script.encode(&session.graphics_profile))
}

pub fn game_encode_mix(
    session: &GameCompatSession,
    script: &MixScript,
) -> Result<EncodedMix, ExitCode> {
    script.validate().map_err(|_| 292)?;
    Ok(script.encode(&session.audio_profile))
}

pub fn game_encode_input(
    session: &GameCompatSession,
    script: &InputScript,
) -> Result<EncodedInput, ExitCode> {
    script.validate().map_err(|_| 296)?;
    Ok(script.encode(&session.input_profile))
}
