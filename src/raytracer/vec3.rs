

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

    pub fn add (&self, other: &Vec3) -> Vec3 {
        Vec3 {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z
        }
    }

    pub fn add_mut (&mut self, other: &Vec3) {
        *self = self.add(other);
    }

    pub fn sub (&self, other: &Vec3) -> Vec3 {
        Vec3 {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z
        }
    }

    pub fn sub_mut (&mut self, other: &Vec3) {
        *self = self.sub(other);
    }

    pub fn mul (&self, other: &Vec3) -> Vec3 {
        Vec3 {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z
        }
    }

    pub fn mul_mut (&mut self, other: &Vec3) {
        *self = self.mul(other);
    }

    pub fn div (&self, other: &Vec3) -> Vec3 {
        Vec3 {
            x: self.x / other.x,
            y: self.y / other.y,
            z: self.z / other.z
        }
    }

    pub fn div_mut (&mut self, other: &Vec3) {
        *self = self.div(other);
    }

    pub fn mul_f (&self, f: f32) -> Vec3 {
        Vec3 {
            x: self.x * f,
            y: self.y * f,
            z: self.z * f
        }
    }

    pub fn mul_f_mut (&mut self, f: f32) {
        *self = self.mul_f(f);
    }

    pub fn div_f (&self, f: f32) -> Vec3 {
        Vec3 {
            x: self.x / f,
            y: self.y / f,
            z: self.z / f
        }
    }

    pub fn div_f_mut (&mut self, f: f32) {
        *self = self.div_f(f);
    }

    pub fn negate (&self) -> Vec3 {
        Vec3 {
            x: -self.x,
            y: -self.y,
            z: -self.z
        }
    }

    pub fn unit_vector (&self) -> Vec3 {
        self.div_f(self.length())
    }

    pub fn length (&self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn length_squared (&self) -> f32 {
        (self.x * self.x) + (self.y * self.y) + (self.z * self.z)
    }
}

pub fn vec3_m (x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

pub fn vec3_dot (a: &Vec3, b: &Vec3) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

pub fn vec3_cross (a: &Vec3, b: &Vec3) -> Vec3 {
    Vec3 {
        x: a.y * b.z - a.z * b.y,
        y: a.x * b.z - a.z * b.x,
        z: a.y * b.x - a.x * b.y
    }
}