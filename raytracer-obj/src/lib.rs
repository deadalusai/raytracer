mod builder;
mod format;
mod color_map;

pub use builder::{ FileObjLoader, ObjMeshBuilder, ObjLoader, MeshAndTextureData };

#[derive(thiserror::Error, Debug)]
pub enum ObjError {
    #[error("Error parsing OBJ file: {0}")]
    General(String),
    #[error("IO Error")]
    IoError(#[from] std::io::Error),
    #[error("Int parse error")]
    IntParseError(#[from] std::num::ParseIntError),
    #[error("Bitmap parse error")]
    BmpParseError(#[from] bmp::BmpError),
}