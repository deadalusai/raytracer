use std::io::BufReader;

use image::{GenericImageView, Rgba};
use raytracer_impl::texture::ColorMap;
use raytracer_impl::types::V3;

use crate::ObjError;

fn rgba_to_v3(rgba: Rgba<u8>) -> V3 {
    let r = rgba[0] as f32 / 255.0;
    let g = rgba[1] as f32 / 255.0;
    let b = rgba[2] as f32 / 255.0;
    V3(r, g, b)
}

pub fn load_color_map<R: std::io::Read + std::io::Seek>(reader: R, format: image::ImageFormat) -> Result<ColorMap, ObjError> {
    let dynamic = image::load(BufReader::new(reader), format)?;
    let width = dynamic.width();
    let height = dynamic.height();
    let mut pixels = Vec::with_capacity((width * height) as usize);

    // Read all pixels into V3 format with 0,0 being top left
    for (_, _, pixel) in dynamic.pixels() {
        // Pixel data is encoded in RGBA (0-255) bytes
        pixels.push(rgba_to_v3(pixel));
    }

    Ok(ColorMap {
        width: width as usize,
        height: height as usize,
        pixels,
    })
}
