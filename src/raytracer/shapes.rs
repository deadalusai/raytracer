use std::f32::consts::{ FRAC_PI_2 };

pub use raytracer::types::{ V3, Ray };
pub use raytracer::implementation::{ Material, MatRecord, Hitable, HitRecord };

//
// Shapes
//

pub struct Sphere {
    origin: V3,
    radius: f32,
    material: Box<dyn Material>,
}

impl Sphere {
    pub fn new<M> (origin: V3, radius: f32, material: M) -> Self
        where M: Material + 'static
    {
        Sphere { origin, radius: radius, material: Box::new(material) }
    }
}

impl Hitable for Sphere {
    fn hit<'a> (&'a self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'a>> {
        let oc = ray.origin - self.origin;
        let a = V3::dot(ray.normal, ray.normal);
        let b = V3::dot(oc, ray.normal);
        let c = V3::dot(oc, oc) - self.radius * self.radius;
        let discriminant = b * b - a * c;
        if discriminant > 0.0 {
            let t = (-b - discriminant.sqrt()) / a;
            if t < t_max && t > t_min {
                let point = ray.point_at_parameter(t);
                let normal = ((point - self.origin) / self.radius).unit();
                return Some(HitRecord { t: t, p: point, normal: normal, material: self.material.as_ref() });
            }
            let t = (-b + discriminant.sqrt()) / a;
            if t < t_max && t > t_min {
                let point = ray.point_at_parameter(t);
                let normal = ((point - self.origin) / self.radius).unit();
                return Some(HitRecord { t: t, p: point, normal: normal, material: self.material.as_ref() });
            }
        }
        None
    }
}

pub struct Plane {
    origin: V3,
    normal: V3,
    material: Box<dyn Material>,
}

impl Plane {
    pub fn new<M> (origin: V3, normal: V3, material: M) -> Self
        where M: Material + 'static
    {
        Plane { origin, normal: normal.unit(), material: Box::new(material) }
    }
}

// https://www.scratchapixel.com/lessons/3d-basic-rendering/minimal-ray-tracer-rendering-simple-shapes/ray-plane-and-ray-disk-intersection

impl Hitable for Plane {
    fn hit<'a> (&'a self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'a>> {
        
        // intersection of ray with a plane at point `t`
        // t = ((plane_origin - ray_origin) . plane_normal) / (ray_direction . plane_normal)

        let denominator = V3::dot(ray.normal, self.normal);
        // When the plane and ray are nearing parallel the denominator approaches zero.
        if denominator.abs() <= 1.0e-6 {
            return None;
        }

        let numerator = V3::dot(self.origin - ray.origin, self.normal);
        let t = numerator / denominator;

        // A negative value indicates the plane is behind the ray origin.
        // Filter for intersections inside the range we're testing for
        if t < t_min || t > t_max {
            return None;
        }

        let p = ray.point_at_parameter(t);
        // If this plane is facing towards the ray we expect an angle between them approaching 180 degrees (PI).
        // If the the angle passes perpendicular (90 degrees or PI/2) then we flip the plane normal     
        let theta = V3::theta(ray.normal, self.normal);
        let normal = if theta < FRAC_PI_2 { self.normal * -1.0 } else { self.normal };
        return Some(HitRecord { t, p, normal, material: self.material.as_ref() });
    }
}