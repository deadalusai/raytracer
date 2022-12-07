use std::sync::Arc;

use crate::types::{ Ray };
use crate::implementation::{ AABB, Hitable, HitRecord };

// Hacky Bounding Volume Hierachy implementation for Hitable objects

pub struct BvhNode {
    aabb: AABB,
    left: Arc<dyn Hitable>,
    right: Arc<dyn Hitable>,
}

impl BvhNode {
    pub fn new(left: Arc<dyn Hitable>, right: Arc<dyn Hitable>) -> BvhNode {
        let aabb = AABB::surrounding(
            left.bounding_box().expect("Left hitable bounding box"),
            right.bounding_box().expect("Right hitable bounding box"),
        );
        BvhNode { aabb, left, right }
    }
}

impl Hitable for BvhNode {
    fn hit<'a>(&'a self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'a>> {
        if !self.aabb.hit_aabb(ray, t_min, t_max) {
            return None;
        }
        let hit_l = self.left.hit(ray, t_min, t_max);
        let hit_r = self.right.hit(ray, t_min, t_max);
        match (hit_l, hit_r) {
            (Some(l), Some(r)) => Some(if l.t < r.t { l } else { r }),
            (Some(l), None)    => Some(l),
            (None,    Some(r)) => Some(r),
            _                  => None,
        }
    }

    fn bounding_box(&self) -> Option<AABB> {
        Some(self.aabb.clone())
    }
}

#[derive(Clone, Copy)]
pub enum SortAxis { X, Y, Z }
impl SortAxis {
    pub fn next(self) -> SortAxis {
        match self {
            SortAxis::X => SortAxis::Y,
            SortAxis::Y => SortAxis::Z,
            SortAxis::Z => SortAxis::X,
        }
    }
}

pub fn compare_aabb(l: &AABB, r: &AABB, axis: SortAxis) -> std::cmp::Ordering {
    let ordering = match axis {
        SortAxis::X => l.min.x().partial_cmp(&r.min.x()),
        SortAxis::Y => l.min.y().partial_cmp(&r.min.y()),
        SortAxis::Z => l.min.z().partial_cmp(&r.min.z()),
    };
    ordering.unwrap_or(std::cmp::Ordering::Equal)
}

pub fn build_bvh_hierachy(hitables: &mut [(AABB, Arc<dyn Hitable>)]) -> Option<Arc<dyn Hitable>> {

    fn inner(hitables: &mut [(AABB, Arc<dyn Hitable>)], axis: SortAxis) -> Option<Arc<dyn Hitable>> {

        let node = match hitables {
            [] => return None,
            [a] => a.1.clone(),
            _ => {
                hitables.sort_by(|l, r| compare_aabb(&l.0, &r.0, axis));
                let mid = hitables.len() / 2;
                let left = inner(&mut hitables[0..mid], axis.next()).unwrap();
                let right = inner(&mut hitables[mid..], axis.next()).unwrap();
                Arc::new(BvhNode::new(left, right))
            }
        };

        Some(node)
    }

    inner(hitables, SortAxis::X)
}