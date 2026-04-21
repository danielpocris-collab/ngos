/// Vec3: 3-component float vector (fixed-point f32 for no_std compatibility)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    pub const ONE: Self = Self {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    };
    pub const X: Self = Self {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    pub const Y: Self = Self {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    pub const Z: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };

    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3 { x, y, z }
    }

    pub fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    pub fn cross(self, rhs: Self) -> Self {
        Vec3 {
            x: self.y * rhs.z - self.z * rhs.y,
            y: self.z * rhs.x - self.x * rhs.z,
            z: self.x * rhs.y - self.y * rhs.x,
        }
    }

    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    pub fn length(self) -> f32 {
        sqrt_f32(self.length_squared())
    }

    pub fn normalize(self) -> Self {
        let len = self.length();
        if len < 1e-8 {
            return Self::ZERO;
        }
        Vec3 {
            x: self.x / len,
            y: self.y / len,
            z: self.z / len,
        }
    }

    pub fn add(self, rhs: Self) -> Self {
        Vec3 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }

    pub fn sub(self, rhs: Self) -> Self {
        Vec3 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }

    pub fn scale(self, s: f32) -> Self {
        Vec3 {
            x: self.x * s,
            y: self.y * s,
            z: self.z * s,
        }
    }
}

/// Quat: unit quaternion for rotation (w, x, y, z)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    pub w: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Quat {
    pub const IDENTITY: Self = Self {
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub fn new(w: f32, x: f32, y: f32, z: f32) -> Self {
        Quat { w, x, y, z }
    }

    /// Construct from axis-angle (axis must be unit length, angle in radians)
    /// Uses the identity: q = (cos(a/2), sin(a/2)*axis)
    /// Approximation via Taylor series for no_std: cos ≈ 1 - x²/2 + x⁴/24, sin ≈ x - x³/6
    pub fn from_axis_angle(axis: Vec3, angle_rad: f32) -> Self {
        let half = angle_rad * 0.5;
        let s = sin_approx(half);
        let c = cos_approx(half);
        Quat {
            w: c,
            x: axis.x * s,
            y: axis.y * s,
            z: axis.z * s,
        }
    }

    pub fn mul(self, rhs: Self) -> Self {
        Quat {
            w: self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
            x: self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
            y: self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
            z: self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
        }
    }

    pub fn normalize(self) -> Self {
        let len = sqrt_f32(self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z);
        if len < 1e-8 {
            return Self::IDENTITY;
        }
        Quat {
            w: self.w / len,
            x: self.x / len,
            y: self.y / len,
            z: self.z / len,
        }
    }

    /// Convert quaternion to 4×4 rotation matrix (column-major)
    pub fn to_mat4(self) -> Mat4 {
        let q = self.normalize();
        let (w, x, y, z) = (q.w, q.x, q.y, q.z);
        let x2 = x + x;
        let y2 = y + y;
        let z2 = z + z;
        let xx = x * x2;
        let xy = x * y2;
        let xz = x * z2;
        let yy = y * y2;
        let yz = y * z2;
        let zz = z * z2;
        let wx = w * x2;
        let wy = w * y2;
        let wz = w * z2;
        Mat4::from_cols_array([
            1.0 - (yy + zz),
            xy + wz,
            xz - wy,
            0.0,
            xy - wz,
            1.0 - (xx + zz),
            yz + wx,
            0.0,
            xz + wy,
            yz - wx,
            1.0 - (xx + yy),
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
        ])
    }
}

/// Mat4: column-major 4×4 matrix
/// Layout: [col0_row0, col0_row1, col0_row2, col0_row3,  col1_row0, ...]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    pub cols: [[f32; 4]; 4],
}

impl Mat4 {
    pub const IDENTITY: Self = Self {
        cols: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    };

    pub fn from_cols_array(a: [f32; 16]) -> Self {
        Mat4 {
            cols: [
                [a[0], a[1], a[2], a[3]],
                [a[4], a[5], a[6], a[7]],
                [a[8], a[9], a[10], a[11]],
                [a[12], a[13], a[14], a[15]],
            ],
        }
    }

    pub fn translation(t: Vec3) -> Self {
        let mut m = Self::IDENTITY;
        m.cols[3][0] = t.x;
        m.cols[3][1] = t.y;
        m.cols[3][2] = t.z;
        m
    }

