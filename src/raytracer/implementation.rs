use std;

use raytracer::types::{ Rgb };
use raytracer::types::{ Vec3, vec3_dot, vec3_cross };
use raytracer::types::{ Ray };
use raytracer::viewport::{ ViewChunk };

use rand::{ Rng, thread_rng };

// Materials

pub struct MatRecord {
    pub scattered: Ray,
    pub albedo: Vec3,
    pub attenuation: f32,
}

pub trait Material: Send + Sync {
    fn scatter (&self, ray: &Ray, hit_record: &HitRecord) -> Option<MatRecord>;
}

// Hitables

pub struct HitRecord<'mat> {
    pub t: f32,
    pub p: Vec3,
    pub normal: Vec3,
    pub material: &'mat Material,
}

pub trait Hitable: Send + Sync {
    fn hit<'a> (&'a self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'a>>;
}

// Light sources

pub struct LightRecord {
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
}

pub trait LightSource: Send + Sync {
    fn get_direction_and_intensity (&self, p: &Vec3) -> Option<LightRecord>;
}

// Scene

pub struct Scene {
    camera: Camera,
    lights: Vec<Box<LightSource>>,
    hitables: Vec<Box<Hitable>>,
}

impl Scene {
    pub fn new (camera: Camera) -> Scene {
        Scene {
            camera: camera,
            lights: vec!(),
            hitables: vec!(),
        }
    }

    pub fn add_obj<T> (&mut self, hitable: T)
        where T: Hitable + 'static
    {
        self.hitables.push(Box::new(hitable));
    }

    pub fn add_light<T> (&mut self, light: T)
        where T: LightSource + 'static
    {
        self.lights.push(Box::new(light));
    }

    fn hit_any (&self, ray: &Ray, t_min: f32) -> Option<HitRecord> {
        for hitable in self.hitables.iter() {
            if let Some(record) = hitable.hit(ray, t_min, std::f32::MAX) {
                return Some(record);
            }
        }
        None
    }

    fn hit_closest (&self, ray: &Ray, t_min: f32) -> Option<HitRecord> {
        let mut closest_hit_record = None;
        let mut closest_so_far = std::f32::MAX;
        for hitable in self.hitables.iter() {
            if let Some(record) = hitable.hit(ray, t_min, closest_so_far) {
                closest_so_far = record.t;
                closest_hit_record = Some(record);
            }
        }
        closest_hit_record
    }
}

//
// Camera
//

// NOTE:
//   Y-axis goes up
//   X-axis goes right
//   Z-axis goes towards the camera (negative into the screen)

pub struct Camera {
    lower_left_corner: Vec3,
    horizontal: Vec3,
    vertical: Vec3,
    origin: Vec3,
    u: Vec3,
    v: Vec3,
    lens_radius: f32,
}

fn random_point_in_unit_disk () -> Vec3 {
    let mut rng = thread_rng();
    loop {
        let p = Vec3::new(rng.next_f32(), rng.next_f32(), 0.0).mul_f(2.0).sub(&Vec3::new(1.0, 1.0, 0.0));
        if vec3_dot(&p, &p) < 1.0 {
            return p;
        }
    }
}

impl Camera {
    pub fn new (look_from: Vec3, look_at: Vec3, v_fov: f32, aspect_ratio: f32, aperture: f32, focus_dist: f32) -> Camera {
        // NOTE: Hard code v_up as vertical for now
        let v_up = Vec3::new(0.0, 1.0, 0.0);
        let theta = v_fov * std::f32::consts::PI / 180.0;
        let half_height = (theta / 2.0).tan();
        let half_width = aspect_ratio * half_height;
        let w = look_from.sub(&look_at).unit_vector();
        let u = vec3_cross(&v_up, &w).unit_vector();
        let v = vec3_cross(&w, &u);
        let lens_radius = aperture / 2.0;
        Camera {
            lower_left_corner: look_from.sub(&u.mul_f(half_width * focus_dist)).sub(&v.mul_f(half_height * focus_dist)).sub(&w.mul_f(focus_dist)),
            horizontal: u.mul_f(2.0 * half_width * focus_dist),
            vertical: v.mul_f(2.0 * half_height * focus_dist),
            origin: look_from,
            u: u,
            v: v,
            lens_radius: lens_radius,
        }
    }

    pub fn get_ray (&self, s: f32, t: f32) -> Ray {
        let rd = random_point_in_unit_disk().mul_f(self.lens_radius);
        let offset = self.u.mul_f(rd.x).add(&self.v.mul_f(rd.y));
        let origin = self.origin.add(&offset);
        let direction = self.lower_left_corner.add(&self.horizontal.mul_f(s)).add(&self.vertical.mul_f(t)).sub(&self.origin).sub(&offset);
        Ray::new(origin, direction)
    }
}

//
// Core raytracing routine
//

const BIAS: f32 = 0.001;

