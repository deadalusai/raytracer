use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};

use crate::types::{ V2, V3 };

#[derive(thiserror::Error, Debug)]
pub enum ObjError {
    #[error("Error parsing OBJ file: {0}")]
    General(String),
    #[error("IO Error")]
    IoError(#[from] std::io::Error),
    #[error("Int parse error")]
    IntParseError(#[from] std::num::ParseIntError),
}

// Obj parser
//
// When exporting an OBJ file from Blender
// - Select "Export Wavefront (.obj)"
// - Set objects as "OBJ Objects"
// - Set "Triangulate Faces"
// - Set "Include UVs"
//
// TODO(benf): Support other features of the OBJ format
// - Vertex normals
// - ???
//
// See: https://en.wikipedia.org/wiki/Wavefront_.obj_file
// This parser does not implement the spec correctly
// (even for the elements it supports) and makes some assumptions:
// - every vertex has three components `v x y z`
// - every texture coordinate has two components `vt u v`
// - every face has three components `f a b c` (triangles only)

pub struct ObjObject {
    pub vertices: Vec<V3>,
    pub uv: Vec<V2>,
    pub faces: Vec<TriFace>,
}

#[derive(Default, Copy, Clone)]
pub struct TriVertex {
    pub vertex_index: usize,
    pub uv_index: Option<usize>,
    pub normal_index: Option<usize>,
}

#[derive(thiserror::Error, Debug)]
pub enum VertexParseError {
    #[error("Face vertex: Unexpected number of parts")]
    UnexpectedPartCount,
    #[error("Face vertex: Invalid integer")]
    ParseIntError(#[from] std::num::ParseIntError),
}

impl std::str::FromStr for TriVertex {
    type Err = VertexParseError;
    fn from_str(s: &str) -> Result<Self, VertexParseError> {
        // Parses vertex definitions of the form `v/vt?/vn?`
        let mut parts = s.split("/");
        let vertex_index = match parts.next() {
            None => return Err(VertexParseError::UnexpectedPartCount),
            Some(v) => v.parse()?,
        };
        let uv_index = match parts.next() {
            None => None,
            Some("") => None,
            Some(v) => Some(v.parse()?),
        };
        let normal_index = match parts.next() {
            None => None,
            Some("") => None,
            Some(v) => Some(v.parse()?),
        };
        if parts.next().is_some() {
            return Err(VertexParseError::UnexpectedPartCount);
        }
        Ok(TriVertex { vertex_index, uv_index, normal_index })
    }
}

pub struct TriFace {
    pub a: TriVertex,
    pub b: TriVertex,
    pub c: TriVertex,
}

pub struct ObjFile {
    objects: HashMap<String, ObjObject>,
}

impl ObjFile {
    #[allow(unused)]
    pub fn read_from_string(s: &str) -> Result<Self, ObjError> {
        parse_obj_file(s.as_bytes())
    }

    #[allow(unused)]
    pub fn read_from_file(f: &std::fs::File) -> Result<Self, ObjError> {
        parse_obj_file(f)
    }

    pub fn get_object(&self, name: &str) -> &ObjObject {
        self.objects.get(name).expect("Expected object")
    }
}

pub fn try_parse_elements<T, const N: usize>(line: &str) -> Option<[T; N]>
    where T: std::str::FromStr, T: Default, T: Copy
{
    let mut values = [Default::default(); N];
    let mut parts = line.split(char::is_whitespace);
    for i in 0..N {
        let part = parts.next()?;
        values[i] = part.parse().ok()?;
    }
    if parts.next().is_some() {
        return None;
    }
    Some(values)
}

pub fn parse_obj_file(source: impl Read) -> Result<ObjFile, ObjError> {
    
    let mut objects = HashMap::new();

    // Braindead OBJ parser, supports o, v, vt & f directives only.
    let mut name = None;
    let mut vertices = Vec::new();
    let mut uv = Vec::new();
    let mut faces = Vec::new();

    for (line_no, line) in BufReader::new(source).lines().enumerate() {
        let line = line?;
        let line = line.trim();
        // Skip comments
        if line.starts_with("#") {
            continue;
        }
        let directive = line.split(' ').next();
        match directive {
            // Object
            Some("o") => {
                // Starting a new object?
                if let Some(name) = name.take() {
                    objects.insert(name, ObjObject {
                        vertices: std::mem::replace(&mut vertices, Vec::new()),
                        faces: std::mem::replace(&mut faces, Vec::new()),
                        uv: std::mem::replace(&mut uv, Vec::new()),
                    });
                }
                name = Some(line[1..].trim().to_string());
            },
            // Vertex
            Some("v") => {
                let [x, y, z] = try_parse_elements(&line[2..])
                    .ok_or_else(|| ObjError::General(format!("Unable to parse vertex on line: {line_no}")))?;
                vertices.push(V3(x, y, z));
            },
            // Texture vertex
            Some("vt") => {
                let [u, v] = try_parse_elements(&line[3..])
                    .ok_or_else(|| ObjError::General(format!("Unable to parse texture vertex on line: {line_no}")))?;
                uv.push(V2(u, v));
            },
            // Vertex normals
            Some("vn") => {
                // TODO
            },
            // Face
            Some("f") => {
                let [a, b, c] = try_parse_elements(&line[2..])
                    .ok_or_else(|| ObjError::General(format!("Unable to parse face on line: {line_no}")))?;
                faces.push(TriFace { a, b, c });
            },
            _ => {}
        }
    }

    // Emit the last object
    let name = name.unwrap_or_else(|| "default".to_string());
    objects.insert(name, ObjObject {
        vertices,
        faces,
        uv,
    });

    // Ignore comments
    Ok(ObjFile { objects })
}
