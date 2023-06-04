use std::sync::Arc;

use super::implementation::{ HitRecord };
use super::types::{ V2, V3 };

pub enum Texture {
    Color(ColorTexture),
    Checker(CheckerTexture),
    UvTest(UvTestTexture),
    XyzTestTexture(XyzTestTexture),
    MeshTextureSet(MeshTextureSet),
}

impl Texture {
    pub fn value(&self, hit_record: &HitRecord) -> V3 {
        todo!()
    }
}

// Constant colors

#[derive(Clone)]
pub struct ColorTexture(pub V3);

impl ColorTexture {
    fn color_value(&self) -> V3 {
        self.0
    }
}

// Checker texture

#[derive(Clone)]
pub struct CheckerTexture {
    size: f32,
    odd: Arc<Texture>,
    even: Arc<Texture>,
}

impl CheckerTexture {
    pub fn new(size: f32, odd: Arc<Texture>, even: Arc<Texture>) -> Self {
        Self { size, odd, even }
    }
}

impl CheckerTexture {
    fn checker_value(&self, hit_record: &HitRecord) -> V3 {
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

impl UvTestTexture {
    fn uv_test_value(&self, hit_record: &HitRecord) -> V3 {
        let V2(u, v) = hit_record.uv;
        V3(u, v, 1.0 - u - v)
    }
}

pub struct XyzTestTexture(pub f32);

impl XyzTestTexture {
    fn xyz_test_value(&self, hit_record: &HitRecord) -> V3 {
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

impl ColorMap {
    fn uv_to_value(&self, u: f32, v: f32) -> V3 {
        let x = (u * self.width as f32) as usize;
        let y = (v * self.height as f32) as usize;
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

impl MeshTexture {
    fn value(&self, hit_record: &HitRecord) -> V3 {
        match self.diffuse_color_map {
            Some(ref map) => map.uv_to_value(hit_record.uv.0, hit_record.uv.1),
            None => self.diffuse_color.clone()
        }
    }
}

// A collection of OBJ mtl materials.
// Only supported if the HitRecord specifies a {tex_key}
pub struct MeshTextureSet {
    pub textures: Vec<MeshTexture>,
}

impl MeshTextureSet {
    fn mesh_texture_set_value(&self, hit_record: &HitRecord) -> V3 {
        hit_record.tex_key
            .and_then(|key| self.textures.get(key))
            .map(|tex| tex.value(hit_record))
            .unwrap_or_default()
    }
}