//! Canonical subsystem role:
//! - subsystem: UI preview auxiliary surface
//! - owner layer: auxiliary presentation layer
//! - semantic owner: `ui-preview`
//! - truth path role: preview-only rendering surface for development and visual
//!   inspection, not product truth
//!
//! Canonical contract families handled here:
//! - UI preview entry contracts
//! - auxiliary preview rendering contracts
//! - preview export contracts
//!
//! This crate may render preview artifacts for development, but it must not be
//! treated as a final truth surface for subsystem closure.

use ngos_ui::{BootStage, UserInterface};

fn main() {
    let mut args = std::env::args().skip(1);
    let mode = args.next().unwrap_or_else(|| "desktop".to_string());
    let output = args
        .next()
        .unwrap_or_else(|| "ngos-ui-preview.png".to_string());
    let width = args.next().and_then(|v| v.parse().ok()).unwrap_or(1280);
    let height = args.next().and_then(|v| v.parse().ok()).unwrap_or(720);

    let ui = UserInterface::new(width, height);
    let result = match mode.as_str() {
        "desktop" => ui.render_desktop_png(&output),
        "boot" => ui.render_boot_png(BootStage::Loading, &output),
        "both" => {
            let desktop_output = format!("{output}.desktop.png");
            let boot_output = format!("{output}.boot.png");
            ui.render_desktop_png(&desktop_output)
                .and_then(|_| ui.render_boot_png(BootStage::Loading, &boot_output))
        }
        "suite" => ui.render_suite_png(&output),
        "master" => ui.render_master_suite_png(&output),
        other => {
            eprintln!(
                "unknown mode '{other}', expected 'boot', 'desktop', 'both', 'suite', or 'master'"
            );
            std::process::exit(2);
        }
    };

    if let Err(error) = result {
        eprintln!("render failed: {error}");
        std::process::exit(1);
    }

    match mode.as_str() {
        "both" => println!(
            "NGOS UI boot+desktop previews written to {output}.boot.png and {output}.desktop.png"
        ),
        "suite" => println!("NGOS UI suite preview written to {output}"),
        "master" => println!("NGOS UI master suite preview written to {output}"),
        _ => println!("NGOS UI {mode} preview written to {output}"),
    }
}
