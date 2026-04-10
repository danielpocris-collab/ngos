extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use ngos_gfx_translate::{DrawOp, RgbaColor};

#[cfg(feature = "skia-preview")]
use skia_safe::{
    BlurStyle, EncodedImageFormat, Font, MaskFilter, Paint, PaintStyle, Rect, surfaces,
};

#[cfg(feature = "skia-preview")]
fn skia_color(color: RgbaColor) -> skia_safe::Color {
    skia_safe::Color::from_argb(color.a, color.r, color.g, color.b)
}

/// Skia-backed preview renderer for NGOS UI output.
#[cfg(feature = "skia-preview")]
pub struct UiSkiaPreview;

#[cfg(feature = "skia-preview")]
impl UiSkiaPreview {
    pub fn render_to_png(
        ops: &[DrawOp],
        width: u32,
        height: u32,
        output: &str,
    ) -> Result<(), String> {
        let mut surface = surfaces::raster_n32_premul((width as i32, height as i32))
            .ok_or_else(|| "failed to create surface".to_string())?;
        let canvas = surface.canvas();

        for op in ops {
            match op {
                DrawOp::Clear { color } => {
                    canvas.clear(skia_color(*color));
                }
                DrawOp::GradientRect {
                    x,
                    y,
                    width,
                    height,
                    top_left,
                    top_right,
                    ..
                } => {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_style(PaintStyle::Fill);
                    paint.set_color(skia_color(*top_left));
                    canvas.draw_rect(
                        Rect::from_xywh(*x as f32, *y as f32, *width as f32, *height as f32),
                        &paint,
                    );
                    paint.set_color(skia_color(*top_right));
                    canvas.draw_rect(
                        Rect::from_xywh(
                            *x as f32,
                            *y as f32,
                            *width as f32,
                            (*height as f32) * 0.5,
                        ),
                        &paint,
                    );
                }
                DrawOp::Line {
                    x0,
                    y0,
                    x1,
                    y1,
                    color,
                } => {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_style(PaintStyle::Stroke);
                    paint.set_stroke_width(1.0);
                    paint.set_color(skia_color(*color));
                    canvas.draw_line((*x0 as f32, *y0 as f32), (*x1 as f32, *y1 as f32), &paint);
                }
                DrawOp::Rect {
                    x,
                    y,
                    width,
                    height,
                    color,
                } => {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_style(PaintStyle::Fill);
                    paint.set_color(skia_color(*color));
                    canvas.draw_rect(
                        Rect::from_xywh(*x as f32, *y as f32, *width as f32, *height as f32),
                        &paint,
                    );
                }
                DrawOp::RoundedRect {
                    x,
                    y,
                    width,
                    height,
                    radius,
                    color,
                } => {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_style(PaintStyle::Fill);
                    paint.set_color(skia_color(*color));
                    canvas.draw_round_rect(
                        Rect::from_xywh(*x as f32, *y as f32, *width as f32, *height as f32),
                        *radius as f32,
                        *radius as f32,
                        &paint,
                    );
                }
                DrawOp::ShadowRect {
                    x,
                    y,
                    width,
                    height,
                    blur,
                    color,
                } => {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_style(PaintStyle::Fill);
                    paint.set_color(skia_color(*color));
                    paint.set_mask_filter(MaskFilter::blur(BlurStyle::Normal, *blur as f32, None));
                    canvas.draw_rect(
                        Rect::from_xywh(*x as f32, *y as f32, *width as f32, *height as f32),
                        &paint,
                    );
                }
                DrawOp::Backdrop {
                    x,
                    y,
                    width,
                    height,
                    opacity,
                } => {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_style(PaintStyle::Fill);
                    paint.set_color(skia_safe::Color::from_argb(*opacity, 0x0F, 0x16, 0x26));
                    canvas.draw_rect(
                        Rect::from_xywh(*x as f32, *y as f32, *width as f32, *height as f32),
                        &paint,
                    );
                }
                DrawOp::Text {
                    text,
                    x,
                    y,
                    size,
                    color,
                    ..
                } => {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_color(skia_color(*color));
                    let mut font = Font::default();
                    font.set_size(*size as f32);
                    canvas.draw_str(
                        text.as_str(),
                        (*x as f32, *y as f32 + *size as f32),
                        &font,
                        &paint,
                    );
                }
                DrawOp::Icon {
                    icon,
                    x,
                    y,
                    size,
                    color,
                } => {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_color(skia_color(*color));
                    let mut font = Font::default();
                    font.set_size(*size as f32);
                    let glyph = icon.to_string();
                    canvas.draw_str(&glyph, (*x as f32, *y as f32 + *size as f32), &font, &paint);
                }
                DrawOp::PushLayer { .. }
                | DrawOp::PopLayer
                | DrawOp::BeginPass { .. }
                | DrawOp::EndPass
                | DrawOp::SetBlendMode { .. }
                | DrawOp::ClearBlendMode
                | DrawOp::SetClip { .. }
                | DrawOp::ClearClip
                | DrawOp::SetPresentRegion { .. }
                | DrawOp::FlipRegion { .. }
                | DrawOp::Sprite { .. }
                | DrawOp::Triangle { .. }
                | DrawOp::Ellipse { .. }
                | DrawOp::Blit { .. }
                | DrawOp::GaussianBlur { .. }
                | DrawOp::Image { .. } => {}
            }
        }

        let image = surface.image_snapshot();
        let data = image
            .encode(None, EncodedImageFormat::PNG, None)
            .ok_or_else(|| "failed to encode png".to_string())?;
        std::fs::write(output, data.as_bytes()).map_err(|e| format!("{e:?}"))?;
        Ok(())
    }

