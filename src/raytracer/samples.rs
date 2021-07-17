#![allow(unused)]

use raytracer::types::{ Vec3, Ray };
use raytracer::materials::{ MatLambertian, MatDielectric, MatMetal };
use raytracer::shapes::{ Sphere };
use raytracer::viewport::{ Viewport };
use raytracer::lights::{ PointLight, DirectionalLight };
use raytracer::implementation::{ Scene, SceneSky, Camera, Material };

use rand::{ Rng, StdRng, SeedableRng };

fn create_rng_from_seed (seed_text: &str) -> StdRng {
    let bytes: Vec<_> = seed_text.bytes().map(|b| b as usize).collect();
    StdRng::from_seed(&bytes)
}

//
// Sample scenes
//

// Attenuation factory

fn make_albedo (r: u8, g: u8, b: u8) -> Vec3 {
    Vec3::new(
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0
    )
}

// Random material factories

fn make_lambertian<R: Rng> (rng: &mut R) -> MatLambertian {
    let albedo = Vec3::new(
        /* r */ rng.next_f32() * rng.next_f32(),
        /* g */ rng.next_f32() * rng.next_f32(),
        /* b */ rng.next_f32() * rng.next_f32()
    );
    MatLambertian::with_albedo(albedo)
}

fn make_metal<R: Rng> (rng: &mut R) -> MatMetal {
    let albedo = Vec3::new(
        /* r */ 0.5 * (1.0 + rng.next_f32()),
        /* g */ 0.5 * (1.0 + rng.next_f32()),
        /* b */ 0.5 * (1.0 + rng.next_f32())
    );
    let fuzz = 0.5 * rng.next_f32();
    MatMetal::with_albedo(albedo).with_fuzz(fuzz)
}

fn make_glass<R: Rng> (rng: &mut R) -> MatDielectric {
    let refractive_index = 1.5;
    let albedo = Vec3::new(
        /* r */ 0.5 * (1.0 + rng.next_f32()),
        /* g */ 0.5 * (1.0 + rng.next_f32()),
        /* b */ 0.5 * (1.0 + rng.next_f32())
    );
    MatDielectric::with_albedo(albedo).with_ref_index(refractive_index)
}

//
// Scenes
//

