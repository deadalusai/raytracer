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

// Ref: https://www.scratchapixel.com/lessons/3d-basic-rendering/ray-tracing-rendering-a-triangle/ray-triangle-intersection-geometric-solution
fn try_hit_triangle(ray: &Ray, tri: &MeshTri) -> Option<MeshTriHit> {
    // Find the normal of the triangle, using v0 as the origin
    let normal = V3::cross(tri.b - tri.a, tri.c - tri.a).unit();
    // Find the intesection `p` with the tiangle plane
    let t = super::plane::intersect_plane(ray, tri.a, normal)?;
    // `p` is a point on the same plane as all three vertices of the triangle
    let p = ray.point_at_parameter(t);
    // Test if `p` is a point inside the triangle by determining if it is "left" of each edge.
    // (The cross product of the angle of `p` with each point should align with the normal)
    if V3::dot(normal, V3::cross(tri.b - tri.a, p - tri.a)) < 0.0 ||
        V3::dot(normal, V3::cross(tri.c - tri.b, p - tri.b)) < 0.0 ||
        V3::dot(normal, V3::cross(tri.a - tri.c, p - tri.c)) < 0.0 {
        return None;
    }
    let uv = map_p_to_uv(p, tri);
    Some(MeshTriHit { p, normal, t, uv })
}

/// Compute UV co-ordinates {u, v} for point {p} with respect to triangle {tri}.
/// See: https://computergraphics.stackexchange.com/a/1867,
/// See: https://gamedev.stackexchange.com/a/23745
fn map_p_to_uv(p: V3, tri: &MeshTri) -> V2 {
    let v0 = tri.b - tri.a;
    let v1 = tri.c - tri.a;
    let v2 = p - tri.a;

    let d00 = V3::dot(v0, v0);
    let d01 = V3::dot(v0, v1);
    let d11 = V3::dot(v1, v1);
    let d20 = V3::dot(v2, v0);
    let d21 = V3::dot(v2, v1);

    let denominator = d00 * d11 - d01 * d01;
    let bary_a = (d11 * d20 - d01 * d21) / denominator;
    let bary_b = (d00 * d21 - d01 * d20) / denominator;
    let bary_c = 1.0 - bary_a - bary_b;

    (tri.a_uv * bary_a) + (tri.b_uv * bary_b) + (tri.c_uv * bary_c)
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

pub enum MeshReflectionMode {
    MonoDirectional,
    BiDirectional,
}

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
    reflection_mode: MeshReflectionMode,
}

impl Mesh {
    pub fn new(origin: V3, triangles: Vec<MeshTri>, material: impl IntoArc<dyn Material>) -> Self {
        Mesh {
            object_id: None,
            origin,
            mesh_node: build_triangle_bvh_hierachy(&triangles).expect("Expected at least one triangle for mesh"),
            material: material.into_arc(),
            reflection_mode: MeshReflectionMode::MonoDirectional,
        }
    }

    #[allow(unused)]
    pub fn with_id(mut self, id: u32) -> Self {
        self.object_id = Some(id);
        self
    }

    #[allow(unused)]
    pub fn with_reflection_mode(mut self, mode: MeshReflectionMode) -> Self {
        self.reflection_mode = mode;
        self
    }
}

impl Hitable for Mesh {
    fn hit<'a> (&'a self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'a>> {
        // Shift the ray into mesh space
        let mesh_ray = Ray::new(ray.origin - self.origin, ray.direction);
        let mesh_hit = self.mesh_node.hit_node(&mesh_ray, t_min, t_max)?;
        let is_plane_facing_away = V3::dot(ray.direction, mesh_hit.normal) > 0.0;
        let normal = match self.reflection_mode {
            MeshReflectionMode::MonoDirectional => {
                // If the plane is facing away from the ray then consider this a miss
                if is_plane_facing_away { None } else { Some(mesh_hit.normal) }
            },
            MeshReflectionMode::BiDirectional => {
                // If this plane is facing away from the ray we want to flip the reported normal
                // so that reflections work in both directions.
                if is_plane_facing_away { Some(-mesh_hit.normal) } else { Some(mesh_hit.normal) }
            },
        }?;
        Some(HitRecord {
            object_id: self.object_id,
            // Shift the hit back into world space
            t: mesh_hit.t,
            p: mesh_hit.p + self.origin,
            normal,
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
        let mut tris = Vec::new();
        for face in self.faces.iter() {
            tris.push(MeshTri {
                a: self.vertices.get(face.a.vertex_index - 1).cloned().unwrap(),
                b: self.vertices.get(face.b.vertex_index - 1).cloned().unwrap(),
                c: self.vertices.get(face.c.vertex_index - 1).cloned().unwrap(),
                a_uv: face.a.uv_index.and_then(|i| self.uv.get(i - 1)).cloned().unwrap_or_default(),
                b_uv: face.b.uv_index.and_then(|i| self.uv.get(i - 1)).cloned().unwrap_or_default(),
                c_uv: face.c.uv_index.and_then(|i| self.uv.get(i - 1)).cloned().unwrap_or_default(),
            });
        }
        tris
    }
}
