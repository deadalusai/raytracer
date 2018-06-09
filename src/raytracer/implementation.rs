use std;

use raytracer::types::{ Rgb };
use raytracer::types::{ Vec3, vec3_dot, vec3_cross };
use raytracer::types::{ Ray };

use image::{ RgbaImage };
use rand::{ Rng, thread_rng };

// Materials

pub struct MatRecord {
    pub attenuation: Vec3,
    pub scattered: Ray,
}

pub trait Material {
    fn scatter (&self, ray: &Ray, hit_record: &HitRecord) -> Option<MatRecord>;
}

// Hitables

pub struct HitRecord<'mat> {
    pub t: f32,
    pub p: Vec3,
    pub normal: Vec3,
    pub material: &'mat Material,
}

pub trait Hitable {
    fn hit<'a> (&'a self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'a>>;
}

// World

pub struct World {
    camera: Camera,
    things: Vec<Box<Hitable>>,
}

impl World {
    pub fn new (camera: Camera) -> World {
        World { camera: camera, things: vec!() }
    }

    pub fn add_thing<T> (&mut self, hitable: T)
        where T: Hitable + 'static
    {
        let b = Box::new(hitable);
        self.things.push(b);
    }
}

impl Hitable for World {
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
        let w = (look_from.sub(&look_at)).unit_vector();
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

/** Determines the color which the given ray resolves to. */
fn color (ray: &Ray, world: &World) -> Vec3 {

    // Returns a sky color gradient based on the vertical element of the ray
    fn color_sky (ray: &Ray) -> Vec3 {
        let unit_direction = ray.direction.unit_vector();
        let t = 0.5 * (unit_direction.y + 1.0);
        let white = Vec3::new(1.0, 1.0, 1.0);
        let sky_blue = Vec3::new(0.5, 0.7, 1.0);
        white.mul_f(1.0 - t).add(&sky_blue.mul_f(t))
    }

    // Internal implementation
    fn color_internal (ray: &Ray, world: &World, depth: i32) -> Vec3 {
        // Hit the world?
        if depth < 50 {
            if let Some(hit_record) = world.hit(ray, 0.001, std::f32::MAX) {
                if let Some(mat) = hit_record.material.scatter(ray, &hit_record) {
                    return color_internal(&mat.scattered, world, depth + 1).mul(&mat.attenuation);
                }
            }
        }

        // Hit the sky instead...
        color_sky(ray)
    }

    color_internal(ray, world, 0)
}

pub fn cast_rays (buffer: &mut RgbaImage, world: &World, samples: u32) {
    let width = buffer.width();
    let height = buffer.height();

    let mut rng = thread_rng();
    for (x, y, pixel) in buffer.enumerate_pixels_mut() {
        let mut col = Vec3::new(0.0, 0.0, 0.0);
        for _ in 0..samples {
            let u = (x as f32 + rng.next_f32()) / width as f32;
            let v = ((height - y) as f32 + rng.next_f32()) / height as f32;
            let ray = world.camera.get_ray(u, v);
            col = col.add(&color(&ray, &world));
        }
        col = col.div_f(samples as f32);
        let col = Rgb::from_vec3(&col);
        pixel.data = [col.r, col.g, col.b, 255];
    }
}