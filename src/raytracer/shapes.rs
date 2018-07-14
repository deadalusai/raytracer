
pub use raytracer::types::{ Vec3, Ray };
pub use raytracer::implementation::{ Material, MatRecord, Hitable, HitRecord };

//
// Shapes
//

pub struct Sphere {
    center: Vec3,
    radius: f32,
    material: Box<Material>,
}

impl Sphere {
    pub fn new<M> (center: Vec3, radius: f32, material: M) -> Sphere
        where M: Material + 'static
    {
        Sphere { center: center, radius: radius, material: Box::new(material) }
    }
}

impl Hitable for Sphere {
    fn hit<'a> (&'a self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'a>> {
        let oc = ray.origin - self.center;
        let a = Vec3::dot(ray.direction, ray.direction);
        let b = Vec3::dot(oc, ray.direction);
        let c = Vec3::dot(oc, oc) - self.radius * self.radius;
        let discriminant = b * b - a * c;
        if discriminant > 0.0 {
            let t = (-b - discriminant.sqrt()) / a;
            if t < t_max && t > t_min {
                let point = ray.point_at_parameter(t);
                let normal = (point - self.center) / self.radius;
                return Some(HitRecord { t: t, p: point, normal: normal, material: &*self.material });
            }
            let t = (-b + discriminant.sqrt()) / a;
            if t < t_max && t > t_min {
                let point = ray.point_at_parameter(t);
                let normal = (point - self.center) / self.radius;
                return Some(HitRecord { t: t, p: point, normal: normal, material: &*self.material });
            }
        }
        None
    }
}