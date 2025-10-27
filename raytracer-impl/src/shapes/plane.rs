use crate::types::{ V2, V3, Ray };
use crate::implementation::{ Hitable, HitRecord, AABB, MatId, TexId };

pub fn intersect_plane(ray: Ray, origin: V3, normal: V3) -> Option<f32> {
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
    normal: V3,
    u_basis: V3,
    v_basis: V3,
    radius: Option<f32>,
    mat_id: MatId,
    tex_id: TexId,
}

impl Plane {
    pub fn new(normal: V3, material: MatId, texture: TexId) -> Self {
        let normal = normal.unit();
        // Calculate a basis for UV mapping on the plane.
        // See: https://gamedev.stackexchange.com/a/172357
        // TODO: pick a more meaningful u_basis?
        let u_basis = match V3::cross(normal, V3::POS_X).unit() {
            // e1 and normal are parallel, pick a new random point
            V3::ZERO => V3::cross(normal, V3::POS_Y).unit(),
            otherwise => otherwise
        };
        let v_basis = V3::cross(normal, u_basis).unit();
        Plane {
            normal,
            u_basis,
            v_basis,
            radius: None,
            mat_id: material,
            tex_id: texture,
        }
    }

    #[allow(unused)]
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }
}

// Ref: https://www.scratchapixel.com/lessons/3d-basic-rendering/minimal-ray-tracer-rendering-simple-shapes/ray-plane-and-ray-disk-intersection
impl Hitable for Plane {
    fn hit(&self, ray: Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        let t = intersect_plane(ray, V3::ZERO, self.normal)?;
        if t < t_min || t > t_max {
            return None;
        }
        let p = ray.point_at_parameter(t);
        // If this is a disk plane, ensure the point p falls within the radius
        if let Some(radius) = self.radius {
            if p.length() > radius {
                return None;
            }
        }
        // If this plane is facing away from the ray we want to flip the reported normal
        // so that reflections work in both directions.
        let normal = if V3::dot(ray.direction, self.normal) > 0.0 { -self.normal } else { self.normal };
        // Calculate the uv of this hit, from the origin at 0,0,0
        let uv = {
            let u = V3::dot(self.u_basis, p);
            let v = V3::dot(self.v_basis, p);
            V2(u, v)
        };
        return Some(HitRecord {
            entity_id: None,
            t,
            p,
            normal,
            uv,
            mat_id: self.mat_id,
            tex_id: self.tex_id,
            tex_key: None,
        });
    }

    fn aabb(&self) -> AABB {
        // No bounding box for an infinite plane, unless it's perfectly aligned on two axis?
        AABB::from_min_max(-V3::INFINITY, V3::INFINITY)
    }
}
