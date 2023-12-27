use raytracer_impl::implementation::{Scene, SceneSky};
use raytracer_impl::lights::*;
use raytracer_impl::materials::*;
use raytracer_impl::types::*;
use raytracer_impl::shapes::*;
use raytracer_impl::transform::*;
use raytracer_obj::load_obj_builder;
use crate::util::*;
use crate::scene::*;

pub struct SceneDreadnaught;

impl SceneFactory for SceneDreadnaught {
    fn create_controls(&self) -> SceneControlCollection {
        SceneControlCollection {
            name: "Dreadnaught".into(),
            controls: vec![
                SceneControl::range("Camera Distance",  50.0,   1500.0).with_default(800.0),
                SceneControl::range("Lamp Intensity",   1.0,    200.0).with_default(100.0),
                SceneControl::range("Dreadnaught Yaw",  -180.0, 180.0),
                SceneControl::range("Dreadnaught Roll", -180.0, 180.0),
            ],
        }
    }

    fn create_scene(&self, camera_config: &CameraConfiguration, config: &SceneConfiguration) -> Result<Scene, CreateSceneError> {
        // Camera
        let dist = config.get("Camera Distance");
        let look_to =   V3::ZERO;
        let look_from = V3::ZERO + (V3::POS_Y * dist) + (V3::POS_Z * dist) + (V3::POS_X * dist);
        let camera = camera_config.make_camera(look_to, look_from);

        // Scene
        let mut scene = Scene::new(camera, SceneSky::Black);

        // Lights
        let lamp_intensity = config.get("Lamp Intensity");
        scene.add_light(PointLight::with_origin(look_from).with_intensity(lamp_intensity));
        
        let mesh_builder = load_obj_builder("./raytracer-samples/meshes/Dreadnaught/Dreadnaught.obj").unwrap();
        let mat = scene.add_material(MatLambertian::default());
        let mesh_data = mesh_builder.build_mesh();
        let tex = scene.add_texture(mesh_data.texture_set);
        scene.add_object(MeshObject::new(&mesh_data.mesh, mat, tex)
            .rotated(V3::POS_Z, deg_to_rad(config.get("Dreadnaught Roll")))
            .rotated(V3::POS_Y, deg_to_rad(config.get("Dreadnaught Yaw"))));

        Ok(scene)
    }
}
