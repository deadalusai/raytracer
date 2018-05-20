
use image::{ RgbaImage };

pub fn draw_gradient (buffer: &mut RgbaImage) {
    let width = buffer.width();
    let height = buffer.height();

    for x in 0..width {
        for y in 0..height {
            let r = x as f32 / width as f32;
            let g = y as f32 / height as f32;
            let b = 0.2 as f32;
            let ir = (255.99 * r) as u8;
            let ig = (255.99 * g) as u8;
            let ib = (255.99 * b) as u8;

            buffer.get_pixel_mut(x, y).data = [ir, ig, ib, 255];
        }
    }
}