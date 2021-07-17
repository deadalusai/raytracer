#![allow(unused)]

use std::ops::{ Add, Sub, Mul, Div, Neg };
use std::default::{ Default };

//
// Vec3
//

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32
}

impl Vec3 {
    pub fn new (x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x: x, y: y, z: z }
    }

    pub fn zero () -> Vec3 {
        Vec3::new(0.0, 0.0, 0.0)
    }

    pub fn unit_vector (self) -> Vec3 {
        let len = self.length();
        if len == 0.0 { self } else { self / len }
    }

    pub fn clamp (self) -> Vec3 {  
        Vec3 {
            x: self.x.min(1.0).max(-1.0),
            y: self.y.min(1.0).max(-1.0),
            z: self.z.min(1.0).max(-1.0),
        }
    }

    pub fn length (self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn length_squared (self) -> f32 {
        (self.x * self.x) + (self.y * self.y) + (self.z * self.z)
    }

    pub fn dot (a: Vec3, b: Vec3) -> f32 {
        a.x * b.x + a.y * b.y + a.z * b.z
    }
    
    pub fn cross (a: Vec3, b: Vec3) -> Vec3 {
        Vec3 {
            x:  (a.y * b.z - a.z * b.y),
            y: -(a.x * b.z - a.z * b.x),
            z:  (a.x * b.y - a.y * b.x)
        }
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add (self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z
        }
    }
}

impl Add<f32> for Vec3 {
    type Output = Vec3;
    fn add (self, f: f32) -> Vec3 {
        Vec3 {
            x: self.x + f,
            y: self.y + f,
            z: self.z + f
        }
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub (self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z
        }
    }
}

impl Sub<f32> for Vec3 {
    type Output = Vec3;
    fn sub (self, f: f32) -> Vec3 {
        Vec3 {
            x: self.x - f,
            y: self.y - f,
            z: self.z - f
        }
    }
}

impl Mul for Vec3 {
    type Output = Vec3;
    fn mul (self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z
        }
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul (self, f: f32) -> Vec3 {
        Vec3 {
            x: self.x * f,
            y: self.y * f,
            z: self.z * f
        }
    }
}

impl Div for Vec3 {
    type Output = Vec3;
    fn div (self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.x / other.x,
            y: self.y / other.y,
            z: self.z / other.z
        }
    }
}

impl Div<f32> for Vec3 {
    type Output = Vec3;
    fn div (self, f: f32) -> Vec3 {
        Vec3 {
            x: self.x / f,
            y: self.y / f,
            z: self.z / f
        }
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    fn neg (self) -> Vec3 {
        Vec3 {
            x: -self.x,
            y: -self.y,
            z: -self.z
        }
    }
}

impl Default for Vec3 {
    fn default () -> Vec3 {
        Vec3::zero()
    }
}

//
// Ray
//

pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Ray {
        Ray {
            origin: origin,
            direction: direction
        }
    }

    pub fn point_at_parameter(&self, t: f32) -> Vec3 {
        self.origin + (self.direction * t)
    }
}

//
// Rgb
//

pub type Rgb = [u8; 3];

pub fn rgb_from_vec3(v: &Vec3) -> Rgb {
    [(255.0 * v.x.sqrt()) as u8,
     (255.0 * v.y.sqrt()) as u8,
     (255.0 * v.z.sqrt()) as u8]
}