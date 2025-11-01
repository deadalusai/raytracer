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

pub fn create_render_chunks(size: [usize; 2], counts: [usize; 2]) -> Vec<RenderChunk> {
    let [horiz, vert] = counts;
    let [width, height] = size;
    let chunk_width = width / horiz;
    let chunk_height = height / vert;
    (0..vert)
        .flat_map(|y| (0..horiz).map(move |x| (x, y)))
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
