#![allow(unused)]

use std::f32::consts::PI;
use std::path::Path;

use raytracer_impl::texture::{ ColorTexture, CheckerTexture, UvTestTexture, XyzTestTexture, MeshTextureSet };
use raytracer_impl::types::{ V3, Ray };
use raytracer_impl::materials::{ MatLambertian, MatDielectric, MatSpecular };
use raytracer_impl::shapes::{ Sphere, Plane, MeshObject, MeshFace, Mesh, mesh };
use raytracer_impl::transform::{ Translatable, Rotatable };
use raytracer_impl::viewport::{ Viewport };
use raytracer_impl::lights::{ PointLight, DirectionalLight, LampLight };
use raytracer_impl::implementation::{ Scene, SceneSky, Camera, Material, MatId, TexId };
use raytracer_obj::{ load_obj_builder, load_color_map };

use rand::{ Rng };
use rand_xorshift::{ XorShiftRng };

fn create_rng_from_seed(a: u128) -> XorShiftRng {
    use rand::SeedableRng;
    
    fn set_bytes(bytes: &mut [u8], val: u128) {
        for offset in 0..16 {
            let shift = (15 - offset) * 8;
            bytes[offset] = ((val >> shift) & 0xff) as u8
        }
    }

    let mut seed = <XorShiftRng as SeedableRng>::Seed::default();

    set_bytes(&mut seed[0..16], a);
    
    XorShiftRng::from_seed(seed)
}

//
// Configuration
//

fn deg_to_rad(deg: f32) -> f32 {
    (deg / 180.0) * std::f32::consts::PI
}

pub struct CameraConfiguration {
    pub width: f32,
    pub height: f32,
    pub lens_radius: f32,
    pub fov: f32,
    pub angle_adjust_v: f32,
    pub angle_adjust_h: f32,
    pub focus_dist_adjust: f32,
}

impl CameraConfiguration {
    fn aspect_ratio(&self) -> f32 {
        self.width / self.height
    }

    fn make_camera(&self, look_to: V3, default_look_from: V3) -> Camera {

        let look_from = {
            // Translate into rotation space
            let p = default_look_from - look_to;

            // The vertical axis (to rotate about horizontally)
            let v_axis = V3::POS_Y;
            let p = p.rotate_about_axis(v_axis, deg_to_rad(self.angle_adjust_h));
            
            // The horizontal axis (to rotate about vertically)
            let w = (V3::ZERO - p).unit();             // Vector to origin 
            let h_axis = V3::cross(v_axis, w).unit();  // Vector to camera right
            let p = p.rotate_about_axis(h_axis, deg_to_rad(self.angle_adjust_v));

            // Translate into world space
            p + look_to
        };
        let dist_to_focus = (look_from - look_to).length() + self.focus_dist_adjust;
        
        Camera::new(look_from, look_to, self.fov, self.aspect_ratio(), self.lens_radius, dist_to_focus)
    }
}

//
// Easing functions
//

fn lerp_v3(p1: V3, p2: V3, d: f32) -> V3 {
    let v_between = (p2 - p1) * d;
    p1 + v_between
}

fn lerp_f32(p1: f32, p2: f32, d: f32) -> f32 {
    let v_between = (p2 - p1) * d;
    p1 + v_between
}

fn ease_in(t: f32, scale: f32) -> f32 {
    // y = x ^ 2
    t.powf(scale)
}

fn ease_out(t: f32, scale: f32) -> f32 {
    // y = 1 - ((1 - x) ^ 2)
    1.0 - (1.0 - t).powf(scale)
}

fn ease_in_out(t: f32, scale: f32) -> f32 {
    lerp_f32(ease_in(t, scale), ease_out(t, scale), t)
}

// Positioning helpers

#[derive(Clone, Copy)]
enum Card {
    Origin,
    North(f32),
    South(f32),
    East(f32),
    West(f32),
    Up(f32),
    Down(f32),
}

impl Card {
    #[inline]
    fn v3(self) -> V3 {
        match self {
            Card::Origin   => V3(0.0,  0.0,  0.0),
            Card::North(f) => V3(1.0,  0.0,  0.0)  * f,
            Card::South(f) => V3(-1.0, 0.0,  0.0)  * f,
            Card::East(f)  => V3(0.0,  0.0,  1.0)  * f,
            Card::West(f)  => V3(0.0,  0.0,  -1.0) * f,
            Card::Up(f)    => V3(0.0,  1.0,  0.0)  * f,
            Card::Down(f)  => V3(0.0,  -1.0, 0.0)  * f,
        }
    }
}

