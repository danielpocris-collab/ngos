use crate::{
    RenderError,
    depth_buffer_agent::{DepthBuffer, DepthTest},
    mesh_agent::Vertex,
};
use alloc::vec::Vec;
use ngos_gfx_translate::RgbaColor;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RasterizedPixel {
    pub x: u32,
    pub y: u32,
    pub depth: f32,
    pub color: RgbaColor,
}

#[derive(Debug, Clone)]
pub struct RasterizerConfig {
    pub width: u32,
    pub height: u32,
    pub backface_culling: bool,
    pub depth_test: DepthTest,
}

impl RasterizerConfig {
    pub fn new(width: u32, height: u32) -> Self {
        RasterizerConfig {
            width,
            height,
            backface_culling: true,
            depth_test: DepthTest::default(),
        }
    }

    pub fn with_backface_culling(mut self, enabled: bool) -> Self {
        self.backface_culling = enabled;
        self
    }

    pub fn with_depth_test(mut self, depth_test: DepthTest) -> Self {
        self.depth_test = depth_test;
        self
    }
}

#[derive(Debug, Clone)]
pub struct Rasterizer {
    config: RasterizerConfig,
    pixels: Vec<RgbaColor>,
}

impl Rasterizer {
    pub fn new(config: RasterizerConfig) -> Result<Self, RenderError> {
        if config.width == 0 || config.height == 0 {
            return Err(RenderError::InvalidRenderTarget);
        }
        let size = (config.width * config.height) as usize;
        let mut pixels = Vec::with_capacity(size);
        for _ in 0..size {
            pixels.push(RgbaColor {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            });
        }
        Ok(Rasterizer { config, pixels })
    }

    pub fn clear(&mut self, color: RgbaColor) {
        for pixel in &mut self.pixels {
            *pixel = color;
        }
    }

    pub fn width(&self) -> u32 {
        self.config.width
    }

    pub fn height(&self) -> u32 {
        self.config.height
    }

