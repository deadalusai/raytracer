
mod types;

use std;

use raytracer::types::{ Rgb };
use raytracer::types::{ Vec3, vec3_dot, vec3_cross };
use raytracer::types::{ Ray };

use image::{ RgbaImage };
use rand::{ Rng, thread_rng };

// Materials

struct MatRecord {
    attenuation: Vec3,
    scattered: Ray,
}

trait Material {
    fn scatter (&self, ray: &Ray, hit_record: &HitRecord) -> Option<MatRecord>;
}

struct MatLambertian {
    albedo: Vec3,
}

impl MatLambertian {
    fn with_albedo (albedo: Vec3) -> Box<MatLambertian> {
        Box::new(MatLambertian { albedo: albedo })
    }
}

fn random_point_in_unit_sphere () -> Vec3 {
    let unit = Vec3::new(1.0, 1.0, 1.0);
    let mut rng = thread_rng();
    loop {
        let random_point = Vec3::new(rng.next_f32(), rng.next_f32(), rng.next_f32());
        let p = random_point.mul_f(2.0).sub(&unit);
        // Inside our sphere?
        if p.length_squared() < 1.0 {
            return p;
        }
    }
}

impl Material for MatLambertian {
    fn scatter (&self, _r: &Ray, hit_record: &HitRecord) -> Option<MatRecord> {
        let target = hit_record.p.add(&hit_record.normal).add(&random_point_in_unit_sphere());
        let scattered = Ray::new(hit_record.p.clone(), target.sub(&hit_record.p));
        Some(MatRecord { scattered: scattered, attenuation: self.albedo.clone() })
        // TODO?
        // We could just as well scatter with some probability p and have attenuation be albedo / p
    }
}

struct MatMetal {
    albedo: Vec3,
    fuzz: f32,
}

impl MatMetal {
    fn with_albedo_and_fuzz (albedo: Vec3, fuzz: f32) -> Box<MatMetal> {
        Box::new(MatMetal { albedo: albedo, fuzz: fuzz })
    }
}

fn reflect (v: &Vec3, n: &Vec3) -> Vec3 {
    v.sub(&n.mul_f(vec3_dot(v, n)).mul_f(2.0))
}

impl Material for MatMetal {
    fn scatter (&self, ray: &Ray, hit_record: &HitRecord) -> Option<MatRecord> {
        let reflected = reflect(&ray.direction.unit_vector(), &hit_record.normal);
        let scattered = Ray::new(hit_record.p.clone(), reflected.add(&random_point_in_unit_sphere().mul_f(self.fuzz)));
        if vec3_dot(&scattered.direction, &hit_record.normal) > 0.0 {
            return Some(MatRecord { scattered: scattered, attenuation: self.albedo.clone() });
        }
        None
    }
}

struct MatDielectric {
    ref_index: f32,
}

impl MatDielectric {
    fn with_refractive_index (ref_index: f32) -> Box<MatDielectric> {
        Box::new(MatDielectric { ref_index: ref_index })
    }
}

fn refract (v: &Vec3, n: &Vec3, ni_over_nt: f32) -> Option<Vec3> {
    let uv = v.unit_vector();
    let dt = vec3_dot(&uv, n);
    let discriminant = 1.0 - ni_over_nt * ni_over_nt * (1.0 - dt * dt);
    if discriminant > 0.0 {
        let refracted = uv.sub(&n.mul_f(dt)).mul_f(ni_over_nt).sub(&n.mul_f(discriminant.sqrt()));
        return Some(refracted);
    }
    None
}

fn schlick_reflect_prob (cosine: f32, ref_idx: f32) -> f32 {
    let r0 = (1.0 - ref_idx) / (1.0 + ref_idx);
    let r0 = r0 * r0;
    r0 + (1.0 - r0) * (1.0 - cosine).powf(5.0)
}

impl Material for MatDielectric {
    fn scatter (&self, ray: &Ray, hit_record: &HitRecord) -> Option<MatRecord> {
        let mut rng = thread_rng();

        let dot = vec3_dot(&ray.direction, &hit_record.normal);
        let (outward_normal, ni_over_nt, cosine) =
            if dot > 0.0 {
                (hit_record.normal.negate(), self.ref_index, self.ref_index * dot / ray.direction.length())
            } else {
                (hit_record.normal.clone(), 1.0 / self.ref_index, -dot / ray.direction.length())
            };

        // If prob value <= rand, reflect
        // If prob value > rand, refract
        let reflect_prob = schlick_reflect_prob(cosine, self.ref_index);

        let direction =
            refract(&ray.direction, &outward_normal, ni_over_nt)
                .filter(|_| reflect_prob < rng.next_f32())
                .unwrap_or_else(|| reflect(&ray.direction, &hit_record.normal));
        
        let scattered = Ray::new(hit_record.p.clone(), direction);
        
        // NOTE: Attenuation is always 0.99 (glass absorbs very little)
        Some(MatRecord { scattered: scattered, attenuation: Vec3::new(0.99, 0.99, 0.99) })
    }
}

// Hitables

struct HitRecord<'mat> {
    t: f32,
    p: Vec3,
    normal: Vec3,
    material: &'mat Material,
}

trait Hitable {
    fn hit<'a> (&'a self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'a>>;
}

struct Sphere {
    center: Vec3,
    radius: f32,
    material: Box<Material>,
}

