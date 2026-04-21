use crate::EffectRect;
use alloc::vec::Vec;
use ngos_gfx_translate::DrawOp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TranslucencySpec {
    pub opacity: u8,
    pub blur_radius: u32,
}

impl TranslucencySpec {
    pub fn to_draw_ops(&self, rect: EffectRect) -> Vec<DrawOp> {
        let mut ops = Vec::new();
        if self.blur_radius > 0 {
            ops.push(DrawOp::GaussianBlur {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
                radius: self.blur_radius,
            });
        }
        ops.push(DrawOp::Backdrop {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
            opacity: self.opacity,
        });
        ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> EffectRect {
        EffectRect::new(0, 0, 300, 200)
    }

    #[test]
    fn with_blur_produces_gaussian_blur_then_backdrop() {
        let spec = TranslucencySpec {
            opacity: 180,
            blur_radius: 8,
        };
        let ops = spec.to_draw_ops(rect());
        assert_eq!(ops.len(), 2);
        assert!(matches!(ops[0], DrawOp::GaussianBlur { radius: 8, .. }));
        assert!(matches!(ops[1], DrawOp::Backdrop { opacity: 180, .. }));
    }

    #[test]
    fn without_blur_produces_only_backdrop() {
        let spec = TranslucencySpec {
            opacity: 200,
            blur_radius: 0,
        };
        let ops = spec.to_draw_ops(rect());
        assert_eq!(ops.len(), 1);
        assert!(matches!(ops[0], DrawOp::Backdrop { opacity: 200, .. }));
    }

    #[test]
    fn opacity_zero_produces_fully_transparent_backdrop() {
        let spec = TranslucencySpec {
            opacity: 0,
            blur_radius: 0,
        };
        let ops = spec.to_draw_ops(rect());
        assert!(matches!(ops[0], DrawOp::Backdrop { opacity: 0, .. }));
    }
}
