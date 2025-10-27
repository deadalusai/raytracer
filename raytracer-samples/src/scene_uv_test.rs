use raytracer_impl::implementation::{ Entity, Scene, SceneSky };
use raytracer_impl::lights::*;
use raytracer_impl::materials::*;
use raytracer_impl::types::*;
use raytracer_impl::shapes::*;
use raytracer_impl::texture::*;
use raytracer_obj::{ load_color_map, load_obj_builder };
use crate::util::*;
use crate::scene::*;

pub struct SceneUvTest;

impl SceneFactory for SceneUvTest {
    fn create_controls(&self) -> SceneControlCollection {
        SceneControlCollection {
            name: "UV Test".into(),
            controls: vec![
                SceneControl::range("Mesh Rotation X Deg", -180.0, 180.0).with_default(0.0),
                SceneControl::range("Mesh Rotation Y Deg", -180.0, 180.0).with_default(0.0),
                SceneControl::range("Plane Rotation Deg", -180.0, 180.0).with_default(0.0),
                SceneControl::range("Plane Checker Scale",0.0, 50.0).with_default(1.0),
                SceneControl::range("Plane Offset X", -10.0, 10.0).with_default(0.5),
                SceneControl::range("Plane Offset Y", -10.0, 10.0).with_default(-0.5),
            ],
        }
    }

    fn create_scene(&self, camera_config: &CameraConfiguration, config: &SceneConfiguration) -> Result<Scene, CreateSceneError> {
        // Camera
        let look_from = V3::ZERO + (V3::NEG_Z * 2.0) + (V3::POS_Y * 2.0);
        let look_to   = V3::ZERO;
        let camera    = camera_config.make_camera(look_to, look_from);

        // Scene
        let mut scene = Scene::new(camera, SceneSky::Day);
        let lambertian = scene.add_material(MatLambertian::default());

        // Lights
        let lamp_pos = look_from; // V3::POS_Y * 70.0;
        let lamp_direction = look_to - lamp_pos;
        scene.add_light(
            LampLight::with_origin_and_direction(lamp_pos, lamp_direction)
                .with_intensity(30.0)
                .with_angle(60.0)
        );

        // Plane
        let plane_tex = scene.add_texture(CheckerTexture::new(
            config.get("Plane Checker Scale")?,
            ColorTexture(V3(1.0, 0.5, 0.5)),
            ColorTexture(V3(0.5, 1.0, 0.5))
        ));
        let plane_origin =
            look_to +
            (V3::POS_Y * config.get("Plane Offset Y")?) +
            (V3::POS_X * config.get("Plane Offset X")?);

        scene.add_entity(
            Entity::new(Plane::new(V3::POS_Y, lambertian, plane_tex))
                .translate(plane_origin)
                .rotate(V3::POS_Y, deg_to_rad(config.get("Plane Rotation Deg")?))
                .id(1)
        );

        // Mesh Plane
        let mesh_tex = scene.add_texture(load_color_map(crate::mesh_path!("simple/test.bmp"))?);
        let mesh_origin = look_to + (V3::POS_Y * 0.5);
        let mesh_mesh_data = load_obj_builder(crate::mesh_path!("simple/plane.obj"))?.build_mesh();
        scene.add_entity(
            Entity::new(MeshObject::new(mesh_mesh_data.mesh.clone(), lambertian, mesh_tex))
                .translate(mesh_origin)
                .rotate(V3::POS_Y, deg_to_rad(180.0)) // Face front to camera
                .rotate(V3::POS_Y, deg_to_rad(config.get("Mesh Rotation Y Deg")?))
                .rotate(V3::POS_X, deg_to_rad(config.get("Mesh Rotation X Deg")?))
                .id(2)
        );

        let mesh_uv_tex = scene.add_texture(UvTestTexture);
        let mesh_origin = look_to + (V3::POS_Y * 0.5) + (V3::POS_X * 1.0) + (V3::NEG_Z * 0.5);
        scene.add_entity(
            Entity::new(MeshObject::new(mesh_mesh_data.mesh, lambertian, mesh_uv_tex))
                .translate(mesh_origin)
                .rotate(V3::POS_Y, deg_to_rad(180.0 - 45.0)) // Face front to camera, rotate slightly
                .rotate(V3::POS_Y, deg_to_rad(config.get("Mesh Rotation Y Deg")?))
                .rotate(V3::POS_X, deg_to_rad(config.get("Mesh Rotation X Deg")?))
                .id(3)
        );

        Ok(scene)
    }
}
