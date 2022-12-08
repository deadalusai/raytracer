use std::sync::Arc;

use crate::types::{ V3, Ray, IntoArc };
use crate::implementation::{ Material, Hitable, HitRecord, AABB };

// Triangle Mesh BVH

type Tri = (V3, V3, V3);

struct TriIntersect {
    p: V3,
    normal: V3,
    t: f32,
}

// Ref: https://www.scratchapixel.com/lessons/3d-basic-rendering/ray-tracing-rendering-a-triangle/ray-triangle-intersection-geometric-solution
fn tri_intersect(ray: &Ray, a: V3, b: V3, c: V3) -> Option<TriIntersect> {
    // Find the normal of the triangle, using v0 as the origin
    let normal = V3::cross(b - a, c - a).unit();
    // Find the intesection `p` with the tiangle plane
    let t = super::plane::intersect_plane(ray, a, normal)?;
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

fn tri_aabb(tri: &Tri) -> AABB {
    AABB::from_vertices(&[tri.0, tri.1, tri.2])
}

pub struct MeshBvhLeaf(Tri);

impl MeshBvhLeaf {
    fn hit_node(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<TriIntersect> {
        tri_intersect(ray, self.0.0, self.0.1, self.0.2).filter(|x| t_min < x.t && x.t < t_max)
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
    fn hit_node(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<TriIntersect> {
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
    fn hit_node(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<TriIntersect> {
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

pub fn build_triangle_bvh_hierachy(triangles: &[Tri]) -> Option<MeshBvhNode> {
    use super::bvh::{ SortAxis, compare_aabb };

    fn inner(triangles: &mut [(AABB, &Tri)], axis: SortAxis) -> Option<MeshBvhNode> {

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

pub struct Mesh {
    object_id: Option<u32>,
    origin: V3,
    mesh_node: MeshBvhNode,
    material: Arc<dyn Material>,
    reflection_mode: MeshReflectionMode,
}

impl Mesh {
    pub fn new(origin: V3, triangles: Vec<(V3, V3, V3)>, material: impl IntoArc<dyn Material>) -> Self {
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
            // Shift the hit back into world space
            p: mesh_hit.p + self.origin,
            t: mesh_hit.t,
            object_id: self.object_id,
            material: self.material.as_ref(),
            normal,
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