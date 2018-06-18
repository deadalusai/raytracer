#![allow(unused)]

//
// Vec3
//

#[derive(Debug, Clone, PartialEq)]
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

    pub fn add (&self, other: &Vec3) -> Vec3 {
        Vec3 {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z
        }
    }

    pub fn sub (&self, other: &Vec3) -> Vec3 {
        Vec3 {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z
        }
    }

    pub fn mul (&self, other: &Vec3) -> Vec3 {
        Vec3 {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z
        }
    }

    pub fn div (&self, other: &Vec3) -> Vec3 {
        Vec3 {
            x: self.x / other.x,
            y: self.y / other.y,
            z: self.z / other.z
        }
    }

    pub fn add_f (&self, f: f32) -> Vec3 {
        Vec3 {
            x: self.x + f,
            y: self.y + f,
            z: self.z + f
        }
    }

    pub fn sub_f (&self, f: f32) -> Vec3 {
        Vec3 {
            x: self.x - f,
            y: self.y - f,
            z: self.z - f
        }
    }

    pub fn mul_f (&self, f: f32) -> Vec3 {
        Vec3 {
            x: self.x * f,
            y: self.y * f,
            z: self.z * f
        }
    }

    pub fn div_f (&self, f: f32) -> Vec3 {
        Vec3 {
            x: self.x / f,
            y: self.y / f,
            z: self.z / f
        }
    }

    pub fn negate (&self) -> Vec3 {
        Vec3 {
            x: -self.x,
            y: -self.y,
            z: -self.z
        }
    }

    pub fn unit_vector (&self) -> Vec3 {
        let len = self.length();
        if len == 0.0 {
            self.clone()
        } else {
            self.div_f(len)
        }
    }

    pub fn clamp (&self) -> Vec3 {  
        Vec3 {
            x: self.x.min(1.0).max(-1.0),
            y: self.y.min(1.0).max(-1.0),
            z: self.z.min(1.0).max(-1.0),
        }
    }

    pub fn length (&self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn length_squared (&self) -> f32 {
        (self.x * self.x) + (self.y * self.y) + (self.z * self.z)
    }
}

pub fn vec3 (x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

pub fn vec3_dot (a: &Vec3, b: &Vec3) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

pub fn vec3_cross (a: &Vec3, b: &Vec3) -> Vec3 {
    Vec3 {
        x:  (a.y * b.z - a.z * b.y),
        y: -(a.x * b.z - a.z * b.x),
        z:  (a.x * b.y - a.y * b.x)
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

    pub fn new (origin: Vec3, direction: Vec3) -> Ray {
        Ray {
            origin: origin,
            direction: direction
        }
    }

    pub fn point_at_parameter (&self, t: f32) -> Vec3 {
        self.origin.add(&self.direction.mul_f(t))
    }
}

pub fn ray (origin: Vec3, direction: Vec3) -> Ray {
    Ray::new(origin, direction)
}

//
// Rgb
//

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgb { 
    pub r: u8,
    pub g: u8,
    pub b: u8
}

impl Rgb {
    pub fn new (r: u8, g: u8, b: u8) -> Rgb {
        Rgb { r: r, g: g, b: b }
    }

    pub fn from_vec3 (v: &Vec3) -> Rgb {
        Rgb::new(
            (255.0 * v.x.sqrt()) as u8,
            (255.0 * v.y.sqrt()) as u8,
            (255.0 * v.z.sqrt()) as u8
        )
    }
}