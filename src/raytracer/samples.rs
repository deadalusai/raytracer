#![allow(unused)]

use raytracer::types::{ Vec3, Ray };
use raytracer::materials::{ MatLambertian, MatDielectric, MatMetal };
use raytracer::shapes::{ Sphere };
use raytracer::viewport::{ Viewport };
use raytracer::lights::{ PointLight };
use raytracer::implementation::{ Scene, Camera, Material };

use rand::{ Rng, thread_rng };

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

// Skybox functions

/// Returns a sky color gradient based on the vertical element of the ray
fn background_sky (ray: &Ray) -> Vec3 {
    let unit_direction = ray.direction.unit_vector();
    let t = 0.5 * (unit_direction.y + 1.0);
    let white = Vec3::new(1.0, 1.0, 1.0);
    let sky_blue = Vec3::new(0.5, 0.7, 1.0);
    white.mul_f(1.0 - t).add(&sky_blue.mul_f(t))
}

/// Returns black
fn background_black (ray: &Ray) -> Vec3 {
    Vec3::zero()
}

//
// Scenes
//

pub fn random_sphere_scene (viewport: &Viewport) -> Scene {
    // Camera
    let look_from = Vec3::new(13.0, 2.0, 3.0);
    let look_to = Vec3::new(0.0, 0.0, 0.0);
    let fov = 20.0;
    let aspect_ratio = viewport.width as f32 / viewport.height as f32;
    let aperture = 0.1;
    let dist_to_focus = 10.0; // distance to look target is 13-ish

    let camera = Camera::new(look_from, look_to, fov, aspect_ratio, aperture, dist_to_focus);

    // Scene
    let mut rng = thread_rng();
    let mut scene = Scene::new(camera, background_sky);

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
    scene.add_obj(Sphere::new(hollow_sphere_center.clone(), -0.99, hollow_sphere_mat));

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
            if sphere_centers.iter().any(|pos| center.sub(pos).length() < 1.5) {
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

pub fn simple_scene (viewport: &Viewport) -> Scene {
    // Camera
    let look_from = Vec3::new(5.0, 3.0, 0.0);
    let look_to = Vec3::new(0.0, 1.5, 0.0);
    let fov = 90.0;
    let aspect_ratio = viewport.width as f32 / viewport.height as f32;
    let aperture = 0.1;
    let dist_to_focus = look_from.sub(&look_to).length();

    let camera = Camera::new(look_from, look_to, fov, aspect_ratio, aperture, dist_to_focus);

    // Scene
    let mut rng = thread_rng();
    let mut scene = Scene::new(camera, background_black);

    // Lights
    // let red_color = Vec3::new(1.0, 0.0, 0.0);
    // let blue_color = Vec3::new(0.0, 0.0, 1.0);
    // scene.add_light(PointLight::new(Vec3::new(0.0, 10.0, 8.0), red_color, 100.0));
    // scene.add_light(PointLight::new(Vec3::new(0.0, 10.0, -8.0), blue_color, 100.0));
    
    scene.add_light(PointLight::new(Vec3::new(0.0, 10.0, -4.0), Vec3::new(1.0, 1.0, 1.0), 100.0));

    // World sphere
    scene.add_obj(Sphere::new(Vec3::new(0.0, -1000.0, 0.0), 1000.0, MatLambertian::with_albedo(make_albedo(30, 30, 30))));

    scene.add_obj(Sphere::new(Vec3::new(1.0, 1.5, -1.0), 1.0, make_lambertian(&mut rng).with_attenuation(0.9999)));
    scene.add_obj(Sphere::new(Vec3::new(-1.0, 1.5, 1.0), 1.0, make_metal(&mut rng).with_fuzz(0.1).with_attenuation(0.0001)));

    scene
}