impl Sphere {
    fn new (center: Vec3, radius: f32, material: Box<Material>) -> Sphere {
        Sphere { center: center, radius: radius, material: material }
    }
}

impl Hitable for Sphere {
    fn hit<'a> (&'a self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'a>> {
        let oc = ray.origin.sub(&self.center);
        let a = vec3_dot(&ray.direction, &ray.direction);
        let b = vec3_dot(&oc, &ray.direction);
        let c = vec3_dot(&oc, &oc) - self.radius * self.radius;
        let discriminant = b * b - a * c;
        if discriminant > 0.0 {
            let t = (-b - discriminant.sqrt()) / a;
            if t < t_max && t > t_min {
                let point = ray.point_at_parameter(t);
                let normal = point.sub(&self.center).div_f(self.radius);
                return Some(HitRecord { t: t, p: point, normal: normal, material: &*self.material });
            }
            let t = (-b + discriminant.sqrt()) / a;
            if t < t_max && t > t_min {
                let point = ray.point_at_parameter(t);
                let normal = point.sub(&self.center).div_f(self.radius);
                return Some(HitRecord { t: t, p: point, normal: normal, material: &*self.material });
            }
        }
        None
    }
}

struct World {
    things: Vec<Box<Hitable>>,
}

impl World {
    fn new () -> World {
        World { things: vec!() }
    }

    fn add_thing<T> (&mut self, hitable: T)
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

struct Camera {
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
    fn new (look_from: Vec3, look_at: Vec3, v_up: Vec3, v_fov: f32, aspect_ratio: f32, aperture: f32, focus_dist: f32) -> Camera {
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

    fn get_ray (&self, s: f32, t: f32) -> Ray {
        let rd = random_point_in_unit_disk().mul_f(self.lens_radius);
        let offset = self.u.mul_f(rd.x).add(&self.v.mul_f(rd.y));
        let origin = self.origin.add(&offset);
        let direction = self.lower_left_corner.add(&self.horizontal.mul_f(s)).add(&self.vertical.mul_f(t)).sub(&self.origin).sub(&offset);
        Ray::new(origin, direction)
    }
}

fn color_sky (ray: &Ray) -> Vec3 {
    let unit_direction = ray.direction.unit_vector();
    let t = 0.5 * (unit_direction.y + 1.0);
    let white = Vec3::new(1.0, 1.0, 1.0);
    let sky_blue = Vec3::new(0.5, 0.7, 1.0);
    white.mul_f(1.0 - t).add(&sky_blue.mul_f(t))
}

fn color (ray: &Ray, world: &World, depth: i32) -> Vec3 {
    // Hit the world?
    if depth < 50 {
        if let Some(hit_record) = world.hit(ray, 0.001, std::f32::MAX) {
            if let Some(mat) = hit_record.material.scatter(ray, &hit_record) {
                return color(&mat.scattered, world, depth + 1).mul(&mat.attenuation);
            }
        }
    }

    // Hit the sky instead...
    color_sky(ray)
}

fn random_scene () -> World {
    let mut rng = thread_rng();
    let mut world = World::new();

    // World sphere
    world.add_thing(Sphere::new(Vec3::new(0.0, -1000.0, 0.0), 1000.0, MatLambertian::with_albedo(Vec3::new(0.5, 0.5, 0.5))));

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

                world.add_thing(Sphere::new(center, 0.2, material));
            }
        }
    }

    // Large fixed spheres
    world.add_thing(Sphere::new(Vec3::new(-4.0, 1.0, 0.0), 1.0, MatLambertian::with_albedo(Vec3::new(0.8, 0.2, 0.1))));
    world.add_thing(Sphere::new(Vec3::new(0.0, 1.0, 0.0),  1.0, MatDielectric::with_refractive_index(1.5)));
    world.add_thing(Sphere::new(Vec3::new(4.0, 1.0, 0.0),  1.0, MatMetal::with_albedo_and_fuzz(Vec3::new(0.8, 0.8, 0.8), 0.0)));

    world
}

pub fn cast_rays (buffer: &mut RgbaImage, samples: u32) {
    let width = buffer.width();
    let height = buffer.height();

    // NOTE:
    //   Y-axis goes up
    //   X-axis goes right
    //   Z-axis goes towards the camera (negative into the screen)
    let look_from = Vec3::new(13.0, 2.0, 3.0);
    let look_to = Vec3::new(0.0, 0.0, 0.0);
    let v_up = Vec3::new(0.0, 1.0, 0.0);
    let fov = 20.0;
    let aspect_ratio = width as f32 / height as f32;
    let dist_to_focus = 10.0;
    let aperture = 0.1;

    let camera = Camera::new(look_from, look_to, v_up, fov, aspect_ratio, aperture, dist_to_focus);
    let world = random_scene();

    let mut rng = thread_rng();
    for (x, y, pixel) in buffer.enumerate_pixels_mut() {
        let mut col = Vec3::new(0.0, 0.0, 0.0);
        for _ in 0..samples {
            let u = (x as f32 + rng.next_f32()) / width as f32;
            let v = ((height - y) as f32 + rng.next_f32()) / height as f32;
            let ray = camera.get_ray(u, v);
            col = col.add(&color(&ray, &world, 0));
        }
        col = col.div_f(samples as f32);
        let col = Rgb::from_vec3(&col);
        pixel.data = [col.r, col.g, col.b, 255];
    }
}