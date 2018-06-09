
pub use raytracer::types::{ Vec3, vec3_dot, Ray };
pub use raytracer::implementation::{ Material, MatRecord, Hitable, HitRecord };

//
// Shapes
//

pub struct Sphere {
    center: Vec3,
    radius: f32,
    material: Box<Material + Send + Sync>,
}

impl Sphere {
    pub fn new (center: Vec3, radius: f32, material: Box<Material + Send + Sync>) -> Sphere {
        Sphere { center: center, radius: radius, material: material }
    }
}

impl Hitable for Sphere {
    fn hit<'a> (&'a self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'a>> {
        let oc = ray.origin.sub(&self.center);
        let a = vec3_dot(&ray.direction, &ray.direction);
        let b = vec3_dot(&oc, &ray.direction);
        let c = vec3_dot(&oc, &oc) - self.radius * self.radius;
        let discriminant = b * b - a * c;
        if discriminant > 0.0 {
            let t = (-b - discriminant.sqrt()) / a;
            if t < t_max && t > t_min {
                let point = ray.point_at_parameter(t);
                let normal = point.sub(&self.center).div_f(self.radius);
                return Some(HitRecord { t: t, p: point, normal: normal, material: &*self.material });
            }
            let t = (-b + discriminant.sqrt()) / a;
            if t < t_max && t > t_min {
                let point = ray.point_at_parameter(t);
                let normal = point.sub(&self.center).div_f(self.radius);
                return Some(HitRecord { t: t, p: point, normal: normal, material: &*self.material });
            }
        }
        None
    }
}