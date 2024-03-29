use crate::types::{ V2, V3, Ray };
use crate::implementation::{ Hitable, HitRecord, AABB, MatId, TexId };

pub fn intersect_plane(ray: &Ray, origin: V3, normal: V3) -> Option<f32> {
    // intersection of ray with a plane at point `t`
    // t = ((plane_origin - ray_origin) . plane_normal) / (ray_direction . plane_normal)
    let denominator = V3::dot(ray.direction, normal);
    // When the plane and ray are nearing parallel the denominator approaches zero.
    if denominator.abs() < 1.0e-6 {
        return None;
    }
    let numerator = V3::dot(origin - ray.origin, normal);
    let t = numerator / denominator;
    // NOTE: A negative `t` value indicates the plane is behind the ray origin.
    // Filter for intersections inside the range we're testing for
    Some(t)
}

pub struct Plane {
    object_id: Option<u32>,
    origin: V3,
    normal: V3,
    radius: Option<f32>,
    mat_id: MatId,
    tex_id: TexId,
}

impl Plane {
    pub fn new(normal: V3, material: MatId, texture: TexId) -> Self {
        Plane {
            object_id: None,
            origin: V3::ZERO,
            normal: normal.unit(),
            radius: None,
            mat_id: material, 
            tex_id: texture, 
        }
    }

    #[allow(unused)]
    pub fn with_origin(mut self, origin: V3) -> Self {
        self.origin = origin;
        self
    }

    #[allow(unused)]
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }

    #[allow(unused)]
    pub fn with_id(mut self, id: u32) -> Self {
        self.object_id = Some(id);
        self
    }
}

// Ref: https://www.scratchapixel.com/lessons/3d-basic-rendering/minimal-ray-tracer-rendering-simple-shapes/ray-plane-and-ray-disk-intersection
impl Hitable for Plane {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        let t = intersect_plane(ray, self.origin, self.normal)?;
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
        // If this plane is facing away from the ray we want to flip the reported normal
        // so that reflections work in both directions.
        let normal = if V3::dot(ray.direction, self.normal) > 0.0 { -self.normal } else { self.normal };
        return Some(HitRecord {
            object_id,
            t,
            p,
            normal,
            // TODO(benf): UV mapping for plane
            uv: V2::ZERO,
            mat_id: self.mat_id,
            tex_id: self.tex_id,
            tex_key: None,
        });
    }

    fn origin(&self) -> V3 {
        self.origin.clone()
    }

    fn aabb(&self) -> Option<AABB> {
        // No bounding box for a plane
        None
    }
}
