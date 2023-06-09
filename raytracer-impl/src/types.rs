#![allow(unused)]

use std::ops::{ Add, Sub, Mul, Div, Neg };
use std::default::{ Default };

//
// Vec3
//

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct V3(pub f32, pub f32, pub f32); // x, y, z

impl V3 {
    pub const POS_X: V3 = V3(1.0, 0.0, 0.0);
    pub const POS_Y: V3 = V3(0.0, 1.0, 0.0);
    pub const POS_Z: V3 = V3(0.0, 0.0, 1.0);
    pub const NEG_X: V3 = V3(-1.0, 0.0, 0.0);
    pub const NEG_Y: V3 = V3(0.0, -1.0, 0.0);
    pub const NEG_Z: V3 = V3(0.0, 0.0, -1.0);
    pub const ZERO: V3  = V3(0.0, 0.0, 0.0);
    pub const ONE: V3   = V3(1.0, 1.0, 1.0);

    pub fn x(&self) -> f32 {
        self.0
    }
    
    pub fn y(&self) -> f32 {
        self.1
    }
    
    pub fn z(&self) -> f32 {
        self.2
    }

    pub fn xyz(&self) -> [f32; 3] {
        [self.0, self.1, self.2]
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
        V3::ZERO
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
    pub fn new(origin: V3, direction: V3) -> Ray {
        Ray { origin, direction }
    }

    pub fn point_at_parameter(&self, t: f32) -> V3 {
        self.origin + (self.direction * t)
    }
}

//
// Helpers
//

impl V3 {
    /// Rotates the vector about (0,0,0) using the unit vector {axis} as an axis
    pub fn rotate_about_axis(&self, axis: V3, theta: f32) -> V3 {
        // See: https://en.wikipedia.org/wiki/Rodrigues%27_rotation_formula
        // If P is a vector in ℝ3 and K is a unit vector describing an axis of rotation
        // about which P rotates by an angle θ according to the right hand rule,
        // the Rodrigues formula for the rotated vector Prot is: 
        //
        //      Prot = P cosθ + (K × P) sinθ + K (K · P) (1 - cosθ)

        let p = self.clone();
        (p * theta.cos()) + (V3::cross(axis, p) * theta.sin()) + (axis * V3::dot(axis, p) * (1.0 - theta.cos()))
    }

    pub fn angle_between(a: V3, b: V3) -> f32 {
        // θ = sin-1 [ |a × b| / (|a| |b|) ]
        (V3::cross(a, b).length() / (a.length() * b.length())).asin()
    }
}

//
// Conversion
//

/// A trait for coercing a concrete type O: T into Arc<dyn T>, or Arc<O> into Arc<dyn T>
pub trait IntoArc<T: ?Sized> {
    fn into_arc(self) -> std::sync::Arc<T>;
}

macro_rules! derive_into_arc {
    ($type:ident) => {
        impl<T: 'static> IntoArc<dyn $type> for T where T: $type {
            fn into_arc(self) -> std::sync::Arc<dyn $type> {
                std::sync::Arc::new(self)
            }
        }
        impl<T: 'static> IntoArc<dyn $type> for std::sync::Arc<T> where T: $type {
            fn into_arc(self) -> std::sync::Arc<dyn $type> {
                self
            }
        }
    };
}

pub(crate) use derive_into_arc;

//
// Vec2
//

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct V2(pub f32, pub f32); // x, y

impl V2 {
    pub const ZERO: V2 = V2(0.0, 0.0);
    pub const ONE: V2  = V2(1.0, 1.0);
    
    pub fn x(&self) -> f32 {
        self.0
    }
    
    pub fn y(&self) -> f32 {
        self.1
    }
}

impl Add for V2 {
    type Output = V2;
    fn add(self, other: V2) -> V2 {
        V2(self.0 + other.0,
           self.1 + other.1)
    }
}

impl Add<f32> for V2 {
    type Output = V2;
    fn add(self, f: f32) -> V2 {
        V2(self.0 + f,
           self.1 + f)
    }
}

impl Sub for V2 {
    type Output = V2;
    fn sub(self, other: V2) -> V2 {
        V2(self.0 - other.0,
           self.1 - other.1)
    }
}

impl Sub<f32> for V2 {
    type Output = V2;
    fn sub(self, f: f32) -> V2 {
        V2(self.0 - f,
           self.1 - f)
    }
}

impl Mul for V2 {
    type Output = V2;
    fn mul(self, other: V2) -> V2 {
        V2(self.0 * other.0,
           self.1 * other.1)
    }
}

impl Mul<f32> for V2 {
    type Output = V2;
    fn mul(self, f: f32) -> V2 {
        V2(self.0 * f,
           self.1 * f)
    }
}

impl Div for V2 {
    type Output = V2;
    fn div(self, other: V2) -> V2 {
        V2(self.0 / other.0,
           self.1 / other.1)
    }
}

impl Div<f32> for V2 {
    type Output = V2;
    fn div(self, f: f32) -> V2 {
        V2(self.0 / f,
           self.1 / f)
    }
}

impl Neg for V2 {
    type Output = V2;
    fn neg(self) -> V2 {
        V2(-self.0,
           -self.1)
    }
}

impl Default for V2 {
    fn default() -> V2 {
        V2::ZERO
    }
}
