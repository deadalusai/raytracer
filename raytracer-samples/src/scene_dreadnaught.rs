use raytracer_impl::implementation::{Entity, Scene, SceneSky};
use raytracer_impl::lights::*;
use raytracer_impl::materials::*;
use raytracer_impl::types::*;
use raytracer_impl::shapes::*;
use raytracer_obj::load_obj_builder;
use crate::util::*;
use crate::scene::*;

pub struct SceneDreadnaught;

impl SceneFactory for SceneDreadnaught {
    fn create_controls(&self) -> SceneControlCollection {
        SceneControlCollection {
            name: "Dreadnaught".into(),
            controls: vec![
                SceneControl::range("Camera Distance", 50.0, 1500.0).with_default(800.0),
                SceneControl::range("Global Light Intensity", 1.0, 200.0).with_default(20.0),
                SceneControl::range("Spotlight Intensity", 1.0, 200.0).with_default(80.0),
                SceneControl::range("Spotlight Beam Angle", 1.0, 90.0).with_default(10.0),
                SceneControl::range_angle_deg("Dreadnaught Yaw"),
                SceneControl::range_angle_deg("Dreadnaught Roll"),
                SceneControl::range_angle_deg("Dreadnaught Pitch"),
            ],
        }
    }

    fn create_scene(&self, camera_config: &CameraConfiguration, config: &SceneConfiguration) -> Result<Scene, CreateSceneError> {
        // Camera
        let dist = config.get("Camera Distance")?;
        let look_to =   V3::ZERO;
        let look_from = V3::ZERO + (V3::POS_Y * dist) + (V3::POS_Z * dist) + (V3::POS_X * dist);
        let camera = camera_config.make_camera(look_to, look_from);

        // Scene
        let mut scene = Scene::new(camera, SceneSky::Black);

        // Lights
        // Global illumination
        scene.add_light({
            let global_from = look_from + (V3::POS_Y * 1000.0);
            PointLight::with_origin(global_from)
                .with_intensity(config.get("Global Light Intensity")?)
        });
        // Spotlight attached above camera
        scene.add_light({
            let spotlight_from = look_from + (V3::POS_Y * dist / 10.0);
            LampLight::with_origin_and_direction(spotlight_from, look_to - spotlight_from)
                .with_intensity(config.get("Spotlight Intensity")?)
                .with_angle(config.get("Spotlight Beam Angle")?)
        });

        let mat = scene.add_material(MatLambertian::default());
        let mesh_data = load_obj_builder(crate::mesh_path!("Dreadnaught/Dreadnaught.obj"))?.build_mesh();
        let tex = scene.add_texture(mesh_data.texture_set);
        scene.add_entity(
            Entity::new(MeshObject::new(mesh_data.mesh, mat, tex))
                .rotate(V3::POS_Z, deg_to_rad(config.get("Dreadnaught Roll")?))
                .rotate(V3::POS_X, deg_to_rad(config.get("Dreadnaught Pitch")?))
                .rotate(V3::POS_Y, deg_to_rad(config.get("Dreadnaught Yaw")?))
        );

        Ok(scene)
    }
}