macro_rules! position {
    ( Origin ) => ( Card::Origin.v3() );
    ( $move:tt($v:expr) ) => ( Card::$move($v).v3() );
    ( $move:tt($v:expr), $( $rest:tt($rest_v:expr) ),* ) => ( Card::$move($v).v3() + position!($( $rest($rest_v) ),*) );
}

//
// Sample scenes
//

// Attenuation factory

fn rgb(r: u8, g: u8, b: u8) -> V3 {
    V3(r as f32 / 255.0,
       g as f32 / 255.0,
       b as f32 / 255.0)
}

//
// Scenes
//


// Random texture factories

fn make_matte(scene: &mut Scene, rng: &mut impl Rng) -> (MatId, TexId) {
    let albedo = V3(
        /* r */ rng.gen::<f32>() * rng.gen::<f32>(),
        /* g */ rng.gen::<f32>() * rng.gen::<f32>(),
        /* b */ rng.gen::<f32>() * rng.gen::<f32>()
    );
    (
        scene.add_material(MatLambertian::default()),
        scene.add_texture(ColorTexture(albedo))
    )
}

fn make_metal(scene: &mut Scene, rng: &mut impl Rng) -> (MatId, TexId) {
    let color = V3(
        /* r */ 0.5 * (1.0 + rng.gen::<f32>()),
        /* g */ 0.5 * (1.0 + rng.gen::<f32>()),
        /* b */ 0.5 * (1.0 + rng.gen::<f32>())
    );
    let fuzz = 0.5 * rng.gen::<f32>();
    (
        scene.add_material(MatSpecular::default().with_fuzz(fuzz)),
        scene.add_texture(ColorTexture(color))
    )
}

fn make_glass(scene: &mut Scene, rng: &mut impl Rng) -> (MatId, TexId) {
    let refractive_index = 1.5;
    let color = V3(
        /* r */ 0.5 * (1.0 + rng.gen::<f32>()),
        /* g */ 0.5 * (1.0 + rng.gen::<f32>()),
        /* b */ 0.5 * (1.0 + rng.gen::<f32>())
    );
    (
        scene.add_material(MatDielectric::default().with_ref_index(refractive_index)),
        scene.add_texture(ColorTexture(color))
    )
}

pub fn random_sphere_scene(config: &CameraConfiguration) -> Scene {
    // Camera
    let look_from = V3(13.0, 2.0, 3.0);
    let look_to = V3(0.0, 0.0, 0.0);
    let camera = config.make_camera(look_to, look_from);

    // Scene
    let mut rng = create_rng_from_seed(3178901564);
    let mut scene = Scene::new(camera, SceneSky::Day);

    // Lights
    let lamp_direction = position!(Origin) - V3(4.0, 100.0, 4.0);
    scene.add_light(DirectionalLight::with_direction(lamp_direction).with_intensity(0.5));

    // World sphere
    let world_tex = scene.add_texture(CheckerTexture::new(
        10.0,
        ColorTexture(V3(0.4, 0.5, 0.4)),
        ColorTexture(V3(0.9, 0.8, 0.9))
    ));
    let world_mat = scene.add_material(MatLambertian::default());

    scene.add_object(Sphere::new(1000.0, world_mat, world_tex).with_origin(V3(0.0, -1000.0, 0.0)));

    // Large metal sphere
    let lam_sphere_center = V3(-4.0, 1.0, 0.0);
    let lam_sphere_tex = scene.add_texture(ColorTexture(V3(0.8, 0.2, 0.1)));
    let lam_sphere_mat = scene.add_material(MatLambertian::default());
    scene.add_object(Sphere::new(1.0, lam_sphere_mat, lam_sphere_tex).with_origin(lam_sphere_center.clone()));
    
    // Large hollow glass sphere
    let hollow_sphere_center = V3(0.0, 1.0, 0.0);
    let hollow_sphere_tex = scene.add_texture(ColorTexture(V3(0.95, 0.95, 0.95)));
    let hollow_sphere_mat = scene.add_material(MatDielectric::default().with_ref_index(1.5));
    scene.add_object(Sphere::new(1.0, hollow_sphere_mat, hollow_sphere_tex).with_origin(hollow_sphere_center.clone()));

    // Large mat sphere
    let metal_sphere_center = V3(4.0, 1.0, 0.0);
    let metal_sphere_tex = scene.add_texture(ColorTexture(V3(0.8, 0.8, 0.8)));
    let metal_sphere_mat = scene.add_material(MatSpecular::default().with_fuzz(0.0));
    scene.add_object(Sphere::new(1.0, metal_sphere_mat, metal_sphere_tex).with_origin(metal_sphere_center.clone()));

    let sphere_centers = [lam_sphere_center, hollow_sphere_center, metal_sphere_center];

    // Small random spheres
    for a in -11..11 {
        for b in -11..11 {
            let center = V3(
                /* x */ a as f32 + 0.9 * rng.gen::<f32>(),
                /* y */ 0.2,
                /* z */ b as f32 + 0.9 * rng.gen::<f32>()
            );

            // Only include the sphere if it's not too close to the three large spheres..
            if sphere_centers.iter().any(|&pos| (center - pos).length() < 1.5) {
                continue;
            }

            // Select a material
            let (mat, tex) =
                match rng.gen::<f32>() {
                    v if v < 0.8  => make_matte(&mut scene, &mut rng),
                    v if v < 0.95 => make_metal(&mut scene, &mut rng),
                    _             => make_glass(&mut scene, &mut rng),
                };

            scene.add_object(Sphere::new(0.2, mat, tex).with_origin(center));
        }
    }

    scene
}

