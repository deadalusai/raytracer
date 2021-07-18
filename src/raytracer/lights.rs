#![allow(unused)]

use std;
use std::f32::consts::PI;

use raytracer::types::{ V3 };
use raytracer::implementation::{ LightRecord, LightSource };

pub struct PointLight {
    origin: V3,
    color: V3,
    intensity: f32,
}

impl PointLight {
    pub fn with_origin (origin: V3) -> PointLight {
        PointLight {
            origin: origin,
            color: V3(1.0, 1.0, 1.0),
            intensity: 100.0,
        }
    }

    pub fn with_color (mut self, color: V3) -> PointLight {
        self.color = color;
        self
    }

    pub fn with_intensity (mut self, intensity: f32) -> PointLight {
        self.intensity = intensity;
        self
    }
}

impl LightSource for PointLight {
    fn get_direction_and_intensity(&self, p: V3) -> Option<LightRecord> {
        // Cast a ray from point p back to the light
        let direction = p - self.origin;
        // Point light intensity falls off following the inverse square law
        let intensity = self.intensity / (4.0 * std::f32::consts::PI * direction.length());
        Some(LightRecord {
            direction: direction.unit(),
            color: self.color.clone(),
            intensity: intensity
        })
    }
}


pub struct LampLight {
    origin: V3,
    direction: V3,
    color: V3,
    intensity: f32,
    angle_deg: f32,
}

impl LampLight {
    pub fn with_origin_and_direction(origin: V3, direction: V3) -> LampLight {
        LampLight {
            origin: origin,
            direction: direction.unit(),
            color: V3(1.0, 1.0, 1.0),
            intensity: 1.0,
            angle_deg: 45.0,
        }
    }

    pub fn with_color(mut self, color: V3) -> LampLight {
        self.color = color;
        self
    }

    pub fn with_intensity(mut self, intensity: f32) -> LampLight {
        self.intensity = intensity;
        self
    }

    pub fn with_angle(mut self, angle_deg: f32) -> LampLight {
        self.angle_deg = angle_deg;
        self
    }
}

impl LightSource for LampLight {
    fn get_direction_and_intensity(&self, p: V3) -> Option<LightRecord> {
        // Cast a ray from the light back to point p
        let direction_to_p = p - self.origin;
        let direction_of_lamp = self.direction;
        // Calculate the angle between this lamp's direction and that vector
        let theta = V3::theta(direction_to_p, direction_of_lamp);
        let theta_deg = theta / PI * 180.0;
        // Does the ray fall outside the cone of light?
        if theta_deg > self.angle_deg {
            return None;
        }
        // Lamp light intensity falls off following the inverse square law
        let intensity = self.intensity / (4.0 * PI * direction_to_p.length());
        Some(LightRecord {
            direction: direction_to_p,
            color: self.color,
            intensity: intensity
        })
    }
}


pub struct DirectionalLight {
    origin: V3,
    direction: V3,
    color: V3,
    intensity: f32,
}

impl DirectionalLight {
    pub fn with_origin_and_direction (origin: V3, direction: V3) -> DirectionalLight {
        DirectionalLight {
            origin: origin,
            direction: direction.unit(),
            color: V3(1.0, 1.0, 1.0),
            intensity: 1.0,
        }
    }

    pub fn with_color (mut self, color: V3) -> DirectionalLight {
        self.color = color;
        self
    }

    pub fn with_intensity (mut self, intensity: f32) -> DirectionalLight {
        self.intensity = intensity;
        self
    }
}

impl LightSource for DirectionalLight {
    fn get_direction_and_intensity(&self, p: V3) -> Option<LightRecord> {
        // Directional lights have the same direction + intensity at all locations in the scene
        Some(LightRecord {
            direction: self.direction,
            color: self.color,
            intensity: self.intensity
        })
    }
}