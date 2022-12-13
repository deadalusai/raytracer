use std::sync::Arc;

use super::implementation::{ Texture, ColorMap, HitRecord };
use super::types::{ V2, V3, IntoArc };

// Constant colors

#[derive(Clone)]
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
        let V2(u, v) = hit_record.mtl_uv;
        V3(u, v, 1.0 - u - v)
    }
}


// Image texture / color maps

pub struct ImageColorMap {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<V3>,
}

impl ColorMap for ImageColorMap {
    fn value(&self, u: f32, v: f32) -> V3 {
        let x = (u * self.width as f32) as usize;
        let y = (v * self.height as f32) as usize;
        let offset = y * self.width + x;
        self.pixels.get(offset).cloned().unwrap_or_default()
    }
}

// Can use a color map as a texture directly
impl Texture for ImageColorMap {
    fn value(&self, hit_record: &HitRecord) -> V3 {
        let V2(u, v) = hit_record.mtl_uv;
        ColorMap::value(self, u, v)
    }
}

/// A material loaded from an OBJ mtl ile
pub struct MeshMaterial {
    pub name: String,
    pub diffuse_color: V3,
    pub diffuse_color_map: Option<Arc<dyn ColorMap>>,
}

impl Texture for MeshMaterial {
    fn value(&self, hit_record: &HitRecord) -> V3 {
        match self.diffuse_color_map {
            Some(ref map) => map.value(hit_record.mtl_uv.0, hit_record.mtl_uv.1),
            None => self.diffuse_color.clone()
        }
    }
}

// A collection of OBJ mtl materials.
// Only used if the HitRecord specifies a {mtl_index}
impl Texture for Vec<MeshMaterial> {
    fn value(&self, hit_record: &HitRecord) -> V3 {
        hit_record.mtl_index
            .and_then(|id| self.get(id))
            .map(|mat| mat.value(hit_record))
            .unwrap_or_default()
    }
}