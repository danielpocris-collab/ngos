use alloc::vec::Vec;
use ngos_gfx_translate::RgbaColor;

fn sqrt_f32(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    let mut guess = x / 2.0;
    for _ in 0..10 {
        guess = (guess + x / guess) / 2.0;
    }
    guess
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectionalLight {
    pub direction: [f32; 3],
    pub color: RgbaColor,
    pub intensity: f32,
}

impl DirectionalLight {
    pub fn new(direction: [f32; 3], color: RgbaColor) -> Self {
        DirectionalLight {
            direction,
            color,
            intensity: 1.0,
        }
    }

    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    pub fn normalized_direction(&self) -> [f32; 3] {
        let len = sqrt_f32(
            self.direction[0] * self.direction[0]
                + self.direction[1] * self.direction[1]
                + self.direction[2] * self.direction[2],
        );
        if len > 0.0 {
            [
                self.direction[0] / len,
                self.direction[1] / len,
                self.direction[2] / len,
            ]
        } else {
            [0.0, -1.0, 0.0]
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointLight {
    pub position: [f32; 3],
    pub color: RgbaColor,
    pub intensity: f32,
    pub radius: f32,
}

impl PointLight {
    pub fn new(position: [f32; 3], color: RgbaColor) -> Self {
        PointLight {
            position,
            color,
            intensity: 1.0,
            radius: 10.0,
        }
    }

    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn attenuation_at(&self, point: &[f32; 3]) -> f32 {
        let dx = point[0] - self.position[0];
        let dy = point[1] - self.position[1];
        let dz = point[2] - self.position[2];
        let dist_sq = dx * dx + dy * dy + dz * dz;
        let dist = sqrt_f32(dist_sq);
        if dist >= self.radius {
            0.0
        } else {
            1.0 - (dist / self.radius)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AmbientLight {
    pub color: RgbaColor,
    pub intensity: f32,
}

impl AmbientLight {
    pub fn new(color: RgbaColor) -> Self {
        AmbientLight {
            color,
            intensity: 0.2,
        }
    }

    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LightType {
    Directional(DirectionalLight),
    Point(PointLight),
    Ambient(AmbientLight),
}

impl LightType {
    pub fn color(&self) -> RgbaColor {
        match self {
            LightType::Directional(l) => l.color,
            LightType::Point(l) => l.color,
            LightType::Ambient(l) => l.color,
        }
    }

    pub fn intensity(&self) -> f32 {
        match self {
            LightType::Directional(l) => l.intensity,
            LightType::Point(l) => l.intensity,
            LightType::Ambient(l) => l.intensity,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Light {
    pub light_type: LightType,
    pub enabled: bool,
}

impl Light {
    pub fn directional(direction: [f32; 3], color: RgbaColor) -> Self {
        Light {
            light_type: LightType::Directional(DirectionalLight::new(direction, color)),
            enabled: true,
        }
    }

    pub fn point(position: [f32; 3], color: RgbaColor) -> Self {
        Light {
            light_type: LightType::Point(PointLight::new(position, color)),
            enabled: true,
        }
    }

    pub fn ambient(color: RgbaColor) -> Self {
        Light {
            light_type: LightType::Ambient(AmbientLight::new(color)),
            enabled: true,
        }
    }

    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.light_type = match self.light_type {
            LightType::Directional(mut l) => {
                l.intensity = intensity;
                LightType::Directional(l)
            }
            LightType::Point(mut l) => {
                l.intensity = intensity;
                LightType::Point(l)
            }
            LightType::Ambient(mut l) => {
                l.intensity = intensity;
                LightType::Ambient(l)
            }
        };
        self
    }

    pub fn disable(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn enable(mut self) -> Self {
        self.enabled = true;
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

#[derive(Debug, Clone)]
pub struct LightManager {
    lights: Vec<Light>,
    ambient_override: Option<AmbientLight>,
}

impl LightManager {
    pub fn new() -> Self {
        LightManager {
            lights: Vec::new(),
            ambient_override: None,
        }
    }

    pub fn add_light(&mut self, light: Light) {
        self.lights.push(light);
    }

    pub fn remove_light(&mut self, index: usize) -> Option<Light> {
        if index < self.lights.len() {
            Some(self.lights.remove(index))
        } else {
            None
        }
    }

    pub fn set_ambient(&mut self, ambient: AmbientLight) {
        self.ambient_override = Some(ambient);
    }

    pub fn ambient(&self) -> AmbientLight {
        self.ambient_override
            .unwrap_or(AmbientLight::new(RgbaColor {
                r: 64,
                g: 64,
                b: 64,
                a: 255,
            }))
    }

    pub fn enabled_lights(&self) -> impl Iterator<Item = &Light> {
        self.lights.iter().filter(|l| l.enabled)
    }

    pub fn directional_lights(&self) -> impl Iterator<Item = &DirectionalLight> {
        self.lights.iter().filter_map(|l| {
            if l.enabled {
                if let LightType::Directional(ref dl) = l.light_type {
                    return Some(dl);
                }
            }
            None
        })
    }

    pub fn point_lights(&self) -> impl Iterator<Item = &PointLight> {
        self.lights.iter().filter_map(|l| {
            if l.enabled {
                if let LightType::Point(ref pl) = l.light_type {
                    return Some(pl);
                }
            }
            None
        })
    }

    pub fn light_count(&self) -> usize {
        self.lights.len()
    }

    pub fn enabled_count(&self) -> usize {
        self.lights.iter().filter(|l| l.enabled).count()
    }
}

impl Default for LightManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn white() -> RgbaColor {
        RgbaColor {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        }
    }

    fn red() -> RgbaColor {
        RgbaColor {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        }
    }

    #[test]
    fn directional_light_creation() {
        let light = DirectionalLight::new([0.0, -1.0, 0.0], white());
        assert_eq!(light.direction, [0.0, -1.0, 0.0]);
        assert_eq!(light.intensity, 1.0);
    }

    #[test]
    fn directional_light_normalized() {
        let light = DirectionalLight::new([0.0, -2.0, 0.0], white());
        let norm = light.normalized_direction();
        assert_eq!(norm, [0.0, -1.0, 0.0]);
    }

    #[test]
    fn directional_light_with_intensity() {
        let light = DirectionalLight::new([0.0, -1.0, 0.0], white()).with_intensity(0.5);
        assert_eq!(light.intensity, 0.5);
    }

    #[test]
    fn point_light_creation() {
        let light = PointLight::new([1.0, 2.0, 3.0], red());
        assert_eq!(light.position, [1.0, 2.0, 3.0]);
        assert_eq!(light.radius, 10.0);
    }

    #[test]
    fn point_light_attenuation_at_center() {
        let light = PointLight::new([0.0, 0.0, 0.0], white());
        let attenuation = light.attenuation_at(&[0.0, 0.0, 0.0]);
        assert_eq!(attenuation, 1.0);
    }

    #[test]
    fn point_light_attenuation_at_edge() {
        let light = PointLight::new([0.0, 0.0, 0.0], white()).with_radius(10.0);
        let attenuation = light.attenuation_at(&[10.0, 0.0, 0.0]);
        assert_eq!(attenuation, 0.0);
    }

    #[test]
    fn point_light_attenuation_outside_radius() {
        let light = PointLight::new([0.0, 0.0, 0.0], white()).with_radius(5.0);
        let attenuation = light.attenuation_at(&[10.0, 0.0, 0.0]);
        assert_eq!(attenuation, 0.0);
    }

    #[test]
    fn ambient_light_creation() {
        let light = AmbientLight::new(red());
        assert_eq!(light.color, red());
        assert_eq!(light.intensity, 0.2);
    }

    #[test]
    fn ambient_light_with_intensity() {
        let light = AmbientLight::new(white()).with_intensity(0.8);
        assert_eq!(light.intensity, 0.8);
    }

    #[test]
    fn light_directional_constructor() {
        let light = Light::directional([0.0, -1.0, 0.0], white());
        assert!(light.enabled);
        assert!(matches!(light.light_type, LightType::Directional(_)));
    }

    #[test]
    fn light_point_constructor() {
        let light = Light::point([0.0, 0.0, 0.0], red());
        assert!(light.enabled);
        assert!(matches!(light.light_type, LightType::Point(_)));
    }

    #[test]
    fn light_ambient_constructor() {
        let light = Light::ambient(white());
        assert!(light.enabled);
        assert!(matches!(light.light_type, LightType::Ambient(_)));
    }

    #[test]
    fn light_disable_enable() {
        let light = Light::directional([0.0, -1.0, 0.0], white()).disable();
        assert!(!light.enabled);
        let light = light.enable();
        assert!(light.enabled);
    }

    #[test]
    fn light_manager_add_and_count() {
        let mut manager = LightManager::new();
        manager.add_light(Light::directional([0.0, -1.0, 0.0], white()));
        manager.add_light(Light::point([1.0, 2.0, 3.0], red()));
        assert_eq!(manager.light_count(), 2);
        assert_eq!(manager.enabled_count(), 2);
    }

    #[test]
    fn light_manager_remove_light() {
        let mut manager = LightManager::new();
        manager.add_light(Light::directional([0.0, -1.0, 0.0], white()));
        let removed = manager.remove_light(0);
        assert!(removed.is_some());
        assert_eq!(manager.light_count(), 0);
    }

    #[test]
    fn light_manager_remove_out_of_bounds() {
        let mut manager = LightManager::new();
        let removed = manager.remove_light(99);
        assert!(removed.is_none());
    }

    #[test]
    fn light_manager_directional_iterator() {
        let mut manager = LightManager::new();
        manager.add_light(Light::directional([0.0, -1.0, 0.0], white()));
        manager.add_light(Light::point([0.0, 0.0, 0.0], red()));
        manager.add_light(Light::ambient(red()));
        let count = manager.directional_lights().count();
        assert_eq!(count, 1);
    }

    #[test]
    fn light_manager_point_iterator() {
        let mut manager = LightManager::new();
        manager.add_light(Light::directional([0.0, -1.0, 0.0], white()));
        manager.add_light(Light::point([0.0, 0.0, 0.0], red()));
        manager.add_light(Light::point([1.0, 1.0, 1.0], white()));
        let count = manager.point_lights().count();
        assert_eq!(count, 2);
    }

    #[test]
    fn light_manager_disabled_lights_excluded() {
        let mut manager = LightManager::new();
        manager.add_light(Light::directional([0.0, -1.0, 0.0], white()).disable());
        assert_eq!(manager.enabled_count(), 0);
        assert_eq!(manager.directional_lights().count(), 0);
    }

    #[test]
    fn light_manager_ambient_override() {
        let mut manager = LightManager::new();
        let ambient = AmbientLight::new(red()).with_intensity(0.5);
        manager.set_ambient(ambient);
        assert_eq!(manager.ambient().color, red());
        assert_eq!(manager.ambient().intensity, 0.5);
    }

    #[test]
    fn light_manager_default_ambient() {
        let manager = LightManager::new();
        let ambient = manager.ambient();
        assert_eq!(
            ambient.color,
            RgbaColor {
                r: 64,
                g: 64,
                b: 64,
                a: 255
            }
        );
        assert_eq!(ambient.intensity, 0.2);
    }
}
