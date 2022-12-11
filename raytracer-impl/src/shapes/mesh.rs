use std::sync::Arc;

use crate::obj_format::ObjObject;
use crate::types::{ V3, V2, Ray, IntoArc };
use crate::implementation::{ Material, Hitable, HitRecord, AABB };

// Triangle Mesh BVH

struct MeshTriHit {
    p: V3,
    normal: V3,
    t: f32,
    uv: V2,
}

fn try_hit_triangle(ray: &Ray, tri: &MeshTri) -> Option<MeshTriHit> {
    // compute normal of and area of the triangle
    // NOTE: not normalized! Used for area calculations
    let edge_ab = tri.b - tri.a;
    let edge_ac = tri.c - tri.a;
    let normal = V3::cross(edge_ab, edge_ac);

    // Step 1: find the intersection {P}
    let t = super::plane::intersect_plane(ray, tri.a, normal)?;
    if t < 0.0 {
        // Triangle is behind the ray origin
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

    return Some(MeshTriHit { p, normal: normal.unit(), t, uv })
}

fn tri_aabb(tri: &MeshTri) -> AABB {
    AABB::from_vertices(&[tri.a, tri.b, tri.c])
}

pub struct MeshBvhLeaf(MeshTri);

impl MeshBvhLeaf {
    fn hit_node(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<MeshTriHit> {
        try_hit_triangle(ray, &self.0)
            .filter(|hit| t_min < hit.t && hit.t < t_max)
    }

    fn aabb(&self) -> AABB {
        tri_aabb(&self.0)
    }
}

pub struct MeshBvhBranch {
    aabb: AABB,
    left: Box<MeshBvhNode>,
    right: Box<MeshBvhNode>,
}

impl MeshBvhBranch {
    fn hit_node(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<MeshTriHit> {
        if !self.aabb.hit_aabb(ray, t_min, t_max) {
            return None;
        }
        let left = self.left.hit_node(ray, t_min, t_max);
        let right = self.right.hit_node(ray, t_min, t_max);
        match (left, right) {
            (Some(l), Some(r)) => Some(if l.t < r.t { l } else { r }),
            (Some(l), None)    => Some(l),
            (None,    Some(r)) => Some(r),
            _                  => None,
        }
    }

    fn aabb(&self) -> AABB {
        self.aabb.clone()
    }
}

pub enum MeshBvhNode {
    Leaf(MeshBvhLeaf),
    Branch(MeshBvhBranch)
}

impl MeshBvhNode {
    fn hit_node(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<MeshTriHit> {
        match self {
            MeshBvhNode::Leaf(leaf) => leaf.hit_node(ray, t_min, t_max),
            MeshBvhNode::Branch(branch) => branch.hit_node(ray, t_min, t_max),
        }
    }

    fn aabb(&self) -> AABB {
        match self {
            MeshBvhNode::Leaf(leaf) => leaf.aabb(),
            MeshBvhNode::Branch(branch) => branch.aabb(),
        }
    }
}

pub fn build_triangle_bvh_hierachy(triangles: &[MeshTri]) -> Option<MeshBvhNode> {
    use super::bvh::{ SortAxis, compare_aabb };

    fn inner(triangles: &mut [(AABB, &MeshTri)], axis: SortAxis) -> Option<MeshBvhNode> {

        let node = match triangles {
            [] => return None,
            [a] => MeshBvhNode::Leaf(MeshBvhLeaf(a.1.clone())),
            _ => {
                triangles.sort_by(|l, r| compare_aabb(&l.0, &r.0, axis));
                let mid = triangles.len() / 2;
                let left = inner(&mut triangles[0..mid], axis.next()).unwrap();
                let right = inner(&mut triangles[mid..], axis.next()).unwrap();
                MeshBvhNode::Branch(MeshBvhBranch {
                    aabb: AABB::surrounding(left.aabb(), right.aabb()),
                    left: Box::new(left),
                    right: Box::new(right),
                })
            }
        };

        Some(node)
    }

    // Pre-caculate triangle bounding boxes
    let mut triangles = triangles.iter()
        .map(|tri| (tri_aabb(tri), tri))
        .collect::<Vec<_>>();

    inner(&mut triangles, SortAxis::X)
}

// Mesh

#[derive(Clone, Default)]
pub struct MeshTri {
    a: V3,
    b: V3,
    c: V3,
    a_uv: V2,
    b_uv: V2,
    c_uv: V2,
}

impl MeshTri {
    pub fn from_abc(a: V3, b: V3, c: V3) -> Self {
        Self { a, b, c, ..Default::default() }
    }
}

pub struct Mesh {
    object_id: Option<u32>,
    origin: V3,
    mesh_node: MeshBvhNode,
    material: Arc<dyn Material>,
}

impl Mesh {
    pub fn new(origin: V3, triangles: Vec<MeshTri>, material: impl IntoArc<dyn Material>) -> Self {
        Mesh {
            object_id: None,
            origin,
            mesh_node: build_triangle_bvh_hierachy(&triangles).expect("Expected at least one triangle for mesh"),
            material: material.into_arc(),
        }
    }

    #[allow(unused)]
    pub fn with_id(mut self, id: u32) -> Self {
        self.object_id = Some(id);
        self
    }
}

impl Hitable for Mesh {
    fn hit<'a> (&'a self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'a>> {
        // Shift the ray into mesh space
        let mesh_ray = Ray::new(ray.origin - self.origin, ray.direction);
        let mesh_hit = self.mesh_node.hit_node(&mesh_ray, t_min, t_max)?;
        Some(HitRecord {
            object_id: self.object_id,
            // Shift the hit back into world space
            t: mesh_hit.t,
            p: mesh_hit.p + self.origin,
            normal: mesh_hit.normal,
            uv: mesh_hit.uv,
            material: self.material.as_ref(),
        })
    }

    fn bounding_box(&self) -> Option<AABB> {
        // Shift the mesh bounding box into world space
        let aabb = self.mesh_node.aabb();
        Some(AABB {
            min: self.origin + aabb.min,
            max: self.origin + aabb.max,
        })
    }
}


// Convert OBJ triangles into MeshTri list

pub trait MeshTriConvert {
    fn get_mesh_triangles(&self) -> Vec<MeshTri>;
}

impl MeshTriConvert for ObjObject {
    fn get_mesh_triangles(&self) -> Vec<MeshTri> {
        let get_vertex = |i: usize| self.vertices.get(i - 1).cloned().unwrap();
        let get_uv_vertex = |oi: Option<usize>| oi.and_then(|i| self.uv.get(i - 1).cloned()).unwrap_or_default();
        let mut tris = Vec::new();
        for face in self.faces.iter() {
            tris.push(MeshTri {
                a: get_vertex(face.a.vertex_index),
                b: get_vertex(face.b.vertex_index),
                c: get_vertex(face.c.vertex_index),
                a_uv: get_uv_vertex(face.a.uv_index),
                b_uv: get_uv_vertex(face.b.uv_index),
                c_uv: get_uv_vertex(face.c.uv_index),
            });
        }
        tris
    }
}
