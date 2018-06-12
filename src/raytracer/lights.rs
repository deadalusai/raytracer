#![allow(unused)]

use std;

use raytracer::types::{ Vec3 };
use raytracer::implementation::{ LightRecord, LightSource };

pub struct PointLight {
    origin: Vec3,
    color: Vec3,
    intensity: f32,
}

impl PointLight {
    pub fn with_origin (origin: Vec3) -> PointLight {
        PointLight {
            origin: origin,
            color: Vec3::new(1.0, 1.0, 1.0),
            intensity: 100.0,
        }
    }

    pub fn with_color (mut self, color: Vec3) -> PointLight {
        self.color = color;
        self
    }

    pub fn with_intensity (mut self, intensity: f32) -> PointLight {
        self.intensity = intensity;
        self
    }
}

impl LightSource for PointLight {
    fn get_direction_and_intensity (&self, p: &Vec3) -> Option<LightRecord> {
        // Cast a ray from point p back to the light
        let direction = p.sub(&self.origin);
        // Point light intensity falls off following the inverse square law
        let intensity = self.intensity / (4.0 * std::f32::consts::PI * direction.length());
        Some(LightRecord {
            direction: direction.unit_vector(),
            color: self.color.clone(),
            intensity: intensity
        })
    }
}


pub struct DirectionalLight {
    origin: Vec3,
    direction: Vec3,
    color: Vec3,
    intensity: f32,
}

impl DirectionalLight {
    pub fn with_origin_and_direction (origin: Vec3, direction: Vec3) -> DirectionalLight {
        DirectionalLight {
            origin: origin,
            direction: direction.unit_vector(),
            color: Vec3::new(1.0, 1.0, 1.0),
            intensity: 1.0,
        }
    }

    pub fn with_color (mut self, color: Vec3) -> DirectionalLight {
        self.color = color;
        self
    }

    pub fn with_intensity (mut self, intensity: f32) -> DirectionalLight {
        self.intensity = intensity;
        self
    }
}

impl LightSource for DirectionalLight {
    fn get_direction_and_intensity (&self, p: &Vec3) -> Option<LightRecord> {
        // Directional lights have the same direction + intensity at all locations in the scene
        Some(LightRecord {
            direction: self.direction.clone(),
            color: self.color.clone(),
            intensity: self.intensity
        })
    }
}