fn add_cardinal_markers(scene: &mut Scene) {
    // Direction makers
    let mat   = scene.add_material(MatLambertian::default());
    let red   = scene.add_texture(ColorTexture(rgb(128, 0,   0)));
    let green = scene.add_texture(ColorTexture(rgb(0,   128, 0)));
    let blue  = scene.add_texture(ColorTexture(rgb(0,   0,   128)));
    let white = scene.add_texture(ColorTexture(rgb(255, 255, 255)));
    scene.add_object(Sphere::new(0.25, mat, red).with_origin(position!(North(2.0))));
    scene.add_object(Sphere::new(0.25, mat, green).with_origin(position!(East(2.0))));
    scene.add_object(Sphere::new(0.25, mat, blue).with_origin(position!(West(2.0))));
    scene.add_object(Sphere::new(0.25, mat, white).with_origin(position!(South(2.0))));
}

fn add_coordinates_marker(scene: &mut Scene) {
    // Direction makers
    let mat   = scene.add_material(MatLambertian::default());
    let red   = scene.add_texture(ColorTexture(rgb(128, 0,   0)));
    let green = scene.add_texture(ColorTexture(rgb(0,   128, 0)));
    let blue  = scene.add_texture(ColorTexture(rgb(0,   0,   128)));
    scene.add_object(Sphere::new(0.05, mat, red).with_origin(V3(1.0, 0.0, 0.0)));
    scene.add_object(Sphere::new(0.05, mat, green).with_origin(V3(0.0, 1.0, 0.0)));
    scene.add_object(Sphere::new(0.05, mat, blue).with_origin(V3(0.0, 0.0, 1.0)));
}

