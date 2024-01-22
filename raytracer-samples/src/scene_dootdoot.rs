use std::f32::consts::PI;

use raytracer_impl::implementation::{Scene, SceneSky};
use raytracer_impl::lights::*;
use raytracer_impl::materials::*;
use raytracer_impl::types::*;
use raytracer_impl::shapes::*;
use raytracer_impl::transform::*;
use raytracer_obj::load_obj_builder;
use crate::util::*;
use crate::scene::*;

pub struct SceneDootDoot;

impl SceneFactory for SceneDootDoot {
    fn create_controls(&self) -> SceneControlCollection {
        SceneControlCollection {
            name: "Doot Doot".into(),
            controls: vec![
                SceneControl::select_list("Sky", vec!["Black".into(), "Day".into()]),
                SceneControl::range("Camera Distance", 0.1, 200.0).with_default(70.0),
                SceneControl::range("Global Light Intensity", 0.1, 2000.0).with_default(1100.0),
                SceneControl::range_angle_deg("Doot Doot Yaw"),
                SceneControl::range_angle_deg("Doot Doot Roll"),
                SceneControl::range_angle_deg("Doot Doot Pitch"),
                SceneControl::range("Spacing", 0.0, 1000.0).with_default(30.0),
                SceneControl::range("Count", 1.0, 1000.0).with_default(10.0),
                SceneControl::toggle("Spin em round"),
            ],
        }
    }

    fn create_scene(&self, camera_config: &CameraConfiguration, config: &SceneConfiguration) -> Result<Scene, CreateSceneError> {
        use rand::{ thread_rng, Rng };

        // Camera
        let dist = config.get("Camera Distance")?;
        let look_to = V3::ZERO;
        let look_from = look_to + (V3::ONE * dist).rotate_about_axis(V3::POS_Y, PI);
        let camera = camera_config.make_camera(look_to, look_from);

        // Scene
        let mut scene = Scene::new(camera, if config.get("Sky")? == 0.0 { SceneSky::Black } else { SceneSky::Day });

        // Lights
        // Global illumination
        scene.add_light({
            let global_from = look_from + (V3::POS_Y * 100.0);
            PointLight::with_origin(global_from)
                .with_intensity(config.get("Global Light Intensity")?)
        });
        
        let mat = scene.add_material(MatLambertian::default());
        let mesh_data = load_obj_builder(crate::mesh_path!("skeleton/SKELETON.obj"))?.build_mesh();
        let tex = scene.add_texture(mesh_data.texture_set);
        
        let mesh = MeshObject::new(&mesh_data.mesh, mat, tex)
            .rotated(V3::POS_Z, deg_to_rad(config.get("Doot Doot Roll")?))
            .rotated(V3::POS_X, deg_to_rad(config.get("Doot Doot Pitch")?))
            .rotated(V3::POS_Y, deg_to_rad(config.get("Doot Doot Yaw")?));
        
        let spacing = config.get("Spacing")?;
        let count = config.get("Count")? as usize;
        let half_w = count as f32 * spacing / 2.0;
        let spin = config.get("Spin em round")? != 0.0;

        let mut rand = thread_rng();

        for x in 0..count {
            for z in 0..count {
                let x = (x as f32 * spacing) - half_w;
                let z = (z as f32 * spacing) - half_w;
                let rot = if spin { rand.gen_range(0.0..2.0) * PI } else { 0.0 };
                scene.add_object(
                    mesh.clone()
                        .rotated(V3::POS_Y, rot)
                        .translated(V3(x, 0.0, z))
                );
            }
        }

        Ok(scene)
    }
}
