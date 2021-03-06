use std;

use raytracer::types::{ Rgb };
use raytracer::types::{ Vec3 };
use raytracer::types::{ Ray };
use raytracer::viewport::{ ViewChunk };

use rand::{ Rng };

// Util

const PI: f32 = std::f32::consts::PI;
const TWO_PI: f32 = 2.0 * PI;
const HALF_PI: f32 = PI / 2.0;

pub fn random_point_in_unit_sphere (rng: &mut Rng) -> Vec3 {
    //  Select a random point in a unit sphere using spherical co-ordinates.
    //  Pick
    //      - theta with range [0, 2pi)
    //      - phi with range [-pi/2, pi/2]
    //      - radius with range [0, 1]

    let theta = rng.next_f32() * TWO_PI;
    let phi = rng.next_f32() * HALF_PI * 2.0 - HALF_PI;
    let r = rng.next_f32();
    
    let x = r * theta.cos() * phi.cos();
    let y = r * phi.sin();
    let z = r * theta.sin() * phi.cos();

    Vec3::new(x, y, z)
}

// Materials

pub struct Reflect {
    pub ray: Ray,
    pub intensity: f32,
}

pub struct Refract {
    pub ray: Ray,
    pub intensity: f32,
}

pub struct MatRecord {
    pub reflection: Option<Reflect>,
    pub refraction: Option<Refract>,
    pub albedo: Vec3,
}

pub trait Material: Send + Sync {
    fn scatter (&self, ray: &Ray, hit_record: &HitRecord, rng: &mut Rng) -> MatRecord;
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
    fn get_direction_and_intensity (&self, p: Vec3) -> Option<LightRecord>;
}

// Scene

pub enum SceneSky { Day, Black }

pub struct Scene {
    camera: Camera,
    sky: SceneSky,
    lights: Vec<Box<LightSource>>,
    hitables: Vec<Box<Hitable>>,
}