pub fn simple_scene(config: &CameraConfiguration) -> Scene {

    // Camera
    let look_from = position!(South(6.0), East(1.5), Up(3.0));
    let look_to =   position!(Up(1.0));
    let camera = config.make_camera(look_to, look_from);

    // Scene
    let mut scene = Scene::new(camera, SceneSky::Black);

    // Lights
    let lamp_pos = position!(Up(20.0), North(4.0));
    let lamp_direction = position!(Origin) - lamp_pos;
    scene.add_light(LampLight::with_origin_and_direction(lamp_pos, lamp_direction).with_intensity(80.0).with_angle(12.0));

    let lamp_pos = position!(Up(10.0), East(4.0));
    let lamp_direction = position!(Origin) - lamp_pos;
    scene.add_light(LampLight::with_origin_and_direction(lamp_pos, lamp_direction).with_intensity(80.0).with_angle(20.0));

    add_cardinal_markers(&mut scene);

    // World sphere
    let world_mat = scene.add_material(MatLambertian::default());
    let world_tex = scene.add_texture(ColorTexture(rgb(200, 200, 200)));
    let world_pos = position!(Down(1000.0));
    scene.add_object(Sphere::new(1000.0, world_mat, world_tex).with_origin(world_pos));

    // Wall
    let wall_mat = scene.add_material(MatLambertian::default().with_reflectivity(1.0));
    let wall_tex = scene.add_texture(ColorTexture(rgb(200, 200, 200)));
    let wall_pos = position!(North(4.5));
    let wall_facing = wall_pos - position!(Origin);
    scene.add_object(Plane::new(wall_facing, wall_mat, wall_tex).with_origin(wall_pos));

    // Plastic sphere
    let plastic_mat = scene.add_material(MatLambertian::default());
    let plastic_tex = scene.add_texture(ColorTexture(rgb(226, 226, 226)));
    let plastic_pos = position!(Up(1.0));
    scene.add_object(Sphere::new(1.0, plastic_mat, plastic_tex).with_origin(plastic_pos));

    // Glass sphere (large)
    let glass_mat = scene.add_material(MatDielectric::default());
    let glass_tex = scene.add_texture(ColorTexture(rgb(130, 255, 140)));
    let glass_pos = position!(Up(1.0), South(2.0), East(2.0));
    scene.add_object(Sphere::new(1.0, glass_mat, glass_tex).with_origin(glass_pos.clone()));
    
    // Glass sphere (small)
    let small_glass_mat = scene.add_material(MatDielectric::default().with_opacity(0.01).with_reflectivity(0.98));
    let small_glass_tex = scene.add_texture(ColorTexture(rgb(66, 206, 245)));
    let small_glass_pos = lerp_v3(plastic_pos, lamp_pos, 0.2); // Find a point between the lamp and the plastic sphere
    scene.add_object(Sphere::new(0.5, small_glass_mat, small_glass_tex).with_origin(small_glass_pos));

    // Metal sphere
    let metal_mat = scene.add_material(MatSpecular::default().with_fuzz(0.001).with_reflectivity(0.91));
    let metal_tex = scene.add_texture(ColorTexture(rgb(147, 154, 186)));
    let metal_pos = position!(Up(1.0), North(2.0), West(2.0));
    scene.add_object(Sphere::new(1.0, metal_mat, metal_tex).with_origin(metal_pos).with_id(1));


    // Small metal spheres (buried) drawn between these points
    let small_metal_mat = scene.add_material(MatSpecular::default().with_fuzz(0.01).with_reflectivity(0.4));
    let small_metal_tex = scene.add_texture(ColorTexture(V3(0.8, 0.1, 0.1)));
    let small_metal_sphere_count = 6;
    let small_metal_start_pos = position!(West(3.5), North(1.0));
    let small_metal_end_pos = position!(West(2.5), South(3.5));
    for i in 0..small_metal_sphere_count {
        let t = i as f32 / small_metal_sphere_count as f32;
        let small_metal_pos = lerp_v3(small_metal_start_pos, small_metal_end_pos, ease_out(t, 2.0));
        let small_metal_radius = lerp_f32(0.5, 0.05, ease_out(t, 2.0));
        scene.add_object(Sphere::new(small_metal_radius, small_metal_mat, small_metal_tex).with_origin(small_metal_pos));
    }

    // Small plastic spheres (buried) drawn between these points
    let small_plastic_mat = scene.add_material(MatDielectric::default());
    let small_plastic_tex = scene.add_texture(ColorTexture(V3(0.1, 0.9, 0.1)));
    let small_plastic_sphere_count = 12;
    let small_plastic_start_pos = position!(West(2.5), South(0.5));
    let small_plastic_end_pos = position!(West(0.5), South(2.5));
    for i in 0..small_plastic_sphere_count {
        let t = i as f32 / small_plastic_sphere_count as f32;
        // Ease towards the target around a curve
        let small_plastic_pos = V3(
            lerp_f32(small_plastic_start_pos.x(), small_plastic_end_pos.x(), ease_out(t, 2.0)),
            0.0,
            lerp_f32(small_plastic_start_pos.z(), small_plastic_end_pos.z(), ease_in(t, 2.0))
        );
        let small_plastic_radius = lerp_f32(0.10, 0.02, ease_in_out(t, 2.0));
        scene.add_object(Sphere::new(small_plastic_radius, small_plastic_mat, small_plastic_tex).with_origin(small_plastic_pos));
    }

    scene
}

pub fn planes_scene(config: &CameraConfiguration) -> Scene {

    // Camera
    let look_from = position!(South(6.0), East(1.5), Up(3.0));
    let look_to =   position!(Up(1.0));
    let camera = config.make_camera(look_to, look_from);

    // Scene
    let mut scene = Scene::new(camera, SceneSky::Day);

    // Lights
    let lamp_pos = position!(Up(6.0), East(5.0));
    let lamp_normal = position!(Up(3.0)) - lamp_pos;
    scene.add_light(LampLight::with_origin_and_direction(lamp_pos, lamp_normal).with_intensity(80.0).with_angle(20.0));

    add_cardinal_markers(&mut scene);

    // World sphere
    let world_mat = scene.add_material(MatLambertian::default().with_reflectivity(0.01));
    let world_tex = scene.add_texture(ColorTexture(rgb(255, 255, 255)));
    let world_pos = position!(Down(1000.0));
    scene.add_object(Sphere::new(1000.0, world_mat, world_tex).with_origin(world_pos));

    let plane_mat = scene.add_material(MatSpecular::default().with_reflectivity(0.8).with_fuzz(0.02));
    let plane_tex = scene.add_texture(ColorTexture(rgb(240, 240, 240)));
    let plane_pos = position!(West(1.0));
    let plane_normal = position!(Origin) - plane_pos; // normal facing world origin
    scene.add_object(Plane::new(plane_normal, plane_mat, plane_tex).with_origin(plane_pos));

    scene
}

