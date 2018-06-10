use std;

use raytracer::types::{ Rgb };
use raytracer::types::{ Vec3, vec3_dot, vec3_cross };
use raytracer::types::{ Ray };

use rand::{ Rng, thread_rng };

// Materials

pub struct MatRecord {
    pub attenuation: Vec3,
    pub scattered: Ray,
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

// Scene

pub struct Scene {
    camera: Camera,
    things: Vec<Box<Hitable>>,
}

impl Scene {
    pub fn new (camera: Camera) -> Scene {
        Scene { camera: camera, things: vec!() }
    }

    pub fn add_thing<T> (&mut self, hitable: T)
        where T: Hitable + 'static
    {
        self.things.push(Box::new(hitable));
    }
}

impl Hitable for Scene {
    fn hit (&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        let mut closest_hit_record = None;
        let mut closest_so_far = t_max;
        for hitable in self.things.iter() {
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
// View tracking and chunk primitives
//

#[derive(Debug)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

impl Viewport {
    pub fn new (width: u32, height: u32) -> Viewport {
        Viewport { width: width, height: height }
    }

    pub fn iter_view_chunks (&self, h_count: u32, v_count: u32) -> impl Iterator<Item=ViewChunk> {
        let view_width = self.width;
        let view_height = self.height;
        let chunk_width = view_width / h_count;
        let chunk_height = view_height / v_count;
        (0..v_count)
            .flat_map(move |y| (0..h_count).map(move |x| (x, y)))
            .enumerate()
            .map(move |(id, (x, y))| {
                let top_left_x = x * chunk_width;
                let top_left_y = y * chunk_height;
                ViewChunk {
                    id: id as u32,
                    view_width: view_width,
                    view_height: view_height,
                    chunk_top_left: (top_left_x, top_left_y),
                    width: chunk_width,
                    height: chunk_height,
                    data: vec!(Rgb::new(0, 0, 0); chunk_width as usize * chunk_height as usize)
                }
            })
    }
}

pub struct ViewChunk {
    pub id: u32,

    view_width: u32,
    view_height: u32,
    chunk_top_left: (u32, u32),

    pub width: u32,
    pub height: u32,
    
    data: Vec<Rgb>,
}

impl ViewChunk {
    /// Sets a pixel using chunk-relative co-ordinates
    pub fn set_chunk_pixel (&mut self, chunk_x: u32, chunk_y: u32, value: Rgb) {
        let pos = (chunk_y * self.width + chunk_x) as usize;
        self.data[pos] = value;
    }

    /// Gets a pixel using view-relative co-ordinates
    pub fn get_chunk_pixel (&self, chunk_x: u32, chunk_y: u32) -> &Rgb {
        let pos = (chunk_y * self.width + chunk_x) as usize;
        &self.data[pos]
    }

    /// Gets a pixel using view-relative co-ordinates
    pub fn get_view_relative_coords (&self, chunk_x: u32, chunk_y: u32) -> (u32, u32) {
        // Convert to chunk-relative coords
        let view_x = self.chunk_top_left.0 + chunk_x;
        let view_y = self.chunk_top_left.1 + chunk_y;
        (view_x, view_y)
    }
}

//
// Core raytracing routine
//

/// Determines the color which the given ray resolves to.
fn cast_ray (ray: &Ray, world: &Scene) -> Vec3 {

    // Returns a sky color gradient based on the vertical element of the ray
    fn color_sky (ray: &Ray) -> Vec3 {
        let unit_direction = ray.direction.unit_vector();
        let t = 0.5 * (unit_direction.y + 1.0);
        let white = Vec3::new(1.0, 1.0, 1.0);
        let sky_blue = Vec3::new(0.5, 0.7, 1.0);
        white.mul_f(1.0 - t).add(&sky_blue.mul_f(t))
    }

    // Internal implementation
    fn color_internal (ray: &Ray, scene: &Scene, depth: i32) -> Vec3 {
        // Hit the world?
        if depth < 50 {
            if let Some(hit_record) = scene.hit(ray, 0.001, std::f32::MAX) {
                if let Some(mat) = hit_record.material.scatter(ray, &hit_record) {
                    return color_internal(&mat.scattered, scene, depth + 1).mul(&mat.attenuation);
                }
            }
        }

        // Hit the sky instead...
        color_sky(ray)
    }

    color_internal(ray, world, 0)
}

pub fn cast_rays_into_scene (chunk: &mut ViewChunk, scene: &Scene, samples_per_pixel: u32) {
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
                let u = (view_x as f32 + rand_x) / chunk.view_width as f32;
                let v = ((chunk.view_height - view_y) as f32 + rand_y) / chunk.view_height as f32;

                // Cast a ray, and determine the color
                let ray = scene.camera.get_ray(u, v);
                col = col.add(&cast_ray(&ray, &scene));
            }
            // Find the average
            col = col.div_f(samples_per_pixel as f32);

            chunk.set_chunk_pixel(chunk_x, chunk_y, Rgb::from_vec3(&col));
        }
    }
}