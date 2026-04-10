use crate::render_command_agent::{DrawOp, DrawOpClass};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameProfile {
    pub geometry_ops: usize,
    pub composition_ops: usize,
    pub effect_ops: usize,
    pub presentation_ops: usize,
    pub text_ops: usize,
    pub image_ops: usize,
    pub pass_ops: usize,
    pub blend_ops: usize,
    pub total_ops: usize,
}

impl FrameProfile {
    pub fn from_ops(ops: &[DrawOp]) -> Self {
        let mut geometry = 0usize;
        let mut composition = 0usize;
        let mut effect = 0usize;
        let mut presentation = 0usize;
        let mut text = 0usize;
        let mut image = 0usize;
        let mut pass_ops = 0usize;
        let mut blend_ops = 0usize;
        for op in ops {
            match op.class() {
                DrawOpClass::Geometry => geometry += 1,
                DrawOpClass::Composition => composition += 1,
                DrawOpClass::Effect => effect += 1,
                DrawOpClass::Presentation => presentation += 1,
                DrawOpClass::Text => text += 1,
                DrawOpClass::Image => image += 1,
            }
            match op {
                DrawOp::BeginPass { .. } | DrawOp::EndPass => pass_ops += 1,
                DrawOp::SetBlendMode { .. } | DrawOp::ClearBlendMode => blend_ops += 1,
                _ => {}
            }
        }
        FrameProfile {
            geometry_ops: geometry,
            composition_ops: composition,
            effect_ops: effect,
            presentation_ops: presentation,
            text_ops: text,
            image_ops: image,
            pass_ops,
            blend_ops,
            total_ops: ops.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render_command_agent::{DrawOp, RgbaColor};

    fn black() -> RgbaColor {
        RgbaColor {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }
    }

    #[test]
    fn profile_counts_all_classes() {
        let ops = vec![
            DrawOp::Clear { color: black() },
            DrawOp::Rect {
                x: 0,
                y: 0,
                width: 100,
                height: 100,
                color: black(),
            },
            DrawOp::GaussianBlur {
                x: 0,
                y: 0,
                width: 100,
                height: 100,
                radius: 4,
            },
            DrawOp::PushLayer { opacity: 200 },
            DrawOp::PopLayer,
            DrawOp::SetPresentRegion {
                x: 0,
                y: 0,
                width: 640,
                height: 480,
            },
        ];
        let p = FrameProfile::from_ops(&ops);
        assert_eq!(p.geometry_ops, 2);
        assert_eq!(p.effect_ops, 1);
        assert_eq!(p.composition_ops, 2);
        assert_eq!(p.presentation_ops, 1);
        assert_eq!(p.total_ops, 6);
    }

    #[test]
    fn profile_empty_ops() {
        let p = FrameProfile::from_ops(&[]);
        assert_eq!(p.total_ops, 0);
        assert_eq!(p.geometry_ops, 0);
        assert_eq!(p.composition_ops, 0);
        assert_eq!(p.effect_ops, 0);
        assert_eq!(p.presentation_ops, 0);
    }
}
