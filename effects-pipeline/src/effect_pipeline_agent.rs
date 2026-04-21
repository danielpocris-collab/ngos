use crate::{
    EffectError, EffectRect, backdrop_agent::BackdropSpec, gradient_agent::GradientSpec,
    shadow_agent::ShadowSpec, temporal_agent::TemporalSpec, translucency_agent::TranslucencySpec,
};
use alloc::vec::Vec;
use ngos_gfx_translate::DrawOp;

pub enum Effect {
    Gradient(GradientSpec),
    Shadow(ShadowSpec),
    Translucency(TranslucencySpec),
    Backdrop(BackdropSpec),
    Temporal(TemporalSpec),
}

pub struct PipelineInspect {
    pub rect: EffectRect,
    pub effect_count: usize,
    pub gradient_count: usize,
    pub shadow_count: usize,
    pub translucency_count: usize,
    pub backdrop_count: usize,
    pub temporal_count: usize,
}

pub struct EffectPipeline {
    rect: EffectRect,
    effects: Vec<Effect>,
}

impl EffectPipeline {
    pub fn new(rect: EffectRect) -> Result<Self, EffectError> {
        if rect.width == 0 || rect.height == 0 {
            return Err(EffectError::ZeroDimensions);
        }
        Ok(EffectPipeline {
            rect,
            effects: Vec::new(),
        })
    }

    pub fn add(&mut self, effect: Effect) -> Result<(), EffectError> {
        match &effect {
            Effect::Gradient(g) => g.validate()?,
            Effect::Shadow(_) => {}
            Effect::Translucency(_) => {}
            Effect::Backdrop(b) => b.validate()?,
            Effect::Temporal(_) => {}
        }
        self.effects.push(effect);
        Ok(())
    }

    pub fn compile(&self) -> Result<Vec<DrawOp>, EffectError> {
        if self.effects.is_empty() {
            return Err(EffectError::EmptyPipeline);
        }
        let mut ops = Vec::new();
        for effect in &self.effects {
            match effect {
                Effect::Gradient(g) => ops.extend(g.to_draw_ops(self.rect)),
                Effect::Shadow(s) => ops.extend(s.to_draw_ops(self.rect)),
                Effect::Translucency(t) => ops.extend(t.to_draw_ops(self.rect)),
                Effect::Backdrop(b) => ops.extend(b.to_draw_ops(self.rect)),
                Effect::Temporal(t) => ops.extend(t.to_draw_ops(self.rect)),
            }
        }
        Ok(ops)
    }

