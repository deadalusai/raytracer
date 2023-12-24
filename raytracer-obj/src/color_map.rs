use raytracer_impl::texture::ColorMap;
use raytracer_impl::types::V3;

use crate::ObjError;

fn rgb_to_v3(pixel: &bmp::Pixel) -> V3 {
    let r = pixel.r as f32 / 255.0;
    let g = pixel.g as f32 / 255.0;
    let b = pixel.b as f32 / 255.0;
    V3(r, g, b)
}

pub fn load_bitmap_color_map<R: std::io::Read>(mut reader: R) -> Result<ColorMap, ObjError> {
    let image = bmp::from_reader(&mut reader)?;
    let width = image.get_width();
    let height = image.get_height();
    let mut pixels = Vec::with_capacity((width * height) as usize);
    
    // Read all pixels into V3 format with 0,0 being bottom left
    // The {bmp} crate inverts the Y coordinates, so need to flip them when reading.

    for y in 0..height {
        for x in 0..width {
            let pixel = image.get_pixel(x, height - y - 1);
            pixels.push(rgb_to_v3(&pixel));
        }
    }

    Ok(ColorMap {
        width: width as usize,
        height: height as usize,
        pixels,
    })
}
