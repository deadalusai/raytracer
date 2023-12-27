pub mod util;
pub mod scene;

use std::sync::Arc;
use scene::SceneFactory;
use crate::scene::BasicSceneFactory;

mod samples;
mod scene_dreadnaught;

pub fn make_sample_scene_factories() -> Vec<Arc<dyn SceneFactory + Send + Sync>> {
    vec![
        Arc::new(BasicSceneFactory::new("Random Spheres", samples::random_sphere_scene)),
        Arc::new(BasicSceneFactory::new("Simple",         samples::simple_scene)),
        Arc::new(BasicSceneFactory::new("Planes",         samples::planes_scene)),
        Arc::new(BasicSceneFactory::new("Mirrors",        samples::hall_of_mirrors)),
        Arc::new(BasicSceneFactory::new("Triangles",      samples::triangle_world)),
        Arc::new(BasicSceneFactory::new("Mesh",           samples::mesh_demo)),
        Arc::new(BasicSceneFactory::new("Capsule",        samples::capsule)),
        Arc::new(BasicSceneFactory::new("Mesh Plane",     samples::mesh_plane)),
        Arc::new(BasicSceneFactory::new("Point Cloud",    samples::point_cloud)),
        Arc::new(BasicSceneFactory::new("Mega Cube",      samples::mega_cube)),
        Arc::new(BasicSceneFactory::new("Spaceships",     samples::spaceships)),
        Arc::new(BasicSceneFactory::new("Fleet",          samples::fleet)),
        Arc::new(scene_dreadnaught::SceneDreadnaught),
    ]
}