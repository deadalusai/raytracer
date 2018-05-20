

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

    pub fn div_f (&self, f: f32) -> Vec3 {
        Vec3 {
            x: self.x / f,
            y: self.y / f,
            z: self.z / f
        }
    }

    pub fn dot (&self, other: &Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross (&self, other: &Vec3) -> Vec3 {
        Vec3 {
            x: self.y * other.z - self.z * other.y,
            y: self.x * other.z - self.z * other.x,
            z: self.y * other.x - self.x * other.y
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
        self.div_f(self.length())
    }

    pub fn length (&self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn length_squared (&self) -> f32 {
        ((self.x * self.x) + (self.y * self.y) + (self.z * self.z)).sqrt()
    }
}