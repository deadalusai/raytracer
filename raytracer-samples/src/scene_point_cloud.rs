use rand::Rng;
use raytracer_impl::implementation::{Scene, SceneSky};
use raytracer_impl::lights::*;
use raytracer_impl::materials::*;
use raytracer_impl::types::*;
use raytracer_impl::shapes::*;
use raytracer_impl::texture::*;
use crate::util::*;
use crate::scene::*;

pub struct ScenePointCloud;

impl SceneFactory for ScenePointCloud {
    fn create_controls(&self) -> SceneControlCollection {
        SceneControlCollection {
            name: "Point Cloud".into(),
            controls: vec![
                SceneControl::range("Camera Distance", 1.0, 1500.0).with_default(100.0),
                SceneControl::range("Cloud Width",  1.0, 200.0).with_default(50.0),
                SceneControl::range("Cloud Depth",  1.0, 200.0).with_default(50.0),
                SceneControl::range("Cloud Height", 1.0, 200.0).with_default(50.0),
                SceneControl::range("Cloud Point Count", 1.0, 10_000_000.0).with_default(1_000_000.0),
                SceneControl::range("Global Intensity", 0.0, 1.0).with_default(0.5),
                SceneControl::range("Spotlight Intensity", 0.0, 2000.0).with_default(200.0),
                SceneControl::range("Spotlight Beam Angle", 1.0, 90.0).with_default(10.0),
            ],
        }
    }

    fn create_scene(&self, camera_config: &CameraConfiguration, config: &SceneConfiguration) -> Result<Scene, CreateSceneError> {
        // Camera
        let dist = config.get("Camera Distance")?;
        let look_from = V3::ZERO + (V3::ONE * dist);
        let look_to   = V3::ZERO;
        let camera    = camera_config.make_camera(look_to, look_from);
    
        // Scene
        let mut scene = Scene::new(camera, SceneSky::Black);
    
        // Lights
        let lamp_pos = look_from;
        let lamp_direction = look_to - lamp_pos;
        scene.add_light(
            DirectionalLight::with_direction(lamp_direction)
                .with_intensity(config.get("Global Intensity")?)
        );
        scene.add_light(
            LampLight::with_origin_and_direction(look_from, lamp_direction)
                .with_intensity(config.get("Spotlight Intensity")?)
                .with_angle(config.get("Spotlight Beam Angle")?)
        );
    
        let point_mat = scene.add_material(MatLambertian::default());
        let point_radius = 0.05;
    
        let mut rng = create_rng_from_seed(432789012409);

        let x_len = config.get("Cloud Width")?;
        let z_len = config.get("Cloud Depth")?;
        let y_len = config.get("Cloud Height")?;
    
        for _ in 0..(config.get("Cloud Point Count")? as usize) {
    
            let a = rng.gen::<f32>();
            let b = rng.gen::<f32>();
            let c = rng.gen::<f32>();
    
            let x = (a * x_len) - (x_len / 2.0);
            let y = (b * z_len) - (z_len / 2.0);
            let z = (c * y_len) - (y_len / 2.0);
    
            let point_tex = scene.add_texture(ColorTexture(V3(a, b, c)));
            scene.add_object(Sphere::new(point_radius, point_mat, point_tex).with_origin(V3(x, y, z)))
        }
    
        Ok(scene)
    }
}