impl Scene {
    pub fn new (camera: Camera, sky: SceneSky) -> Scene {
        Scene {
            camera: camera,
            sky: sky,
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

    fn hit_closest (&self, ray: &Ray, t_min: f32) -> Option<HitRecord> {
        let mut closest_hit_record = None;
        let mut closest_so_far = std::f32::INFINITY;
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

impl Camera {
    pub fn new (look_from: Vec3, look_at: Vec3, v_fov: f32, aspect_ratio: f32, lens_aperture: f32, focus_dist: f32) -> Camera {
        // NOTE: Hard code v_up as vertical for now
        let v_up = Vec3::new(0.0, 1.0, 0.0);
        let theta = v_fov * PI / 180.0;
        let half_height = (theta / 2.0).tan();
        let half_width = aspect_ratio * half_height;
        let w = (look_from - look_at).unit_vector(); // Vector from camera origin to target
        let u = Vec3::cross(v_up, w).unit_vector();  // Vector from camera origin to camera right
        let v = Vec3::cross(w, u);                   // Vector from camera origin to camera top
        let lens_radius = lens_aperture / 2.0;
        Camera {
            lower_left_corner: look_from - (u * half_width * focus_dist) - (v * half_height * focus_dist) - (w * focus_dist),
            horizontal: u * (2.0 * half_width * focus_dist),
            vertical: v * (2.0 * half_height * focus_dist),
            origin: look_from,
            u: u,
            v: v,
            lens_radius: lens_radius,
        }
    }

    pub fn get_ray (&self, s: f32, t: f32, rng: &mut Rng) -> Ray {
        let lens_deflection = random_point_in_unit_sphere(rng) * self.lens_radius;
        let offset = (self.u * lens_deflection.x) + (self.v * lens_deflection.y);
        let origin = self.origin + offset;
        let direction = self.lower_left_corner + (self.horizontal * s) + (self.vertical * t) - self.origin - offset;
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

// Sky

fn color_sky_black () -> Vec3 {
    Vec3::zero()
}

fn color_sky_day (ray: &Ray) -> Vec3 {
    let unit_direction = ray.direction.unit_vector();
    let t = 0.5 * (unit_direction.y + 1.0);
    let white = Vec3::new(1.0, 1.0, 1.0);
    let sky_blue = Vec3::new(0.5, 0.7, 1.0);
    white * (1.0 - t) + (sky_blue * t)
}

fn color_sky (ray: &Ray, scene: &Scene) -> Vec3 {
    match scene.sky {
        SceneSky::Day => color_sky_day(ray),
        SceneSky::Black => color_sky_black(),
    }
}

// Lights and shadows

// Casts a ray *back* towards a lamp, testing for possibly shadowing objects
fn cast_light_ray (hit_point: Vec3, light_record: &LightRecord, scene: &Scene, rng: &mut Rng) -> Vec3 {

    // Get the inital light value
    let mut light_color = light_record.color * light_record.intensity;
    
    // Test to see if there is any shape blocking light from this lamp by casting a ray from the shadow back to the light source
    let light_ray = Ray::new(hit_point, -light_record.direction);
                
    let mut closest_so_far = 0.0;

    // Perform hit tests until we escape
    loop {
        if let Some(shadow_hit) = scene.hit_closest(&light_ray, closest_so_far) {
            let shadow_mat = shadow_hit.material.scatter(&light_ray, &shadow_hit, rng);
            if let Some(shadow_refraction) = shadow_mat.refraction {
                // Hit transparent object
                // Hack: simulate colored shadows by taking the albedo of transparent materials.
                light_color = light_color * (shadow_mat.albedo * shadow_refraction.intensity);
                closest_so_far = shadow_hit.t;
                continue;
            }
            // Hit opaque object (in shadow)
            return Vec3::zero();
        }
        // Escaped.
        return light_color;
    }
}

/// Determines the color which the given ray resolves to.
fn cast_ray (ray: &Ray, scene: &Scene, rng: &mut Rng, max_reflections: u32) -> Vec3 {

    // Internal implementation
    fn cast_ray_recursive (ray: &Ray, scene: &Scene, rng: &mut Rng, recurse_limit: u32) -> Vec3 {

        // Exceeded our reflection limit?
        if recurse_limit == 0 {
            return color_sky(ray, scene);
        }
        
        // Hit anything in the scene?
        if let Some(hit_record) = scene.hit_closest(ray, BIAS) {
            let mat_record = hit_record.material.scatter(ray, &hit_record, rng);

            // NOTE: Move hit point slightly above p along surface normal to avoid "shadow acne"
            let hit_point = hit_record.p + (hit_record.normal * BIAS);

            // Determine color from lights in the scene.
            let mut color_from_lights = Vec3::zero();

            for light in scene.lights.iter() {
                if let Some(light_record) = light.get_direction_and_intensity(hit_point) {

                    let light_color =
                        cast_light_ray(hit_point, &light_record, scene, rng)
                            * mat_record.albedo // Material albedo
                            * max_f(0.0, Vec3::dot(hit_record.normal, -light_record.direction)); // Adjust intensity as reflection normal changes

                    color_from_lights = color_from_lights + light_color;
                }
            }

            // We may need to recurse more than once, depending on the material we hit.
            // In this case, split the recursion limit to avoid doubling our work.
            let (reflect_limit, refract_limit) = {
                let recurse_limit = recurse_limit - 1;
                match (&mat_record.reflection, &mat_record.refraction) {
                    (&Some(_), &Some(_)) => {
                        let reflect_limit = recurse_limit / 2;
                        let refract_limit = recurse_limit - reflect_limit;
                        (reflect_limit, refract_limit)
                    },
                    (&Some(_), &None) => (recurse_limit, 0),
                    (&None, &Some(_)) => (0, recurse_limit),
                    (&None, &None)    => (0, 0)
                }
            };

            // Determine color from material reflection.
            let mut color_from_reflection = Vec3::zero();
            if let Some(reflect) = mat_record.reflection {
                if reflect.intensity > 0.0 {
                    color_from_reflection = cast_ray_recursive(&reflect.ray, scene, rng, reflect_limit) * reflect.intensity;
                }
            }

            // Determine color from material refraction.
            let mut color_from_refraction = Vec3::zero();
            if let Some(refract) = mat_record.refraction {
                if refract.intensity > 0.0 {
                    color_from_refraction = cast_ray_recursive(&refract.ray, scene, rng, refract_limit) * refract.intensity;
                }
            }

            return (color_from_lights + color_from_reflection + color_from_refraction) * mat_record.albedo;
        }

        // Hit the sky instead
        color_sky(ray, scene)
    }

    cast_ray_recursive(ray, scene, rng, max_reflections).clamp()
}

pub fn cast_rays_into_scene (chunk: &mut ViewChunk, rng: &mut Rng, scene: &Scene, samples_per_pixel: u32, max_reflections: u32) {
    if samples_per_pixel == 0 {
        panic!("samples_per_pixel cannot be zero");
    }
    
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
                let ray = scene.camera.get_ray(u, v, rng);
                col = col + cast_ray(&ray, scene, rng, max_reflections);
            }
            // Find the average
            col = col / samples_per_pixel as f32;
            chunk.set_chunk_pixel(chunk_x, chunk_y, Rgb::from_vec3(&col));
        }
    }
}