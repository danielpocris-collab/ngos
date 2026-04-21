use crate::SceneError;
use crate::math_agent::{Mat4, Transform, Vec3};

/// Perspective camera — produces a view-projection matrix
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PerspectiveCamera {
    /// Vertical field-of-view in radians
    pub fov_y_rad: f32,
    /// Aspect ratio: width / height
    pub aspect: f32,
    /// Near clip plane distance (must be > 0)
    pub near: f32,
    /// Far clip plane distance (must be > near)
    pub far: f32,
}

impl PerspectiveCamera {
    pub fn validate(&self) -> Result<(), SceneError> {
        if self.near <= 0.0 {
            return Err(SceneError::InvalidCamera {
                reason: "near must be > 0",
            });
        }
        if self.far <= self.near {
            return Err(SceneError::InvalidCamera {
                reason: "far must be > near",
            });
        }
        if self.aspect <= 0.0 {
            return Err(SceneError::InvalidCamera {
                reason: "aspect must be > 0",
            });
        }
        if self.fov_y_rad <= 0.0 || self.fov_y_rad >= 3.14159265 {
            return Err(SceneError::InvalidCamera {
                reason: "fov_y_rad must be in (0, π)",
            });
        }
        Ok(())
    }

    /// Right-handed, depth range [-1, 1] (OpenGL convention)
    pub fn projection_matrix(&self) -> Mat4 {
        let f = cos_over_sin_half_fov(self.fov_y_rad);
        let nf = 1.0 / (self.near - self.far);
        Mat4::from_cols_array([
            f / self.aspect,
            0.0,
            0.0,
            0.0,
            0.0,
            f,
            0.0,
            0.0,
            0.0,
            0.0,
            (self.far + self.near) * nf,
            -1.0,
            0.0,
            0.0,
            2.0 * self.far * self.near * nf,
            0.0,
        ])
    }
}

/// Orthographic camera
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrthographicCamera {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub near: f32,
    pub far: f32,
}

impl OrthographicCamera {
    pub fn validate(&self) -> Result<(), SceneError> {
        if self.right <= self.left {
            return Err(SceneError::InvalidCamera {
                reason: "right must be > left",
            });
        }
        if self.top <= self.bottom {
            return Err(SceneError::InvalidCamera {
                reason: "top must be > bottom",
            });
        }
        if self.far <= self.near {
            return Err(SceneError::InvalidCamera {
                reason: "far must be > near",
            });
        }
        Ok(())
    }

    pub fn projection_matrix(&self) -> Mat4 {
        let rl = 1.0 / (self.right - self.left);
        let tb = 1.0 / (self.top - self.bottom);
        let fn_ = 1.0 / (self.far - self.near);
        Mat4::from_cols_array([
            2.0 * rl,
            0.0,
            0.0,
            0.0,
            0.0,
            2.0 * tb,
            0.0,
            0.0,
            0.0,
            0.0,
            -2.0 * fn_,
            0.0,
            -(self.right + self.left) * rl,
            -(self.top + self.bottom) * tb,
            -(self.far + self.near) * fn_,
            1.0,
        ])
    }
}

/// Camera: either perspective or orthographic
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Camera {
    Perspective(PerspectiveCamera),
    Orthographic(OrthographicCamera),
}

impl Camera {
    pub fn validate(&self) -> Result<(), SceneError> {
        match self {
            Camera::Perspective(p) => p.validate(),
            Camera::Orthographic(o) => o.validate(),
        }
    }

    pub fn projection_matrix(&self) -> Mat4 {
        match self {
            Camera::Perspective(p) => p.projection_matrix(),
            Camera::Orthographic(o) => o.projection_matrix(),
        }
    }

    /// View matrix: inverse of the camera's world transform (look-at convention)
    pub fn view_matrix(&self, world_transform: Transform) -> Mat4 {
        // Camera position = translation
        let pos = world_transform.translation;
        // Forward direction: camera looks along -Z in local space, rotated by orientation
        let fwd = world_transform
            .rotation
            .to_mat4()
            .transform_vector(Vec3::new(0.0, 0.0, -1.0));
        let up = world_transform.rotation.to_mat4().transform_vector(Vec3::Y);
        look_at(pos, pos.add(fwd), up)
    }
}

