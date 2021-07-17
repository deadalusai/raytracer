#![allow(unused)]

use raytracer::types::{ V3, Ray };
use raytracer::materials::{ MatLambertian, MatDielectric, MatMetal };
use raytracer::shapes::{ Sphere };
use raytracer::viewport::{ Viewport };
use raytracer::lights::{ PointLight, DirectionalLight, LampLight };
use raytracer::implementation::{ Scene, SceneSky, Camera, Material };

use rand::{ Rng, StdRng, SeedableRng };

fn create_rng_from_seed (seed_text: &str) -> StdRng {
    let bytes: Vec<_> = seed_text.bytes().map(|b| b as usize).collect();
    StdRng::from_seed(&bytes)
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

static WORLD_ORIGIN: V3 = V3(0.0,  0.0,  0.0);

#[derive(Clone, Copy)]
enum Card {
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
    ( $move:tt($v:expr) ) => ( Card::$move($v).v3() );
    ( $move:tt($v:expr), $( $rest:tt($rest_v:expr) ),* ) => ( Card::$move($v).v3() + position!($( $rest($rest_v) ),*) );
}

//
// Sample scenes
//

// Attenuation factory

fn rgb (r: u8, g: u8, b: u8) -> V3 {
    V3(
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0
    )
}

// Random material factories

fn make_lambertian<R: Rng> (rng: &mut R) -> MatLambertian {
    let albedo = V3(
        /* r */ rng.next_f32() * rng.next_f32(),
        /* g */ rng.next_f32() * rng.next_f32(),
        /* b */ rng.next_f32() * rng.next_f32()
    );
    MatLambertian::with_albedo(albedo)
}

fn make_metal<R: Rng> (rng: &mut R) -> MatMetal {
    let albedo = V3(
        /* r */ 0.5 * (1.0 + rng.next_f32()),
        /* g */ 0.5 * (1.0 + rng.next_f32()),
        /* b */ 0.5 * (1.0 + rng.next_f32())
    );
    let fuzz = 0.5 * rng.next_f32();
    MatMetal::with_albedo(albedo).with_fuzz(fuzz)
}

fn make_glass<R: Rng> (rng: &mut R) -> MatDielectric {
    let refractive_index = 1.5;
    let albedo = V3(
        /* r */ 0.5 * (1.0 + rng.next_f32()),
        /* g */ 0.5 * (1.0 + rng.next_f32()),
        /* b */ 0.5 * (1.0 + rng.next_f32())
    );
    MatDielectric::with_albedo(albedo).with_ref_index(refractive_index)
}

//
// Scenes
//

pub fn random_sphere_scene(viewport: &Viewport, camera_aperture: f32) -> Scene {
    // Camera
    let look_from = V3(13.0, 2.0, 3.0);
    let look_to = V3(0.0, 0.0, 0.0);
    let fov = 20.0;
    let aspect_ratio = viewport.width as f32 / viewport.height as f32;
    let aperture = 0.1;
    let dist_to_focus = 10.0; // distance to look target is 13-ish

    let camera = Camera::new(look_from, look_to, fov, aspect_ratio, camera_aperture, dist_to_focus);

    // Scene
    let mut rng = create_rng_from_seed("random sphere scene");
    let mut scene = Scene::new(camera, SceneSky::Day);

    // Lights
    let lamp_origin = V3(4.0, 100.0, 4.0);
    let lamp_direction = WORLD_ORIGIN - lamp_origin;
    scene.add_light(DirectionalLight::with_origin_and_direction(lamp_origin, lamp_direction).with_intensity(0.5));

    // World sphere
    scene.add_obj(Sphere::new(V3(0.0, -1000.0, 0.0), 1000.0, MatLambertian::with_albedo(V3(0.5, 0.5, 0.5))));

    // Large metal sphere
    let lam_sphere_center = V3(-4.0, 1.0, 0.0);
    let lam_sphere_mat = MatLambertian::with_albedo(V3(0.8, 0.2, 0.1));
    scene.add_obj(Sphere::new(lam_sphere_center.clone(), 1.0, lam_sphere_mat));
    
    // Large hollow glass sphere
    let hollow_sphere_center = V3(0.0, 1.0, 0.0);
    let hollow_sphere_mat = MatDielectric::with_albedo(V3(0.95, 0.95, 0.95)).with_ref_index(1.5);
    scene.add_obj(Sphere::new(hollow_sphere_center.clone(),  1.0, hollow_sphere_mat.clone()));
    // scene.add_obj(Sphere::new(hollow_sphere_center.clone(), -0.99, hollow_sphere_mat));

    // Large mat sphere
    let metal_sphere_center = V3(4.0, 1.0, 0.0);
    let metal_sphere_mat = MatMetal::with_albedo(V3(0.8, 0.8, 0.8)).with_fuzz(0.0);
    scene.add_obj(Sphere::new(metal_sphere_center.clone(),  1.0, metal_sphere_mat));

    let sphere_centers = [lam_sphere_center, hollow_sphere_center, metal_sphere_center];

    // Small random spheres
    for a in -11..11 {
        for b in -11..11 {
            let center = V3(
                /* x */ a as f32 + 0.9 * rng.next_f32(),
                /* y */ 0.2,
                /* z */ b as f32 + 0.9 * rng.next_f32()
            );
            let radius = 0.2;

            // Only include the sphere if it's not too close to the three large spheres..
            if sphere_centers.iter().any(|&pos| (center - pos).length() < 1.5) {
                continue;
            }

            // Select a material
            let sphere =
                match rng.next_f32() {
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
    scene.add_obj(Sphere::new(position!(North(2.0)), 0.25, MatDielectric::with_albedo(rgb(128, 0,   0))));
    scene.add_obj(Sphere::new(position!(East(2.0)),  0.25, MatDielectric::with_albedo(rgb(0,   128, 0))));
    scene.add_obj(Sphere::new(position!(West(2.0)),  0.25, MatDielectric::with_albedo(rgb(0,   0,   128))));
    scene.add_obj(Sphere::new(position!(South(2.0)), 0.25, MatDielectric::with_albedo(rgb(255, 255, 255))));
}

pub fn simple_scene(viewport: &Viewport, camera_aperture: f32) -> Scene {

    // Camera
    let look_from = position!(South(6.0), East(1.5), Up(3.0));
    let look_to =   position!(Up(1.0));
    let fov = 45.0;
    let aspect_ratio = viewport.width as f32 / viewport.height as f32;
    let dist_to_focus = (look_from - look_to).length();

    let camera = Camera::new(look_from, look_to, fov, aspect_ratio, camera_aperture, dist_to_focus);

    // Scene
    let mut scene = Scene::new(camera, SceneSky::Black);

    // Lights
    let lamp_pos = position!(Up(10.0), East(4.0));
    let lamp_direction = WORLD_ORIGIN - lamp_pos;
    scene.add_light(LampLight::with_origin_and_direction(lamp_pos, lamp_direction).with_intensity(8.0).with_angle(20.0));

    let lamp_pos = position!(Up(20.0), North(4.0));
    let lamp_direction = WORLD_ORIGIN - lamp_pos;
    scene.add_light(LampLight::with_origin_and_direction(lamp_pos, lamp_direction).with_intensity(5.0).with_angle(12.0));

    add_cardinal_markers(&mut scene);

    // World sphere
    let world_mat = MatLambertian::with_albedo(rgb(255, 255, 255));
    let world_pos = position!(Down(1000.0));
    scene.add_obj(Sphere::new(world_pos, 1000.0, world_mat));

    // Plastic sphere
    let plastic_mat = MatLambertian::with_albedo(rgb(226, 226, 226));
    let plastic_pos = position!(Up(1.0));
    scene.add_obj(Sphere::new(plastic_pos, 1.0, plastic_mat));

    // Glass sphere (large)
    let glass_mat = MatDielectric::with_albedo(rgb(130, 255, 140));
    let glass_pos = position!(Up(1.0), South(2.0), East(2.0));
    scene.add_obj(Sphere::new(glass_pos.clone(), 1.0, glass_mat));
    
    // Glass sphere (small)
    let small_glass_mat = MatDielectric::with_albedo(rgb(66, 206, 245)).with_opacity(0.4);
    let small_glass_pos = lerp_v3(glass_pos, lamp_pos, 0.2); // Find a point between the lamp and the large glass sphere
    scene.add_obj(Sphere::new(small_glass_pos, 0.5, small_glass_mat));

    // Metal sphere
    let metal_mat = MatMetal::with_albedo(rgb(147, 154, 186)).with_fuzz(0.001);
    let metal_pos = position!(Up(1.0), North(2.0), West(2.0));
    scene.add_obj(Sphere::new(metal_pos, 1.0, metal_mat));


    // Small metal spheres (buried) drawn between these points
    let small_metal_mat = MatMetal::with_albedo(V3(0.8, 0.1, 0.1)).with_fuzz(0.001);
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
    let small_plastic_mat = MatDielectric::with_albedo(V3(0.1, 0.9, 0.1));
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