pub fn hall_of_mirrors(config: &CameraConfiguration) -> Scene {

    // Camera
    let look_from = position!(Up(1.0), South(2.0), East(1.5));
    let look_to =   position!(Up(0.5));
    let camera = config.make_camera(look_to, look_from);

    // Scene
    let mut scene = Scene::new(camera, SceneSky::Day);

    // Lights
    let lamp_pos = position!(Up(10.0));
    let lamp_normal = position!(Origin) - lamp_pos;
    scene.add_light(LampLight::with_origin_and_direction(lamp_pos, lamp_normal).with_intensity(80.0).with_angle(20.0));

    add_cardinal_markers(&mut scene);

    add_coordinates_marker(&mut scene);

    // World sphere
    let world_mat = scene.add_material(MatLambertian::default().with_reflectivity(0.01));
    let world_tex = scene.add_texture(ColorTexture(rgb(255, 255, 255)));
    let world_pos = position!(Down(1000.0));
    scene.add_object(Sphere::new(1000.0, world_mat, world_tex).with_origin(world_pos));

    let plane_mat = scene.add_material(MatSpecular::default().with_reflectivity(0.98).with_fuzz(0.01));
    let plane_tex = scene.add_texture(ColorTexture(V3::ONE));
    let cardinals = [
        position!(North(3.0)),
        position!(South(3.0)),
        position!(East(3.0)),
        position!(West(3.0))
    ];
    for plane_origin in cardinals {
        let plane_normal = position!(Origin) - plane_origin; // normal facing world origin
        scene.add_object(
            Plane::new(plane_normal, plane_mat, plane_tex)
                .with_origin(plane_origin)
                .with_radius(30.0)
        );
    }

    scene
}

pub fn triangle_world(config: &CameraConfiguration) -> Scene {

    // Camera
    let look_from = position!(Up(5.0), South(6.0), East(1.5));
    let look_to =   position!(Up(0.0));
    let camera = config.make_camera(look_to, look_from);

    // Scene
    let mut scene = Scene::new(camera, SceneSky::Black);

    // Lights

    let lamp_pos = position!(Up(20.0), North(4.0));
    let lamp_direction = position!(Origin) - lamp_pos;
    scene.add_light(LampLight::with_origin_and_direction(lamp_pos, lamp_direction).with_intensity(80.0).with_angle(12.0));

    let lamp_pos = position!(Up(10.0), East(4.0));
    let lamp_direction = position!(Origin) - lamp_pos;
    scene.add_light(LampLight::with_origin_and_direction(lamp_pos, lamp_direction).with_intensity(80.0).with_angle(20.0));

    add_cardinal_markers(&mut scene);

    // World sphere
    let world_mat = scene.add_material(MatLambertian::default());
    let world_tex = scene.add_texture(ColorTexture(rgb(200, 200, 200)));
    let world_pos = position!(Down(1000.0));
    scene.add_object(Sphere::new(1000.0, world_mat, world_tex).with_origin(world_pos));

    // Triangle
    let tri_pos = position!(Origin);
    let tri_mat = scene.add_material(MatLambertian::default().with_reflectivity(0.0));
    let tri_tex = scene.add_texture(ColorTexture(rgb(200, 100, 80)));
    let tri_mesh = Mesh {
        faces: vec![
            MeshFace::from_abc(
                position!(Up(0.3), South(1.0)),
                position!(Up(0.6), East(1.0)),
                position!(Up(0.8), West(1.0))
            )
        ],
    };
    scene.add_object(MeshObject::new(&tri_mesh, tri_mat, tri_tex).with_origin(tri_pos));

    let tri_pos = position!(Up(1.0));
    let tri_mat = scene.add_material(MatLambertian::default().with_reflectivity(0.0));
    let tri_tex = scene.add_texture(ColorTexture(rgb(100, 100, 200)));
    let tri_mesh = Mesh {
        faces: vec![
            MeshFace::from_abc(
                position!(Up(0.4), North(1.0)),
                position!(Up(0.8), South(1.0)),
                position!(Up(0.6), East(1.0))
            )
        ],
    };
    scene.add_object(MeshObject::new(&tri_mesh, tri_mat, tri_tex).with_origin(tri_pos));

    scene
}

