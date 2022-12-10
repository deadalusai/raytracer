use std::path::{ Path };
use std::fs::{ File };

use bmp::{ from_reader, Pixel };

use raytracer_impl::texture::{ ImageTexture };
use raytracer_impl::types::{ V3 };


fn rgb_to_v3(pixel: &Pixel) -> V3 {
    let r = pixel.r as f32 * 1.0 / 255.0;
    let g = pixel.g as f32 * 1.0 / 255.0;
    let b = pixel.b as f32 * 1.0 / 255.0;
    V3(r, g, b)
}

fn load_bitmap_texture<R: std::io::Read>(mut reader: R) -> ImageTexture {
    let image = from_reader(&mut reader).unwrap();
    let width = image.get_width();
    let height = image.get_height();
    let mut pixels = Vec::with_capacity((width * height) as usize);

    // Read all pixels, with 0,0 being bottom left
    for y in 0..height {
        for x in 0..width {
            let pixel = image.get_pixel(x, y);
            pixels.push(rgb_to_v3(&pixel));
        }
    }

    ImageTexture {
        width: width as usize,
        height: height as usize,
        pixels,
    }
}

#[allow(unused)]
pub fn load_bitmap_texture_from_path(path: &Path) -> ImageTexture {
    let mut file = File::open(path).unwrap();
    load_bitmap_texture(file)
}

#[allow(unused)]
pub fn load_bitmap_texture_from_bytes(bytes: &[u8]) -> ImageTexture {
    load_bitmap_texture(bytes)
}