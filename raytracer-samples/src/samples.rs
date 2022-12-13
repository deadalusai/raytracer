#![allow(unused)]

use raytracer_impl::texture::{ ColorTexture, CheckerTexture, TestTexture };
use raytracer_impl::types::{ V3, Ray };
use raytracer_impl::materials::{ MatLambertian, MatDielectric, MatMetal };
use raytracer_impl::shapes::{ Sphere, Plane, Mesh, MeshFace };
use raytracer_impl::viewport::{ Viewport };
use raytracer_impl::lights::{ PointLight, DirectionalLight, LampLight };
use raytracer_impl::implementation::{ Scene, SceneSky, Camera, Material };
use raytracer_impl::obj_data::{ ObjMeshBuilder };
use crate::texture_loader::{ load_bitmap_from_bytes };

use rand::{ Rng, SeedableRng, rngs::StdRng };

fn create_rng_from_seed(a: u128, b: u128) -> StdRng {
    
    fn set_bytes(bytes: &mut [u8], val: u128) {
        for offset in 0..16 {
            let shift = (15 - offset) * 8;
            bytes[offset] = ((val >> shift) & 0xff) as u8
        }
    }

    let mut seed = <StdRng as SeedableRng>::Seed::default();

    set_bytes(&mut seed[0..16], a);
    set_bytes(&mut seed[16..32], b);
    
    StdRng::from_seed(seed)
}

//
// Configuration
//

pub struct CameraConfiguration {
    pub width: f32,
    pub height: f32,
    pub aperture: f32,
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
        fn deg_to_rad(deg: f32) -> f32 {
            (deg / 180.0) * std::f32::consts::PI
        }

        let look_from = {
            // Translate into rotation space
            let p = default_look_from - look_to;

            // The vertical axis (to rotate about horizontally)
            let v_axis = V3(0.0, 1.0, 0.0);
            let p = p.rotate_about_axis(v_axis, deg_to_rad(self.angle_adjust_h));
            
            // The horizontal axis (to rotate about vertically)
            let w = (V3::zero() - p).unit();            // Vector to origin 
            let h_axis = V3::cross(v_axis, w).unit();  // Vector to camera right
            let p = p.rotate_about_axis(h_axis, deg_to_rad(self.angle_adjust_v));

            // Translate into world space
            p + look_to
        };
        let dist_to_focus = (look_from - look_to).length() + self.focus_dist_adjust;
        