pub fn mesh_demo(config: &CameraConfiguration) -> Scene {
    
    // Camera
    let look_from = position!(Up(1.5), South(4.0), East(4.0));
    let look_to =   position!(Up(1.0));
    let camera = config.make_camera(look_to, look_from);

    // Scene
    let mut scene = Scene::new(camera, SceneSky::Black);

    // Lights
    let lamp_pos = position!(Up(5.0), East(4.0));
    let lamp_direction = position!(Origin) - lamp_pos;
    scene.add_light(LampLight::with_origin_and_direction(lamp_pos, lamp_direction).with_intensity(80.0).with_angle(20.0));
    
    let lamp_pos = position!(Up(3.0), West(6.0), North(1.5));
    let lamp_direction = position!(Origin) - lamp_pos;
    scene.add_light(LampLight::with_origin_and_direction(lamp_pos, lamp_direction).with_intensity(80.0).with_angle(20.0));

    // add_cardinal_markers(&mut scene);

    // World sphere
    let world_mat = scene.add_material(MatLambertian::default());
    let world_tex = scene.add_texture(ColorTexture(rgb(200, 200, 200)));
    let world_pos = position!(Down(1000.0));
    scene.add_object(
        Sphere::new(1000.0, world_mat, world_tex)
            .with_origin(world_pos)
            .with_id(0));

    // Cube
    let cube_mat = scene.add_material(MatLambertian::default().with_reflectivity(0.0));
    let cube_tex = scene.add_texture(ColorTexture(rgb(36, 193, 89)));
    let cube_origin = position!(South(1.5), West(1.5));
    let cube_mesh_builder = load_obj_builder("./raytracer-samples/meshes/simple/cube.obj").unwrap();
    let cube_mesh_data = cube_mesh_builder.build_mesh_data("Cube");
    scene.add_object(
        MeshObject::new(&cube_mesh_data.mesh, cube_mat, cube_tex)
            .with_origin(cube_origin)
            .with_id(1)
            .rotated(V3::POS_Y, PI / 4.0)
    );

    // Thing
    let thing_mat = scene.add_material(MatSpecular::default().with_reflectivity(0.8).with_fuzz(0.02));
    let thing_tex = scene.add_texture(ColorTexture(rgb(89, 172, 255)));
    let thing_origin = position!(North(1.5), East(1.5));
    let thing_mesh_builder = load_obj_builder("./raytracer-samples/meshes/simple/thing.obj").unwrap();
    let thing_mesh_data = thing_mesh_builder.build_mesh_data("Thing");
    scene.add_object(
        MeshObject::new(&thing_mesh_data.mesh, thing_mat, thing_tex)
            .with_origin(thing_origin)
            .with_id(2)
    );

    // Suzanne
    let suz_mat = scene.add_material(MatDielectric::default().with_opacity(0.2).with_ref_index(0.8).with_reflectivity(0.0));
    let suz_tex = scene.add_texture(ColorTexture(rgb(255, 137, 58)));
    let suz_origin = position!(Origin);
    let suz_mesh_builder = load_obj_builder("./raytracer-samples/meshes/simple/suzanne.obj").unwrap();
    let suz_mesh_data = suz_mesh_builder.build_mesh_data("Suzanne");
    scene.add_object(
        MeshObject::new(&suz_mesh_data.mesh, suz_mat, suz_tex)
            .with_origin(suz_origin)
            .with_id(3)
    );

    scene
}

