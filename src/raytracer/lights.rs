
use std;

use raytracer::types::{ Vec3 };
use raytracer::implementation::{ LightRecord, LightSource };

pub struct PointLight {
    origin: Vec3,
    color: Vec3,
    intensity: f32,
}

impl PointLight {
    pub fn new (origin: Vec3, color: Vec3, intensity: f32) -> PointLight {
        PointLight { origin: origin, color: color, intensity: intensity }
    }
}

impl LightSource for PointLight {
    fn get_direction_and_intensity (&self, p: &Vec3) -> Option<LightRecord> {
        // Cast a ray from point p back to the light
        let direction = p.sub(&self.origin);
        // Point light intensity falls off following the inverse square law
        let intensity = self.intensity / (4.0 * std::f32::consts::PI * direction.length());
        let light = LightRecord { direction: direction.unit_vector(), color: self.color.clone(), intensity: intensity };
        Some(light)
    }
}