use std::f32::consts::PI;

use rand::Rng;
use raytracer_impl::implementation::{Entity, Scene, SceneSky};
use raytracer_impl::lights::*;
use raytracer_impl::materials::*;
use raytracer_impl::types::*;
use raytracer_impl::shapes::*;
use raytracer_impl::texture::*;
use crate::util::*;
use crate::scene::*;

pub struct EntityTransformTest;

impl SceneFactory for EntityTransformTest {
    fn create_controls(&self) -> SceneControlCollection {
        SceneControlCollection {
            name: "Entity Transforms Test".into(),
            controls: vec![
                SceneControl::range("Lamp Height",        0.0, 300.0).with_default(10.0),
                SceneControl::range("Lamp Intensity",     0.0, 1500.0).with_default(400.0),
                SceneControl::range("Lamp Angle",         0.0, 180.0).with_default(120.0),
                SceneControl::range("Swirl Rotation Deg", 0.0, 360.0).with_default(0.0),
                SceneControl::range("Plane Rotation Deg", 0.0, 360.0).with_default(0.0),
                SceneControl::range("Plane Checker Scale",0.0, 50.0).with_default(1.0),
            ],
        }
    }

    fn create_scene(&self, camera_config: &CameraConfiguration, config: &SceneConfiguration) -> Result<Scene, CreateSceneError> {
        // Camera
        let look_from = V3::ZERO + (V3::NEG_Z * 100.0) + (V3::POS_Y * 100.0);
        let look_to   = V3::ZERO;
        let camera    = camera_config.make_camera(look_to, look_from);

        // Scene
        let mut scene = Scene::new(camera, SceneSky::Day);
        let lambertian = scene.add_material(MatLambertian::default());
        let checker = scene.add_texture(CheckerTexture::new(
            config.get("Plane Checker Scale")?,
            ColorTexture(V3(1.0, 0.5, 0.5)),
            ColorTexture(V3(0.5, 1.0, 0.5))
        ));

        // Lights
        let lamp_pos = V3::POS_Y * config.get("Lamp Height")?;
        let lamp_direction = look_to - lamp_pos;
        scene.add_light(
            LampLight::with_origin_and_direction(lamp_pos, lamp_direction)
                .with_intensity(config.get("Lamp Intensity")?)
                .with_angle(config.get("Lamp Angle")?)
        );

        // Plane
        scene.add_entity(
            Entity::new(Plane::new(V3::POS_Y, lambertian, checker))
                .translate(V3::NEG_Y * 0.5)
                .translate(V3::POS_X * 0.5)
                .rotate(V3::POS_Y, deg_to_rad(config.get("Plane Rotation Deg")?))
                .id(1)
        );

        // Points
        let additional_rot = config.get("Swirl Rotation Deg")?;
        let mut rng = create_rng_from_seed(138219031);

        for _ in 0..1000 {
            let dist = rng.random::<f32>();
            let angle = rng.random::<f32>();

            let translation = (V3::POS_X * dist * 50.0).rotate_about_axis(V3::POS_Y, (angle * 2.0 * PI) + deg_to_rad(additional_rot));

            let point_tex = scene.add_texture(ColorTexture(hsl_to_rgb(angle, dist, 0.6)));
            scene.add_entity(
                Entity::new(Sphere::new(1.0, lambertian, point_tex))
                    .translate(translation)
            );
        }

        Ok(scene)
    }
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> V3 {

    assert!(0. <= h && h <= 1.);
    assert!(0. <= s && s <= 1.);
    assert!(0. <= l && l <= 1.);

    fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
        if t < 0. { t += 1.; }
        if t > 1. { t -= 1.; }
        match t {
            t if t < 1.0/6.0 => p + (q - p) * 6.0 * t,
            t if t < 1.0/2.0 => q,
            t if t < 2.0/3.0 => p + (q - p) * (2.0/3.0 - t) * 6.0,
            _ => p
        }
    }

    if s == 0.0 {
        V3(1.0, 1.0, 1.0) // achromatic
    }
    else {
        let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
        let p = 2.0 * l - q;

        let r = hue_to_rgb(p, q, h + 1.0/3.0);
        let g = hue_to_rgb(p, q, h);
        let b = hue_to_rgb(p, q, h - 1.0/3.0);

        V3(r, g, b)
    }
}
