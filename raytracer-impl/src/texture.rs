use std::sync::Arc;

use super::implementation::{ Texture, ColorMap, HitRecord };
use super::types::{ V2, V3 };

// Constant colors

#[derive(Clone)]
pub struct ColorTexture(pub V3);

impl Texture for ColorTexture {
    fn value(&self, _hit_record: &HitRecord) -> V3 {
        self.0
    }
}

// Checker texture

#[derive(Clone)]
pub struct CheckerTexture<T1: Texture, T2: Texture> {
    size: f32,
    odd: T1,
    even: T2,
}

impl<T1: Texture, T2: Texture> CheckerTexture<T1, T2> {
    pub fn new(size: f32, odd: T1, even: T2) -> Self {
        Self { size, odd, even }
    }
}

impl<T1: Texture, T2: Texture> Texture for CheckerTexture<T1, T2> {
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

pub struct UVColorMap {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<V3>,
}

impl ColorMap for UVColorMap {
    fn value(&self, u: f32, v: f32) -> V3 {
        let x = (u * self.width as f32) as usize;
        let y = (v * self.height as f32) as usize;
        let offset = y * self.width + x;
        self.pixels.get(offset).cloned().unwrap_or_default()
    }
}

// Can use a color map as a texture directly
impl Texture for UVColorMap {
    fn value(&self, hit_record: &HitRecord) -> V3 {
        let V2(u, v) = hit_record.mtl_uv;
        ColorMap::value(self, u, v)
    }
}

/// A texture loaded from an OBJ mtl ile
pub struct MeshTexture {
    pub name: String,
    pub diffuse_color: V3,
    pub diffuse_color_map: Option<Arc<dyn ColorMap>>,
}

impl Texture for MeshTexture {
    fn value(&self, hit_record: &HitRecord) -> V3 {
        match self.diffuse_color_map {
            Some(ref map) => map.value(hit_record.mtl_uv.0, hit_record.mtl_uv.1),
            None => self.diffuse_color.clone()
        }
    }
}

// A collection of OBJ mtl materials.
// Only supported if the HitRecord specifies a {mtl_index}
impl Texture for Vec<MeshTexture> {
    fn value(&self, hit_record: &HitRecord) -> V3 {
        hit_record.mtl_index
            .and_then(|id| self.get(id))
            .map(|mat| mat.value(hit_record))
            .unwrap_or_default()
    }
}