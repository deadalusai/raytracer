pub mod scene;
pub mod util;

use scene::{
    // BasicSceneFactory,
    SceneFactory,
};
use std::sync::Arc;

// mod samples;
// mod scene_dootdoot;
// mod scene_dreadnaught;
mod scene_interceptor_spin;
// mod scene_point_cloud;

pub fn make_sample_scene_factories() -> Vec<Arc<dyn SceneFactory + Send + Sync>> {
    vec![
        // Arc::new(BasicSceneFactory::new("Random Spheres", samples::random_sphere_scene)),
        // Arc::new(BasicSceneFactory::new("Simple",         samples::simple_scene)),
        // Arc::new(BasicSceneFactory::new("Planes",         samples::planes_scene)),
        // Arc::new(BasicSceneFactory::new("Mirrors",        samples::hall_of_mirrors)),
        // Arc::new(BasicSceneFactory::new("Triangles",      samples::triangle_world)),
        // Arc::new(BasicSceneFactory::new("Mesh",           samples::mesh_demo)),
        // Arc::new(BasicSceneFactory::new("Capsule",        samples::capsule)),
        // Arc::new(BasicSceneFactory::new("Mesh Plane",     samples::mesh_plane)),
        // Arc::new(scene_point_cloud::ScenePointCloud),
        // Arc::new(BasicSceneFactory::new("Mega Cube",      samples::mega_cube)),
        // Arc::new(BasicSceneFactory::new("Spaceships",     samples::spaceships)),
        // Arc::new(BasicSceneFactory::new("Fleet",          samples::fleet)),
        Arc::new(scene_interceptor_spin::SceneInterceptorSpin),
        // Arc::new(scene_dreadnaught::SceneDreadnaught),
        // Arc::new(scene_dootdoot::SceneDootDoot),
    ]
}

//
// Macros
//

/// Gets an absolute path to any sub-path under raytracer-samples/meshes
macro_rules! mesh_path {
    ($sub_path: expr) => {
        concat!(env!("CARGO_MANIFEST_DIR"), "/meshes/", $sub_path)
    };
}
pub(crate) use mesh_path;
