use std::sync::Arc;
use super::implementation::{ Texture };
use super::types::{ V3, IntoArc };

// Constant colors

pub struct ColorTexture(pub V3);

impl Texture for ColorTexture {
    fn value(&self, _u: f32, _v: f32, _p: &V3) -> V3 {
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
    fn value(&self, u: f32, v: f32, p: &V3) -> V3 {
        let sines = (self.size * p.x()).sin() * (self.size * p.y()).sin() * (self.size * p.z()).sin();
        if sines < 0.0 {
            self.odd.value(u, v, p)
        }
        else {
            self.even.value(u, v, p)
        }
    }
}
