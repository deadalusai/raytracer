use std::sync::Arc;

use crate::types::{ V3, Ray };
use crate::implementation::{ Material, Hitable, HitRecord, AABB };
use super::{ TriIntersect, intersect_tri };

// Triangle Mesh BVH

type Tri = (V3, V3, V3);

fn tri_bbox(tri: &Tri) -> AABB {
    AABB::from_vertices(&[tri.0, tri.1, tri.2])
}

pub struct MeshBvhLeaf(Tri);

impl MeshBvhLeaf {
    fn hit_node(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<TriIntersect> {
        intersect_tri(ray, self.0.0, self.0.1, self.0.2).filter(|ti| t_min < ti.t && ti.t < t_max)
    }
}

pub struct MeshBvhBranch {
    aabb: AABB,
    left: Arc<MeshBvhNode>,
    right: Arc<MeshBvhNode>,
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
            MeshBvhNode::Leaf(leaf) => tri_bbox(&leaf.0),
            MeshBvhNode::Branch(branch) => branch.aabb.clone(),
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
                    left: Arc::new(left),
                    right: Arc::new(right),
                })
            }
        };

        Some(node)
    }

    let mut triangles = triangles.iter()
        .map(|tri| (tri_bbox(tri), tri))
        .collect::<Vec<_>>();

    inner(&mut triangles, SortAxis::X)
}

// Mesh

pub struct Mesh {
    object_id: Option<u32>,
    origin: V3,
    mesh_node: MeshBvhNode,
    material: Box<dyn Material>,
}

impl Mesh {
    pub fn new<M>(origin: V3, triangles: Vec<(V3, V3, V3)>, material: M) -> Self
        where M: Material + 'static
    {
        let mesh_node = build_triangle_bvh_hierachy(&triangles).expect("Expected at least one triangle for mesh");

        Mesh { object_id: None, origin, mesh_node, material: Box::new(material) }
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
        // Shift the hit back into world space
        let p = mesh_hit.p + self.origin;
        let t = mesh_hit.t;
        let object_id = self.object_id;
        let material = self.material.as_ref();
        // If this plane is facing away from the ray we want to flip the reported normal
        // so that reflections work in both directions.
        let normal = if V3::dot(ray.direction, mesh_hit.normal) > 0.0 { -mesh_hit.normal } else { mesh_hit.normal };
        Some(HitRecord { object_id, p, t, normal, material })
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