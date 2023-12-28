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
    pub width: usize,
    pub height: usize,
    pub data: &'a [u8],
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

    pub fn copy_from_sub_buffer(&mut self, off_x: usize, off_y: usize, sub_buffer: &RgbaBuffer) {
        // Copy one row at a time
        for y in 0..sub_buffer.height {
            let t_start = self.index(off_x, off_y + y);
            let s_start = sub_buffer.index(0, y);
            let len = (sub_buffer.width * 4) as usize;
            
            let target = &mut self.data[t_start..t_start + len];
            let source = &sub_buffer.data[s_start..s_start + len];

            target.copy_from_slice(source);
        }
    }
    
    pub fn get_raw_rgba_data(&self) -> RgbaRaw {
        RgbaRaw {
            width: self.width as usize,
            height: self.height as usize,
            data: self.data.as_ref(),
        }
    }
}