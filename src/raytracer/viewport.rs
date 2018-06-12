
use raytracer::types::{ Rgb };

//
// View tracking and chunk primitives
//

#[derive(Debug, Clone)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

impl Viewport {
    pub fn new (width: u32, height: u32) -> Viewport {
        Viewport { width: width, height: height }
    }

    pub fn iter_view_chunks (&self, h_count: u32, v_count: u32) -> impl Iterator<Item=ViewChunk> {
        let chunk_width = self.width / h_count;
        let chunk_height = self.height / v_count;
        let viewport = self.clone();
        (0..v_count)
            .flat_map(move |y| (0..h_count).map(move |x| (x, y)))
            .enumerate()
            .map(move |(id, (x, y))| {
                let top_left_x = x * chunk_width;
                let top_left_y = y * chunk_height;
                ViewChunk {
                    id: id as u32,
                    viewport: viewport.clone(),
                    chunk_top_left: (top_left_x, top_left_y),
                    width: chunk_width,
                    height: chunk_height,
                    data: vec!(Rgb::new(0, 0, 0); chunk_width as usize * chunk_height as usize)
                }
            })
    }
}

pub struct ViewChunk {
    pub id: u32,
    pub viewport: Viewport,

    pub width: u32,
    pub height: u32,
    
    chunk_top_left: (u32, u32),
    data: Vec<Rgb>,
}

impl ViewChunk {
    /// Sets a pixel using chunk-relative co-ordinates
    pub fn set_chunk_pixel (&mut self, chunk_x: u32, chunk_y: u32, value: Rgb) {
        let pos = (chunk_y * self.width + chunk_x) as usize;
        self.data[pos] = value;
    }

    /// Gets a pixel using view-relative co-ordinates
    pub fn get_chunk_pixel (&self, chunk_x: u32, chunk_y: u32) -> &Rgb {
        let pos = (chunk_y * self.width + chunk_x) as usize;
        &self.data[pos]
    }

    /// Gets a pixel using view-relative co-ordinates
    pub fn get_view_relative_coords (&self, chunk_x: u32, chunk_y: u32) -> (u32, u32) {
        // Convert to chunk-relative coords
        let view_x = self.chunk_top_left.0 + chunk_x;
        let view_y = self.chunk_top_left.1 + chunk_y;
        (view_x, view_y)
    }
}