/// Construct a look-at view matrix
fn look_at(eye: Vec3, center: Vec3, up: Vec3) -> Mat4 {
    let f = center.sub(eye).normalize();
    let s = f.cross(up).normalize();
    let u = s.cross(f);
    Mat4::from_cols_array([
        s.x,
        u.x,
        -f.x,
        0.0,
        s.y,
        u.y,
        -f.y,
        0.0,
        s.z,
        u.z,
        -f.z,
        0.0,
        -s.dot(eye),
        -u.dot(eye),
        f.dot(eye),
        1.0,
    ])
}

/// cot(fov/2) = cos(fov/2)/sin(fov/2) — used in perspective projection
fn cos_over_sin_half_fov(fov_y: f32) -> f32 {
    use crate::math_agent::{cos_approx, sin_approx};
    let half = fov_y * 0.5;
    let s = sin_approx(half);
    let c = cos_approx(half);
    if s.abs() < 1e-8 { 1.0 } else { c / s }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perspective() -> PerspectiveCamera {
        PerspectiveCamera {
            fov_y_rad: 1.047,
            aspect: 1.777,
            near: 0.1,
            far: 1000.0,
        }
    }

    fn ortho() -> OrthographicCamera {
        OrthographicCamera {
            left: -1.0,
            right: 1.0,
            bottom: -1.0,
            top: 1.0,
            near: 0.0,
            far: 100.0,
        }
    }

    #[test]
    fn perspective_validates_ok() {
        perspective().validate().unwrap();
    }

    #[test]
    fn perspective_near_zero_refused() {
        let mut p = perspective();
        p.near = 0.0;
        assert!(matches!(
            p.validate(),
            Err(SceneError::InvalidCamera { .. })
        ));
    }

    #[test]
    fn perspective_far_le_near_refused() {
        let mut p = perspective();
        p.far = p.near;
        assert!(matches!(
            p.validate(),
            Err(SceneError::InvalidCamera { .. })
        ));
    }

    #[test]
    fn perspective_projection_w_column() {
        // In column-major: col[2][3] should be -1 (homogeneous w=−z convention)
        let m = perspective().projection_matrix();
        assert!((m.cols[2][3] + 1.0).abs() < 1e-5, "got {}", m.cols[2][3]);
    }

    #[test]
    fn ortho_validates_ok() {
        ortho().validate().unwrap();
    }

    #[test]
    fn ortho_right_le_left_refused() {
        let mut o = ortho();
        o.right = o.left;
        assert!(matches!(
            o.validate(),
            Err(SceneError::InvalidCamera { .. })
        ));
    }

    #[test]
    fn ortho_projection_diagonal() {
        let m = ortho().projection_matrix();
        // For symmetric ortho (-1,1,-1,1): m[0][0]=1, m[1][1]=1
        assert!((m.cols[0][0] - 1.0).abs() < 1e-5);
        assert!((m.cols[1][1] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn camera_view_matrix_identity_transform() {
        let cam = Camera::Perspective(perspective());
        let t = Transform::IDENTITY;
        // Camera at origin looking at -Z: view matrix should be identity (approx)
        let v = cam.view_matrix(t);
        // The view matrix for a camera at origin looking forward should have
        // the translation column near zero
        assert!((v.cols[3][0]).abs() < 1e-4);
        assert!((v.cols[3][1]).abs() < 1e-4);
        assert!((v.cols[3][2]).abs() < 1e-4);
    }

    #[test]
    fn camera_view_matrix_translated() {
        let cam = Camera::Perspective(perspective());
        let mut t = Transform::IDENTITY;
        t.translation = Vec3::new(0.0, 0.0, 5.0);
        let v = cam.view_matrix(t);
        // Camera moved +5 on Z: the view translation Z should be -5 (inverse)
        assert!((v.cols[3][2] + 5.0).abs() < 1e-4, "got {}", v.cols[3][2]);
    }

    #[test]
    fn camera_enum_delegates_to_inner() {
        let cam = Camera::Orthographic(ortho());
        cam.validate().unwrap();
        let m = cam.projection_matrix();
        assert!((m.cols[0][0] - 1.0).abs() < 1e-5);
    }
}
