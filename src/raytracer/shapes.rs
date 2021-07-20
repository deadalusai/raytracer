use std::f32::consts::{ FRAC_PI_2 };

pub use raytracer::types::{ V3, Ray };
pub use raytracer::implementation::{ Material, MatRecord, Hitable, HitRecord };

//
// Shapes
//

pub struct Sphere {
    object_id: Option<u32>,
    origin: V3,
    radius: f32,
    material: Box<dyn Material>,
}

impl Sphere {
    pub fn new<M> (origin: V3, radius: f32, material: M) -> Self
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
    fn hit<'a> (&'a self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'a>> {
        let object_id = self.object_id;
        let material = self.material.as_ref();

        let oc = ray.origin - self.origin;
        let a = V3::dot(ray.normal, ray.normal);
        let b = V3::dot(oc, ray.normal);
        let c = V3::dot(oc, oc) - self.radius * self.radius;
        let discriminant = b * b - a * c;
        if discriminant > 0.0 {
            let t = (-b - discriminant.sqrt()) / a;
            if t < t_max && t > t_min {
                let p = ray.point_at_parameter(t);
                let normal = ((p - self.origin) / self.radius).unit();
                return Some(HitRecord { object_id, t, p, normal, material });
            }
            let t = (-b + discriminant.sqrt()) / a;
            if t < t_max && t > t_min {
                let p = ray.point_at_parameter(t);
                let normal = ((p - self.origin) / self.radius).unit();
                return Some(HitRecord { object_id, t, p, normal, material });
            }
        }
        None
    }
}

pub struct Plane {
    object_id: Option<u32>,
    origin: V3,
    normal: V3,
    material: Box<dyn Material>,
    radius: Option<f32>,
}

impl Plane {
    pub fn new<M> (origin: V3, normal: V3, material: M) -> Self
        where M: Material + 'static
    {
        Plane { object_id: None, origin, normal: normal.unit(), material: Box::new(material), radius: None }
    }

    pub fn with_radius(mut self, radius: f32) -> Plane {
        self.radius = Some(radius);
        self
    }

    #[allow(unused)]
    pub fn with_id(mut self, id: u32) -> Self {
        self.object_id = Some(id);
        self
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
        let numerator = -V3::dot(ray.origin - self.origin, self.normal);
        let t = numerator / denominator;
        // NOTE: A negative value indicates the plane is behind the ray origin.
        // Filter for intersections inside the range we're testing for
        if t < t_min || t > t_max {
            return None;
        }
        let p = ray.point_at_parameter(t);
        // If this is a disk plane, ensure the point p falls within the radius
        if let Some(radius) = self.radius {
            if (self.origin - p).length() > radius {
                return None;
            }
        }
        let object_id = self.object_id;
        let material = self.material.as_ref();
        // If this plane is facing towards the ray we expect an angle between them approaching 180 degrees (PI).
        // If the the angle passes perpendicular (90 degrees or PI/2) then we flip the plane normal     
        let theta = V3::theta(ray.normal, self.normal);
        let normal = if theta < FRAC_PI_2 { -self.normal } else { self.normal };
        return Some(HitRecord { object_id, t, p, normal, material });
    }
}