pub fn spaceships(config: &CameraConfiguration) -> Scene {
    
    // Camera
    let look_from = position!(Up(350.0), South(400.0), East(700.0));
    let look_to =   position!(Down(50.0), West(350.0));
    let camera = config.make_camera(look_to, look_from);

    // Scene
    let mut scene = Scene::new(camera, SceneSky::Black);

    // Lights
    let lamp_pos = position!(Up(1000.0), East(1000.0));
    let lamp_direction = position!(Origin) - lamp_pos;
    scene.add_light(DirectionalLight::with_direction(lamp_direction).with_intensity(0.9));

    // World sphere
    let world_radius = 10_000.0;
    let world_mat = scene.add_material(MatLambertian::default());
    let world_tex = scene.add_texture(ColorTexture(rgb(200, 200, 200)));
    let world_pos = position!(Down(world_radius), Down(1000.0));
    scene.add_object(
        Sphere::new(world_radius, world_mat, world_tex)
            .with_origin(world_pos)
            .with_id(0)
    );

    // Destroyer (facing EAST)
    let mesh_builder = load_obj_builder("./raytracer-samples/meshes/Destroyer-K/Standarddestroyer.obj").unwrap();
    let dest_mesh_data = mesh_builder.build_mesh_data("default");
    let dest_mat = scene.add_material(MatLambertian::default());
    let dest_tex = scene.add_texture(dest_mesh_data.texture_set);
    // NOTE: Destroyer model is facing +Z rotated on its side (X UP)
    let dest_mesh = MeshObject::new(&dest_mesh_data.mesh, dest_mat, dest_tex).rotated(V3::POS_Z, -deg_to_rad(90.0));

    // Interceptor (facing EAST)
    let mesh_builder = load_obj_builder("./raytracer-samples/meshes/Interceptor-T/Heavyinterceptor.obj").unwrap();
    let int_mesh_data = mesh_builder.build_mesh_data("default");
    let int_mat = scene.add_material(MatLambertian::default());
    let int_tex = scene.add_texture(int_mesh_data.texture_set);
    // NOTE: Interceptor model is facing +Z rotated on its side (X UP)
    let int_mesh = MeshObject::new(&int_mesh_data.mesh, int_mat, int_tex).rotated(V3::POS_Z, -deg_to_rad(90.0));

    // Spawn a few interceptors across the bow of the Destroyer
    let int_origin = look_to + position!(Up(200.0), East(300.0), South(30.0));
    scene.add_object(int_mesh.clone().translated(int_origin + position!(East(15.0))));
    scene.add_object(int_mesh.clone().translated(int_origin + position!(East(2.0), North(80.0), Down(30.0))));
    scene.add_object(int_mesh.clone().translated(int_origin + position!(East(1.0), South(65.0), Down(15.0))));

    scene.add_object(dest_mesh.translated(look_to));
    scene
}

pub fn capsule(config: &CameraConfiguration) -> Scene {

    // Example object and texture taken from http://paulbourke.net/dataformats/obj/minobj.html
    
    // Camera
    let look_from = position!(Up(10.0), South(10.0));
    let look_to =   position!(Up(4.0));
    let camera = config.make_camera(look_to, look_from);

    // Scene
    let mut scene = Scene::new(camera, SceneSky::Black);

    // Lights
    let lamp_pos = position!(Up(20.0), East(20.0));
    let lamp_direction = position!(Origin) - lamp_pos;
    scene.add_light(DirectionalLight::with_direction(lamp_direction).with_intensity(0.5));

    // World sphere
    let world_radius = 1000.0;
    let world_mat = scene.add_material(MatLambertian::default());
    let world_tex = scene.add_texture(ColorTexture(rgb(200, 200, 200)));
    let world_pos = position!(Down(world_radius));
    scene.add_object(
        Sphere::new(world_radius, world_mat, world_tex)
            .with_origin(world_pos)
            .with_id(0)
    );
    
    // Capsule
    let mesh_builder = load_obj_builder("./raytracer-samples/meshes/capsule/capsule.obj").unwrap();
    let capsule_mesh_data = mesh_builder.build_mesh_data("default");
    let capsule_mat = scene.add_material(MatLambertian::default());
    let capsule_tex = scene.add_texture(capsule_mesh_data.texture_set);
    let capsule_origin = position!(Up(4.0));
    scene.add_object(
        MeshObject::new(&capsule_mesh_data.mesh, capsule_mat, capsule_tex)
            .with_origin(capsule_origin)
    );

    scene
}

pub fn mesh_plane(config: &CameraConfiguration) -> Scene {
    // Camera
    let look_from = position!(East(2.0));
    let look_to =   position!(Origin);
    let camera = config.make_camera(look_to, look_from);

    // Scene
    let mut scene = Scene::new(camera, SceneSky::Black);

    // Lights
    let lamp_pos = look_from;
    let lamp_direction = look_to - lamp_pos;
    scene.add_light(DirectionalLight::with_direction(lamp_direction).with_intensity(1.0));

    // Plane
    let plane_tex = scene.add_texture(load_color_map("./raytracer-samples/meshes/simple/test.bmp").unwrap());
    let plane_mat = scene.add_material(MatLambertian::default());
    let plane_origin = look_to;
    let plane_mesh_builder = load_obj_builder("./raytracer-samples/meshes/simple/plane.obj").unwrap();
    let plane_mesh_data = plane_mesh_builder.build_mesh_data("plane");
    scene.add_object(
        MeshObject::new(&plane_mesh_data.mesh, plane_mat, plane_tex)
            .with_origin(plane_origin)
            .with_id(1)
    );

    scene
}

