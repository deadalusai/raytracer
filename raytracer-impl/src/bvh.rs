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
    left_index: usize,
    right_index: usize,
}

#[derive(Clone)]
struct BvhLeaf {
    // Index of object in Object collection
    first_index: usize,
    // Number of objects in object collection which this node includes
    length: usize,
}

impl BvhLeaf {
    fn range(&self) -> Range<usize> {
        self.first_index..(self.first_index + self.length)
    }

    fn offset(&self, offset: usize) -> usize {
        self.first_index + offset
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

fn create_leaf_node(leaf: BvhLeaf, object_bounds: &[BvhObjectBounds]) -> BvhNode {
    BvhNode {
        aabb: AABB::from_vertices_iter(
            object_bounds[leaf.range()].iter().flat_map(|b| [b.bounds.aabb.min, b.bounds.aabb.max])
        ),
        data: BvhNodeData::Leaf(leaf),
    }
}

fn select_longest_axis(extent: &V3) -> Axis {
    let (mut axis, mut len) = (Axis::X, extent.x());
    if extent.y() > len {
        (axis, len) = (Axis::Y, extent.y());
    }
    if extent.z() > len {
        axis = Axis::Z;
    }
    axis
}

fn build(mut object_bounds: Vec<BvhObjectBounds>) -> Bvh {
    let mut nodes = Vec::with_capacity(object_bounds.len() * 2);

    // Prepare the root node
    let root = BvhLeaf { first_index: 0, length: object_bounds.len() };
    let root = create_leaf_node(root, &object_bounds);
    nodes.push(root);

    subdivide(0, &mut nodes, &mut object_bounds);
    nodes.shrink_to_fit();

    Bvh {
        object_bounds,
        nodes
    }
}

fn subdivide(node_index: usize, nodes: &mut Vec<BvhNode>, object_bounds: &mut [BvhObjectBounds]) {
    let node = &nodes[node_index];
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
    let (len_left, len_right) = partition_by_key(&mut object_bounds[leaf.range()], split_on, |o| axis.value(&o.bounds.centroid));

    // Create child nodes
    let left = BvhLeaf { first_index: leaf.first_index, length: len_left };
    let right = BvhLeaf { first_index: len_left, length: len_right };

    // Stop subdividing if one of the sides is empty
    if left.length == 0 || right.length == 0 {
        return;
    }

    let left_index = nodes.len();
    nodes.push(create_leaf_node(left, object_bounds));
    let right_index = nodes.len();
    nodes.push(create_leaf_node(right, object_bounds));

    // Convert current node into a branch
    nodes[node_index].data = BvhNodeData::Branch(BvhBranch { left_index, right_index });

    // Recurse
    subdivide(left_index, nodes, object_bounds);
    subdivide(right_index, nodes, object_bounds);
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
                    self.stack.push(State { node_index: branch.left_index, offset: 0 });
                    self.stack.push(State { node_index: branch.right_index, offset: 0 });
                },
                BvhNodeData::Leaf(ref leaf) => {
                    if offset < leaf.length - 1 {
                        // Push the next object to be emitted to the stack
                        self.stack.push(State { node_index, offset: offset + 1 });
                    }
                    // Emit the current object
                    let object_index = self.bvh.object_bounds[leaf.offset(offset)].object_index;
                    return Some(BvhHitCandidate { object_index });
                }
            }
        }
    }
}