    fn index(&self, x: u32, y: u32) -> Option<usize> {
        if x >= self.config.width || y >= self.config.height {
            return None;
        }
        Some((y * self.config.width + x) as usize)
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> Option<RgbaColor> {
        self.index(x, y).map(|i| self.pixels[i])
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, color: RgbaColor) -> Result<(), RenderError> {
        match self.index(x, y) {
            Some(i) => {
                self.pixels[i] = color;
                Ok(())
            }
            None => Err(RenderError::OutOfBounds),
        }
    }

    pub fn pixels(&self) -> &[RgbaColor] {
        &self.pixels
    }

    pub fn config(&self) -> &RasterizerConfig {
        &self.config
    }

    pub fn is_backface(v0: &[f32; 2], v1: &[f32; 2], v2: &[f32; 2]) -> bool {
        let edge1 = [v1[0] - v0[0], v1[1] - v0[1]];
        let edge2 = [v2[0] - v0[0], v2[1] - v0[1]];
        let cross = edge1[0] * edge2[1] - edge1[1] * edge2[0];
        cross < 0.0
    }

    fn barycentric_coords(p: &[f32; 2], v0: &[f32; 2], v1: &[f32; 2], v2: &[f32; 2]) -> [f32; 3] {
        let v0v1 = [v1[0] - v0[0], v1[1] - v0[1]];
        let v0v2 = [v2[0] - v0[0], v2[1] - v0[1]];
        let v0p = [p[0] - v0[0], p[1] - v0[1]];

        let d00 = v0v1[0] * v0v1[0] + v0v1[1] * v0v1[1];
        let d01 = v0v1[0] * v0v2[0] + v0v1[1] * v0v2[1];
        let d11 = v0v2[0] * v0v2[0] + v0v2[1] * v0v2[1];
        let d20 = v0p[0] * v0v1[0] + v0p[1] * v0v1[1];
        let d21 = v0p[0] * v0v2[0] + v0p[1] * v0v2[1];

        let denom = d00 * d11 - d01 * d01;
        if denom.abs() < 1e-10 {
            return [1.0, 0.0, 0.0];
        }

        let v = (d11 * d20 - d01 * d21) / denom;
        let w = (d00 * d21 - d01 * d20) / denom;
        let u = 1.0 - v - w;

        [u, v, w]
    }

    fn interpolate<T: Copy + Into<f32>>(v0: T, v1: T, v2: T, bary: &[f32; 3]) -> f32 {
        bary[0] * v0.into() + bary[1] * v1.into() + bary[2] * v2.into()
    }

    fn interpolate_depth(v0: &Vertex, v1: &Vertex, v2: &Vertex, bary: &[f32; 3]) -> f32 {
        Self::interpolate(v0.position[2], v1.position[2], v2.position[2], bary)
    }

    fn interpolate_color(v0: &Vertex, v1: &Vertex, v2: &Vertex, bary: &[f32; 3]) -> RgbaColor {
        let r = Self::interpolate(
            v0.color.r as f32,
            v1.color.r as f32,
            v2.color.r as f32,
            bary,
        );
        let g = Self::interpolate(
            v0.color.g as f32,
            v1.color.g as f32,
            v2.color.g as f32,
            bary,
        );
        let b = Self::interpolate(
            v0.color.b as f32,
            v1.color.b as f32,
            v2.color.b as f32,
            bary,
        );
        let a = Self::interpolate(
            v0.color.a as f32,
            v1.color.a as f32,
            v2.color.a as f32,
            bary,
        );
        RgbaColor {
            r: r as u8,
            g: g as u8,
            b: b as u8,
            a: a as u8,
        }
    }

    pub fn rasterize_triangle(
        &mut self,
        v0: &Vertex,
        v1: &Vertex,
        v2: &Vertex,
        depth_buffer: &mut DepthBuffer,
    ) -> Result<Vec<RasterizedPixel>, RenderError> {
        let p0 = [v0.position[0], v0.position[1]];
        let p1 = [v1.position[0], v1.position[1]];
        let p2 = [v2.position[0], v2.position[1]];

        if self.config.backface_culling && Self::is_backface(&p0, &p1, &p2) {
            return Ok(Vec::new());
        }

        let min_x = p0[0].min(p1[0]).min(p2[0]).max(0.0) as u32;
        let max_x = p0[0]
            .max(p1[0])
            .max(p2[0])
            .min(self.config.width as f32 - 1.0) as u32;
        let min_y = p0[1].min(p1[1]).min(p2[1]).max(0.0) as u32;
        let max_y = p0[1]
            .max(p1[1])
            .max(p2[1])
            .min(self.config.height as f32 - 1.0) as u32;

        let mut pixels = Vec::new();

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let p = [x as f32, y as f32];
                let bary = Self::barycentric_coords(&p, &p0, &p1, &p2);

                if bary[0] >= -1e-6 && bary[1] >= -1e-6 && bary[2] >= -1e-6 {
                    let depth = Self::interpolate_depth(v0, v1, v2, &bary);
                    let depth_ndc = (depth + 1.0) / 2.0;

                    if depth_buffer
                        .test_and_set(x, y, depth_ndc, self.config.depth_test)
                        .unwrap_or(false)
                    {
                        let color = Self::interpolate_color(v0, v1, v2, &bary);
                        let _ = self.set_pixel(x, y, color);
                        pixels.push(RasterizedPixel {
                            x,
                            y,
                            depth: depth_ndc,
                            color,
                        });
                    }
                }
            }
        }

