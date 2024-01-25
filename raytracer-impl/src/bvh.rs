use std::mem::size_of_val;

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

pub struct Bvh {
    object_indices: Vec<usize>,
    nodes: Vec<BvhNode>,
}

impl Bvh {
    // BVH algorithm adapted
    // from https://jacco.ompf2.com/2022/04/13/how-to-build-a-bvh-part-1-basics/    
    pub fn construct<T: BvhObject>(objects: &[T]) -> Bvh {
        // Initialise the object index map
        let mut object_indices = (0..objects.len()).collect::<Vec<usize>>();
        let mut nodes = Vec::with_capacity(objects.len() * 2);
        
        // Prepare the root node
        let root = BvhLeaf { first_index: 0, length: objects.len() };
        let root = create_leaf_node(root, &mut object_indices, objects);
        nodes.push(root);

        subdivide(&mut nodes, SortAxis::X, 0, &mut object_indices, objects);
        nodes.shrink_to_fit();

        Bvh {
            object_indices,
            nodes
        }
    }

    pub fn aabb(&self) -> &AABB {
        &self.nodes[0].aabb
    }

    pub fn hit_candidates<'a>(&'a self, ray: &'a Ray, t_min: f32, t_max: f32) -> BvhHitCandidateIter<'a> {
        let mut stack = Vec::with_capacity(10);
        stack.push(State { node_index: 0 });
        BvhHitCandidateIter { bvh: self, stack, ray, t_min, t_max }
    }
}

#[derive(Clone, Copy, Debug)]
enum SortAxis { X, Y, Z }
impl SortAxis {
    pub fn next(self) -> SortAxis {
        match self {
            SortAxis::X => SortAxis::Y,
            SortAxis::Y => SortAxis::Z,
            SortAxis::Z => SortAxis::X,
        }
    }
}

fn axis_value(v3: &V3, axis: SortAxis) -> f32 {
    match axis {
        SortAxis::X => v3.x(),
        SortAxis::Y => v3.y(),
        SortAxis::Z => v3.z(),
    }
}

fn create_leaf_node<T: BvhObject>(leaf: BvhLeaf, object_indices: &[usize], objects: &[T]) -> BvhNode {
    BvhNode {
        aabb: object_indices[leaf.first_index..(leaf.first_index + leaf.length)]
                .iter()
                .map(|&i| T::aabb(&objects[i]))
                .reduce(|a, b| AABB::surrounding(a, b))
                .unwrap(),
        data: BvhNodeData::Leaf(leaf),
    }
}

fn subdivide<T: BvhObject>(nodes: &mut Vec<BvhNode>, axis: SortAxis, node_index: usize, object_indices: &mut [usize], objects: &[T]) {

    let node = &nodes[node_index];

    // Stop subdividing nodes when we get to a minimum size
    if node.leaf_data().length <= 1 {
        return;
    }

    // Sort objects along the chosen axis
    let node_data = node.leaf_data();
    let i = node_data.first_index;
    let j = node_data.first_index + node_data.length;
    let nlen = node_data.length / 2;

    object_indices[i..j]
        .sort_by(|&a, &b| {
            let a = axis_value(&T::centroid(&objects[a]), axis);
            let b = axis_value(&T::centroid(&objects[b]), axis);
            a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
        });

    // Create child nodes
    let left = BvhLeaf { first_index: i, length: nlen };
    let right = BvhLeaf { first_index: i + nlen, length: node_data.length - nlen };

    // HACK: Stop subdividing if one of the sides is empty
    if left.length == 0 || right.length == 0 {
        return;
    }

    let left_index = nodes.len();
    nodes.push(create_leaf_node(left, object_indices, objects));
    let right_index = nodes.len();
    nodes.push(create_leaf_node(right, object_indices, objects));

    // Convert current node into a branch
    nodes[node_index].data = BvhNodeData::Branch(BvhBranch { left_index, right_index });
    
    // Recurse
    subdivide(nodes, axis.next(), left_index, object_indices, objects);
    subdivide(nodes, axis.next(), right_index, object_indices, objects);
}

pub struct BvhHitCandidateIter<'a> {
    bvh: &'a Bvh,
    stack: Vec<State>,
    ray: &'a Ray,
    t_min: f32,
    t_max: f32
}

pub struct BvhHitCandidate {
    pub object_index: usize,
}

struct State {
    node_index: usize,
}

/// Iterator over a depth-first search of the bounding volume hierachy
impl<'a> Iterator for BvhHitCandidateIter<'a> {
    type Item=BvhHitCandidate;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let state = self.stack.pop()?;
            let node = &self.bvh.nodes[state.node_index];
            if !node.aabb.hit_aabb(self.ray, self.t_min, self.t_max) {
                continue;
            }
            match node.data {
                BvhNodeData::Branch(ref branch) => {
                    self.stack.push(State { node_index: branch.left_index });
                    self.stack.push(State { node_index: branch.right_index });
                },
                BvhNodeData::Leaf(ref leaf) => {
                    assert_eq!(leaf.length, 1, "Expected leaf to represent exactly one object");
                    let object_index = self.bvh.object_indices[leaf.first_index];
                    return Some(BvhHitCandidate { object_index });
                }
            }
        }
    }
}