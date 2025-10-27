use std::ops::Range;

use arrayvec::ArrayVec;

use crate::implementation::AABB;
use crate::types::{Ray, V3};
use crate::util::{partition_by_key};

pub struct BvhBounds {
    pub centroid: V3,
    pub aabb: AABB,
}

pub trait BvhObject {
    fn calculate_bounds(&self) -> BvhBounds;
}

struct BvhNode {
    aabb: AABB,
    data: BvhNodeData,
}

#[derive(Clone)]
struct BvhBranch {
    // Index of left/right nodes in Node collection
    left_offset: usize,
    right_offset: usize,
}

#[derive(Clone)]
struct BvhLeaf {
    // Index of object in Object collection
    offset: usize,
    // Number of objects in object collection which this node includes
    length: usize,
}

impl BvhLeaf {
    fn range(&self) -> Range<usize> {
        self.offset..(self.offset + self.length)
    }
}

#[derive(Clone)]
enum BvhNodeData {
    Branch(BvhBranch),
    Leaf(BvhLeaf),
}

impl BvhNode {
    fn leaf_data(&self) -> &BvhLeaf {
        match self.data {
            BvhNodeData::Leaf(ref leaf) => leaf,
            _ => panic!("Not a leaf node"),
        }
    }
}

struct BvhObjectBounds {
    object_index: usize,
    bounds: BvhBounds,
}

pub struct Bvh {
    object_bounds: Vec<BvhObjectBounds>,
    nodes: Vec<BvhNode>,
}

impl Bvh {
    // BVH algorithm adapted
    // from https://jacco.ompf2.com/2022/04/13/how-to-build-a-bvh-part-1-basics/
    pub fn from<T: BvhObject>(objects: &[T]) -> Bvh {
        // Precalculate the object bounds map
        let object_bounds = objects.iter()
            .enumerate()
            .map(|(object_index, object)| {
                let bounds = T::calculate_bounds(&object);
                BvhObjectBounds { object_index, bounds }
            })
            .collect();

        build(object_bounds)
    }

    pub fn aabb(&self) -> AABB {
        self.nodes[0].aabb.clone()
    }

    pub fn hit_candidates<'a>(&'a self, ray: Ray, t_min: f32, t_max: f32) -> BvhHitCandidateIter<'a> {
        let mut stack = ArrayVec::new();
        stack.push(State { node_index: 0, offset: 0 });
        BvhHitCandidateIter { bvh: self, stack, ray, t_min, t_max }
    }
}

#[derive(Clone, Copy, Debug)]
enum Axis { X, Y, Z }
impl Axis {
    fn value(&self, v3: &V3) -> f32 {
        match self {
            Axis::X => v3.x(),
            Axis::Y => v3.y(),
            Axis::Z => v3.z(),
        }
    }
}

/// Create a leaf node representing the given object_bounds.
fn create_leaf(offset: usize, object_bounds: &[BvhObjectBounds]) -> BvhNode {
    BvhNode {
        aabb: AABB::from_vertices_iter(
            object_bounds.iter().flat_map(|b| [b.bounds.aabb.min, b.bounds.aabb.max])
        ),
        data: BvhNodeData::Leaf(BvhLeaf { offset, length: object_bounds.len() })
    }
}

fn select_longest_axis(extent: &V3) -> Axis {
    let (mut axis, mut len) = (Axis::X, extent.x().abs());
    if extent.y() > len {
        (axis, len) = (Axis::Y, extent.y().abs());
    }
    if extent.z() > len {
        axis = Axis::Z;
    }
    axis
}

fn build(mut object_bounds: Vec<BvhObjectBounds>) -> Bvh {
    let mut nodes = Vec::with_capacity(object_bounds.len() * 2);

    // Prepare the root node
    nodes.push(create_leaf(0, &object_bounds));

    subdivide(0, &mut nodes, &mut object_bounds);
    nodes.shrink_to_fit();

    Bvh {
        object_bounds,
        nodes
    }
}

fn subdivide(offset: usize, nodes: &mut Vec<BvhNode>, object_bounds: &mut [BvhObjectBounds]) {
    let node = &nodes[offset];
    let leaf = node.leaf_data();

    // Stop subdividing nodes when we get to a minimum size
    if leaf.length <= 2 {
        return;
    }

    // See: https://fileadmin.cs.lth.se/cs/Education/EDAN35/projects/2022/Sanden-BVH.pdf
    // This is rough implementation of Select Longest Axis & Mean Partitioning
    let centroids = object_bounds[leaf.range()].iter().map(|o| o.bounds.centroid);

    // Select the longest axis to subdivide on
    let bounds = AABB::from_vertices_iter(centroids.clone());
    let extent = bounds.max - bounds.min;
    let axis = select_longest_axis(&extent);

    // Find a split point (find the mean position on this axis)
    let split_on = centroids.map(|c| axis.value(&c)).sum::<f32>() / leaf.length as f32;

    // Partition objects in place
    let (left, right) = partition_by_key(&mut object_bounds[leaf.range()], split_on, |o| axis.value(&o.bounds.centroid));

    // Stop subdividing if one of the partitions is empty
    if left.len() == 0 || right.len() == 0 {
        return;
    }

    // Create child nodes
    let l1 = create_leaf(leaf.offset, left);
    let l2 = create_leaf(leaf.offset + left.len(), right);

    let left_offset = nodes.len();
    nodes.push(l1);
    let right_offset = nodes.len();
    nodes.push(l2);

    // Convert current node into a branch
    nodes[offset].data = BvhNodeData::Branch(BvhBranch { left_offset, right_offset });

    // Recurse
    subdivide(left_offset, nodes, object_bounds);
    subdivide(right_offset, nodes, object_bounds);
}

pub struct BvhHitCandidateIter<'a> {
    bvh: &'a Bvh,
    stack: ArrayVec<State, 30>,
    ray: Ray,
    t_min: f32,
    t_max: f32
}

pub struct BvhHitCandidate {
    pub object_index: usize,
}

struct State {
    // The index of the node being evaluated (Leaf or Branch)
    node_index: usize,
    // The offset of the object within the Leaf Node
    offset: usize,
}

/// Iterator over a depth-first search of the bounding volume hierachy
impl<'a> Iterator for BvhHitCandidateIter<'a> {
    type Item=BvhHitCandidate;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let State { node_index, offset } = self.stack.pop()?;
            let node = &self.bvh.nodes[node_index];
            if !node.aabb.hit_aabb(self.ray, self.t_min, self.t_max) {
                continue;
            }
            match node.data {
                BvhNodeData::Branch(ref branch) => {
                    self.stack.push(State { node_index: branch.left_offset, offset: 0 });
                    self.stack.push(State { node_index: branch.right_offset, offset: 0 });
                },
                BvhNodeData::Leaf(ref leaf) => {
                    if offset < leaf.length - 1 {
                        // Push the next object to be emitted to the stack
                        self.stack.push(State { node_index, offset: offset + 1 });
                    }
                    // Emit the current object
                    let object_index = self.bvh.object_bounds[leaf.offset + offset].object_index;
                    return Some(BvhHitCandidate { object_index });
                }
            }
        }
    }
}