fn max_f (a: f32, b: f32) -> f32 {
    a.max(b)
}

fn color_sky (ray: &Ray) -> Vec3 {
    let unit_direction = ray.direction.unit_vector();
    let t = 0.5 * (unit_direction.y + 1.0);
    let white = Vec3::new(1.0, 1.0, 1.0);
    let sky_blue = Vec3::new(0.5, 0.7, 1.0);
    white.mul_f(1.0 - t).add(&sky_blue.mul_f(t))
}

/// Determines the color which the given ray resolves to.
fn cast_ray (ray: &Ray, scene: &Scene, max_reflections: u32) -> Vec3 {

    // Internal implementation
    fn cast_ray_recursive (ray: &Ray, scene: &Scene, reflections_remaining: u32) -> Vec3 {

        // Exceeded our reflection limit?
        if reflections_remaining == 0 { 
            return Vec3::zero();
        }
        
        // Hit anything in the scene?
        if let Some(hit_record) = scene.hit_closest(ray, BIAS) {
            if let Some(mat_record) = hit_record.material.scatter(ray, &hit_record) {

                // NOTE: Shadow origin slightly above p along surface normal to avoid "shadow acne"
                let shadow_origin = hit_record.p.add(&hit_record.normal.mul_f(BIAS));

                // Determine color from lights in the scene
                let mut color_from_lights = Vec3::zero();

                for light in scene.lights.iter() {
                    if let Some(light_record) = light.get_direction_and_intensity(&shadow_origin) {
                        // color_from_light = light color * light intensity * max(0.0, dot(hit normal, inverse light direction))
                        let color_from_light = light_record.color.mul_f(light_record.intensity).mul_f(max_f(0.0, vec3_dot(&hit_record.normal, &light_record.direction.negate())));
                        // Test to see if there is any shape blocking light from this lamp by casting a ray from the shadow back to the light source
                        // TODO: Allow semi-opaque materials to attenuate, color and refract (?) light...
                        let shadow_ray = Ray::new(shadow_origin.clone(), light_record.direction.negate());
                        let is_shadowed = scene.hit_any(&shadow_ray, BIAS).is_some();
                        if !is_shadowed {
                            // multiply by material albedo
                            color_from_lights = color_from_lights.add(&mat_record.albedo.mul(&color_from_light));
                        }
                    }
                }

                // Determine color from material scattering, and attenuate
                // NOTE: no need to recurse if attenuation is high enough...
                let mut color_from_reflection =
                    if mat_record.attenuation >= 1.0 {
                        Vec3::zero()
                    }
                    else {
                        let color_from_reflection = cast_ray_recursive(&mat_record.scattered, scene, reflections_remaining - 1).mul_f(1.0 - mat_record.attenuation);
                        // multiply by material albedo
                        mat_record.albedo.mul(&color_from_reflection)
                    };

                // Apply the inverse attenuation for color taken from light
                color_from_lights = color_from_lights.mul_f(mat_record.attenuation);

                return color_from_reflection.add(&color_from_lights);
            }
        }

        // Hit the sky instead
        color_sky(ray)
    }

    cast_ray_recursive(ray, scene, max_reflections).clamp()
}

pub fn cast_rays_into_scene (chunk: &mut ViewChunk, scene: &Scene, samples_per_pixel: u32, max_reflections: u32) {
    if samples_per_pixel == 0 {
        panic!("samples_per_pixel cannot be zero");
    }

    let mut rng = thread_rng();
    
    // For each x, y coordinate in this view chunk
    for chunk_y in 0..chunk.height {
        for chunk_x in 0..chunk.width {
            // Convert to view-relative coordinates
            let (view_x, view_y) = chunk.get_view_relative_coords(chunk_x, chunk_y);
            // Implement anti-aliasing by taking the average color of random rays cast around these x, y coordinates.
            let mut col = Vec3::new(0.0, 0.0, 0.0);
            for _ in 0..samples_per_pixel {
                // When taking more than one sample (for anti-aliasing), randomize the x, y coordinates a little
                let (rand_x, rand_y) = match samples_per_pixel {
                    1 => (0.0, 0.0),
                    _ => (rng.next_f32(), rng.next_f32())
                };

                // NOTE:
                // View coordinates are from upper left corner, but World coordinates are from lower left corner. 
                // Need to convert coordinate systems with (height - y)
                let u = (view_x as f32 + rand_x) / chunk.viewport.width as f32;
                let v = ((chunk.viewport.height - view_y) as f32 + rand_y) / chunk.viewport.height as f32;

                // Cast a ray, and determine the color
                let ray = scene.camera.get_ray(u, v);
                col = col.add(&cast_ray(&ray, &scene, max_reflections));
            }
            // Find the average
            col = col.div_f(samples_per_pixel as f32);

            chunk.set_chunk_pixel(chunk_x, chunk_y, Rgb::from_vec3(&col));
        }
    }
}