    pub fn scale(s: Vec3) -> Self {
        let mut m = Self::IDENTITY;
        m.cols[0][0] = s.x;
        m.cols[1][1] = s.y;
        m.cols[2][2] = s.z;
        m
    }

    /// Matrix multiplication: self * rhs
    pub fn mul(self, rhs: Self) -> Self {
        let mut out = [[0.0f32; 4]; 4];
        for col in 0..4 {
            for row in 0..4 {
                let mut sum = 0.0f32;
                for k in 0..4 {
                    sum += self.cols[k][row] * rhs.cols[col][k];
                }
                out[col][row] = sum;
            }
        }
        Mat4 { cols: out }
    }

    /// Transform a point (w=1)
    pub fn transform_point(self, p: Vec3) -> Vec3 {
        let x =
            self.cols[0][0] * p.x + self.cols[1][0] * p.y + self.cols[2][0] * p.z + self.cols[3][0];
        let y =
            self.cols[0][1] * p.x + self.cols[1][1] * p.y + self.cols[2][1] * p.z + self.cols[3][1];
        let z =
            self.cols[0][2] * p.x + self.cols[1][2] * p.y + self.cols[2][2] * p.z + self.cols[3][2];
        Vec3 { x, y, z }
    }

    /// Transform a direction vector (w=0)
    pub fn transform_vector(self, v: Vec3) -> Vec3 {
        let x = self.cols[0][0] * v.x + self.cols[1][0] * v.y + self.cols[2][0] * v.z;
        let y = self.cols[0][1] * v.x + self.cols[1][1] * v.y + self.cols[2][1] * v.z;
        let z = self.cols[0][2] * v.x + self.cols[1][2] * v.y + self.cols[2][2] * v.z;
        Vec3 { x, y, z }
    }
}

/// Transform: TRS (translation + rotation + scale)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    pub fn to_mat4(self) -> Mat4 {
        let t = Mat4::translation(self.translation);
        let r = self.rotation.to_mat4();
        let s = Mat4::scale(self.scale);
        t.mul(r).mul(s)
    }

    pub fn combine(parent: Self, child: Self) -> Self {
        let parent_mat = parent.to_mat4();
        let child_pos = parent_mat.transform_point(child.translation);
        let child_rot = parent.rotation.mul(child.rotation).normalize();
        let child_scale = Vec3 {
            x: parent.scale.x * child.scale.x,
            y: parent.scale.y * child.scale.y,
            z: parent.scale.z * child.scale.z,
        };
        Transform {
            translation: child_pos,
            rotation: child_rot,
            scale: child_scale,
        }
    }
}

// ── no_std math helpers ────────────────────────────────────────────────────────

/// Integer sqrt via Newton-Raphson, then cast to f32.
/// For f32 we use the standard f32::sqrt when available (std test),
/// otherwise a software approximation.
#[cfg(not(test))]
pub(crate) fn sqrt_f32(x: f32) -> f32 {
    // Bit-manipulation initial guess (Quake-style) + 3 NR iterations
    if x <= 0.0 {
        return 0.0;
    }
    let bits = x.to_bits();
    let guess_bits = (bits >> 1).wrapping_add(0x1fbb4f2e);
    let mut s = f32::from_bits(guess_bits);
    s = 0.5 * (s + x / s);
    s = 0.5 * (s + x / s);
    s = 0.5 * (s + x / s);
    s
}

#[cfg(test)]
pub(crate) fn sqrt_f32(x: f32) -> f32 {
    x.sqrt()
}

/// sin approximation via Taylor series (accurate for |x| < π)
pub(crate) fn sin_approx(x: f32) -> f32 {
    // Reduce to [-π, π]
    let x = reduce_angle(x);
    let x2 = x * x;
    // sin(x) ≈ x - x³/6 + x⁵/120 - x⁷/5040
    x * (1.0 - x2 * (1.0 / 6.0 - x2 * (1.0 / 120.0 - x2 / 5040.0)))
}

/// cos approximation via Taylor series (accurate for |x| < π)
pub(crate) fn cos_approx(x: f32) -> f32 {
    let x = reduce_angle(x);
    let x2 = x * x;
    // cos(x) ≈ 1 - x²/2 + x⁴/24 - x⁶/720
    1.0 - x2 * (0.5 - x2 * (1.0 / 24.0 - x2 / 720.0))
}

