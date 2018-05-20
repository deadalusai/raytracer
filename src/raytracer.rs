
use image::{ RgbaImage };

#[derive(Debug, Clone, PartialEq, Eq)]
struct Rgb { r: u8, g: u8, b: u8 }

impl Rgb {
    fn new (r: u8, g: u8, b: u8) -> Rgb {
        Rgb { r: r, g: g, b: b }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Vec3 { x: f32, y: f32, z: f32 }

impl Vec3 {
    fn new (x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x: x, y: y, z: z }
    }

    fn add (&self, other: &Vec3) -> Vec3 {
        Vec3 {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z
        }
    }

    fn add_mut (&mut self, other: &Vec3) {
        *self = self.add(other);
    }

    fn sub (&self, other: &Vec3) -> Vec3 {
        Vec3 {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z
        }
    }

    fn sub_mut (&mut self, other: &Vec3) {
        *self = self.sub(other);
    }

    fn mul (&self, other: &Vec3) -> Vec3 {
        Vec3 {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z
        }
    }

    fn mul_mut (&mut self, other: &Vec3) {
        *self = self.mul(other);
    }

    fn div (&self, other: &Vec3) -> Vec3 {
        Vec3 {
            x: self.x / other.x,
            y: self.y / other.y,
            z: self.z / other.z
        }
    }

    fn div_mut (&mut self, other: &Vec3) {
        *self = self.div(other);
    }

    fn mul_f (&self, f: f32) -> Vec3 {
        Vec3 {
            x: self.x * f,
            y: self.y * f,
            z: self.z * f
        }
    }

    fn div_f (&self, f: f32) -> Vec3 {
        Vec3 {
            x: self.x / f,
            y: self.y / f,
            z: self.z / f
        }
    }

    fn dot (&self, other: &Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn cross (&self, other: &Vec3) -> Vec3 {
        Vec3 {
            x: self.y * other.z - self.z * other.y,
            y: self.x * other.z - self.z * other.x,
            z: self.y * other.x - self.x * other.y
        }
    }

    fn negate (&self) -> Vec3 {
        Vec3 {
            x: -self.x,
            y: -self.y,
            z: -self.z
        }
    }

    fn unit_vector (&self) -> Vec3 {
        self.div_f(self.length())
    }

    fn length (&self) -> f32 {
        self.length_squared().sqrt()
    }

    fn length_squared (&self) -> f32 {
        ((self.x * self.x) + (self.y * self.y) + (self.z * self.z)).sqrt()
    }
}

fn set_pixel (image: &mut RgbaImage, pos: (u32, u32), value: &Rgb) {
    image.get_pixel_mut(pos.0, pos.1).data = [value.r, value.g, value.b, 255];
}

pub fn draw_gradient (buffer: &mut RgbaImage) {
    let width = buffer.width();
    let height = buffer.height();

    for x in 0..width {
        for y in 0..height {
            let r = x as f32 / width as f32;
            let g = y as f32 / height as f32;
            let b = 0.2 as f32;
            let rgb = Rgb::new(
                (255.99 * r) as u8,
                (255.99 * g) as u8,
                (255.99 * b) as u8
            );
            set_pixel(buffer, (x, y), &rgb);
        }
    }
}