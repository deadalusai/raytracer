use std::ops::Range;

use arrayvec::ArrayVec;

use crate::implementation::AABB;
use crate::types::{Ray, V3};

pub trait BvhObject {
    fn aabb(&self) -> AABB;
    fn centroid(&self) -> V3;
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
    aabb: AABB,
    centroid: V3,
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
            .map(|(object_index, object)| BvhObjectBounds {
                object_index,
                aabb: T::aabb(&object),
                centroid: T::centroid(&object),
            })
            .collect();

        build(object_bounds)
    }

    pub fn aabb(&self) -> &AABB {
        &self.nodes[0].aabb
    }

    pub fn hit_candidates<'a>(&'a self, ray: &'a Ray, t_min: f32, t_max: f32) -> BvhHitCandidateIter<'a> {
        let mut stack = ArrayVec::new();
        stack.push(State::Branch(0));
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
            object_bounds[leaf.range()].iter().flat_map(|b| [b.aabb.min, b.aabb.max])
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

    // Select an axis to subdivide on and a position on that axis to split
    let bounds = AABB::from_vertices_iter(object_bounds[leaf.range()].iter().map(|o| o.centroid));
    let extent = bounds.max - bounds.min;
    let axis = select_longest_axis(&extent);
    let split_pos = axis.value(&bounds.min) + (axis.value(&extent) * 0.5);

    // Roughly partition objects in place
    let mut i = leaf.first_index;
    let mut j = leaf.first_index + leaf.length - 1;
    while i <= j {
        let pos = axis.value(&object_bounds[i].centroid);
        if pos < split_pos {
            // Count object into left partition
            i += 1;
        }
        else if j == 0 {
            // Halt if `j` is about to underflow
            break;
        }
        else {
            // Swap object to the end of the right partition
            object_bounds.swap(i, j);
            j -= 1;
        }
    }

    // Create child nodes
    let left = BvhLeaf { first_index: leaf.first_index, length: i - leaf.first_index };
    let right = BvhLeaf { first_index: i, length: leaf.length - left.length };
    
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
    ray: &'a Ray,
    t_min: f32,
    t_max: f32
}

pub struct BvhHitCandidate {
    pub object_index: usize,
}

enum State {
    Branch(usize), // node_index
    Leaf(usize, usize), // node_index, offset
}

/// Iterator over a depth-first search of the bounding volume hierachy
impl<'a> Iterator for BvhHitCandidateIter<'a> {
    type Item=BvhHitCandidate;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.stack.pop()? {
                State::Branch(node_index) => {
                    let node = &self.bvh.nodes[node_index];
                    if !node.aabb.hit_aabb(self.ray, self.t_min, self.t_max) {
                        continue;
                    }
                    match node.data {
                        BvhNodeData::Branch(ref branch) => {
                            self.stack.push(State::Branch(branch.left_index));
                            self.stack.push(State::Branch(branch.right_index));
                        },
                        BvhNodeData::Leaf(_) => {
                            self.stack.push(State::Leaf(node_index, 0));
                        }
                    }
                },
                State::Leaf(node_index, offset) => {
                    let leaf = self.bvh.nodes[node_index].leaf_data();
                    if offset < leaf.length - 1 {
                        // Push the next object to be emitted to the stack
                        self.stack.push(State::Leaf(node_index, offset + 1));
                    }
                    // Emit the current object
                    let object_index = self.bvh.object_bounds[leaf.offset(offset)].object_index;
                    return Some(BvhHitCandidate { object_index });
                },
                
            }
        }
    }
}