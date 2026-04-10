use crate::RenderError;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepthFunc {
    Never,
    Less,
    Equal,
    LessOrEqual,
    Greater,
    NotEqual,
    GreaterOrEqual,
    Always,
}

impl DepthFunc {
    pub fn test(&self, depth: f32, stored: f32) -> bool {
        match self {
            DepthFunc::Never => false,
            DepthFunc::Less => depth < stored,
            DepthFunc::Equal => (depth - stored).abs() < 1e-6,
            DepthFunc::LessOrEqual => depth <= stored,
            DepthFunc::Greater => depth > stored,
            DepthFunc::NotEqual => (depth - stored).abs() >= 1e-6,
            DepthFunc::GreaterOrEqual => depth >= stored,
            DepthFunc::Always => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepthTest {
    Disabled,
    Enabled(DepthFunc),
}

impl DepthTest {
    pub fn should_test(&self) -> bool {
        matches!(self, DepthTest::Enabled(_))
    }

    pub fn func(&self) -> DepthFunc {
        match self {
            DepthTest::Disabled => DepthFunc::Always,
            DepthTest::Enabled(func) => *func,
        }
    }

    pub fn should_write(&self) -> bool {
        matches!(self, DepthTest::Enabled(_))
    }
}

impl Default for DepthTest {
    fn default() -> Self {
        DepthTest::Enabled(DepthFunc::Less)
    }
}

#[derive(Debug, Clone)]
pub struct DepthBuffer {
    width: u32,
    height: u32,
    depths: Vec<f32>,
}

impl DepthBuffer {
    pub fn new(width: u32, height: u32) -> Result<Self, RenderError> {
        if width == 0 || height == 0 {
            return Err(RenderError::InvalidRenderTarget);
        }
        let size = (width * height) as usize;
        let mut depths = Vec::with_capacity(size);
        for _ in 0..size {
            depths.push(1.0);
        }
        Ok(DepthBuffer {
            width,
            height,
            depths,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn clear(&mut self, depth: f32) {
        for d in &mut self.depths {
            *d = depth;
        }
    }

    pub fn clear_default(&mut self) {
        self.clear(1.0);
    }

    fn index(&self, x: u32, y: u32) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some((y * self.width + x) as usize)
    }

    pub fn get(&self, x: u32, y: u32) -> Option<f32> {
        self.index(x, y).map(|i| self.depths[i])
    }

    pub fn set(&mut self, x: u32, y: u32, depth: f32) -> Result<(), RenderError> {
        match self.index(x, y) {
            Some(i) => {
                self.depths[i] = depth;
                Ok(())
            }
            None => Err(RenderError::OutOfBounds),
        }
    }

    pub fn test_and_set(
        &mut self,
        x: u32,
        y: u32,
        depth: f32,
        depth_test: DepthTest,
    ) -> Result<bool, RenderError> {
        if !depth_test.should_test() {
            if depth_test.should_write() {
                self.set(x, y, depth)?;
            }
            return Ok(true);
        }

        let idx = self.index(x, y).ok_or(RenderError::OutOfBounds)?;
        let stored = self.depths[idx];
        let func = depth_test.func();

        if func.test(depth, stored) {
            if depth_test.should_write() {
                self.depths[idx] = depth;
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn sample(&self, x: u32, y: u32) -> Option<f32> {
        self.get(x, y)
    }

    pub fn write(&mut self, x: u32, y: u32, depth: f32) -> Result<(), RenderError> {
        self.set(x, y, depth)
    }

    pub fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, depth: f32) {
        for dy in 0..h {
            for dx in 0..w {
                let _ = self.set(x + dx, y + dy, depth);
            }
        }
    }

    pub fn depths(&self) -> &[f32] {
        &self.depths
    }

    pub fn mut_depths(&mut self) -> &mut [f32] {
        &mut self.depths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_func_never() {
        assert!(!DepthFunc::Never.test(0.5, 0.5));
    }

    #[test]
    fn depth_func_less() {
        assert!(DepthFunc::Less.test(0.3, 0.5));
        assert!(!DepthFunc::Less.test(0.5, 0.5));
        assert!(!DepthFunc::Less.test(0.7, 0.5));
    }

    #[test]
    fn depth_func_equal() {
        assert!(DepthFunc::Equal.test(0.5, 0.5));
        assert!(!DepthFunc::Equal.test(0.5001, 0.5));
    }

    #[test]
    fn depth_func_less_or_equal() {
        assert!(DepthFunc::LessOrEqual.test(0.3, 0.5));
        assert!(DepthFunc::LessOrEqual.test(0.5, 0.5));
        assert!(!DepthFunc::LessOrEqual.test(0.7, 0.5));
    }

    #[test]
    fn depth_func_greater() {
        assert!(!DepthFunc::Greater.test(0.3, 0.5));
        assert!(!DepthFunc::Greater.test(0.5, 0.5));
        assert!(DepthFunc::Greater.test(0.7, 0.5));
    }

    #[test]
    fn depth_func_not_equal() {
        assert!(!DepthFunc::NotEqual.test(0.5, 0.5));
        assert!(DepthFunc::NotEqual.test(0.5001, 0.5));
    }

    #[test]
    fn depth_func_greater_or_equal() {
        assert!(!DepthFunc::GreaterOrEqual.test(0.3, 0.5));
        assert!(DepthFunc::GreaterOrEqual.test(0.5, 0.5));
        assert!(DepthFunc::GreaterOrEqual.test(0.7, 0.5));
    }

    #[test]
    fn depth_func_always() {
        assert!(DepthFunc::Always.test(0.5, 0.5));
    }

    #[test]
    fn depth_test_disabled() {
        let dt = DepthTest::Disabled;
        assert!(!dt.should_test());
        assert!(!dt.should_write());
        assert_eq!(dt.func(), DepthFunc::Always);
    }

    #[test]
    fn depth_test_enabled() {
        let dt = DepthTest::Enabled(DepthFunc::Less);
        assert!(dt.should_test());
        assert!(dt.should_write());
        assert_eq!(dt.func(), DepthFunc::Less);
    }

    #[test]
    fn depth_test_default() {
        let dt = DepthTest::default();
        assert!(dt.should_test());
        assert_eq!(dt.func(), DepthFunc::Less);
    }

    #[test]
    fn depth_buffer_creation() {
        let db = DepthBuffer::new(640, 480).unwrap();
        assert_eq!(db.width(), 640);
        assert_eq!(db.height(), 480);
        assert_eq!(db.depths.len(), 640 * 480);
    }

    #[test]
    fn depth_buffer_zero_dimensions_rejected() {
        assert!(matches!(
            DepthBuffer::new(0, 480),
            Err(RenderError::InvalidRenderTarget)
        ));
        assert!(matches!(
            DepthBuffer::new(640, 0),
            Err(RenderError::InvalidRenderTarget)
        ));
    }

    #[test]
    fn depth_buffer_clear() {
        let mut db = DepthBuffer::new(10, 10).unwrap();
        db.clear(0.5);
        for &d in &db.depths {
            assert_eq!(d, 0.5);
        }
    }

    #[test]
    fn depth_buffer_clear_default() {
        let mut db = DepthBuffer::new(10, 10).unwrap();
        db.clear_default();
        for &d in &db.depths {
            assert_eq!(d, 1.0);
        }
    }

    #[test]
    fn depth_buffer_get_set() {
        let mut db = DepthBuffer::new(10, 10).unwrap();
        db.set(5, 5, 0.3).unwrap();
        assert_eq!(db.get(5, 5), Some(0.3));
    }

    #[test]
    fn depth_buffer_get_out_of_bounds() {
        let db = DepthBuffer::new(10, 10).unwrap();
        assert!(db.get(10, 5).is_none());
        assert!(db.get(5, 10).is_none());
    }

    #[test]
    fn depth_buffer_set_out_of_bounds() {
        let mut db = DepthBuffer::new(10, 10).unwrap();
        assert!(matches!(db.set(10, 5, 0.5), Err(RenderError::OutOfBounds)));
    }

    #[test]
    fn depth_buffer_test_and_set_pass() {
        let mut db = DepthBuffer::new(10, 10).unwrap();
        let result = db
            .test_and_set(5, 5, 0.3, DepthTest::Enabled(DepthFunc::Less))
            .unwrap();
        assert!(result);
        assert_eq!(db.get(5, 5), Some(0.3));
    }

    #[test]
    fn depth_buffer_test_and_set_fail() {
        let mut db = DepthBuffer::new(10, 10).unwrap();
        db.set(5, 5, 0.3).unwrap();
        let result = db
            .test_and_set(5, 5, 0.5, DepthTest::Enabled(DepthFunc::Less))
            .unwrap();
        assert!(!result);
        assert_eq!(db.get(5, 5), Some(0.3));
    }

    #[test]
    fn depth_buffer_test_disabled_always_passes() {
        let mut db = DepthBuffer::new(10, 10).unwrap();
        db.set(5, 5, 0.3).unwrap();
        let result = db.test_and_set(5, 5, 0.9, DepthTest::Disabled).unwrap();
        assert!(result);
        // DepthTest::Disabled doesn't write depth, so it should still be 0.3
        assert_eq!(db.get(5, 5), Some(0.3));
    }

    #[test]
    fn depth_buffer_fill_rect() {
        let mut db = DepthBuffer::new(10, 10).unwrap();
        db.fill_rect(2, 2, 4, 4, 0.5);
        for y in 0..10 {
            for x in 0..10 {
                let d = db.get(x, y).unwrap();
                if x >= 2 && x < 6 && y >= 2 && y < 6 {
                    assert_eq!(d, 0.5);
                } else {
                    assert_eq!(d, 1.0);
                }
            }
        }
    }

    #[test]
    fn depth_buffer_sample() {
        let mut db = DepthBuffer::new(10, 10).unwrap();
        db.set(3, 3, 0.7).unwrap();
        assert_eq!(db.sample(3, 3), Some(0.7));
    }
}
