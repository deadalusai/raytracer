
use raytracer::types::{ Vec3 };
use raytracer::materials::{ MatLambertian, MatDielectric, MatMetal };
use raytracer::shapes::{ Sphere };
use raytracer::implementation::{ Scene, Viewport, Camera, Material };

use rand::{ Rng, thread_rng };

//
// Sample scenes
//

pub fn random_shpere_scene (viewport: &Viewport) -> Scene {
    // Camera
    let look_from = Vec3::new(13.0, 2.0, 3.0);
    let look_to = Vec3::new(0.0, 0.0, 0.0);
    let fov = 20.0;
    let aspect_ratio = viewport.width as f32 / viewport.height as f32;
    let aperture = 0.1;
    let dist_to_focus = 10.0;

    let camera = Camera::new(look_from, look_to, fov, aspect_ratio, aperture, dist_to_focus);

    // Scene
    let mut rng = thread_rng();
    let mut scene = Scene::new(camera);

    // World sphere
    scene.add_thing(Sphere::new(Vec3::new(0.0, -1000.0, 0.0), 1000.0, MatLambertian::with_albedo(Vec3::new(0.5, 0.5, 0.5))));

    // Small random spheres
    for a in -11..11 {
        for b in -11..11 {
            let center = Vec3::new(a as f32 + 0.9 * rng.next_f32(), 0.2, b as f32 + 0.9 * rng.next_f32());
            if center.sub(&Vec3::new(4.0, 0.2, 0.0)).length() > 0.9 {
                let material: Box<Material> =
                    match rng.next_f32() {
                        v if v < 0.8 => {
                            // Diffuse
                            let albedo = Vec3::new(
                                /* r */ rng.next_f32() * rng.next_f32(),
                                /* g */ rng.next_f32() * rng.next_f32(),
                                /* b */ rng.next_f32() * rng.next_f32()
                            );
                            MatLambertian::with_albedo(albedo)
                        },
                        v if v < 0.95 => {
                            // Metal
                            let albedo = Vec3::new(
                                /* r */ 0.5 * (1.0 + rng.next_f32()),
                                /* g */ 0.5 * (1.0 + rng.next_f32()),
                                /* b */ 0.5 * (1.0 + rng.next_f32())
                            );
                            let fuzz = 0.5 * rng.next_f32();
                            MatMetal::with_albedo_and_fuzz(albedo, fuzz)
                        },
                        _ => {
                            // Glass
                            let refractive_index = 1.5;
                            MatDielectric::with_refractive_index(refractive_index)
                        }
                    };

                scene.add_thing(Sphere::new(center, 0.2, material));
            }
        }
    }

    // Large fixed spheres
    scene.add_thing(Sphere::new(Vec3::new(-4.0, 1.0, 0.0), 1.0, MatLambertian::with_albedo(Vec3::new(0.8, 0.2, 0.1))));
    scene.add_thing(Sphere::new(Vec3::new(0.0, 1.0, 0.0),  1.0, MatDielectric::with_refractive_index(1.5)));
    scene.add_thing(Sphere::new(Vec3::new(4.0, 1.0, 0.0),  1.0, MatMetal::with_albedo_and_fuzz(Vec3::new(0.8, 0.8, 0.8), 0.0)));

    scene
}