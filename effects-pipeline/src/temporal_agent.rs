use crate::{EffectError, EffectRect};
use alloc::vec::Vec;
use ngos_gfx_translate::{DrawOp, RgbaColor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TemporalState {
    pub tick: u64,
    pub stride: u8,
}

impl TemporalState {
    pub fn new(stride: u8) -> Result<Self, EffectError> {
        if stride == 0 || stride > 16 {
            return Err(EffectError::InvalidTemporalStride);
        }
        Ok(TemporalState { tick: 0, stride })
    }

    pub fn advance(&self) -> Self {
        TemporalState {
            tick: self.tick.wrapping_add(1),
            stride: self.stride,
        }
    }

    pub fn pulse_alpha(&self, accent: &AccentPulse) -> u8 {
        // Phase: 0-511 (rising 0-255, falling 256-511)
        let phase = (self.tick.wrapping_mul(self.stride as u64)) % 512;
        let offset = if phase < 256 {
            phase as u8
        } else {
            (511 - phase) as u8
        };
        accent
            .base_alpha
            .saturating_add(offset.min(accent.pulse_range))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccentPulse {
    pub base_alpha: u8,
    pub pulse_range: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TemporalSpec {
    pub state: TemporalState,
    pub color: RgbaColor,
    pub accent: AccentPulse,
}

impl TemporalSpec {
    pub fn to_draw_ops(&self, rect: EffectRect) -> Vec<DrawOp> {
        let alpha = self.state.pulse_alpha(&self.accent);
        let mut ops = Vec::new();
        ops.push(DrawOp::Rect {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
            color: RgbaColor {
                r: self.color.r,
                g: self.color.g,
                b: self.color.b,
                a: alpha,
            },
        });
        ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stride_zero_is_refused() {
        let err = TemporalState::new(0).unwrap_err();
        assert!(matches!(err, EffectError::InvalidTemporalStride));
        assert!(err.describe().contains("stride"));
    }

    #[test]
    fn stride_above_max_is_refused() {
        let err = TemporalState::new(17).unwrap_err();
        assert!(matches!(err, EffectError::InvalidTemporalStride));
    }

    #[test]
    fn valid_stride_creates_state() {
        let s = TemporalState::new(4).unwrap();
        assert_eq!(s.tick, 0);
        assert_eq!(s.stride, 4);
    }

    #[test]
    fn advance_increments_tick() {
        let s = TemporalState::new(1).unwrap();
        let s2 = s.advance();
        assert_eq!(s2.tick, 1);
        let s3 = s2.advance();
        assert_eq!(s3.tick, 2);
    }

    #[test]
    fn pulse_alpha_at_tick_zero_is_base() {
        let s = TemporalState { tick: 0, stride: 1 };
        let accent = AccentPulse {
            base_alpha: 100,
            pulse_range: 50,
        };
        assert_eq!(s.pulse_alpha(&accent), 100);
    }

    #[test]
    fn pulse_alpha_rises_then_falls() {
        let accent = AccentPulse {
            base_alpha: 50,
            pulse_range: 100,
        };
        let s_peak = TemporalState {
            tick: 255,
            stride: 1,
        };
        let alpha_peak = s_peak.pulse_alpha(&accent);
        assert_eq!(alpha_peak, 150);

        let s_valley = TemporalState {
            tick: 511,
            stride: 1,
        };
        let alpha_valley = s_valley.pulse_alpha(&accent);
        assert_eq!(alpha_valley, 50);

        // tick=460: phase=460, offset=511-460=51, 51 < pulse_range(100) → alpha=50+51=101
        let s_mid_fall = TemporalState {
            tick: 460,
            stride: 1,
        };
        let alpha_mid = s_mid_fall.pulse_alpha(&accent);
        assert!(
            alpha_mid > 50 && alpha_mid < 150,
            "mid-fall should be between base and peak"
        );
    }

    #[test]
    fn temporal_spec_produces_rect_with_pulsed_alpha() {
        let state = TemporalState {
            tick: 128,
            stride: 1,
        };
        let spec = TemporalSpec {
            state,
            color: RgbaColor {
                r: 0x6a,
                g: 0xb2,
                b: 0xff,
                a: 0,
            },
            accent: AccentPulse {
                base_alpha: 80,
                pulse_range: 60,
            },
        };
        let ops = spec.to_draw_ops(EffectRect::new(10, 20, 100, 4));
        assert_eq!(ops.len(), 1);
        if let DrawOp::Rect { color, .. } = &ops[0] {
            assert!(color.a > 80, "alpha should be above base at tick=128");
        }
    }
}