        Ok(pixels)
    }

    pub fn rasterize_triangle_wireframe(
        &mut self,
        v0: &Vertex,
        v1: &Vertex,
        v2: &Vertex,
        color: RgbaColor,
    ) -> Result<(), RenderError> {
        self.draw_line(
            v0.position[0] as i32,
            v0.position[1] as i32,
            v1.position[0] as i32,
            v1.position[1] as i32,
            color,
        )?;
        self.draw_line(
            v1.position[0] as i32,
            v1.position[1] as i32,
            v2.position[0] as i32,
            v2.position[1] as i32,
            color,
        )?;
        self.draw_line(
            v2.position[0] as i32,
            v2.position[1] as i32,
            v0.position[0] as i32,
            v0.position[1] as i32,
            color,
        )?;
        Ok(())
    }

    fn draw_line(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        color: RgbaColor,
    ) -> Result<(), RenderError> {
        // Handle vertical and horizontal lines specially
        if x0 == x1 {
            let (start_y, end_y) = if y0 < y1 { (y0, y1) } else { (y1, y0) };
            for y in start_y..=end_y {
                if y >= 0
                    && (y as u32) < self.config.height
                    && x0 >= 0
                    && (x0 as u32) < self.config.width
                {
                    let _ = self.set_pixel(x0 as u32, y as u32, color);
                }
            }
            return Ok(());
        }
        if y0 == y1 {
            let (start_x, end_x) = if x0 < x1 { (x0, x1) } else { (x1, x0) };
            for x in start_x..=end_x {
                if x >= 0
                    && (x as u32) < self.config.width
                    && y0 >= 0
                    && (y0 as u32) < self.config.height
                {
                    let _ = self.set_pixel(x as u32, y0 as u32, color);
                }
            }
            return Ok(());
        }

        // Bresenham's line algorithm for diagonal lines
        let mut x0 = x0;
        let mut y0 = y0;
        let x1 = x1;
        let y1 = y1;

        let dx = i32::abs(x1 - x0);
        let dy = i32::abs(y1 - y0);
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx / 2;
        let mut e2: i32;
        let mut iterations = 0;
        let max_iterations = (dx + dy + 2) as usize;

        loop {
            if iterations > max_iterations {
                break;
            }
            iterations += 1;

            if x0 >= 0
                && y0 >= 0
                && (x0 as u32) < self.config.width
                && (y0 as u32) < self.config.height
            {
                let _ = self.set_pixel(x0 as u32, y0 as u32, color);
            }
            if x0 == x1 && y0 == y1 {
                break;
            }
            e2 = err;
            if e2 >= dy {
                err -= dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::depth_buffer_agent::DepthFunc;

    fn test_vertex(x: f32, y: f32, z: f32) -> Vertex {
        Vertex::new(
            [x, y, z],
            [0.0, 0.0, 1.0],
            [0.0, 0.0],
            RgbaColor {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            },
        )
    }

    #[test]
    fn rasterizer_creation() {
        let config = RasterizerConfig::new(640, 480);
        let rasterizer = Rasterizer::new(config).unwrap();
        assert_eq!(rasterizer.width(), 640);
        assert_eq!(rasterizer.height(), 480);
    }

    #[test]
    fn rasterizer_zero_dimensions_rejected() {
        let config = RasterizerConfig::new(0, 480);
        assert!(matches!(
            Rasterizer::new(config),
            Err(RenderError::InvalidRenderTarget)
        ));
    }

    #[test]
    fn rasterizer_clear() {
        let config = RasterizerConfig::new(10, 10);
        let mut rasterizer = Rasterizer::new(config).unwrap();
        let color = RgbaColor {
            r: 100,
            g: 150,
            b: 200,
            a: 255,
        };
        rasterizer.clear(color);
        for &pixel in rasterizer.pixels() {
            assert_eq!(pixel, color);
        }
    }

    #[test]
    fn rasterizer_set_get_pixel() {
        let config = RasterizerConfig::new(10, 10);
        let mut rasterizer = Rasterizer::new(config).unwrap();
        let color = RgbaColor {
            r: 50,
            g: 100,
            b: 150,
            a: 200,
        };
        rasterizer.set_pixel(5, 5, color).unwrap();
        assert_eq!(rasterizer.get_pixel(5, 5), Some(color));
    }

    #[test]
    fn rasterizer_set_pixel_out_of_bounds() {
        let config = RasterizerConfig::new(10, 10);
        let mut rasterizer = Rasterizer::new(config).unwrap();
        let color = RgbaColor {
            r: 50,
            g: 100,
            b: 150,
            a: 200,
        };
        assert!(matches!(
            rasterizer.set_pixel(10, 5, color),
            Err(RenderError::OutOfBounds)
        ));
    }

    #[test]
    fn backface_culling_front_facing() {
        let v0 = [0.0, 0.0];
        let v1 = [100.0, 0.0];
        let v2 = [50.0, 100.0];
        assert!(!Rasterizer::is_backface(&v0, &v1, &v2));
    }

    #[test]
    fn backface_culling_back_facing() {
        let v0 = [0.0, 0.0];
        let v1 = [50.0, 100.0];
        let v2 = [100.0, 0.0];
        assert!(Rasterizer::is_backface(&v0, &v1, &v2));
    }

    #[test]
    fn barycentric_coords_vertex() {
        let v0 = [0.0, 0.0];
        let v1 = [100.0, 0.0];
        let v2 = [0.0, 100.0];
        let bary = Rasterizer::barycentric_coords(&v0, &v0, &v1, &v2);
        assert!((bary[0] - 1.0).abs() < 1e-4);
        assert!(bary[1].abs() < 1e-4);
        assert!(bary[2].abs() < 1e-4);
    }

    #[test]
    fn barycentric_coords_center() {
        let v0 = [0.0, 0.0];
        let v1 = [100.0, 0.0];
        let v2 = [0.0, 100.0];
        let p = [33.33, 33.33];
        let bary = Rasterizer::barycentric_coords(&p, &v0, &v1, &v2);
        assert!(bary[0] > 0.3 && bary[0] < 0.4);
        assert!(bary[1] > 0.3 && bary[1] < 0.4);
        assert!(bary[2] > 0.3 && bary[2] < 0.4);
    }

    #[test]
    fn rasterize_triangle_solid() {
        let config = RasterizerConfig::new(100, 100)
            .with_backface_culling(false)
            .with_depth_test(DepthTest::Disabled);
        let mut rasterizer = Rasterizer::new(config).unwrap();
        let mut depth_buffer = DepthBuffer::new(100, 100).unwrap();

        let v0 = test_vertex(10.0, 10.0, 0.5);
        let v1 = test_vertex(90.0, 10.0, 0.5);
        let v2 = test_vertex(50.0, 90.0, 0.5);

        let pixels = rasterizer
            .rasterize_triangle(&v0, &v1, &v2, &mut depth_buffer)
            .unwrap();

        assert!(!pixels.is_empty());
    }

    #[test]
    fn rasterize_triangle_depth_test() {
        let config = RasterizerConfig::new(100, 100)
            .with_backface_culling(false)
            .with_depth_test(DepthTest::Enabled(DepthFunc::Less));
        let mut rasterizer = Rasterizer::new(config).unwrap();
        let mut depth_buffer = DepthBuffer::new(100, 100).unwrap();

        let v0_front = test_vertex(10.0, 10.0, 0.1);
        let v1_front = test_vertex(90.0, 10.0, 0.1);
        let v2_front = test_vertex(50.0, 90.0, 0.1);

        let v0_back = test_vertex(10.0, 10.0, 0.9);
        let v1_back = test_vertex(90.0, 10.0, 0.9);
        let v2_back = test_vertex(50.0, 90.0, 0.9);

        let pixels_front = rasterizer
            .rasterize_triangle(&v0_front, &v1_front, &v2_front, &mut depth_buffer)
            .unwrap();

        let pixels_back = rasterizer
            .rasterize_triangle(&v0_back, &v1_back, &v2_back, &mut depth_buffer)
            .unwrap();

        assert!(!pixels_front.is_empty());
        assert!(pixels_back.is_empty());
    }

    #[test]
    fn draw_line_horizontal() {
        let config = RasterizerConfig::new(100, 100);
        let mut rasterizer = Rasterizer::new(config).unwrap();
        let color = RgbaColor {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        };
        rasterizer.draw_line(10, 50, 90, 50, color).unwrap();

        for x in 10..=90 {
            assert_eq!(
                rasterizer.get_pixel(x, 50),
                Some(color),
                "pixel at ({}, 50) should be set",
                x
            );
        }
    }

    #[test]
    fn draw_line_vertical() {
        let config = RasterizerConfig::new(100, 100);
        let mut rasterizer = Rasterizer::new(config).unwrap();
        let color = RgbaColor {
            r: 0,
            g: 255,
            b: 0,
            a: 255,
        };
        rasterizer.draw_line(50, 10, 50, 90, color).unwrap();

        for y in 10..=90 {
            assert_eq!(
                rasterizer.get_pixel(50, y),
                Some(color),
                "pixel at (50, {}) should be set",
                y
            );
        }
    }

    #[test]
    fn rasterize_triangle_wireframe() {
        let config = RasterizerConfig::new(100, 100);
        let mut rasterizer = Rasterizer::new(config).unwrap();
        let color = RgbaColor {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        };

        let v0 = test_vertex(10.0, 10.0, 0.5);
        let v1 = test_vertex(90.0, 10.0, 0.5);
        let v2 = test_vertex(50.0, 90.0, 0.5);

        rasterizer
            .rasterize_triangle_wireframe(&v0, &v1, &v2, color)
            .unwrap();

        assert!(rasterizer.get_pixel(10, 10).is_some());
        assert!(rasterizer.get_pixel(90, 10).is_some());
        assert!(rasterizer.get_pixel(50, 90).is_some());
    }

    #[test]
    fn rasterizer_config_builder() {
        let config = RasterizerConfig::new(640, 480)
            .with_backface_culling(false)
            .with_depth_test(DepthTest::Disabled);
        assert!(!config.backface_culling);
        assert!(matches!(config.depth_test, DepthTest::Disabled));
    }
}
