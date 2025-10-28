use raytracer_impl::types::V3;

// Unmultiplied RGBA data
pub type Rgba = [u8; 4];

pub fn v3_to_rgba(v3: V3) -> Rgba {
    let r = (255.0 * v3.0) as u8;
    let g = (255.0 * v3.1) as u8;
    let b = (255.0 * v3.2) as u8;
    let a = 255;
    [r, g, b, a]
}

#[derive(Clone)]
pub struct RgbaBuffer {
    width: usize,
    height: usize,
    data: Vec<u8>,
}

pub struct RgbaRaw<'a> {
    pub size: [usize; 2],
    pub rgba: &'a [u8],
}

impl RgbaBuffer {
    pub fn new(width: usize, height: usize) -> RgbaBuffer {
        RgbaBuffer {
            width,
            height,
            data: vec![0; (width * height * 4) as usize],
        }
    }

    fn index(&self, x: usize, y: usize) -> usize {
        let i = (y * self.width + x) * 4;
        i as usize
    }

    pub fn put_pixel(&mut self, x: usize, y: usize, rgba: Rgba) {
        let i = self.index(x, y);
        self.data[i + 0] = rgba[0];
        self.data[i + 1] = rgba[1];
        self.data[i + 2] = rgba[2];
        self.data[i + 3] = rgba[3];
    }

    pub fn get_raw_rgba_data(&'_ self) -> RgbaRaw<'_> {
        RgbaRaw {
            size: [self.width, self.height],
            rgba: self.data.as_ref(),
        }
    }
}
