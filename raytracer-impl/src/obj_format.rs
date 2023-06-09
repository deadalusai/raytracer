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
    pub name: String,
    pub vertices: Vec<V3>,
    pub uv: Vec<V2>,
    pub faces: Vec<ObjFace>,
}

pub struct ObjMaterial {
    pub name: String,
    pub ambient_color: V3,
    pub specular_color: V3,
    pub diffuse_color: V3,
    pub diffuse_color_map: Option<String>,
}

#[derive(Default, Copy, Clone)]
pub struct ObjVertex {
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

impl std::str::FromStr for ObjVertex {
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
        Ok(ObjVertex { vertex_index, uv_index, normal_index })
    }
}

pub struct ObjFace {
    pub a: ObjVertex,
    pub b: ObjVertex,
    pub c: ObjVertex,
    pub mtl: Option<String>,
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

pub fn parse_obj_file(source: impl Read) -> Result<Vec<ObjObject>, ObjError> {
    
    let mut objects = Vec::new();

    // Braindead OBJ parser, supports o, v, vt & f directives only.
    let mut name = None;
    let mut vertices = Vec::new();
    let mut uv = Vec::new();
    let mut faces = Vec::new();
    let mut mtl = None;

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
                    objects.push(ObjObject {
                        name,
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
            Some("usemtl") => {
                mtl = Some(line[6..].trim().to_string());
            },
            // Face
            Some("f") => {
                let mtl = mtl.clone();
                let [a, b, c] = try_parse_elements(&line[2..])
                    .ok_or_else(|| ObjError::General(format!("Unable to parse face on line: {line_no}")))?;
                faces.push(ObjFace { a, b, c, mtl });
            },
            _ => {}
        }
    }

    // Emit the last object
    let name = name.unwrap_or_else(|| "default".to_string());
    objects.push(ObjObject {
        name,
        vertices,
        faces,
        uv,
    });

    Ok(objects)
}

pub fn parse_mtl_file(source: impl Read) -> Result<Vec<ObjMaterial>, ObjError> {
    
    let mut materials = Vec::new();

    // Braindead MTL parser, supports newmtl, Kd and Kd_map directives only.
    let mut name: Option<String> = None;
    let mut ambient_color = V3::ZERO;
    let mut specular_color = V3::ZERO;
    let mut diffuse_color = V3::ZERO;
    let mut diffuse_color_map = None;

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
            Some("newmtl") => {
                // Starting a new object?
                if let Some(name) = name.take() {
                    materials.push(ObjMaterial {
                        name: name.to_string(),
                        ambient_color: std::mem::replace(&mut ambient_color, V3::ZERO),
                        specular_color: std::mem::replace(&mut specular_color, V3::ZERO),
                        diffuse_color: std::mem::replace(&mut diffuse_color, V3::ZERO),
                        diffuse_color_map: diffuse_color_map.take(),
                    });
                }
                name = Some(line[6..].trim().to_string());
            },
            // Ambient color
            Some("Ka") => {
                let [r, g, b] = try_parse_elements(&line[2..].trim())
                    .ok_or_else(|| ObjError::General(format!("Unable to parse Ka on line: {line_no}")))?;
                ambient_color = V3(r, g, b);
            },
            // Specular color
            Some("Ks") => {
                let [r, g, b] = try_parse_elements(&line[2..].trim())
                    .ok_or_else(|| ObjError::General(format!("Unable to parse Ks on line: {line_no}")))?;
                specular_color = V3(r, g, b);
            },
            // Diffuse color
            Some("Kd") => {
                let [r, g, b] = try_parse_elements(&line[2..].trim())
                    .ok_or_else(|| ObjError::General(format!("Unable to parse Kd on line: {line_no}")))?;
                diffuse_color = V3(r, g, b);
            },
            // Diffuse color map
            Some("map_Kd") => {
                diffuse_color_map = Some(line[6..].trim().to_string());
            },
            _ => {}
        }
    }

    // Emit the last object
    if let Some(name) = name {
        materials.push(ObjMaterial {
            name,
            ambient_color,
            specular_color,
            diffuse_color,
            diffuse_color_map,
        });
    }

    Ok(materials)
}
