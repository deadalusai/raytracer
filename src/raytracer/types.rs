#![allow(unused)]

use std::ops::{ Add, Sub, Mul, Div, Neg };
use std::default::{ Default };

//
// Vec3
//

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct V3(pub f32, pub f32, pub f32); // x, y, z

impl V3 {
    pub fn x(&self) -> f32 {
        self.0
    }
    
    pub fn y(&self) -> f32 {
        self.1
    }
    
    pub fn z(&self) -> f32 {
        self.2
    }

    pub fn zero() -> V3 {
        V3(0.0, 0.0, 0.0)
    }

    pub fn one() -> V3 {
        V3(1.0, 1.0, 1.0)
    }

    pub fn unit(self) -> V3 {
        let len = self.length();
        if len == 0.0 { self } else { self / len }
    }

    pub fn clamp(self) -> V3 {  
        V3(self.0.min(1.0).max(-1.0),
           self.1.min(1.0).max(-1.0),
           self.2.min(1.0).max(-1.0))
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn length_squared(self) -> f32 {
        (self.0 * self.0) + (self.1 * self.1) + (self.2 * self.2)
    }

    pub fn dot(a: V3, b: V3) -> f32 {
        a.0 * b.0 + a.1 * b.1 + a.2 * b.2
    }
    
    pub fn cross(a: V3, b: V3) -> V3 {
        V3((a.1 * b.2 - a.2 * b.1),
          -(a.0 * b.2 - a.2 * b.0),
           (a.0 * b.1 - a.1 * b.0))
    }

    // Calculate the angle between two vectors
    pub fn theta(a: V3, b: V3) -> f32 {
        // theta_rad = acos((a . b) / (|a| * |b|))
        let theta = (V3::dot(a, b) / (a.length() * b.length())).acos();
        theta
    }
}

impl Add for V3 {
    type Output = V3;
    fn add(self, other: V3) -> V3 {
        V3(self.0 + other.0,
           self.1 + other.1,
           self.2 + other.2)
    }
}

impl Add<f32> for V3 {
    type Output = V3;
    fn add(self, f: f32) -> V3 {
        V3(self.0 + f,
           self.1 + f,
           self.2 + f)
    }
}

impl Sub for V3 {
    type Output = V3;
    fn sub(self, other: V3) -> V3 {
        V3(self.0 - other.0,
           self.1 - other.1,
           self.2 - other.2)
    }
}

impl Sub<f32> for V3 {
    type Output = V3;
    fn sub(self, f: f32) -> V3 {
        V3(self.0 - f,
           self.1 - f,
           self.2 - f)
    }
}

impl Mul for V3 {
    type Output = V3;
    fn mul(self, other: V3) -> V3 {
        V3(self.0 * other.0,
           self.1 * other.1,
           self.2 * other.2)
    }
}

impl Mul<f32> for V3 {
    type Output = V3;
    fn mul(self, f: f32) -> V3 {
        V3(self.0 * f,
           self.1 * f,
           self.2 * f)
    }
}

impl Div for V3 {
    type Output = V3;
    fn div(self, other: V3) -> V3 {
        V3(self.0 / other.0,
           self.1 / other.1,
           self.2 / other.2)
    }
}

impl Div<f32> for V3 {
    type Output = V3;
    fn div(self, f: f32) -> V3 {
        V3(self.0 / f,
           self.1 / f,
           self.2 / f)
    }
}

impl Neg for V3 {
    type Output = V3;
    fn neg(self) -> V3 {
        V3(-self.0,
           -self.1,
           -self.2)
    }
}

impl Default for V3 {
    fn default() -> V3 {
        V3::zero()
    }
}

//
// Ray
//

pub struct Ray {
    pub origin: V3,
    pub direction: V3
}

impl Ray {
    pub fn new(origin: V3, normal: V3) -> Ray {
        Ray { origin, direction: normal }
    }

    pub fn point_at_parameter(&self, t: f32) -> V3 {
        self.origin + (self.direction * t)
    }
}