    pub fn inspect(&self) -> PipelineInspect {
        let mut gradient_count = 0;
        let mut shadow_count = 0;
        let mut translucency_count = 0;
        let mut backdrop_count = 0;
        let mut temporal_count = 0;
        for effect in &self.effects {
            match effect {
                Effect::Gradient(_) => gradient_count += 1,
                Effect::Shadow(_) => shadow_count += 1,
                Effect::Translucency(_) => translucency_count += 1,
                Effect::Backdrop(_) => backdrop_count += 1,
                Effect::Temporal(_) => temporal_count += 1,
            }
        }
        PipelineInspect {
            rect: self.rect,
            effect_count: self.effects.len(),
            gradient_count,
            shadow_count,
            translucency_count,
            backdrop_count,
            temporal_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gradient_agent::{GradientDirection, GradientStop},
        shadow_agent::ShadowSpec,
        temporal_agent::{AccentPulse, TemporalState},
    };
    use ngos_gfx_translate::RgbaColor;

    fn rect() -> EffectRect {
        EffectRect::new(0, 0, 400, 200)
    }

    fn color(r: u8, g: u8, b: u8) -> RgbaColor {
        RgbaColor { r, g, b, a: 255 }
    }

    fn two_stop_gradient() -> Effect {
        Effect::Gradient(GradientSpec {
            direction: GradientDirection::Horizontal,
            stops: vec![
                GradientStop {
                    position: 0,
                    color: color(0, 0, 0),
                },
                GradientStop {
                    position: 255,
                    color: color(255, 255, 255),
                },
            ],
        })
    }

    fn shadow() -> Effect {
        Effect::Shadow(ShadowSpec {
            offset_x: 0,
            offset_y: 4,
            blur: 8,
            spread: 0,
            color: RgbaColor {
                r: 0,
                g: 0,
                b: 0,
                a: 80,
            },
        })
    }

    fn translucency() -> Effect {
        Effect::Translucency(TranslucencySpec {
            opacity: 180,
            blur_radius: 6,
        })
    }

    fn backdrop() -> Effect {
        Effect::Backdrop(BackdropSpec {
            blur_radius: 10,
            tint_color: color(10, 15, 25),
            tint_opacity: 140,
        })
    }

    fn temporal() -> Effect {
        Effect::Temporal(TemporalSpec {
            state: TemporalState { tick: 0, stride: 2 },
            color: color(100, 180, 255),
            accent: AccentPulse {
                base_alpha: 60,
                pulse_range: 40,
            },
        })
    }

    #[test]
    fn zero_dimensions_refused() {
        match EffectPipeline::new(EffectRect::new(0, 0, 0, 100)) {
            Err(e) => assert!(matches!(e, EffectError::ZeroDimensions)),
            Ok(_) => panic!("expected ZeroDimensions error"),
        }
    }

    #[test]
    fn empty_pipeline_compile_refused() {
        let pipeline = EffectPipeline::new(rect()).unwrap();
        match pipeline.compile() {
            Err(e) => {
                assert!(matches!(e, EffectError::EmptyPipeline));
                assert!(e.describe().contains("empty"));
            }
            Ok(_) => panic!("expected EmptyPipeline error"),
        }
    }

    #[test]
    fn gradient_with_one_stop_refused_at_add() {
        let mut pipeline = EffectPipeline::new(rect()).unwrap();
        let bad = Effect::Gradient(GradientSpec {
            direction: GradientDirection::Horizontal,
            stops: vec![GradientStop {
                position: 0,
                color: color(0, 0, 0),
            }],
        });
        let err = pipeline.add(bad).unwrap_err();
        assert!(matches!(
            err,
            EffectError::InsufficientGradientStops { count: 1 }
        ));
    }

    #[test]
    fn backdrop_with_zero_blur_refused_at_add() {
        let mut pipeline = EffectPipeline::new(rect()).unwrap();
        let bad = Effect::Backdrop(BackdropSpec {
            blur_radius: 0,
            tint_color: color(10, 10, 10),
            tint_opacity: 100,
        });
        let err = pipeline.add(bad).unwrap_err();
        assert!(matches!(err, EffectError::BackdropRequiresBlur));
    }

    #[test]
    fn pipeline_compiles_gradient() {
        let mut pipeline = EffectPipeline::new(rect()).unwrap();
        pipeline.add(two_stop_gradient()).unwrap();
        let ops = pipeline.compile().unwrap();
        assert!(!ops.is_empty());
        assert!(
            ops.iter()
                .any(|op| matches!(op, DrawOp::GradientRect { .. }))
        );
    }

    #[test]
    fn pipeline_compiles_shadow() {
        let mut pipeline = EffectPipeline::new(rect()).unwrap();
        pipeline.add(shadow()).unwrap();
        let ops = pipeline.compile().unwrap();
        assert!(ops.iter().any(|op| matches!(op, DrawOp::ShadowRect { .. })));
    }

    #[test]
    fn pipeline_compiles_translucency() {
        let mut pipeline = EffectPipeline::new(rect()).unwrap();
        pipeline.add(translucency()).unwrap();
        let ops = pipeline.compile().unwrap();
        assert!(
            ops.iter()
                .any(|op| matches!(op, DrawOp::GaussianBlur { .. }))
        );
        assert!(ops.iter().any(|op| matches!(op, DrawOp::Backdrop { .. })));
    }

    #[test]
    fn pipeline_compiles_backdrop() {
        let mut pipeline = EffectPipeline::new(rect()).unwrap();
        pipeline.add(backdrop()).unwrap();
        let ops = pipeline.compile().unwrap();
        assert!(
            ops.iter()
                .any(|op| matches!(op, DrawOp::GaussianBlur { .. }))
        );
        assert!(ops.iter().any(|op| matches!(op, DrawOp::Backdrop { .. })));
        assert!(ops.iter().any(|op| matches!(op, DrawOp::Rect { .. })));
    }

    #[test]
    fn pipeline_compiles_temporal() {
        let mut pipeline = EffectPipeline::new(rect()).unwrap();
        pipeline.add(temporal()).unwrap();
        let ops = pipeline.compile().unwrap();
        assert!(ops.iter().any(|op| matches!(op, DrawOp::Rect { .. })));
    }

    #[test]
    fn pipeline_chains_multiple_effects() {
        let mut pipeline = EffectPipeline::new(rect()).unwrap();
        pipeline.add(two_stop_gradient()).unwrap();
        pipeline.add(shadow()).unwrap();
        pipeline.add(translucency()).unwrap();
        let ops = pipeline.compile().unwrap();
        assert!(
            ops.iter()
                .any(|op| matches!(op, DrawOp::GradientRect { .. }))
        );
        assert!(ops.iter().any(|op| matches!(op, DrawOp::ShadowRect { .. })));
        assert!(ops.iter().any(|op| matches!(op, DrawOp::Backdrop { .. })));
    }

    #[test]
    fn inspect_counts_effects_by_type() {
        let mut pipeline = EffectPipeline::new(rect()).unwrap();
        pipeline.add(two_stop_gradient()).unwrap();
        pipeline.add(shadow()).unwrap();
        pipeline.add(translucency()).unwrap();
        pipeline.add(backdrop()).unwrap();
        pipeline.add(temporal()).unwrap();
        let insp = pipeline.inspect();
        assert_eq!(insp.effect_count, 5);
        assert_eq!(insp.gradient_count, 1);
        assert_eq!(insp.shadow_count, 1);
        assert_eq!(insp.translucency_count, 1);
        assert_eq!(insp.backdrop_count, 1);
        assert_eq!(insp.temporal_count, 1);
        assert_eq!(insp.rect, rect());
    }
}
