use crate::RenderError;
use alloc::vec::Vec;
use ngos_gfx_translate::RgbaColor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MaterialId(pub u32);

impl MaterialId {
    pub fn new(id: u32) -> Self {
        MaterialId(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TextureId(pub u32);

impl TextureId {
    pub fn new(id: u32) -> Self {
        TextureId(id)
    }
}

#[derive(Debug, Clone)]
pub struct Texture {
    id: TextureId,
    width: u32,
    height: u32,
    pixels: Vec<RgbaColor>,
}

impl Texture {
    pub fn new(
        id: TextureId,
        width: u32,
        height: u32,
        pixels: Vec<RgbaColor>,
    ) -> Result<Self, RenderError> {
        let expected_size = (width * height) as usize;
        if pixels.len() != expected_size {
            return Err(RenderError::OutOfBounds);
        }
        if width == 0 || height == 0 {
            return Err(RenderError::InvalidRenderTarget);
        }
        Ok(Texture {
            id,
            width,
            height,
            pixels,
        })
    }

    pub fn id(&self) -> TextureId {
        self.id
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn sample(&self, u: f32, v: f32) -> RgbaColor {
        let u_clamped = u.clamp(0.0, 1.0);
        let v_clamped = v.clamp(0.0, 1.0);
        let x = (u_clamped * (self.width - 1) as f32) as u32;
        let y = ((1.0 - v_clamped) * (self.height - 1) as f32) as u32;
        let index = (y * self.width + x) as usize;
        self.pixels.get(index).copied().unwrap_or(RgbaColor {
            r: 255,
            g: 0,
            b: 255,
            a: 255,
        })
    }

    pub fn pixels(&self) -> &[RgbaColor] {
        &self.pixels
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Material {
    pub id: MaterialId,
    pub diffuse: RgbaColor,
    pub specular: RgbaColor,
    pub ambient: RgbaColor,
    pub emissive: RgbaColor,
    pub shininess: f32,
    pub roughness: f32,
    pub metallic: f32,
    pub transmission: f32,
    pub texture_id: Option<TextureId>,
    pub normal_map_id: Option<TextureId>,
    pub emissive_map_id: Option<TextureId>,
}

impl Material {
    pub fn new(id: MaterialId, diffuse: RgbaColor) -> Self {
        Material {
            id,
            diffuse,
            specular: RgbaColor {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            ambient: RgbaColor {
                r: 64,
                g: 64,
                b: 64,
                a: 255,
            },
            emissive: RgbaColor {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            shininess: 32.0,
            roughness: 0.45,
            metallic: 0.0,
            transmission: 0.0,
            texture_id: None,
            normal_map_id: None,
            emissive_map_id: None,
        }
    }

    pub fn with_specular(mut self, specular: RgbaColor) -> Self {
        self.specular = specular;
        self
    }

    pub fn with_ambient(mut self, ambient: RgbaColor) -> Self {
        self.ambient = ambient;
        self
    }

    pub fn with_shininess(mut self, shininess: f32) -> Self {
        self.shininess = shininess;
        self
    }

    pub fn with_emissive(mut self, emissive: RgbaColor) -> Self {
        self.emissive = emissive;
        self
    }

    pub fn with_roughness(mut self, roughness: f32) -> Self {
        self.roughness = roughness.clamp(0.0, 1.0);
        self
    }

    pub fn with_metallic(mut self, metallic: f32) -> Self {
        self.metallic = metallic.clamp(0.0, 1.0);
        self
    }

    pub fn with_transmission(mut self, transmission: f32) -> Self {
        self.transmission = transmission.clamp(0.0, 1.0);
        self
    }

    pub fn with_texture(mut self, texture_id: TextureId) -> Self {
        self.texture_id = Some(texture_id);
        self
    }

    pub fn with_normal_map(mut self, texture_id: TextureId) -> Self {
        self.normal_map_id = Some(texture_id);
        self
    }

    pub fn with_emissive_map(mut self, texture_id: TextureId) -> Self {
        self.emissive_map_id = Some(texture_id);
        self
    }

    pub fn diffuse_color(&self) -> RgbaColor {
        self.diffuse
    }

    pub fn specular_color(&self) -> RgbaColor {
        self.specular
    }

    pub fn ambient_color(&self) -> RgbaColor {
        self.ambient
    }

    pub fn emissive_color(&self) -> RgbaColor {
        self.emissive
    }

    pub fn is_pbr_heavy(&self) -> bool {
        self.roughness > 0.55
            || self.metallic > 0.2
            || self.transmission > 0.0
            || self.normal_map_id.is_some()
            || self.emissive_map_id.is_some()
    }

    pub fn monster_surface(
        id: MaterialId,
        diffuse: RgbaColor,
        emissive: RgbaColor,
        texture_id: Option<TextureId>,
    ) -> Self {
        let mut material = Self::new(id, diffuse)
            .with_specular(RgbaColor {
                r: 0xf8,
                g: 0xfb,
                b: 0xff,
                a: 0xff,
            })
            .with_ambient(RgbaColor {
                r: 0x10,
                g: 0x16,
                b: 0x24,
                a: 0xff,
            })
            .with_emissive(emissive)
            .with_shininess(96.0)
            .with_roughness(0.28)
            .with_metallic(0.72);
        if let Some(texture_id) = texture_id {
            material = material
                .with_texture(texture_id)
                .with_normal_map(texture_id)
                .with_emissive_map(texture_id);
        }
        material
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_texture() -> Texture {
        let pixels = vec![
            RgbaColor {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            },
            RgbaColor {
                r: 0,
                g: 255,
                b: 0,
                a: 255,
            },
            RgbaColor {
                r: 0,
                g: 0,
                b: 255,
                a: 255,
            },
            RgbaColor {
                r: 255,
                g: 255,
                b: 0,
                a: 255,
            },
        ];
        Texture::new(TextureId::new(1), 2, 2, pixels).unwrap()
    }

    #[test]
    fn material_id_creation() {
        let id = MaterialId::new(42);
        assert_eq!(id.0, 42);
    }

    #[test]
    fn texture_id_creation() {
        let id = TextureId::new(99);
        assert_eq!(id.0, 99);
    }

    #[test]
    fn texture_creation_valid() {
        let pixels = vec![RgbaColor {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        }];
        let tex = Texture::new(TextureId::new(1), 1, 1, pixels).unwrap();
        assert_eq!(tex.width(), 1);
        assert_eq!(tex.height(), 1);
    }

    #[test]
    fn texture_wrong_size_rejected() {
        let pixels = vec![RgbaColor {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        }];
        let err = Texture::new(TextureId::new(1), 2, 2, pixels).unwrap_err();
        assert!(matches!(err, RenderError::OutOfBounds));
    }

    #[test]
    fn texture_zero_dimensions_rejected() {
        let pixels = vec![];
        let err = Texture::new(TextureId::new(1), 0, 1, pixels).unwrap_err();
        assert!(matches!(err, RenderError::InvalidRenderTarget));
    }

    #[test]
    fn texture_sample_corners() {
        let tex = test_texture();
        let tl = tex.sample(0.0, 1.0);
        assert_eq!(tl.r, 255);
        assert_eq!(tl.g, 0);
        let br = tex.sample(1.0, 0.0);
        assert_eq!(br.r, 255);
        assert_eq!(br.g, 255);
    }

    #[test]
    fn texture_sample_clamped() {
        let tex = test_texture();
        let out_of_bounds = tex.sample(2.0, -1.0);
        assert!(out_of_bounds.a == 255);
    }

    #[test]
    fn material_default_values() {
        let mat = Material::new(
            MaterialId::new(1),
            RgbaColor {
                r: 128,
                g: 64,
                b: 32,
                a: 255,
            },
        );
        assert_eq!(mat.diffuse.r, 128);
        assert_eq!(
            mat.specular,
            RgbaColor {
                r: 255,
                g: 255,
                b: 255,
                a: 255
            }
        );
        assert_eq!(
            mat.ambient,
            RgbaColor {
                r: 64,
                g: 64,
                b: 64,
                a: 255
            }
        );
        assert_eq!(mat.shininess, 32.0);
        assert!(mat.texture_id.is_none());
    }

    #[test]
    fn material_with_specular() {
        let mat = Material::new(
            MaterialId::new(1),
            RgbaColor {
                r: 100,
                g: 100,
                b: 100,
                a: 255,
            },
        )
        .with_specular(RgbaColor {
            r: 200,
            g: 200,
            b: 200,
            a: 255,
        });
        assert_eq!(mat.specular.r, 200);
    }

    #[test]
    fn material_with_texture() {
        let mat = Material::new(
            MaterialId::new(1),
            RgbaColor {
                r: 100,
                g: 100,
                b: 100,
                a: 255,
            },
        )
        .with_texture(TextureId::new(5));
        assert_eq!(mat.texture_id, Some(TextureId::new(5)));
    }

    #[test]
    fn material_builder_pattern() {
        let mat = Material::new(
            MaterialId::new(1),
            RgbaColor {
                r: 50,
                g: 50,
                b: 50,
                a: 255,
            },
        )
        .with_specular(RgbaColor {
            r: 100,
            g: 100,
            b: 100,
            a: 255,
        })
        .with_ambient(RgbaColor {
            r: 25,
            g: 25,
            b: 25,
            a: 255,
        })
        .with_shininess(64.0)
        .with_texture(TextureId::new(3));
        assert_eq!(mat.diffuse.r, 50);
        assert_eq!(mat.specular.r, 100);
        assert_eq!(mat.ambient.r, 25);
        assert_eq!(mat.shininess, 64.0);
        assert_eq!(mat.texture_id, Some(TextureId::new(3)));
    }
}
