
mod rgb;
mod vec3;
mod ray;

use raytracer::rgb::*;
use raytracer::vec3::*;
use raytracer::ray::*;

use image::{ RgbaImage };

fn set_pixel (image: &mut RgbaImage, pos: (u32, u32), value: &Rgb) {
    image.get_pixel_mut(pos.0, pos.1).data = [value.r, value.g, value.b, 255];
}

pub fn draw_gradient (buffer: &mut RgbaImage) {
    let width = buffer.width();
    let height = buffer.height();

    for x in 0..width {
        for y in (0..height).rev() {
            let r = x as f32 / width as f32;
            let g = y as f32 / height as f32;
            let b = 0.2 as f32;
            let pixel = Rgb::new(
                (255.0 * r) as u8,
                (255.0 * g) as u8,
                (255.0 * b) as u8
            );
            set_pixel(buffer, (x, y), &pixel);
        }
    }
}

fn vec3_to_rgb (v: &Vec3) -> Rgb {
    Rgb::new(
        (255.0 * v.x) as u8,
        (255.0 * v.y) as u8,
        (255.0 * v.z) as u8
    )
}

fn color (ray: &Ray) -> Rgb {
    // Hit a sphere?
    let sphere_center = Vec3::new(0.0, 0.0, -1.0);
    let sphere_radius = 0.5;
    if let Some(t) = test_hit_sphere(&sphere_center, sphere_radius, ray) {
        let n = ray.point_at_parameter(t).sub(&Vec3::new(0.0, 0.0, -1.0));
        let c = Vec3::new(n.x + 1.0, n.y + 1.0, n.z + 1.0).mul_f(0.55);
        return vec3_to_rgb(&c);
    }

    let unit_direction = ray.direction.unit_vector();
    let t = 0.5 * (unit_direction.y + 1.0);
    // HACK use Vec3 for multiplication
    let white = Vec3::new(1.0, 1.0, 1.0);
    let sky_blue = Vec3::new(0.5, 0.7, 1.0);
    let v = white.mul_f(1.0 - t).add(&sky_blue.mul_f(t));
    vec3_to_rgb(&v)
}

fn test_hit_sphere (sphere_center: &Vec3, radius: f32, ray: &Ray) -> Option<f32> {
    let oc = ray.origin.sub(sphere_center);
    let a = vec3_dot(&ray.direction, &ray.direction);
    let b = 2.0 * vec3_dot(&oc, &ray.direction);
    let c = vec3_dot(&oc, &oc) - radius * radius;
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return None
    }
    let v = (-b - discriminant.sqrt()) / (2.0 * a);
    Some(v)
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

    for x in 0..width {
        for y in 0..height {
            let u = x as f32 / width as f32;
            let v = (height - y) as f32 / height as f32;
            let r = Ray::new(origin.clone(), lower_left_corner.add(&horizontal.mul_f(u).add(&vertical.mul_f(v))));
            let mut col = color(&r);
            set_pixel(buffer, (x, y), &col);
        }
    }
}