        Camera::new(look_from, look_to, self.fov, self.aspect_ratio(), self.aperture, dist_to_focus)
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

// Random material factories

fn make_lambertian<R: Rng> (rng: &mut R) -> MatLambertian<ColorTexture> {
    let albedo = V3(
        /* r */ rng.gen::<f32>() * rng.gen::<f32>(),
        /* g */ rng.gen::<f32>() * rng.gen::<f32>(),
        /* b */ rng.gen::<f32>() * rng.gen::<f32>()
    );
    MatLambertian::with_texture(ColorTexture(albedo))
}

fn make_metal<R: Rng> (rng: &mut R) -> MatMetal<ColorTexture> {
    let color = V3(
        /* r */ 0.5 * (1.0 + rng.gen::<f32>()),
        /* g */ 0.5 * (1.0 + rng.gen::<f32>()),
        /* b */ 0.5 * (1.0 + rng.gen::<f32>())
    );
    let fuzz = 0.5 * rng.gen::<f32>();
    MatMetal::with_texture(ColorTexture(color)).with_fuzz(fuzz)
}

fn make_glass<R: Rng> (rng: &mut R) -> MatDielectric<ColorTexture> {
    let refractive_index = 1.5;
    let color = V3(
        /* r */ 0.5 * (1.0 + rng.gen::<f32>()),
        /* g */ 0.5 * (1.0 + rng.gen::<f32>()),
        /* b */ 0.5 * (1.0 + rng.gen::<f32>())
    );
    MatDielectric::with_texture(ColorTexture(color)).with_ref_index(refractive_index)
}

//
// Scenes
//

pub fn random_sphere_scene(config: &CameraConfiguration) -> Scene {
    // Camera
    let look_from = V3(13.0, 2.0, 3.0);
    let look_to = V3(0.0, 0.0, 0.0);
    let camera = config.make_camera(look_to, look_from);

    // Scene
    let mut rng = create_rng_from_seed(1, 1);
    let mut scene = Scene::new(camera, SceneSky::Day);

    // Lights
    let lamp_direction = position!(Origin) - V3(4.0, 100.0, 4.0);
    scene.add_light(DirectionalLight::with_direction(lamp_direction).with_intensity(0.5));

    // World sphere
    let world_texture = CheckerTexture::new(
        10.0,
        ColorTexture(V3(0.4, 0.5, 0.4)),
        ColorTexture(V3(0.9, 0.8, 0.9))
    );

    scene.add_obj(Sphere::new(V3(0.0, -1000.0, 0.0), 1000.0, MatLambertian::with_texture(world_texture)));

    // Large metal sphere
    let lam_sphere_center = V3(-4.0, 1.0, 0.0);
    let lam_sphere_mat = MatLambertian::with_texture(ColorTexture(V3(0.8, 0.2, 0.1)));
    scene.add_obj(Sphere::new(lam_sphere_center.clone(), 1.0, lam_sphere_mat));
    
    // Large hollow glass sphere
    let hollow_sphere_center = V3(0.0, 1.0, 0.0);
    let hollow_sphere_mat = MatDielectric::with_texture(ColorTexture(V3(0.95, 0.95, 0.95))).with_ref_index(1.5);
    scene.add_obj(Sphere::new(hollow_sphere_center.clone(),  1.0, hollow_sphere_mat.clone()));
    // scene.add_obj(Sphere::new(hollow_sphere_center.clone(), -0.99, hollow_sphere_mat));

    // Large mat sphere
    let metal_sphere_center = V3(4.0, 1.0, 0.0);
    let metal_sphere_mat = MatMetal::with_texture(ColorTexture(V3(0.8, 0.8, 0.8))).with_fuzz(0.0);
    scene.add_obj(Sphere::new(metal_sphere_center.clone(), 1.0, metal_sphere_mat));

    let sphere_centers = [lam_sphere_center, hollow_sphere_center, metal_sphere_center];

    // Small random spheres
    for a in -11..11 {
        for b in -11..11 {
            let center = V3(
                /* x */ a as f32 + 0.9 * rng.gen::<f32>(),
                /* y */ 0.2,
                /* z */ b as f32 + 0.9 * rng.gen::<f32>()
            );
            let radius = 0.2;

            // Only include the sphere if it's not too close to the three large spheres..
            if sphere_centers.iter().any(|&pos| (center - pos).length() < 1.5) {
                continue;
            }

            // Select a material
            let sphere =
                match rng.gen::<f32>() {
                    v if v < 0.8  => Sphere::new(center, radius, make_lambertian(&mut rng)),
                    v if v < 0.95 => Sphere::new(center, radius, make_metal(&mut rng)),
                    _             => Sphere::new(center, radius, make_glass(&mut rng))
                };

            scene.add_obj(sphere);
        }
    }

    scene
}

fn add_cardinal_markers(scene: &mut Scene) {
    // Direction makers
    scene.add_obj(Sphere::new(position!(North(2.0)), 0.25, MatLambertian::with_texture(ColorTexture(rgb(128, 0,   0)))));
    scene.add_obj(Sphere::new(position!(East(2.0)),  0.25, MatLambertian::with_texture(ColorTexture(rgb(0,   128, 0)))));
    scene.add_obj(Sphere::new(position!(West(2.0)),  0.25, MatLambertian::with_texture(ColorTexture(rgb(0,   0,   128)))));
    scene.add_obj(Sphere::new(position!(South(2.0)), 0.25, MatLambertian::with_texture(ColorTexture(rgb(255, 255, 255)))));
}

fn add_coordinates_marker(scene: &mut Scene) {
    // Direction makers
    scene.add_obj(Sphere::new(V3(1.0, 0.0, 0.0), 0.05, MatLambertian::with_texture(ColorTexture(rgb(128, 0,   0)))));
    scene.add_obj(Sphere::new(V3(0.0, 1.0, 0.0), 0.05, MatLambertian::with_texture(ColorTexture(rgb(0,   128, 0)))));
    scene.add_obj(Sphere::new(V3(0.0, 0.0, 1.0), 0.05, MatLambertian::with_texture(ColorTexture(rgb(0,   0,   128)))));
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
    let world_mat = MatLambertian::with_texture(ColorTexture(rgb(200, 200, 200)));
    let world_pos = position!(Down(1000.0));
    scene.add_obj(Sphere::new(world_pos, 1000.0, world_mat));

    // Wall
    let wall_mat = MatLambertian::with_texture(ColorTexture(rgb(200, 200, 200))).with_reflectivity(1.0);
    let wall_pos = position!(North(4.5));
    let wall_facing = wall_pos - position!(Origin);
    scene.add_obj(Plane::new(wall_pos, wall_facing, wall_mat));

    // Plastic sphere
    let plastic_mat = MatLambertian::with_texture(ColorTexture(rgb(226, 226, 226)));
    let plastic_pos = position!(Up(1.0));
    scene.add_obj(Sphere::new(plastic_pos, 1.0, plastic_mat));

    // Glass sphere (large)
    let glass_mat = MatDielectric::with_texture(ColorTexture(rgb(130, 255, 140)));
    let glass_pos = position!(Up(1.0), South(2.0), East(2.0));
    scene.add_obj(Sphere::new(glass_pos.clone(), 1.0, glass_mat));
    
    // Glass sphere (small)
    let small_glass_mat = MatDielectric::with_texture(ColorTexture(rgb(66, 206, 245))).with_opacity(0.01).with_reflectivity(0.98);
    let small_glass_pos = lerp_v3(plastic_pos, lamp_pos, 0.2); // Find a point between the lamp and the plastic sphere
    scene.add_obj(Sphere::new(small_glass_pos, 0.5, small_glass_mat));

    // Metal sphere
    let metal_mat = MatMetal::with_texture(ColorTexture(rgb(147, 154, 186))).with_fuzz(0.001).with_reflectivity(0.91);
    let metal_pos = position!(Up(1.0), North(2.0), West(2.0));
    scene.add_obj(Sphere::new(metal_pos, 1.0, metal_mat).with_id(1));


    // Small metal spheres (buried) drawn between these points
    let small_metal_mat = MatMetal::with_texture(ColorTexture(V3(0.8, 0.1, 0.1))).with_fuzz(0.01).with_reflectivity(0.4);
    let small_metal_sphere_count = 6;
    let small_metal_start_pos = position!(West(3.5), North(1.0));
    let small_metal_end_pos = position!(West(2.5), South(3.5));
    for i in 0..small_metal_sphere_count {
        let t = i as f32 / small_metal_sphere_count as f32;
        let small_metal_pos = lerp_v3(small_metal_start_pos, small_metal_end_pos, ease_out(t, 2.0));
        let small_metal_radius = lerp_f32(0.5, 0.05, ease_out(t, 2.0));
        scene.add_obj(Sphere::new(small_metal_pos, small_metal_radius, small_metal_mat.clone()));
    }

    // Small plastic spheres (buried) drawn between these points
    let small_plastic_mat = MatDielectric::with_texture(ColorTexture(V3(0.1, 0.9, 0.1)));
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
        scene.add_obj(Sphere::new(small_plastic_pos, small_plastic_radius, small_plastic_mat.clone()));
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
    let world_mat = MatLambertian::with_texture(ColorTexture(rgb(255, 255, 255))).with_reflectivity(0.01);
    let world_pos = position!(Down(1000.0));
    scene.add_obj(Sphere::new(world_pos, 1000.0, world_mat));

    let plane_mat = MatMetal::with_texture(ColorTexture(rgb(240, 240, 240))).with_reflectivity(0.8).with_fuzz(0.02);
    let plane_pos = position!(West(1.0));
    let plane_normal = position!(Origin) - plane_pos; // normal facing world origin
    scene.add_obj(Plane::new(plane_pos, plane_normal, plane_mat));

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
    let world_mat = MatLambertian::with_texture(ColorTexture(rgb(255, 255, 255))).with_reflectivity(0.01);
    let world_pos = position!(Down(1000.0));
    scene.add_obj(Sphere::new(world_pos, 1000.0, world_mat));

    let cardinals = [
        position!(North(3.0)),
        position!(South(3.0)),
        position!(East(3.0)),
        position!(West(3.0))
    ];
    for plane_origin in cardinals {
        let plane_mat = MatMetal::with_texture(ColorTexture(V3::one())).with_reflectivity(0.98).with_fuzz(0.01);
        let plane_normal = position!(Origin) - plane_origin; // normal facing world origin
        scene.add_obj(
            Plane::new(plane_origin, plane_normal, plane_mat)
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
    let world_mat = MatLambertian::with_texture(ColorTexture(rgb(200, 200, 200)));
    let world_pos = position!(Down(1000.0));
    scene.add_obj(Sphere::new(world_pos, 1000.0, world_mat));

    // Triangle
    let tri_pos = position!(Origin);
    let tri_mat = MatLambertian::with_texture(ColorTexture(rgb(200, 100, 80))).with_reflectivity(0.0);
    let tri_vertices = MeshFace::from_abc(
        position!(Up(0.3), South(1.0)),
        position!(Up(0.6), East(1.0)),
        position!(Up(0.8), West(1.0))
    );
    scene.add_obj(Mesh::new(tri_pos, vec![tri_vertices], tri_mat));

    let tri_pos = position!(Up(1.0));
    let tri_mat = MatLambertian::with_texture(ColorTexture(rgb(100, 100, 200))).with_reflectivity(0.0);
    let tri_vertices = MeshFace::from_abc(
        position!(Up(0.4), North(1.0)),
        position!(Up(0.8), South(1.0)),
        position!(Up(0.6), East(1.0))
    );
    scene.add_obj(Mesh::new(tri_pos, vec![tri_vertices], tri_mat));

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
    let world_mat = MatLambertian::with_texture(ColorTexture(rgb(200, 200, 200)));
    let world_pos = position!(Down(1000.0));
    scene.add_obj(Sphere::new(world_pos, 1000.0, world_mat).with_id(0));

    let mut mesh_builder = ObjMeshBuilder::default();
    mesh_builder.load_objects_from_string(include_str!("../meshes/cube.obj"));
    mesh_builder.load_objects_from_string(include_str!("../meshes/thing.obj"));
    mesh_builder.load_objects_from_string(include_str!("../meshes/suzanne.obj"));

    // Cube
    let cube_mat = MatLambertian::with_texture(ColorTexture(rgb(36, 193, 89))).with_reflectivity(0.0);
    let cube_origin = position!(South(1.5), West(1.5));
    let (cube_faces, _) = mesh_builder.build_mesh_and_materials("Cube");
    scene.add_obj(Mesh::new(cube_origin, cube_faces, cube_mat).with_id(1));

    // Thing
    let thing_mat = MatMetal::with_texture(ColorTexture(rgb(89, 172, 255))).with_reflectivity(0.8).with_fuzz(0.02);
    let thing_origin = position!(North(1.5), East(1.5));
    let (thing_faces, _) = mesh_builder.build_mesh_and_materials("Thing");
    scene.add_obj(Mesh::new(thing_origin, thing_faces, thing_mat).with_id(2));

    // Suzanne
    let suz_mat = MatDielectric::with_texture(ColorTexture(rgb(255, 137, 58))).with_opacity(0.2).with_ref_index(0.8).with_reflectivity(0.0);
    let suz_origin = position!(Origin);
    let (suz_faces, _) = mesh_builder.build_mesh_and_materials("Suzanne");
    scene.add_obj(Mesh::new(suz_origin, suz_faces, suz_mat).with_id(3));

    scene
}

pub fn interceptor(config: &CameraConfiguration) -> Scene {
    
    // Camera
    let look_from = position!(Up(18.0), South(26.0), East(26.0));
    let look_to =   position!(Up(4.0), East(1.0));
    let camera = config.make_camera(look_to, look_from);

    // Scene
    let mut scene = Scene::new(camera, SceneSky::Black);

    // Lights
    let lamp_pos = position!(Up(20.0), East(20.0));
    let lamp_direction = position!(Origin) - lamp_pos;
    scene.add_light(DirectionalLight::with_direction(lamp_direction).with_intensity(0.9));

    // World sphere
    let world_radius = 1000.0;
    let world_mat = MatLambertian::with_texture(ColorTexture(rgb(200, 200, 200)));
    let world_pos = position!(Down(world_radius));
    scene.add_obj(Sphere::new(world_pos, world_radius, world_mat).with_id(0));

    let mut mesh_builder = ObjMeshBuilder::default();
    mesh_builder.load_objects_from_string(include_str!("../meshes/Interceptor-T/Heavyinterceptor.obj"));
    mesh_builder.load_materials_from_string(include_str!("../meshes/Interceptor-T/Heavyinterceptor.mtl"));
    mesh_builder.load_objects_from_string(include_str!("../meshes/suzanne.obj"));
    mesh_builder.add_color_map("engine_back.bmp", load_bitmap_from_bytes(include_bytes!("../meshes/Interceptor-T/engine_back.bmp")));
    mesh_builder.add_color_map("intake_front.bmp", load_bitmap_from_bytes(include_bytes!("../meshes/Interceptor-T/intake_front.bmp")));
    mesh_builder.add_color_map("page0.bmp", load_bitmap_from_bytes(include_bytes!("../meshes/Interceptor-T/page0.bmp")));
    mesh_builder.add_color_map("page1.bmp", load_bitmap_from_bytes(include_bytes!("../meshes/Interceptor-T/page1.bmp")));
    mesh_builder.add_color_map("page2.bmp", load_bitmap_from_bytes(include_bytes!("../meshes/Interceptor-T/page2.bmp")));
    mesh_builder.add_color_map("Rwingbottem.bmp", load_bitmap_from_bytes(include_bytes!("../meshes/Interceptor-T/Rwingbottem.bmp")));
    mesh_builder.add_color_map("rwinginside.bmp", load_bitmap_from_bytes(include_bytes!("../meshes/Interceptor-T/rwinginside.bmp")));
    mesh_builder.add_color_map("topfin_sides.bmp", load_bitmap_from_bytes(include_bytes!("../meshes/Interceptor-T/topfin_sides.bmp")));

    // Interceptor
    let int_tex = load_bitmap_from_bytes(include_bytes!("../textures/test.bmp"));
    let int_origin = position!(Up(4.0));
    let (int_faces, int_materials) = mesh_builder.build_mesh_and_materials("default");
    let int_mat = MatLambertian::with_texture(int_materials);
    scene.add_obj(Mesh::new(int_origin, int_faces, int_mat));

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
    let world_mat = MatLambertian::with_texture(ColorTexture(rgb(200, 200, 200)));
    let world_pos = position!(Down(world_radius));
    scene.add_obj(Sphere::new(world_pos, world_radius, world_mat).with_id(0));
    
    // Capsule
    let mut mesh_builder = ObjMeshBuilder::default();
    mesh_builder.load_objects_from_string(include_str!("../meshes/capsule.obj"));
    mesh_builder.load_materials_from_string(include_str!("../meshes/capsule.mtl"));
    mesh_builder.add_color_map("capsule.bmp", load_bitmap_from_bytes(include_bytes!("../textures/capsule.bmp")));

    let (capsule_faces, capsule_materials) = mesh_builder.build_mesh_and_materials("default");
    let capsule_mat = MatLambertian::with_texture(capsule_materials);
    let capsule_origin = position!(Up(4.0));
    scene.add_obj(Mesh::new(capsule_origin, capsule_faces, capsule_mat));

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

    let mut mesh_builder = ObjMeshBuilder::default();
    mesh_builder.load_objects_from_string(include_str!("../meshes/plane.obj"));

    let plane_color_map = load_bitmap_from_bytes(include_bytes!("../textures/test.bmp"));

    // Plane
    let plane_mat = MatLambertian::with_texture(plane_color_map);
    let plane_origin = look_to;
    let (plane_faces, _) = mesh_builder.build_mesh_and_materials("plane");
    scene.add_obj(Mesh::new(plane_origin, plane_faces, plane_mat).with_id(1));

    scene
}