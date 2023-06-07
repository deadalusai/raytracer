use std::sync::Arc;

use super::implementation::{ Texture, HitRecord };
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

pub struct UvTestTexture;

impl Texture for UvTestTexture {
    fn value(&self, hit_record: &HitRecord) -> V3 {
        let V2(u, v) = hit_record.uv;
        V3(u, v, 1.0 - u - v)
    }
}

pub struct XyzTestTexture(pub f32);

impl Texture for XyzTestTexture {
    fn value(&self, hit_record: &HitRecord) -> V3 {
        fn map_into_range(max: f32, v: f32) -> f32 {
            // Values on {-max..0..+max} range mapped to {0..1} range, so {0} always corresponds to {0.5}
            0.5 + (v / max / 2.0)
        }
        let V3(x, y, z) = hit_record.p;
        V3(map_into_range(self.0, x),
           map_into_range(self.0, y),
           map_into_range(self.0, z))
    }
}


// Image texture / color maps

pub struct ColorMap {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<V3>,
}

impl Texture for ColorMap {
    fn value(&self, hit_record: &HitRecord) -> V3 {
        let x = (hit_record.uv.0 * self.width as f32) as usize;
        let y = (hit_record.uv.1 * self.height as f32) as usize;
        let offset = y * self.width + x;
        self.pixels.get(offset).cloned().unwrap_or_default()
    }
}

/// A texture loaded from an OBJ mtl ile
pub struct MeshTexture {
    pub name: String,
    pub diffuse_color: V3,
    pub diffuse_color_map: Option<Arc<ColorMap>>,
}

impl Texture for MeshTexture {
    fn value(&self, hit_record: &HitRecord) -> V3 {
        match self.diffuse_color_map {
            Some(ref map) => map.value(hit_record),
            None => self.diffuse_color.clone()
        }
    }
}

// A collection of OBJ mtl materials.
// Only supported if the HitRecord specifies a {tex_key}
pub struct MeshTextureSet {
    pub textures: Vec<MeshTexture>,
}

const NOT_FOUND_COLOR: V3 = V3(1.0, 0.41, 0.70); // #FF69B4

impl Texture for MeshTextureSet {
    fn value(&self, hit_record: &HitRecord) -> V3 {
        hit_record.tex_key
            .and_then(|key| self.textures.get(key))
            .map(|mat| mat.value(hit_record))
            // texture not found
            .unwrap_or(NOT_FOUND_COLOR)
    }
}