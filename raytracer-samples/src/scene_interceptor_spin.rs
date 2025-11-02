use std::f32::consts::PI;
use std::time::SystemTime;
use raytracer_impl::implementation::{ Entity, Scene, SceneSky };
use raytracer_impl::lights::*;
use raytracer_impl::materials::*;
use raytracer_impl::types::*;
use raytracer_impl::shapes::*;
use raytracer_obj::load_obj_builder;
use crate::util::*;
use crate::scene::*;

pub struct SceneInterceptorSpin;

impl SceneFactory for SceneInterceptorSpin {
    fn name(&self) -> &str {
        "Interceptor Spin"
    }

    fn create_controls(&self) -> SceneControlCollection {
        SceneControlCollection {
            name: self.name().into(),
            controls: vec![
                SceneControl::range("Camera Distance", 1.0, 1500.0).with_default(100.0),
                SceneControl::range("Seconds Per Rotation",  1.0, 1000.0).with_default(10.0),
                SceneControl::range("Global Light Intensity", 1.0, 800.0).with_default(20.0),
                SceneControl::range("Spotlight Intensity", 0.0, 2000.0).with_default(1200.0),
                SceneControl::range("Spotlight Beam Angle", 1.0, 90.0).with_default(60.0),
            ],
        }
    }

    fn create_scene(&self, camera_config: &CameraConfiguration, config: &SceneConfiguration) -> Result<Scene, CreateSceneError> {
        // Time and rotation angle
        let ms_per_rotation = config.get("Seconds Per Rotation")? * 1000.0;
        let ms_since_epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|err| CreateSceneError(err.to_string()))?
            .as_millis();

        let rot_rads = ((ms_since_epoch % ms_per_rotation as u128) as f32 / ms_per_rotation) * PI * 2.0;

        // Camera
        let dist = config.get("Camera Distance")?;
        let look_to =   V3::ZERO;
        let look_from = V3::ONE * dist;
        let camera = camera_config.make_camera(look_to, look_from);

        // Scene
        let mut scene = Scene::new(camera, SceneSky::Black);

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

        let int_mesh_data = load_obj_builder(crate::mesh_path!("Interceptor-T/Heavyinterceptor.obj"))?.build_mesh();
        let int_mat = scene.add_material(MatLambertian::default());
        let int_tex = scene.add_texture(int_mesh_data.texture_set);
        let int_mesh = Entity::new(MeshObject::new(int_mesh_data.mesh, int_mat, int_tex))
            // Interceptor model spins as time passes
            .rotate(V3::POS_Y, rot_rads)
            // Interceptor model is facing +Z rotated on its side
            .rotate(V3::POS_Z, deg_to_rad(90.0));

        scene.add_entity(int_mesh);

        Ok(scene)
    }
}
