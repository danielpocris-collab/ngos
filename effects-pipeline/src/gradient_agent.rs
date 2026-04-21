use crate::{EffectError, EffectRect, lerp_color};
use alloc::vec::Vec;
use ngos_gfx_translate::{DrawOp, RgbaColor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientDirection {
    Horizontal,
    Vertical,
    DiagonalTopLeft,
    Radial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GradientStop {
    pub position: u8,
    pub color: RgbaColor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GradientSpec {
    pub direction: GradientDirection,
    pub stops: Vec<GradientStop>,
}

impl GradientSpec {
    pub fn validate(&self) -> Result<(), EffectError> {
        if self.stops.len() < 2 {
            return Err(EffectError::InsufficientGradientStops {
                count: self.stops.len(),
            });
        }
        Ok(())
    }

    pub fn to_draw_ops(&self, rect: EffectRect) -> Vec<DrawOp> {
        match self.direction {
            GradientDirection::Horizontal => self.linear_ops(rect, true),
            GradientDirection::Vertical => self.linear_ops(rect, false),
            GradientDirection::DiagonalTopLeft => self.diagonal_ops(rect),
            GradientDirection::Radial => self.radial_ops(rect),
        }
    }

    fn linear_ops(&self, rect: EffectRect, horizontal: bool) -> Vec<DrawOp> {
        let n = self.stops.len();
        let mut ops = Vec::new();
        for i in 0..n - 1 {
            let a = self.stops[i];
            let b = self.stops[i + 1];
            let pos_a = a.position as u32;
            let pos_b = b.position as u32;
            if horizontal {
                let x = rect.x + rect.width * pos_a / 255;
                let w = if i == n - 2 {
                    (rect.x + rect.width).saturating_sub(x)
                } else {
                    let x_next = rect.x + rect.width * pos_b / 255;
                    x_next.saturating_sub(x)
                }
                .max(1);
                ops.push(DrawOp::GradientRect {
                    x,
                    y: rect.y,
                    width: w,
                    height: rect.height,
                    top_left: a.color,
                    top_right: b.color,
                    bottom_left: a.color,
                    bottom_right: b.color,
                });
            } else {
                let y = rect.y + rect.height * pos_a / 255;
                let h = if i == n - 2 {
                    (rect.y + rect.height).saturating_sub(y)
                } else {
                    let y_next = rect.y + rect.height * pos_b / 255;
                    y_next.saturating_sub(y)
                }
                .max(1);
                ops.push(DrawOp::GradientRect {
                    x: rect.x,
                    y,
                    width: rect.width,
                    height: h,
                    top_left: a.color,
                    top_right: a.color,
                    bottom_left: b.color,
                    bottom_right: b.color,
                });
            }
        }
        ops
    }

    fn diagonal_ops(&self, rect: EffectRect) -> Vec<DrawOp> {
        let n = self.stops.len();
        let first = self.stops[0].color;
        let last = self.stops[n - 1].color;
        let mid = if n > 2 {
            self.stops[n / 2].color
        } else {
            lerp_color(first, last, 128)
        };
        let mut ops = Vec::new();
        ops.push(DrawOp::GradientRect {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
            top_left: first,
            top_right: mid,
            bottom_left: mid,
            bottom_right: last,
        });
        ops
    }

    fn radial_ops(&self, rect: EffectRect) -> Vec<DrawOp> {
        // Concentric filled ellipses, drawn outside-in.
        // stops[0] = outermost color, stops[n-1] = innermost color.
        // 8 interpolation steps per stop pair for smooth approximation.
        let n = self.stops.len();
        let cx = rect.x + rect.width / 2;
        let cy = rect.y + rect.height / 2;
        let steps_per_pair: u32 = 8;
        let total_steps = (n as u32 - 1) * steps_per_pair;
        let mut ops = Vec::new();

        for step in 0..=total_steps {
            // scale: step=0 → full size (1.0), step=total_steps → minimum (center dot)
            let ew = if total_steps > 0 {
                let s = (total_steps - step) * rect.width / total_steps;
                if step == total_steps { s.max(2) } else { s }
            } else {
                2
            };
            let eh = if total_steps > 0 {
                let s = (total_steps - step) * rect.height / total_steps;
                if step == total_steps { s.max(2) } else { s }
            } else {
                2
            };
            if ew == 0 || eh == 0 {
                continue;
            }

            // Color: interpolate through stops
            let (pair_idx, pair_t) = if step >= total_steps {
                (n - 2, 255u8)
            } else {
                let pidx = (step / steps_per_pair) as usize;
                let pfrac = step % steps_per_pair;
                let pt = (pfrac * 255 / steps_per_pair.saturating_sub(1).max(1)) as u8;
                (pidx.min(n - 2), pt)
            };
            let color = lerp_color(
                self.stops[pair_idx].color,
                self.stops[pair_idx + 1].color,
                pair_t,
            );

            ops.push(DrawOp::Ellipse {
                x: cx.saturating_sub(ew / 2),
                y: cy.saturating_sub(eh / 2),
                width: ew,
                height: eh,
                color,
            });
        }
        ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color(r: u8, g: u8, b: u8) -> RgbaColor {
        RgbaColor { r, g, b, a: 255 }
    }

    fn stop(pos: u8, r: u8, g: u8, b: u8) -> GradientStop {
        GradientStop {
            position: pos,
            color: color(r, g, b),
        }
    }

    fn rect() -> EffectRect {
        EffectRect::new(0, 0, 400, 200)
    }

    #[test]
    fn rejects_single_stop() {
        let spec = GradientSpec {
            direction: GradientDirection::Horizontal,
            stops: vec![stop(0, 255, 0, 0)],
        };
        let err = spec.validate().unwrap_err();
        assert!(matches!(
            err,
            EffectError::InsufficientGradientStops { count: 1 }
        ));
        assert!(err.describe().contains("at least 2"));
    }

    #[test]
    fn horizontal_two_stops_produces_gradient_rect() {
        let spec = GradientSpec {
            direction: GradientDirection::Horizontal,
            stops: vec![stop(0, 0, 0, 255), stop(255, 255, 0, 0)],
        };
        let ops = spec.to_draw_ops(rect());
        assert_eq!(ops.len(), 1);
        assert!(matches!(ops[0], DrawOp::GradientRect { .. }));
    }

    #[test]
    fn horizontal_three_stops_produces_two_gradient_rects() {
        let spec = GradientSpec {
            direction: GradientDirection::Horizontal,
            stops: vec![
                stop(0, 0, 0, 0),
                stop(128, 128, 128, 128),
                stop(255, 255, 255, 255),
            ],
        };
        let ops = spec.to_draw_ops(rect());
        assert_eq!(ops.len(), 2);
        assert!(
            ops.iter()
                .all(|op| matches!(op, DrawOp::GradientRect { .. }))
        );
    }

    #[test]
    fn vertical_two_stops_produces_gradient_rect() {
        let spec = GradientSpec {
            direction: GradientDirection::Vertical,
            stops: vec![stop(0, 0, 0, 255), stop(255, 255, 0, 0)],
        };
        let ops = spec.to_draw_ops(rect());
        assert_eq!(ops.len(), 1);
        assert!(
            matches!(ops[0], DrawOp::GradientRect { top_left, bottom_left, .. }
            if top_left != bottom_left)
        );
    }

    #[test]
    fn diagonal_produces_single_gradient_rect() {
        let spec = GradientSpec {
            direction: GradientDirection::DiagonalTopLeft,
            stops: vec![stop(0, 0, 0, 0), stop(255, 255, 255, 255)],
        };
        let ops = spec.to_draw_ops(rect());
        assert_eq!(ops.len(), 1);
        assert!(matches!(ops[0], DrawOp::GradientRect { .. }));
    }

    #[test]
    fn radial_produces_concentric_ellipses() {
        let spec = GradientSpec {
            direction: GradientDirection::Radial,
            stops: vec![stop(0, 0, 0, 255), stop(255, 255, 0, 0)],
        };
        let ops = spec.to_draw_ops(rect());
        assert!(!ops.is_empty());
        assert!(ops.iter().all(|op| matches!(op, DrawOp::Ellipse { .. })));
        // outermost ellipse should be at most rect size
        if let DrawOp::Ellipse { width, height, .. } = &ops[0] {
            assert!(*width <= 400);
            assert!(*height <= 200);
        }
    }

    #[test]
    fn radial_outermost_larger_than_innermost() {
        let spec = GradientSpec {
            direction: GradientDirection::Radial,
            stops: vec![stop(0, 255, 0, 0), stop(255, 0, 0, 255)],
        };
        let ops = spec.to_draw_ops(rect());
        let widths: Vec<u32> = ops
            .iter()
            .filter_map(|op| {
                if let DrawOp::Ellipse { width, .. } = op {
                    Some(*width)
                } else {
                    None
                }
            })
            .collect();
        let first = *widths.first().unwrap();
        let last = *widths.last().unwrap();
        assert!(
            first > last,
            "outermost ellipse must be larger than innermost"
        );
    }
}
