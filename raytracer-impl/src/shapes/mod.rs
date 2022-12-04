pub mod mesh;
pub mod plane;
pub mod sphere;
pub mod bvh;

use crate::types::{ V3, Ray };
use crate::implementation::{ Material, Hitable, HitRecord, AABB };

pub use mesh::{ Mesh };
pub use plane::{ Plane };
pub use sphere::{ Sphere };
pub use bvh::{ BvhNode };

//
// Shapes
//

struct TriIntersect {
    p: V3,
    normal: V3,
    t: f32,
}

// Ref: https://www.scratchapixel.com/lessons/3d-basic-rendering/ray-tracing-rendering-a-triangle/ray-triangle-intersection-geometric-solution
fn intersect_tri(ray: &Ray, a: V3, b: V3, c: V3) -> Option<TriIntersect> {
    // Find the normal of the triangle, using v0 as the origin
    let normal = V3::cross(b - a, c - a).unit();
    // Find the intesection `p` with the tiangle plane
    let t = plane::intersect_plane(ray, a, normal)?;
    // `p` is a point on the same plane as all three vertices of the triangle
    let p = ray.point_at_parameter(t);
    // Test if `p` is a point inside the triangle by determining if it is "left" of each edge.
    // (The cross product of the angle of `p` with each point should align with the normal)
    if V3::dot(normal, V3::cross(b - a, p - a)) < 0.0 ||
        V3::dot(normal, V3::cross(c - b, p - b)) < 0.0 ||
        V3::dot(normal, V3::cross(a - c, p - c)) < 0.0 {
        return None;
    }
    Some(TriIntersect { p, normal, t })
}

pub struct Triangle {
    object_id: Option<u32>,
    origin: V3,
    vertices: (V3, V3, V3),
    material: Box<dyn Material>,
}

impl Triangle {
    pub fn new<M>(origin: V3, vertices: (V3, V3, V3), material: M) -> Self
        where M: Material + 'static
    {
        Triangle { object_id: None, origin, vertices, material: Box::new(material) }
    }

    #[allow(unused)]
    pub fn with_id(mut self, id: u32) -> Self {
        self.object_id = Some(id);
        self
    }
}

impl Hitable for Triangle {
    fn hit<'a>(&'a self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'a>> {
        let ti = intersect_tri(
            ray,
            self.origin + self.vertices.0,
            self.origin + self.vertices.1,
            self.origin + self.vertices.2,
        )?;
        if ti.t < t_min || ti.t > t_max {
            return None;
        }
        let object_id = self.object_id;
        let material = self.material.as_ref();
        // If this plane is facing away from the ray we want to flip the reported normal
        // so that reflections work in both directions.
        let normal = if V3::dot(ray.direction, ti.normal) > 0.0 { -ti.normal } else { ti.normal };
        Some(HitRecord { object_id, p: ti.p, t: ti.t, normal, material })
    }

    fn bounding_box(&self) -> Option<AABB> {
        Some(AABB::from_vertices(&[
            self.origin + self.vertices.0,
            self.origin + self.vertices.1,
            self.origin + self.vertices.2,
        ]))
    }
}