    pub fn render_desktop_to_png(ui: &crate::UserInterface, output: &str) -> Result<(), String> {
        let (width, height) = ui.dimensions();
        Self::render_to_png(&ui.render_desktop(), width, height, output)
    }

    pub fn render_boot_to_png(
        ui: &crate::UserInterface,
        stage: crate::BootStage,
        output: &str,
    ) -> Result<(), String> {
        let (width, height) = ui.dimensions();
        Self::render_to_png(&ui.render_boot(stage), width, height, output)
    }

    pub fn render_suite_to_png(ui: &crate::UserInterface, output: &str) -> Result<(), String> {
        let (width, height) = ui.dimensions();
        let panel_width = width.max(1);
        let suite_width = panel_width.saturating_mul(3).saturating_add(48);
        let suite_height = height.max(1);

        let mut surface = surfaces::raster_n32_premul((suite_width as i32, suite_height as i32))
            .ok_or_else(|| String::from("failed to create suite surface"))?;
        let canvas = surface.canvas();

        canvas.clear(skia_safe::Color::from_argb(0xff, 0x08, 0x0a, 0x12));

        let mut boot_loading =
            surfaces::raster_n32_premul((panel_width as i32, suite_height as i32))
                .ok_or_else(|| String::from("failed to create boot loading surface"))?;
        let mut boot_complete =
            surfaces::raster_n32_premul((panel_width as i32, suite_height as i32))
                .ok_or_else(|| String::from("failed to create boot complete surface"))?;
        let mut desktop = surfaces::raster_n32_premul((panel_width as i32, suite_height as i32))
            .ok_or_else(|| String::from("failed to create desktop surface"))?;

        Self::paint_ops(
            boot_loading.canvas(),
            &ui.render_boot(crate::BootStage::Loading),
        );
        Self::paint_ops(
            boot_complete.canvas(),
            &ui.render_boot(crate::BootStage::Complete),
        );
        Self::paint_ops(desktop.canvas(), &ui.render_desktop());

        let boot_loading_image = boot_loading.image_snapshot();
        let boot_complete_image = boot_complete.image_snapshot();
        let desktop_image = desktop.image_snapshot();
        let mut paint = Paint::default();
        paint.set_anti_alias(true);

        let boot_loading_rect = Rect::from_xywh(0.0, 0.0, panel_width as f32, suite_height as f32);
        let boot_complete_rect = Rect::from_xywh(
            (panel_width + 16) as f32,
            0.0,
            panel_width as f32,
            suite_height as f32,
        );
        let desktop_rect = Rect::from_xywh(
            (panel_width * 2 + 32) as f32,
            0.0,
            panel_width as f32,
            suite_height as f32,
        );
        canvas.draw_image_rect(boot_loading_image, None, boot_loading_rect, &paint);
        canvas.draw_image_rect(boot_complete_image, None, boot_complete_rect, &paint);
        canvas.draw_image_rect(desktop_image, None, desktop_rect, &paint);

        let divider_a_x = panel_width + 8;
        let divider_b_x = panel_width * 2 + 24;
        let mut divider_paint = Paint::default();
        divider_paint.set_color(skia_safe::Color::from_argb(0xff, 0x23, 0x2f, 0x46));
        canvas.draw_rect(
            Rect::from_xywh(
                divider_a_x as f32,
                24.0,
                2.0,
                suite_height.saturating_sub(48) as f32,
            ),
            &divider_paint,
        );
        canvas.draw_rect(
            Rect::from_xywh(
                divider_b_x as f32,
                24.0,
                2.0,
                suite_height.saturating_sub(48) as f32,
            ),
            &divider_paint,
        );

        let label_paint = {
            let mut p = Paint::default();
            p.set_anti_alias(true);
            p.set_color(skia_safe::Color::from_argb(0xff, 0xe8, 0xed, 0xf5));
            p
        };
        let mut font = Font::default();
        font.set_size(20.0);
        canvas.draw_str("NGOS boot loading", (24.0, 32.0), &font, &label_paint);
        canvas.draw_str(
            "NGOS boot complete",
            ((panel_width + 32) as f32, 32.0),
            &font,
            &label_paint,
        );
        canvas.draw_str(
            "NGOS desktop",
            ((panel_width * 2 + 48) as f32, 32.0),
            &font,
            &label_paint,
        );

        let image = surface.image_snapshot();
        let data = image
            .encode(None, EncodedImageFormat::PNG, None)
            .ok_or_else(|| String::from("failed to encode suite png"))?;
        std::fs::write(output, data.as_bytes()).map_err(|e| format!("{e:?}"))?;
        Ok(())
    }

