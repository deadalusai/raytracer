use std::sync::Arc;

use crate::types::{ V3, V2, Ray };
use crate::implementation::{ Hitable, HitRecord, AABB, MatId, TexId };

// Triangle Mesh BVH

struct MeshFaceHit {
    p: V3,
    normal: V3,
    t: f32,
    uv: V2,
    tex_key: Option<usize>,
}

fn try_hit_face(ray: &Ray, face: &MeshFace) -> Option<MeshFaceHit> {
    // compute normal of and area of the triangle
    // NOTE: not normalized! Used for area calculations
    let edge_ab = face.b - face.a;
    let edge_ac = face.c - face.a;
    let normal = V3::cross(edge_ab, edge_ac);

    // Step 1: find the intersection {P}
    let t = super::plane::intersect_plane(ray, face.a, normal)?;
    if t < 0.0 {
        // Triangle is behind the ray origin
        return None;
    }
    
    let p = ray.point_at_parameter(t);

    // Step 2: determine if p is inside the triangle
    // by checking to see if it is to the left of each edge

    // edge ab
    let normal_abp = V3::cross(edge_ab, p - face.a);
    if V3::dot(normal, normal_abp) < 0.0 {
        // P is to the right of edge ab
        return None;
    }

    // edge bc
    let normal_bcp = V3::cross(face.c - face.b, p - face.b);
    if V3::dot(normal, normal_bcp) < 0.0 {
        // P is to the right of edge bc
        return None;
    }

    // edge ca
    let normal_cap = V3::cross(face.a - face.c, p - face.c);
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
    let uv = (face.a_uv * w) + (face.b_uv * u) + (face.c_uv * v);
    let tex_key = face.tex_key.clone();

    return Some(MeshFaceHit { p, normal: normal.unit(), t, uv, tex_key })
}

fn face_aabb(tri: &MeshFace) -> AABB {
    AABB::from_vertices(&[tri.a, tri.b, tri.c])
}

#[derive(Clone)]
pub struct MeshBvhLeaf(MeshFace);

impl MeshBvhLeaf {
    fn hit_node(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<MeshFaceHit> {
        try_hit_face(ray, &self.0)
            .filter(|hit| t_min < hit.t && hit.t < t_max)
    }

    fn aabb(&self) -> AABB {
        face_aabb(&self.0)
    }
}

#[derive(Clone)]
pub struct MeshBvhBranch {
    aabb: AABB,
    left: Box<MeshBvhNode>,
    right: Box<MeshBvhNode>,
}

impl MeshBvhBranch {
    fn hit_node(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<MeshFaceHit> {
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

#[derive(Clone)]
pub enum MeshBvhNode {
    Leaf(MeshBvhLeaf),
    Branch(MeshBvhBranch)
}

impl MeshBvhNode {
    fn hit_node(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<MeshFaceHit> {
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

pub fn build_face_bounding_volume_hierachy(faces: &[MeshFace]) -> MeshBvhNode {
    use super::bvh::{ SortAxis, compare_aabb };

    fn inner(faces: &mut [(AABB, &MeshFace)], axis: SortAxis) -> MeshBvhNode {
        match faces {
            [] => panic!("Expected at least one face for mesh"),
            [a] => MeshBvhNode::Leaf(MeshBvhLeaf(a.1.clone())),
            _ => {
                faces.sort_by(|l, r| compare_aabb(&l.0, &r.0, axis));
                let mid = faces.len() / 2;
                let left = inner(&mut faces[0..mid], axis.next());
                let right = inner(&mut faces[mid..], axis.next());
                MeshBvhNode::Branch(MeshBvhBranch {
                    aabb: AABB::surrounding(left.aabb(), right.aabb()),
                    left: Box::new(left),
                    right: Box::new(right),
                })
            }
        }
    }

    // Pre-caculate triangle bounding boxes
    let mut faces = faces.iter()
        .map(|f| (face_aabb(f), f))
        .collect::<Vec<_>>();

    inner(&mut faces, SortAxis::X)
}

// Mesh

pub struct Mesh {
    pub faces: Vec<MeshFace>,
}

#[derive(Clone, Default)]
pub struct MeshFace {
    pub a: V3,
    pub b: V3,
    pub c: V3,
    pub a_uv: V2,
    pub b_uv: V2,
    pub c_uv: V2,
    pub tex_key: Option<usize>,
}

impl MeshFace {
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
    root_node: Arc<MeshBvhNode>,
    mat_id: MatId,
    tex_id: TexId,
}

impl MeshObject {
    pub fn new(mesh: &Mesh, mat_id: MatId, tex_id: TexId) -> Self {
        MeshObject {
            object_id: None,
            origin: V3::ZERO,
            root_node: Arc::new(build_face_bounding_volume_hierachy(&mesh.faces)),
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
        let mesh_hit = self.root_node.hit_node(&mesh_ray, t_min, t_max)?;
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
        let aabb = self.root_node.aabb();
        Some(AABB {
            min: self.origin + aabb.min,
            max: self.origin + aabb.max,
        })
    }
}
