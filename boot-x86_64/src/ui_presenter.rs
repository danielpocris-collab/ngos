#[cfg(target_os = "none")]
use crate::ui_framebuffer::FramebufferRenderer;

use ngos_gfx_translate::DrawOp;
#[cfg(not(target_os = "none"))]
use ngos_ui::UiPresenter;
#[cfg(target_os = "none")]
use platform_x86_64::FramebufferInfo;

/// Unified boot UI presenter.
///
/// Host builds use Skia through `ngos-ui`.
/// Native boot builds use the framebuffer renderer directly.
pub struct BootUiPresenter;

impl BootUiPresenter {
    #[cfg(not(target_os = "none"))]
    pub fn present_ops_to_png(
        ops: &[DrawOp],
        width: u32,
        height: u32,
        output: &str,
    ) -> Result<(), String> {
        UiPresenter::present_ops(ops, width, height, output)
    }

    #[cfg(target_os = "none")]
    pub fn present_ops_to_framebuffer(
        framebuffer: &mut [u8],
        info: FramebufferInfo,
        ops: &[DrawOp],
    ) {
        let mut renderer = FramebufferRenderer::new(
            framebuffer,
            info.width,
            info.height,
            info.pitch,
            info.bpp.into(),
        );
        renderer.render_ops(ops);
    }
}

#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use super::*;
    use ngos_ui::{BootStage, UserInterface};

    #[test]
    fn host_presenter_uses_skia_via_ui() {
        let ui = UserInterface::new(320, 240);
        let output = std::env::temp_dir().join("ngos-boot-host-presenter.png");
        BootUiPresenter::present_ops_to_png(
            &ui.render_boot(BootStage::Loading),
            320,
            240,
            output.to_str().unwrap(),
        )
        .expect("host presenter should render png");
        assert!(std::fs::metadata(&output).unwrap().len() > 0);
        let _ = std::fs::remove_file(output);
    }
}
