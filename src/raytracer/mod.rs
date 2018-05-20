
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
    /*
    The color(ray) function linearly blends white and blue depending on the up/downess of the y
    coordinate. I first made it a unit vector so -1.0 < y < 1.0. I then did a standard graphics trick of
    scaling that to 0.0 < t < 1.0. When t=1.0 I want blue. When t = 0.0 I want white. In between, I
    want a blend. This forms a “linear blend”, or “linear interpolation”, or “lerp” for short, between two
    things. A lerp is always of the form: blended_value = (1-t)*start_value + t*end_value, with t
    going from zero to one.
    */
    let unit_direction = ray.direction.unit_vector();
    let t = 0.5 * (unit_direction.y + 1.0);
    // HACK use Vec3 for multiplication
    let white = Vec3::new(1.0, 1.0, 1.0);
    let sky_blue = Vec3::new(0.5, 0.7, 1.0);
    let v = white.mul_f(1.0 - t).add(&sky_blue.mul_f(t));
    vec3_to_rgb(&v)
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
            let col = color(&r);
            set_pixel(buffer, (x, y), &col);
        }
    }
}