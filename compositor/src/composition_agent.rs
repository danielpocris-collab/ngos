use crate::chrome_agent::{ChromeStyle, chrome_ops_for_window};
use crate::surface_agent::{Surface, SurfaceRole};
use alloc::{format, vec::Vec};
use ngos_effects_pipeline::{Effect, EffectPipeline, EffectRect};
use ngos_gfx_translate::{DrawOp, RenderPassClass, RgbaColor};

pub fn compose_surface(surface: &Surface) -> Vec<DrawOp> {
    if !surface.visible {
        return Vec::new();
    }
    let rect = surface.rect;
    let mut ops = Vec::new();

    let pass_class = match surface.role {
        SurfaceRole::Background => RenderPassClass::Background,
        SurfaceRole::Panel => RenderPassClass::Panel,
        SurfaceRole::Window => RenderPassClass::Chrome,
        SurfaceRole::Overlay => RenderPassClass::Overlay,
        SurfaceRole::Cursor => RenderPassClass::Presentation,
    };

    ops.push(DrawOp::PushLayer {
        opacity: surface.alpha,
    });
    ops.push(DrawOp::BeginPass {
        label: format!("{}-{}", surface.pass_name, surface.id),
        class: pass_class,
    });
    ops.push(DrawOp::SetBlendMode {
        mode: surface.blend_mode,
    });
    ops.push(DrawOp::SetClip {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    });

    let effect_rect = EffectRect::new(rect.x, rect.y, rect.width, rect.height);
    if let Ok(mut pipeline) = EffectPipeline::new(effect_rect) {
        if let Some(ref shadow) = surface.material.shadow {
            let _ = pipeline.add(Effect::Shadow(*shadow));
        }
        if let Some(ref translucency) = surface.material.translucency {
            let _ = pipeline.add(Effect::Translucency(*translucency));
        }
        if let Some(ref backdrop) = surface.material.backdrop {
            let _ = pipeline.add(Effect::Backdrop(*backdrop));
        }
        if let Some(ref temporal) = surface.material.temporal {
            let _ = pipeline.add(Effect::Temporal(*temporal));
        }
        if let Some(ref gradient) = surface.material.gradient {
            let _ = pipeline.add(Effect::Gradient(gradient.clone()));
        }
        if let Ok(effect_ops) = pipeline.compile() {
            ops.extend(effect_ops);
        }
    }

    if surface.shadow_enabled && surface.material.shadow.is_none() {
        let shadow_spread = match surface.role {
            SurfaceRole::Window => 14,
            SurfaceRole::Overlay => 18,
            SurfaceRole::Panel => 10,
            SurfaceRole::Cursor | SurfaceRole::Background => 0,
        };
        let shadow_blur = match surface.role {
            SurfaceRole::Window => {
                if surface.focused {
                    28
                } else {
                    20
                }
            }
            SurfaceRole::Overlay => 30,
            SurfaceRole::Panel => 14,
            SurfaceRole::Cursor | SurfaceRole::Background => 0,
        };
        if shadow_spread > 0 && shadow_blur > 0 {
            ops.push(DrawOp::ShadowRect {
                x: rect.x.saturating_sub(shadow_spread),
                y: rect.y.saturating_sub(shadow_spread / 2),
                width: rect.width.saturating_add(shadow_spread * 2),
                height: rect.height.saturating_add(shadow_spread * 2),
                blur: shadow_blur,
                color: RgbaColor {
                    r: 0x04,
                    g: 0x08,
                    b: 0x12,
                    a: if surface.focused { 0x44 } else { 0x30 },
                },
            });
        }
    }

    if surface.material.backdrop.is_none()
        && surface.material.translucency.is_none()
        && let Some(opacity) = surface.backdrop_opacity
    {
        ops.push(DrawOp::Backdrop {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
            opacity,
        });
    }

    if surface.role != SurfaceRole::Background && surface.corner_radius > 0 {
        ops.push(DrawOp::RoundedRect {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
            radius: surface.corner_radius,
            color: RgbaColor {
                r: 0xfa,
                g: 0xfd,
                b: 0xff,
                a: match surface.role {
                    SurfaceRole::Panel => 0x0b,
                    SurfaceRole::Window => 0x10,
                    SurfaceRole::Overlay => 0x14,
                    SurfaceRole::Cursor | SurfaceRole::Background => 0x00,
                },
            },
        });
    }

    if let Some(color) = surface.material.accent_line {
        ops.push(DrawOp::Rect {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: 1,
            color,
        });
    }

    if surface.role == SurfaceRole::Window {
        let style = if surface.focused {
            ChromeStyle::focused_window()
        } else {
            ChromeStyle::default_window()
        };
        for op in chrome_ops_for_window(rect, &style, surface.focused, surface.title.as_deref()) {
            ops.push(op);
        }
    }

    for op in &surface.content {
        ops.push(op.clone());
    }

    ops.push(DrawOp::ClearClip);
    ops.push(DrawOp::ClearBlendMode);
    ops.push(DrawOp::EndPass);
    ops.push(DrawOp::PopLayer);

    ops
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface_agent::{Surface, SurfaceRect, SurfaceRole};
    use ngos_gfx_translate::RgbaColor;

    fn window_surface(id: u32, focused: bool) -> Surface {
        let mut s = Surface::new(
            id,
            SurfaceRole::Window,
            SurfaceRect {
                x: 100,
                y: 80,
                width: 800,
                height: 600,
            },
        )
        .unwrap();
        s.focused = focused;
        s
    }

    fn bg_surface() -> Surface {
        Surface::new(
            0,
            SurfaceRole::Background,
            SurfaceRect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
        )
        .unwrap()
    }

    #[test]
    fn invisible_surface_produces_no_ops() {
        let mut s = window_surface(1, false);
        s.visible = false;
        let ops = compose_surface(&s);
        assert!(ops.is_empty());
    }

    #[test]
    fn visible_surface_wrapped_in_push_pop_layer() {
        let s = bg_surface();
        let ops = compose_surface(&s);
        assert!(matches!(ops.first(), Some(DrawOp::PushLayer { .. })));
        assert!(matches!(ops.last(), Some(DrawOp::PopLayer)));
    }

    #[test]
    fn visible_surface_has_set_clip_and_clear_clip() {
        let s = bg_surface();
        let ops = compose_surface(&s);
        assert!(ops.iter().any(|op| matches!(op, DrawOp::SetClip { .. })));
        assert!(ops.iter().any(|op| matches!(op, DrawOp::ClearClip)));
    }

    #[test]
    fn window_surface_includes_chrome() {
        let s = window_surface(1, false);
        let ops = compose_surface(&s);
        assert!(ops.iter().any(|op| matches!(op, DrawOp::ShadowRect { .. })));
        assert!(
            ops.iter()
                .any(|op| matches!(op, DrawOp::RoundedRect { .. }))
        );
    }

    #[test]
    fn background_surface_has_no_chrome() {
        let s = bg_surface();
        let ops = compose_surface(&s);
        assert!(!ops.iter().any(|op| matches!(op, DrawOp::ShadowRect { .. })));
        assert!(
            !ops.iter()
                .any(|op| matches!(op, DrawOp::RoundedRect { .. }))
        );
    }

    #[test]
    fn surface_content_ops_are_included() {
        let mut s = window_surface(1, false);
        s.content.push(DrawOp::Rect {
            x: 110,
            y: 120,
            width: 200,
            height: 100,
            color: RgbaColor {
                r: 0xff,
                g: 0,
                b: 0,
                a: 0xff,
            },
        });
        let ops = compose_surface(&s);
        let has_content_rect = ops
            .iter()
            .any(|op| matches!(op, DrawOp::Rect { x: 110, y: 120, .. }));
        assert!(has_content_rect);
    }

    #[test]
    fn surface_opacity_set_in_push_layer() {
        let mut s = bg_surface();
        s.alpha = 200;
        let ops = compose_surface(&s);
        assert!(matches!(
            ops.first(),
            Some(DrawOp::PushLayer { opacity: 200 })
        ));
    }
}
