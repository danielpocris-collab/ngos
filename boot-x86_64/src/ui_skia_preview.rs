#[cfg(not(target_os = "none"))]
use ngos_ui::UserInterface;

#[cfg(not(target_os = "none"))]
pub fn render_boot_suite_preview(output: &str, width: u32, height: u32) -> Result<(), String> {
    let ui = UserInterface::new(width, height);
    ui.render_master_suite_png(output)
}

#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use super::*;

    #[test]
    fn boot_crate_can_render_ui_master_suite_png() {
        let output = std::env::temp_dir().join("ngos-boot-master-suite.png");
        render_boot_suite_preview(output.to_str().unwrap(), 640, 360).expect("suite png");
        assert!(std::fs::metadata(&output).unwrap().len() > 0);
        let _ = std::fs::remove_file(output);
    }
}
