use rand::Rng;
use raytracer_impl::implementation::{ Entity, Scene, SceneSky };
use raytracer_impl::lights::*;
use raytracer_impl::materials::*;
use raytracer_impl::types::*;
use raytracer_impl::shapes::*;
use raytracer_impl::texture::*;
use crate::util::*;
use crate::scene::*;

pub struct ScenePointCloud;

impl SceneFactory for ScenePointCloud {
    fn name(&self) -> &str {
        "Point Cloud"
    }

    fn create_controls(&self) -> SceneControlCollection {
        SceneControlCollection {
            name: self.name().into(),
            controls: vec![
                SceneControl::range("Camera Distance", 1.0, 1500.0).with_default(100.0),
                SceneControl::range("Cloud Width",  1.0, 1000.0).with_default(100.0),
                SceneControl::range("Cloud Depth",  1.0, 1000.0).with_default(100.0),
                SceneControl::range("Cloud Height", 1.0, 1000.0).with_default(100.0),
                SceneControl::range("Cloud Point Count", 1.0, 10_000_000.0).with_default(1_00_000.0),
                SceneControl::range("Cloud Point Diameter", 0.05, 100.0).with_default(0.5),
                SceneControl::range("Spotlight Intensity", 0.0, 2000.0).with_default(1300.0),
                SceneControl::range("Spotlight Beam Angle", 1.0, 90.0).with_default(60.0),
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
        let lamp_pos = look_from.rotate_about_axis(V3::POS_Y, deg_to_rad(15.0));
        let lamp_direction = look_to - lamp_pos;
        scene.add_light(
            LampLight::with_origin_and_direction(look_from, lamp_direction)
                .with_intensity(config.get("Spotlight Intensity")?)
                .with_angle(config.get("Spotlight Beam Angle")?)
        );

        let point_mat = scene.add_material(MatLambertian::default());
        let point_radius = config.get("Cloud Point Diameter")?;

        let mut rng = create_rng_from_seed(432789012409);

        let x_len = config.get("Cloud Width")?;
        let z_len = config.get("Cloud Height")?;
        let y_len = config.get("Cloud Depth")?;

        for _ in 0..(config.get("Cloud Point Count")? as usize) {

            let a = rng.random::<f32>();
            let b = rng.random::<f32>();
            let c = rng.random::<f32>();

            let x = (a * x_len) - (x_len / 2.0);
            let y = (b * z_len) - (z_len / 2.0);
            let z = (c * y_len) - (y_len / 2.0);

            let point_tex = scene.add_texture(ColorTexture(V3(a, b, c)));
            scene.add_entity(
                Entity::new(Sphere::new(point_radius, point_mat, point_tex))
                    .translate(V3(x, y, z))
            );
        }

        Ok(scene)
    }
}
