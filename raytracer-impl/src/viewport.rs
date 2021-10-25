
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
}

#[derive(Clone)]
pub struct RenderChunk {
    pub id: u32,
    pub viewport: Viewport,
    pub top: u32,
    pub left: u32,
    pub width: u32,
    pub height: u32,
}

pub struct ViewChunkCoords {
    pub chunk_x: u32,
    pub chunk_y: u32,
    pub viewport_x: u32,
    pub viewport_y: u32,
}

impl RenderChunk {
    //  Iterates over pixel positions within the chunk
    pub fn iter_pixels<'a>(&'a self) -> impl Iterator<Item=ViewChunkCoords> + 'a {
        (0..self.height)
            .flat_map(move |y| (0..self.width)
                .map(move |x| ViewChunkCoords {
                    chunk_x: x,
                    chunk_y: y,
                    viewport_x: self.left + x,
                    viewport_y: self.top + y
                }))
    }
}

pub fn create_render_chunks(viewport: &Viewport, chunk_count: u32) -> Vec<RenderChunk> {
    let divisions = (chunk_count as f32).sqrt();
    let h_divisions = divisions.ceil() as u32;
    let v_divisions = divisions.floor() as u32;
    let chunk_width = viewport.width / h_divisions;
    let chunk_height = viewport.height / v_divisions;
    (0..v_divisions)
        .flat_map(|y| (0..h_divisions).map(move |x| (x, y)))
        .enumerate()
        .map(|(id, (x, y))| {
            RenderChunk {
                id: id as u32,
                viewport: viewport.clone(),
                top: y * chunk_height,
                left: x * chunk_width,
                width: chunk_width,
                height: chunk_height,
            }
        })
        .collect::<Vec<_>>()
}