    pub fn render_master_suite_to_png(
        ui: &crate::UserInterface,
        output: &str,
    ) -> Result<(), String> {
        let (width, height) = ui.dimensions();
        let panel_width = width.max(1);
        let panel_height = height.max(1);
        let cols = 3u32;
        let rows = 2u32;
        let gap = 24u32;
        let suite_width = panel_width
            .saturating_mul(cols)
            .saturating_add(gap * (cols + 1));
        let suite_height = panel_height
            .saturating_mul(rows)
            .saturating_add(gap * (rows + 1));

        let mut surface = surfaces::raster_n32_premul((suite_width as i32, suite_height as i32))
            .ok_or_else(|| String::from("failed to create master suite surface"))?;
        let canvas = surface.canvas();
        canvas.clear(skia_safe::Color::from_argb(0xff, 0x08, 0x0a, 0x12));

        let panels: [(&str, Vec<DrawOp>); 6] = [
            ("Boot loading", ui.render_boot(crate::BootStage::Loading)),
            ("Boot complete", ui.render_boot(crate::BootStage::Complete)),
            ("Desktop", ui.render_desktop()),
            ("Widgets", {
                let mut temp = crate::UserInterface::new(width, height);
                temp.toggle_widgets_panel();
                temp.render_desktop()
            }),
            ("Notifications", {
                let mut temp = crate::UserInterface::new(width, height);
                temp.toggle_notification_center();
                temp.render_desktop()
            }),
            ("Control center", {
                let mut temp = crate::UserInterface::new(width, height);
                temp.toggle_control_center();
                temp.render_desktop()
            }),
        ];

        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        let mut title_paint = Paint::default();
        title_paint.set_anti_alias(true);
        title_paint.set_color(skia_safe::Color::from_argb(0xff, 0xe8, 0xed, 0xf5));
        let mut font = Font::default();
        font.set_size(18.0);

        for (idx, (label, ops)) in panels.iter().enumerate() {
            let col = (idx as u32) % cols;
            let row = (idx as u32) / cols;
            let x = gap + col * (panel_width + gap);
            let y = gap + row * (panel_height + gap);

            let mut frame = surfaces::raster_n32_premul((panel_width as i32, panel_height as i32))
                .ok_or_else(|| String::from("failed to create master suite frame"))?;
            Self::paint_ops(frame.canvas(), ops);
            let image = frame.image_snapshot();
            let rect = Rect::from_xywh(x as f32, y as f32, panel_width as f32, panel_height as f32);
            canvas.draw_image_rect(image, None, rect, &paint);
            canvas.draw_str(
                label,
                (x as f32 + 16.0, y as f32 + 28.0),
                &font,
                &title_paint,
            );
        }

        let image = surface.image_snapshot();
        let data = image
            .encode(None, EncodedImageFormat::PNG, None)
            .ok_or_else(|| String::from("failed to encode master suite png"))?;
        std::fs::write(output, data.as_bytes()).map_err(|e| format!("{e:?}"))?;
        Ok(())
    }

