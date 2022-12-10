use std::sync::Arc;
use super::implementation::{ Texture, HitRecord };
use super::types::{ V2, V3, IntoArc };

// Constant colors

pub struct ColorTexture(pub V3);

impl Texture for ColorTexture {
    fn value(&self, _hit_record: &HitRecord) -> V3 {
        self.0
    }
}

// Checker texture

pub struct CheckerTexture {
    size: f32,
    odd: Arc<dyn Texture>,
    even: Arc<dyn Texture>,
}

impl CheckerTexture {
    pub fn new(size: f32, odd: impl IntoArc<dyn Texture>, even: impl IntoArc<dyn Texture>) -> CheckerTexture {
        CheckerTexture {
            size,
            odd: odd.into_arc(),
            even: even.into_arc(),
        }
    }
}

impl Texture for CheckerTexture {
    fn value(&self, hit_record: &HitRecord) -> V3 {
        let sines =
            (self.size * hit_record.p.x()).sin() *
            (self.size * hit_record.p.y()).sin() *
            (self.size * hit_record.p.z()).sin();

        if sines < 0.0 {
            self.odd.value(hit_record)
        }
        else {
            self.even.value(hit_record)
        }
    }
}

// Test texture

pub struct TestTexture;

impl Texture for TestTexture {
    fn value(&self, hit_record: &HitRecord) -> V3 {
        let red = V3(1.0, 0.0, 0.0);
        let blue = V3(0.0, 0.0, 1.0);
        (blue * hit_record.uv.0) + (red * hit_record.uv.1)
    }
}


// Image texture

pub struct ImageTexture {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<V3>,
}

impl Texture for ImageTexture {
    fn value(&self, hit_record: &HitRecord) -> V3 {
        let V2(u, v) = hit_record.uv;
        let x = (u * self.width as f32) as usize;
        let y = ((1.0 - v) * self.height as f32) as usize;
        let offset = y * self.width + x;
        self.pixels.get(offset).cloned().unwrap_or_else(|| V3::zero())
    }
}
