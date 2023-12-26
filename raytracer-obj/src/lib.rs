mod builder;
mod format;
mod color_map;

pub use builder::{ ObjMeshBuilder, MeshAndTextureData, load_obj_builder, load_obj, load_mtl, load_color_map };

#[derive(thiserror::Error, Debug)]
pub enum ObjError {
    #[error("Error parsing OBJ file: {0}")]
    General(String),
    #[error("IO Error")]
    IoError(#[from] std::io::Error),
    #[error("Int parse error")]
    IntParseError(#[from] std::num::ParseIntError),
    #[error("Image read error")]
    BmpParseError(#[from] image::ImageError),
}