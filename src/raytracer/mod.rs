
mod rgb;
mod vec3;
mod ray;

use std;

use raytracer::rgb::*;
use raytracer::vec3::*;
use raytracer::ray::*;

use image::{ RgbaImage };

fn vec3_to_rgb (v: &Vec3) -> Rgb {
    Rgb::new(
        (255.0 * v.x) as u8,
        (255.0 * v.y) as u8,
        (255.0 * v.z) as u8
    )
}

struct HitRecord {
    t: f32,
    p: Vec3,
    normal: Vec3,
}

trait Hitable {
    fn hit (&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord>;
}

struct Sphere {
    center: Vec3,
    radius: f32,
}

impl Sphere {
    fn new (center: Vec3, radius: f32) -> Sphere {
        Sphere { center: center, radius: radius }
    }
}

impl Hitable for Sphere {
    fn hit (&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        let oc = ray.origin.sub(&self.center);
        let a = vec3_dot(&ray.direction, &ray.direction);
        let b = 2.0 * vec3_dot(&oc, &ray.direction);
        let c = vec3_dot(&oc, &oc) - self.radius * self.radius;
        let discriminant = b * b - 4.0 * a * c;
        if discriminant > 0.0 {
            let temp = (-b - (b * b - a * c).sqrt()) / a;
            if temp > t_min && temp < t_max {
                let point = ray.point_at_parameter(temp);
                let normal = point.sub(&self.center).div_f(self.radius);
                return Some(HitRecord { t: temp, p: point, normal: normal });
            }
            let temp = (-b + (b * b - a * c).sqrt()) / a;
            if temp > t_min && temp < t_max {
                let point = ray.point_at_parameter(temp);
                let normal = point.sub(&self.center).div_f(self.radius);
                return Some(HitRecord { t: temp, p: point, normal: normal });
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

fn color (ray: &Ray, world: &World) -> Rgb {
    // Hit the world?
    if let Some(record) = world.hit(ray, 0.0, std::f32::MAX) {
        let n = record.normal;
        let t = Vec3::new(n.x + 1.0, n.y + 1.0, n.z + 1.0).mul_f(0.5);
        return vec3_to_rgb(&t);
    }

    // Hit the sky instead...
    let unit_direction = ray.direction.unit_vector();
    let t = 0.5 * (unit_direction.y + 1.0);
    // HACK use Vec3 for multiplication
    let white = Vec3::new(1.0, 1.0, 1.0);
    let sky_blue = Vec3::new(0.5, 0.7, 1.0);
    let v = white.mul_f(1.0 - t).add(&sky_blue.mul_f(t));
    vec3_to_rgb(&v)
}

fn set_pixel (image: &mut RgbaImage, pos: (u32, u32), value: &Rgb) {
    image.get_pixel_mut(pos.0, pos.1).data = [value.r, value.g, value.b, 255];
}

pub fn cast_rays (buffer: &mut RgbaImage) {
    let width = buffer.width();
    let height = buffer.height();

    // NOTE:
    //   Y-axis goes up
    //   X-axis goes right
    //   Z-axis goes towards the camera (negative into the screen)

    let lower_left_corner = Vec3::new(-2.0, -1.0, -1.0);
    let horizontal = Vec3::new(4.0, 0.0, 0.0);
    let vertical = Vec3::new(0.0, 2.0, 0.0);
    let origin = Vec3::new(0.0, 0.0, 0.0);

    let mut world = World::new();

    world.add_thing(Sphere::new(Vec3::new(0.0, 0.0, -1.0), 0.5));
    world.add_thing(Sphere::new(Vec3::new(0.0, -100.5, -1.0), 100.0));

    for x in 0..width {
        for y in 0..height {
            let u = x as f32 / width as f32;
            let v = (height - y) as f32 / height as f32;
            let r = Ray::new(origin.clone(), lower_left_corner.add(&horizontal.mul_f(u)).add(&vertical.mul_f(v)));
            let mut col = color(&r, &world);
            set_pixel(buffer, (x, y), &col);
        }
    }
}