pub fn random_sphere_scene(viewport: &Viewport) -> Scene {
    // Camera
    let look_from = Vec3::new(13.0, 2.0, 3.0);
    let look_to = Vec3::new(0.0, 0.0, 0.0);
    let fov = 20.0;
    let aspect_ratio = viewport.width as f32 / viewport.height as f32;
    let aperture = 0.1;
    let dist_to_focus = 10.0; // distance to look target is 13-ish

    let camera = Camera::new(look_from, look_to, fov, aspect_ratio, aperture, dist_to_focus);

    // Scene
    let mut rng = create_rng_from_seed("random sphere scene");
    let mut scene = Scene::new(camera, SceneSky::Day);

    // Lights
    let lamp_origin = Vec3::new(4.0, 100.0, 4.0);
    let lamp_direction = Vec3::zero() - lamp_origin;
    scene.add_light(DirectionalLight::with_origin_and_direction(lamp_origin, lamp_direction).with_intensity(0.5));

    // World sphere
    scene.add_obj(Sphere::new(Vec3::new(0.0, -1000.0, 0.0), 1000.0, MatLambertian::with_albedo(Vec3::new(0.5, 0.5, 0.5))));

    // Large metal sphere
    let lam_sphere_center = Vec3::new(-4.0, 1.0, 0.0);
    let lam_sphere_mat = MatLambertian::with_albedo(Vec3::new(0.8, 0.2, 0.1));
    scene.add_obj(Sphere::new(lam_sphere_center.clone(), 1.0, lam_sphere_mat));
    
    // Large hollow glass sphere
    let hollow_sphere_center = Vec3::new(0.0, 1.0, 0.0);
    let hollow_sphere_mat = MatDielectric::with_albedo(Vec3::new(0.95, 0.95, 0.95)).with_ref_index(1.5);
    scene.add_obj(Sphere::new(hollow_sphere_center.clone(),  1.0, hollow_sphere_mat.clone()));
    // scene.add_obj(Sphere::new(hollow_sphere_center.clone(), -0.99, hollow_sphere_mat));

    // Large mat sphere
    let metal_sphere_center = Vec3::new(4.0, 1.0, 0.0);
    let metal_sphere_mat = MatMetal::with_albedo(Vec3::new(0.8, 0.8, 0.8)).with_fuzz(0.0);
    scene.add_obj(Sphere::new(metal_sphere_center.clone(),  1.0, metal_sphere_mat));

    let sphere_centers = [lam_sphere_center, hollow_sphere_center, metal_sphere_center];

    // Small random spheres
    for a in -11..11 {
        for b in -11..11 {
            let center = Vec3::new(
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

fn interpolate_points(p1: Vec3, p2: Vec3, d: f32) -> Vec3 {
    let v_between = (p2 - p1) * d;
    p1 + v_between
}

pub fn simple_scene(viewport: &Viewport) -> Scene {
    // Camera
    let look_from = Vec3::new(6.0, 3.0, -1.5);
    let look_to = Vec3::new(0.0, 1.0, 0.0);
    let fov = 45.0;
    let aspect_ratio = viewport.width as f32 / viewport.height as f32;
    let aperture = 0.1;
    let dist_to_focus = (look_from - look_to).length();

    let camera = Camera::new(look_from, look_to, fov, aspect_ratio, aperture, dist_to_focus);

    // Scene
    let mut scene = Scene::new(camera, SceneSky::Black);

    // Lights
    let light_pos = Vec3::new(0.0, 10.0, -4.0);
    // scene.add_light(PointLight::with_origin(light_pos.clone()).with_intensity(100.0));

    let lamp_origin = Vec3::new(-30.0, 100.0, 0.0);
    let lamp_direction = Vec3::zero() - lamp_origin;
    scene.add_light(DirectionalLight::with_origin_and_direction(lamp_origin, lamp_direction).with_intensity(0.5));

    // World sphere
    let world_mat = MatLambertian::with_albedo(make_albedo(255, 255, 255));
    let world_pos = Vec3::new(0.0, -1000.0, 0.0);
    scene.add_obj(Sphere::new(world_pos, 1000.0, world_mat));

    // Plastic sphere
    let plastic_mat = MatLambertian::with_albedo(make_albedo(179, 45, 0));
    let plastic_pos = Vec3::new(2.0, 1.0, -2.0);
    scene.add_obj(Sphere::new(plastic_pos, 1.0, plastic_mat));

    // Glass sphere (large)
    let glass_mat = MatDielectric::with_albedo(make_albedo(245, 227, 66));
    let glass_pos = Vec3::new(0.0, 1.0, 0.0);
    scene.add_obj(Sphere::new(glass_pos.clone(), 1.0, glass_mat));
    // scene.add_obj(Sphere::new(Vec3::new(0.0, 1.0, 0.0), -0.8, mat_glass));
    
    // Glass sphere (small)
    let small_glass_mat = MatDielectric::with_albedo(make_albedo(66, 206, 245)).with_opacity(0.4);
    let small_glass_pos = interpolate_points(glass_pos, light_pos, 0.2); // Find a point between the lamp and the large glass sphere
    scene.add_obj(Sphere::new(small_glass_pos, 0.5, small_glass_mat));

    // Metal sphere
    let metal_mat = MatMetal::with_albedo(make_albedo(230, 230, 230)).with_fuzz(0.001);
    let metal_pos = Vec3::new(-2.0, 1.0, 2.0);
    scene.add_obj(Sphere::new(metal_pos, 1.0, metal_mat));

    scene
}