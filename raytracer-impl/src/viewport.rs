//
// Chunk primitives
//

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct RenderChunk {
    pub left: usize,
    pub top: usize,
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

pub fn create_render_chunks(size: [usize; 2], count: [usize; 2]) -> Vec<RenderChunk> {
    #[derive(Copy, Clone)]
    struct Segment { offset: usize, size: usize }

    // Divide one dimension into segments
    fn dim_segments(length: usize, count: usize) -> impl Iterator<Item=Segment> {
        let size = length / count;
        let remainder = length % count;
        (0..count).map(move |i| Segment {
            offset: i * size,
            // The last segment may be larger, to account for remainder of integer division
            size: if i == count - 1 { size + remainder } else { size }
        })
    }

    dim_segments(size[1], count[1])
        .flat_map(|v| dim_segments(size[0], count[0]).map(move |h| (h, v)))
        .map(|(h, v)| RenderChunk {
            left: h.offset,
            top: v.offset,
            width: h.size,
            height: v.size,
        })
        .collect()
}

#[cfg(test)]
mod test {
    use super::{RenderChunk, create_render_chunks};

    #[test]
    fn create_render_chunks_test() {
        let chunks = create_render_chunks([10, 10], [3, 3]);
        assert_eq!(chunks, [
            RenderChunk { left: 0, top: 0, width: 3, height: 3 },
            RenderChunk { left: 3, top: 0, width: 3, height: 3 },
            RenderChunk { left: 6, top: 0, width: 4, height: 3 },
            RenderChunk { left: 0, top: 3, width: 3, height: 3 },
            RenderChunk { left: 3, top: 3, width: 3, height: 3 },
            RenderChunk { left: 6, top: 3, width: 4, height: 3 },
            RenderChunk { left: 0, top: 6, width: 3, height: 4 },
            RenderChunk { left: 3, top: 6, width: 3, height: 4 },
            RenderChunk { left: 6, top: 6, width: 4, height: 4 },
        ]);
    }
}
