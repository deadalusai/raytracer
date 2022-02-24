// Unmultiplied RGBA data
pub type Rgba = [u8; 4];

pub fn v3_to_rgba(v3: raytracer::V3) -> Rgba {
    let r = (255.0 * v3.0.sqrt()) as u8;
    let g = (255.0 * v3.1.sqrt()) as u8;
    let b = (255.0 * v3.2.sqrt()) as u8;
    let a = 255;
    [r, g, b, a]
}

#[derive(Clone)]
pub struct RgbaBuffer {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl RgbaBuffer {
    pub fn new(width: u32, height: u32) -> RgbaBuffer {
        RgbaBuffer {
            width,
            height,
            data: vec![0; (width * height * 4) as usize],
        }
    }

    fn index(&self, x: u32, y: u32) -> usize {
        let i = (y * self.width + x) * 4;
        i as usize
    }

    pub fn put_pixel(&mut self, x: u32, y: u32, rgba: Rgba) {
        let i = self.index(x, y);
        self.data[i + 0] = rgba[0];
        self.data[i + 1] = rgba[1];
        self.data[i + 2] = rgba[2];
        self.data[i + 3] = rgba[3];
    }

    pub fn copy_from_sub_buffer(&mut self, off_x: u32, off_y: u32, sub_buffer: &RgbaBuffer) {
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
    
    pub fn get_raw_rgba_data(&self) -> ([usize; 2], &[u8]) {
        ([self.width as usize, self.height as usize], self.data.as_ref())
    }
}