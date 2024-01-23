use crate::implementation::AABB;
use crate::types::{Ray, V3};

pub trait BvhObject {
    fn vertices(&self) -> impl Iterator<Item=V3>;
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

pub struct BvhHit {
    t: f32,
    object_index: usize,
}

impl Bvh {
    // BVH algorithm adapted
    // from https://jacco.ompf2.com/2022/04/13/how-to-build-a-bvh-part-1-basics/    
    pub fn construct<T: BvhObject>(objects: &[T]) -> Bvh {
        // Initialise the object index map
        let mut object_indices = (0..objects.len()).collect::<Vec<usize>>();
        let mut nodes = Vec::default();
        
        // Prepare the root node
        let root = BvhLeaf { first_index: 0, length: objects.len() };
        let root = create_leaf_node(root, &mut object_indices, objects);
        nodes.push(root);

        subdivide(&mut nodes, 0, &mut object_indices, objects);
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
        stack.push(State::Branch(0));
        BvhHitCandidateIter { bvh: self, stack, ray, t_min, t_max }
    }
}

#[derive(Copy, Clone)]
enum SplitAxis { X, Y, Z }

fn axis_value(v3: &V3, axis: SplitAxis) -> f32 {
    match axis {
        SplitAxis::X => v3.x(),
        SplitAxis::Y => v3.y(),
        SplitAxis::Z => v3.z(),
    }
}

fn create_leaf_node<T: BvhObject>(leaf: BvhLeaf, object_indices: &[usize], objects: &[T]) -> BvhNode {
    BvhNode {
        aabb: AABB::from_vertices_iter(
            object_indices[leaf.first_index..(leaf.first_index + leaf.length)]
                .iter()
                .map(|&i| &objects[i])
                .flat_map(T::vertices)
        ),
        data: BvhNodeData::Leaf(leaf)
    }
}

fn subdivide<T: BvhObject>(nodes: &mut Vec<BvhNode>, node_index: usize, object_indices: &mut [usize], objects: &[T]) {

    let node = &nodes[node_index];

    // Terminate recursion?
    if node.leaf_data().length <= 2 {
        return;
    }

    // Select an axis to split on (Pick the longest axis for now)
    let AABB { min, max } = node.aabb;
    let extent = max - min;
    let mut axis = SplitAxis::X;
    if extent.y() > extent.x() {
        axis = SplitAxis::Y;
    }
    if extent.z() > extent.y() {
        axis = SplitAxis::Z;
    }

    // Partition objects around the middle of the chosen axis
    let split_at = axis_value(&min, axis) + axis_value(&extent, axis) * 0.5;

    // Partition objects along this axis
    let node_data = node.leaf_data();
    let mut i = node_data.first_index;
    let mut j = node_data.length - 1;
    while i <= j {
        if axis_value(&T::centroid(&objects[object_indices[i]]), axis) < split_at {
            // object already sorted into the left partition
            i += 1;
        }
        else {
            // swap with an object from the right partition
            object_indices.swap(i, j);
            j -= 1;
        }
    }

    // Create child nodes
    let left = BvhLeaf { first_index: node_data.first_index, length: i - node_data.first_index };
    let right = BvhLeaf { first_index: i, length: node_data.length - left.length };

    let left_index = nodes.len();
    nodes.push(create_leaf_node(left, object_indices, objects));
    let right_index = nodes.len();
    nodes.push(create_leaf_node(right, object_indices, objects));

    // Convert current node into a branch
    nodes[node_index].data = BvhNodeData::Branch(BvhBranch { left_index, right_index });
    
    // Recurse
    subdivide(nodes, left_index, object_indices, objects);
    subdivide(nodes, right_index, object_indices, objects);
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

enum State {
    Branch(usize),
    Leaf(usize),
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
                        BvhNodeData::Leaf(ref leaf) => {
                            for object_index in leaf.first_index..(leaf.first_index + leaf.length) {
                                self.stack.push(State::Leaf(object_index))
                            }
                        },
                    }
                },
                State::Leaf(object_index) => {
                    return Some(BvhHitCandidate { object_index });
                }
            }
        }
    }
}