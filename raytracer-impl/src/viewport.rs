
//
// Chunk primitives
//

#[derive(Clone)]
pub struct RenderChunk {
    pub id: usize,
    pub top: usize,
    pub left: usize,
    pub width: usize,
    pub height: usize,
}

pub struct ViewChunkCoords {
    pub chunk_pos: [usize; 2],
    pub view_pos: [usize; 2],
}

impl RenderChunk {
    //  Iterates over pixel positions within the chunk
    pub fn iter_pixels<'a>(&'a self) -> impl Iterator<Item=ViewChunkCoords> + 'a {
        (0..self.height)
            .flat_map(move |y| (0..self.width)
                .map(move |x| ViewChunkCoords {
                    chunk_pos: [x, y],
                    view_pos: [self.left + x, self.top + y],
                }))
    }
}

pub fn create_render_chunks(chunk_count: u32, width: usize, height: usize) -> Vec<RenderChunk> {
    let divisions = (chunk_count as f32).sqrt();
    let h_divisions = divisions.ceil() as usize;
    let v_divisions = divisions.floor() as usize;
    let chunk_width = width / h_divisions;
    let chunk_height = height / v_divisions;
    (0..v_divisions)
        .flat_map(|y| (0..h_divisions).map(move |x| (x, y)))
        .enumerate()
        .map(|(id, (x, y))| {
            RenderChunk {
                id,
                top: y * chunk_height,
                left: x * chunk_width,
                width: chunk_width,
                height: chunk_height,
            }
        })
        .collect::<Vec<_>>()
}