fn reduce_angle(x: f32) -> f32 {
    const TWO_PI: f32 = 6.283185307;
    const PI: f32 = 3.141592653;
    let mut x = x;
    while x > PI {
        x -= TWO_PI;
    }
    while x < -PI {
        x += TWO_PI;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec3_dot_product() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        let d = a.dot(b);
        assert!((d - 32.0).abs() < 1e-5);
    }

    #[test]
    fn vec3_cross_product() {
        let c = Vec3::X.cross(Vec3::Y);
        assert!((c.x).abs() < 1e-5);
        assert!((c.y).abs() < 1e-5);
        assert!((c.z - 1.0).abs() < 1e-5);
    }

    #[test]
    fn vec3_normalize_unit_length() {
        let v = Vec3::new(3.0, 4.0, 0.0);
        let n = v.normalize();
        let len = sqrt_f32(n.x * n.x + n.y * n.y + n.z * n.z);
        assert!((len - 1.0).abs() < 1e-5);
    }

    #[test]
    fn quat_identity_to_mat4_is_identity() {
        let m = Quat::IDENTITY.to_mat4();
        for col in 0..4 {
            for row in 0..4 {
                let expected = if col == row { 1.0f32 } else { 0.0f32 };
                assert!(
                    (m.cols[col][row] - expected).abs() < 1e-5,
                    "col={} row={} expected={} got={}",
                    col,
                    row,
                    expected,
                    m.cols[col][row]
                );
            }
        }
    }

    #[test]
    fn quat_mul_identity_is_identity() {
        let q = Quat::from_axis_angle(Vec3::Y, 0.5);
        let result = q.mul(Quat::IDENTITY);
        assert!((result.w - q.w).abs() < 1e-5);
        assert!((result.x - q.x).abs() < 1e-5);
        assert!((result.y - q.y).abs() < 1e-5);
        assert!((result.z - q.z).abs() < 1e-5);
    }

    #[test]
    fn mat4_mul_identity_is_self() {
        let t = Mat4::translation(Vec3::new(1.0, 2.0, 3.0));
        let r = t.mul(Mat4::IDENTITY);
        for col in 0..4 {
            for row in 0..4 {
                assert!((r.cols[col][row] - t.cols[col][row]).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn mat4_translation_transforms_point() {
        let t = Mat4::translation(Vec3::new(5.0, -3.0, 2.0));
        let p = t.transform_point(Vec3::ZERO);
        assert!((p.x - 5.0).abs() < 1e-5);
        assert!((p.y + 3.0).abs() < 1e-5);
        assert!((p.z - 2.0).abs() < 1e-5);
    }

    #[test]
    fn mat4_scale_transforms_point() {
        let s = Mat4::scale(Vec3::new(2.0, 3.0, 4.0));
        let p = s.transform_point(Vec3::new(1.0, 1.0, 1.0));
        assert!((p.x - 2.0).abs() < 1e-5);
        assert!((p.y - 3.0).abs() < 1e-5);
        assert!((p.z - 4.0).abs() < 1e-5);
    }

    #[test]
    fn transform_identity_to_mat4_is_identity() {
        let m = Transform::IDENTITY.to_mat4();
        for col in 0..4 {
            for row in 0..4 {
                let expected = if col == row { 1.0f32 } else { 0.0f32 };
                assert!(
                    (m.cols[col][row] - expected).abs() < 1e-5,
                    "col={} row={} expected={} got={}",
                    col,
                    row,
                    expected,
                    m.cols[col][row]
                );
            }
        }
    }

    #[test]
    fn transform_combine_translation() {
        let parent = Transform {
            translation: Vec3::new(10.0, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };
        let child = Transform {
            translation: Vec3::new(5.0, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };
        let combined = Transform::combine(parent, child);
        assert!((combined.translation.x - 15.0).abs() < 1e-4);
        assert!((combined.translation.y).abs() < 1e-4);
        assert!((combined.translation.z).abs() < 1e-4);
    }

    #[test]
    fn sin_approx_zero() {
        assert!(sin_approx(0.0).abs() < 1e-5);
    }

    #[test]
    fn cos_approx_zero() {
        assert!((cos_approx(0.0) - 1.0).abs() < 1e-5);
    }
}
