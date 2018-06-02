use raytracer::vec3::Vec3;

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

pub fn ray_m (origin: Vec3, direction: Vec3) -> Ray {
    Ray::new(origin, direction)
}