use std::sync::Arc;

use crate::bvh::{Bvh, BvhObject};
use crate::types::{ V3, V2, Ray };
use crate::implementation::{ Hitable, HitRecord, AABB, MatId, TexId };

// Triangle Mesh BVH

#[derive(Debug)]
struct MeshTriHit {
    p: V3,
    normal: V3,
    t: f32,
    uv: V2,
    tex_key: Option<usize>,
}

fn try_hit_tri(ray: &Ray, t_min: f32, t_max: f32, tri: &MeshTri) -> Option<MeshTriHit> {

    // compute normal of and area of the triangle
    // NOTE: not normalized! Used for area calculations
    let edge_ab = tri.b - tri.a;
    let edge_ac = tri.c - tri.a;
    let normal = V3::cross(edge_ab, edge_ac);

    // Step 1: find the intersection {P}
    let t = super::plane::intersect_plane(ray, tri.a, normal)?;
    if t < t_min || t > t_max {
        // Triangle is outside the search range
        return None;
    }

    let p = ray.point_at_parameter(t);

    // Step 2: determine if p is inside the triangle
    // by checking to see if it is to the left of each edge

    // edge ab
    let normal_abp = V3::cross(edge_ab, p - tri.a);
    if V3::dot(normal, normal_abp) < 0.0 {
        // P is to the right of edge ab
        return None;
    }

    // edge bc
    let normal_bcp = V3::cross(tri.c - tri.b, p - tri.b);
    if V3::dot(normal, normal_bcp) < 0.0 {
        // P is to the right of edge bc
        return None;
    }

    // edge ca
    let normal_cap = V3::cross(tri.a - tri.c, p - tri.c);
    if V3::dot(normal, normal_cap) < 0.0 {
        // P is to the right of edge ca
        return None;
    }
    
    // TODO(benf): Only need to calculate UV if there is a {mtl_index} set?
    // Could refactor this to skip the uv calculations if we don't need to do them.

    // Calculate uv/barycentric coordinates.
    // Given a triangle ABC and point P:
    //                  C  
    //                u P w
    //               A  v  B
    // u = {area of CAP} / {area of ABC}
    // v = {area of ABP} / {area of ABC}
    // w = {area of BCP} / {area of ABC}
    // P = wA + uB + vC
    let area2 = normal.length();
    let u = normal_cap.length() / area2;
    let v = normal_abp.length() / area2;
    let w = 1.0 - u - v;
    let uv = (tri.a_uv * w) + (tri.b_uv * u) + (tri.c_uv * v);
    let tex_key = tri.tex_key.clone();

    return Some(MeshTriHit { p, normal: normal.unit(), t, uv, tex_key })
}

struct MeshBvhRoot {
    bvh: Bvh,
    tris: Vec<MeshTri>,
}

impl MeshBvhRoot {
    fn new(tris: Vec<MeshTri>) -> MeshBvhRoot {
        MeshBvhRoot {
            bvh: Bvh::from(&tris),
            tris,
        }
    }

    fn try_hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<MeshTriHit> {
        self.bvh.hit_candidates(ray, t_min, t_max)
            .filter_map(|candidate| try_hit_tri(ray, t_min, t_max, &self.tris[candidate.object_index]))
            .reduce(|closest, next| {
                if next.t < closest.t { next } else { closest }
            })
    }
}

// Mesh

pub struct Mesh {
    pub tris: Vec<MeshTri>,
}

#[derive(Clone, Default)]
pub struct MeshTri {
    pub a: V3,
    pub b: V3,
    pub c: V3,
    pub a_uv: V2,
    pub b_uv: V2,
    pub c_uv: V2,
    pub tex_key: Option<usize>,
}

// Allow MeshTri to be used with the Bvh algorithm

impl BvhObject for MeshTri {
    fn aabb(&self) -> AABB {
        AABB::from_vertices(&[self.a, self.b, self.c])
    }

    fn centroid(&self) -> V3 {
        (self.a + self.b + self.c) * 0.33333
    }
}

impl MeshTri {
    pub fn from_abc(a: V3, b: V3, c: V3) -> Self {
        Self { a, b, c, ..Default::default() }
    }
}

#[derive(Clone)]
pub struct MeshObject {
    object_id: Option<u32>,
    origin: V3,
    // NOTE: Store the root node in an Arc so that all
    // clones of this MeshObject will share their internal mesh representation.
    root: Arc<MeshBvhRoot>,
    mat_id: MatId,
    tex_id: TexId,
}

impl MeshObject {
    pub fn new(mesh: &Mesh, mat_id: MatId, tex_id: TexId) -> Self {
        MeshObject {
            object_id: None,
            origin: V3::ZERO,
            root: Arc::new(MeshBvhRoot::new(mesh.tris.clone())),
            mat_id,
            tex_id,
        }
    }

    #[allow(unused)]
    pub fn with_origin(mut self, origin: V3) -> Self {
        self.origin = origin;
        self
    }

    #[allow(unused)]
    pub fn with_id(mut self, id: u32) -> Self {
        self.object_id = Some(id);
        self
    }
}

impl Hitable for MeshObject {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        // Shift the ray into mesh space
        let mesh_ray = Ray::new(ray.origin - self.origin, ray.direction);
        let mesh_hit = self.root.try_hit(&mesh_ray, t_min, t_max)?;
        Some(HitRecord {
            object_id: self.object_id,
            // Shift the hit back into world space
            t: mesh_hit.t,
            p: mesh_hit.p + self.origin,
            normal: mesh_hit.normal,
            uv: mesh_hit.uv,
            mat_id: self.mat_id,
            tex_id: self.tex_id,
            tex_key: mesh_hit.tex_key,
        })
    }

    fn origin(&self) -> V3 {
        self.origin.clone()
    }

    fn aabb(&self) -> Option<AABB> {
        // Shift the mesh bounding box into world space
        let aabb = self.root.bvh.aabb();
        Some(AABB {
            min: self.origin + aabb.min,
            max: self.origin + aabb.max,
        })
    }
}
