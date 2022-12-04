use crate::types::{ V3, Ray };
use crate::implementation::{ Material, Hitable, HitRecord, AABB };

fn intersect_sphere(ray: &Ray, origin: V3, radius: f32) -> Option<[f32; 2]> {
    let oc = ray.origin - origin;
    let a = V3::dot(ray.direction, ray.direction);
    let b = V3::dot(oc, ray.direction);
    let c = V3::dot(oc, oc) - radius * radius;
    let discriminant = b * b - a * c;
    if discriminant > 0.0 {
        // Every ray must necessarily intersect with the sphere twice
        let t0 = (-b - discriminant.sqrt()) / a;
        let t1 = (-b + discriminant.sqrt()) / a;
        return Some([t0, t1]);
    }
    None
}

pub struct Sphere {
    object_id: Option<u32>,
    origin: V3,
    radius: f32,
    material: Box<dyn Material>,
}

impl Sphere {
    pub fn new<M>(origin: V3, radius: f32, material: M) -> Self
        where M: Material + 'static
    {
        Sphere { object_id: None, origin, radius: radius, material: Box::new(material) }
    }

    #[allow(unused)]
    pub fn with_id(mut self, id: u32) -> Self {
        self.object_id = Some(id);
        self
    }
}

impl Hitable for Sphere {
    fn hit<'a>(&'a self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'a>> {
        let object_id = self.object_id;
        let material = self.material.as_ref();

        if let Some(ts) = intersect_sphere(ray, self.origin, self.radius) {
            // Identify the best candidate intersection point
            let t = ts.iter().cloned().filter(|&t| t_min < t && t < t_max).reduce(f32::min);
            if let Some(t) = t {
                let p = ray.point_at_parameter(t);
                let normal = ((p - self.origin) / self.radius).unit();
                return Some(HitRecord { object_id, t, p, normal, material });
            }
        }

        None
    }

    fn bounding_box(&self) -> Option<AABB> {
        // Find the bounding box for a sphere
        Some(AABB::from_min_max(self.origin - self.radius, self.origin + self.radius))
    }
}