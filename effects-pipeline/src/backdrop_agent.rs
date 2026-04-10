use crate::{EffectError, EffectRect};
use alloc::vec::Vec;
use ngos_gfx_translate::{DrawOp, RgbaColor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackdropSpec {
    pub blur_radius: u32,
    pub tint_color: RgbaColor,
    pub tint_opacity: u8,
}

impl BackdropSpec {
    pub fn validate(&self) -> Result<(), EffectError> {
        if self.blur_radius == 0 {
            return Err(EffectError::BackdropRequiresBlur);
        }
        Ok(())
    }

    pub fn to_draw_ops(&self, rect: EffectRect) -> Vec<DrawOp> {
        let mut ops = Vec::new();
        ops.push(DrawOp::GaussianBlur {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
            radius: self.blur_radius,
        });
        ops.push(DrawOp::Backdrop {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
            opacity: self.tint_opacity,
        });
        ops.push(DrawOp::Rect {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
            color: RgbaColor {
                r: self.tint_color.r,
                g: self.tint_color.g,
                b: self.tint_color.b,
                a: self.tint_opacity,
            },
        });
        ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tint() -> RgbaColor {
        RgbaColor {
            r: 20,
            g: 30,
            b: 50,
            a: 180,
        }
    }

    fn rect() -> EffectRect {
        EffectRect::new(50, 40, 600, 400)
    }

    #[test]
    fn zero_blur_is_refused() {
        let spec = BackdropSpec {
            blur_radius: 0,
            tint_color: tint(),
            tint_opacity: 180,
        };
        let err = spec.validate().unwrap_err();
        assert!(matches!(err, EffectError::BackdropRequiresBlur));
        assert!(err.describe().contains("blur_radius > 0"));
    }

    #[test]
    fn valid_backdrop_produces_three_ops() {
        let spec = BackdropSpec {
            blur_radius: 12,
            tint_color: tint(),
            tint_opacity: 160,
        };
        spec.validate().unwrap();
        let ops = spec.to_draw_ops(rect());
        assert_eq!(ops.len(), 3);
        assert!(matches!(ops[0], DrawOp::GaussianBlur { radius: 12, .. }));
        assert!(matches!(ops[1], DrawOp::Backdrop { .. }));
        assert!(matches!(ops[2], DrawOp::Rect { .. }));
    }

    #[test]
    fn backdrop_ops_cover_full_rect() {
        let spec = BackdropSpec {
            blur_radius: 8,
            tint_color: tint(),
            tint_opacity: 100,
        };
        let ops = spec.to_draw_ops(rect());
        for op in &ops {
            match op {
                DrawOp::GaussianBlur {
                    x,
                    y,
                    width,
                    height,
                    ..
                }
                | DrawOp::Backdrop {
                    x,
                    y,
                    width,
                    height,
                    ..
                }
                | DrawOp::Rect {
                    x,
                    y,
                    width,
                    height,
                    ..
                } => {
                    assert_eq!(*x, 50);
                    assert_eq!(*y, 40);
                    assert_eq!(*width, 600);
                    assert_eq!(*height, 400);
                }
                _ => panic!("unexpected op"),
            }
        }
    }
}