    fn paint_ops(canvas: &skia_safe::Canvas, ops: &[DrawOp]) {
        for op in ops {
            match op {
                DrawOp::Clear { color } => {
                    canvas.clear(skia_color(*color));
                }
                DrawOp::GradientRect {
                    x,
                    y,
                    width,
                    height,
                    top_left,
                    top_right,
                    ..
                } => {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_style(PaintStyle::Fill);
                    paint.set_color(skia_color(*top_left));
                    canvas.draw_rect(
                        Rect::from_xywh(*x as f32, *y as f32, *width as f32, *height as f32),
                        &paint,
                    );
                    paint.set_color(skia_color(*top_right));
                    canvas.draw_rect(
                        Rect::from_xywh(
                            *x as f32,
                            *y as f32,
                            *width as f32,
                            (*height as f32) * 0.5,
                        ),
                        &paint,
                    );
                }
                DrawOp::Line {
                    x0,
                    y0,
                    x1,
                    y1,
                    color,
                } => {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_style(PaintStyle::Stroke);
                    paint.set_stroke_width(1.0);
                    paint.set_color(skia_color(*color));
                    canvas.draw_line((*x0 as f32, *y0 as f32), (*x1 as f32, *y1 as f32), &paint);
                }
                DrawOp::Rect {
                    x,
                    y,
                    width,
                    height,
                    color,
                } => {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_style(PaintStyle::Fill);
                    paint.set_color(skia_color(*color));
                    canvas.draw_rect(
                        Rect::from_xywh(*x as f32, *y as f32, *width as f32, *height as f32),
                        &paint,
                    );
                }
                DrawOp::RoundedRect {
                    x,
                    y,
                    width,
                    height,
                    radius,
                    color,
                } => {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_style(PaintStyle::Fill);
                    paint.set_color(skia_color(*color));
                    canvas.draw_round_rect(
                        Rect::from_xywh(*x as f32, *y as f32, *width as f32, *height as f32),
                        *radius as f32,
                        *radius as f32,
                        &paint,
                    );
                }
                DrawOp::ShadowRect {
                    x,
                    y,
                    width,
                    height,
                    blur,
                    color,
                } => {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_style(PaintStyle::Fill);
                    paint.set_color(skia_color(*color));
                    paint.set_mask_filter(MaskFilter::blur(BlurStyle::Normal, *blur as f32, None));
                    canvas.draw_rect(
                        Rect::from_xywh(*x as f32, *y as f32, *width as f32, *height as f32),
                        &paint,
                    );
                }
                DrawOp::Backdrop {
                    x,
                    y,
                    width,
                    height,
                    opacity,
                } => {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_style(PaintStyle::Fill);
                    paint.set_color(skia_safe::Color::from_argb(*opacity, 0x0F, 0x16, 0x26));
                    canvas.draw_rect(
                        Rect::from_xywh(*x as f32, *y as f32, *width as f32, *height as f32),
                        &paint,
                    );
                }
                DrawOp::Text {
                    text,
                    x,
                    y,
                    size,
                    color,
                    ..
                } => {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_color(skia_color(*color));
                    let mut font = Font::default();
                    font.set_size(*size as f32);
                    canvas.draw_str(
                        text.as_str(),
                        (*x as f32, *y as f32 + *size as f32),
                        &font,
                        &paint,
                    );
                }
                DrawOp::Icon {
                    icon,
                    x,
                    y,
                    size,
                    color,
                } => {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_color(skia_color(*color));
                    let mut font = Font::default();
                    font.set_size(*size as f32);
                    let glyph = icon.to_string();
                    canvas.draw_str(&glyph, (*x as f32, *y as f32 + *size as f32), &font, &paint);
                }
                DrawOp::PushLayer { .. }
                | DrawOp::PopLayer
                | DrawOp::BeginPass { .. }
                | DrawOp::EndPass
                | DrawOp::SetBlendMode { .. }
                | DrawOp::ClearBlendMode
                | DrawOp::SetClip { .. }
                | DrawOp::ClearClip
                | DrawOp::SetPresentRegion { .. }
                | DrawOp::FlipRegion { .. }
                | DrawOp::Sprite { .. }
                | DrawOp::Triangle { .. }
                | DrawOp::Ellipse { .. }
                | DrawOp::Blit { .. }
                | DrawOp::GaussianBlur { .. }
                | DrawOp::Image { .. } => {}
            }
        }
    }
}

#[cfg(not(feature = "skia-preview"))]
pub struct UiSkiaPreview;

#[cfg(all(test, feature = "skia-preview"))]
mod tests {
    use super::*;
    use crate::{BootStage, UserInterface};

    #[test]
    fn skia_preview_renders_desktop_and_boot_png() {
        let ui = UserInterface::new(320, 240);
        let desktop_path = std::env::temp_dir().join("ngos-ui-desktop-test.png");
        let boot_path = std::env::temp_dir().join("ngos-ui-boot-test.png");

        UiSkiaPreview::render_desktop_to_png(&ui, desktop_path.to_str().unwrap())
            .expect("desktop PNG should render");
        UiSkiaPreview::render_boot_to_png(&ui, BootStage::Loading, boot_path.to_str().unwrap())
            .expect("boot PNG should render");

        assert!(std::fs::metadata(&desktop_path).unwrap().len() > 0);
        assert!(std::fs::metadata(&boot_path).unwrap().len() > 0);

        let _ = std::fs::remove_file(desktop_path);
        let _ = std::fs::remove_file(boot_path);
    }
}
