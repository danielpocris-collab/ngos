use crate::EffectRect;
use alloc::vec::Vec;
use ngos_gfx_translate::{DrawOp, RgbaColor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShadowSpec {
    pub offset_x: i32,
    pub offset_y: i32,
    pub blur: u32,
    pub spread: u32,
    pub color: RgbaColor,
}

impl ShadowSpec {
    pub fn to_draw_ops(&self, rect: EffectRect) -> Vec<DrawOp> {
        let x = (rect.x as i64 + self.offset_x as i64 - self.spread as i64).max(0) as u32;
        let y = (rect.y as i64 + self.offset_y as i64 - self.spread as i64).max(0) as u32;
        let w = rect.width + self.spread * 2;
        let h = rect.height + self.spread * 2;
        let mut ops = Vec::new();
        ops.push(DrawOp::ShadowRect {
            x,
            y,
            width: w,
            height: h,
            blur: self.blur,
            color: self.color,
        });
        ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shadow(ox: i32, oy: i32, blur: u32, spread: u32) -> ShadowSpec {
        ShadowSpec {
            offset_x: ox,
            offset_y: oy,
            blur,
            spread,
            color: RgbaColor {
                r: 0,
                g: 0,
                b: 0,
                a: 100,
            },
        }
    }

    fn rect() -> EffectRect {
        EffectRect::new(100, 80, 400, 300)
    }

    #[test]
    fn zero_offset_zero_spread_mirrors_rect() {
        let spec = shadow(0, 0, 8, 0);
        let ops = spec.to_draw_ops(rect());
        assert_eq!(ops.len(), 1);
        if let DrawOp::ShadowRect {
            x,
            y,
            width,
            height,
            blur,
            ..
        } = ops[0]
        {
            assert_eq!(x, 100);
            assert_eq!(y, 80);
            assert_eq!(width, 400);
            assert_eq!(height, 300);
            assert_eq!(blur, 8);
        }
    }

    #[test]
    fn positive_offset_shifts_shadow() {
        let spec = shadow(10, 5, 4, 0);
        let ops = spec.to_draw_ops(rect());
        if let DrawOp::ShadowRect { x, y, .. } = ops[0] {
            assert_eq!(x, 110);
            assert_eq!(y, 85);
        }
    }

    #[test]
    fn spread_expands_shadow() {
        let spec = shadow(0, 0, 4, 8);
        let ops = spec.to_draw_ops(rect());
        if let DrawOp::ShadowRect {
            x,
            y,
            width,
            height,
            ..
        } = ops[0]
        {
            assert_eq!(x, 92);
            assert_eq!(y, 72);
            assert_eq!(width, 416);
            assert_eq!(height, 316);
        }
    }

    #[test]
    fn negative_offset_clamps_to_zero() {
        let spec = shadow(-200, -200, 0, 0);
        let ops = spec.to_draw_ops(rect());
        if let DrawOp::ShadowRect { x, y, .. } = ops[0] {
            assert_eq!(x, 0);
            assert_eq!(y, 0);
        }
    }
}
