#[cfg(feature = "skia-preview")]
use alloc::string::String;

#[cfg(feature = "skia-preview")]
use ngos_gfx_translate::DrawOp;

#[cfg(feature = "skia-preview")]
use crate::skia_preview::UiSkiaPreview;

/// Presentation backend used by NGOS UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiPresentationBackend {
    Skia,
}

/// Unified presentation entrypoint for UI output.
///
/// On host-side builds, NGOS currently presents through Skia by default.
pub struct UiPresenter;

impl UiPresenter {
    #[cfg(feature = "skia-preview")]
    pub fn present_ops(
        ops: &[DrawOp],
        width: u32,
        height: u32,
        output: &str,
    ) -> Result<(), String> {
        UiSkiaPreview::render_to_png(ops, width, height, output)
    }

    #[cfg(feature = "skia-preview")]
    pub fn backend() -> UiPresentationBackend {
        UiPresentationBackend::Skia
    }
}