pub fn point_cloud(config: &CameraConfiguration) -> Scene {
    const LENGTH: f32 = 100.0;

    // Camera
    let look_from = position!(East(LENGTH), South(LENGTH), Up(LENGTH));
    let look_to =   position!(Origin);
    let camera = config.make_camera(look_to, look_from);

    // Scene
    let mut scene = Scene::new(camera, SceneSky::Black);
    
    // Lights
    let lamp_pos = look_from;
    let lamp_direction = look_to - lamp_pos;
    scene.add_light(DirectionalLight::with_direction(lamp_direction).with_intensity(1.0));

    let point_mat = scene.add_material(MatLambertian::default());
    let point_radius = 0.05;

    let mut rng = create_rng_from_seed(432789012409);

    for _ in 0..1_000_000 {
        
        let a = rng.gen::<f32>();
        let b = rng.gen::<f32>();
        let c = rng.gen::<f32>();

        let x = (a * LENGTH) - (LENGTH / 2.0);
        let y = (b * LENGTH) - (LENGTH / 2.0);
        let z = (c * LENGTH) - (LENGTH / 2.0);

        let point_tex = scene.add_texture(ColorTexture(V3(a, b, c)));
        scene.add_object(Sphere::new(point_radius, point_mat, point_tex).with_origin(V3(x, y, z)))
    }

    scene
}

pub fn mega_cube(config: &CameraConfiguration) -> Scene {
    // Camera
    let look_dist = 300.0;
    let look_from = position!(Up(look_dist), South(look_dist), East(look_dist));
    let look_to =   position!(Origin);
    let camera = config.make_camera(look_to, look_from);

    // Scene
    let mut scene = Scene::new(camera, SceneSky::Black);

    // Lights
    let lamp_pos = look_from;
    let lamp_direction = look_to - lamp_pos;
    scene.add_light(DirectionalLight::with_direction(lamp_direction).with_intensity(0.9));

    let int_mat = scene.add_material(MatLambertian::default());
    let int_tex = scene.add_texture(ColorTexture(V3(1.0, 0.41, 0.70))); // #FF69B4;
    
    let range = (-100..=100).step_by(25);

    for x in range.clone() {
        for y in range.clone() {
            for z in range.clone() {
                scene.add_object(
                    Sphere::new(10.0, int_mat, int_tex)
                        .translated(V3(x as f32, y as f32, z as f32))
                );
            }
        }
    }

    scene
}

pub fn fleet(config: &CameraConfiguration) -> Scene {
    
    let dist = 100.0;

    // Camera
    let look_from = position!(Up(dist), North(dist), East(dist));
    let look_to =   position!(Origin);
    let camera = config.make_camera(look_to, look_from);

    // Scene
    let mut scene = Scene::new(camera, SceneSky::Black);

    // Lights
    scene.add_light(PointLight::with_origin(look_from).with_intensity(2000.0));
    
    let int_mesh_builder = load_obj_builder("./raytracer-samples/meshes/Interceptor-T/Heavyinterceptor.obj").unwrap();
    let int_mesh_data = int_mesh_builder.build_mesh_data("default");
    let int_mat = scene.add_material(MatLambertian::default());
    let int_tex = scene.add_texture(int_mesh_data.texture_set);
    let int_mesh = MeshObject::new(&int_mesh_data.mesh, int_mat, int_tex)
        // Interceptor model is facing +Z rotated on its side (X UP?)
        .rotated(V3::POS_Z, -deg_to_rad(90.0));
    
    let range = (-600..=0).step_by(60);

    for x in range.clone() {
        for y in range.clone() {
            for z in range.clone() {
                let origin = V3(x as f32, y as f32, z as f32);
                scene.add_object(int_mesh.clone().translated(origin));
            }
        }
    }

    scene
}

pub fn dreadnaught(config: &CameraConfiguration) -> Scene {
    
    let dist = 800.0;

    // Camera
    let look_from = position!(Up(dist), North(dist), East(dist));
    let look_to =   position!(Origin);
    let camera = config.make_camera(look_to, look_from);

    // Scene
    let mut scene = Scene::new(camera, SceneSky::Black);

    // Lights
    scene.add_light(PointLight::with_origin(look_from).with_intensity(100.0));
    
    let mesh_builder = load_obj_builder("./raytracer-samples/meshes/Dreadnaught/Dreadnaught.obj").unwrap();
    let mat = scene.add_material(MatLambertian::default());

    for name in mesh_builder.object_names() {
        let mesh_data = mesh_builder.build_mesh_data(name);
        let tex = scene.add_texture(mesh_data.texture_set);
        scene.add_object(MeshObject::new(&mesh_data.mesh, mat, tex));
    }

    scene
}