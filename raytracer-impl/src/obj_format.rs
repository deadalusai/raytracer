use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};

use super::types::V3;

#[derive(Debug)]
pub enum ObjParseError {
    ParserError(String),
    IoError(std::io::Error)
}

impl std::convert::From<std::io::Error> for ObjParseError {
    fn from(err: std::io::Error) -> Self {
        ObjParseError::IoError(err)
    }
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
    vertices: Vec<V3>,
    faces: Vec<TriFace>,
    uv: Vec<(f32, f32)>,
}

#[derive(Default, Copy, Clone)]
pub struct TriVertex {
    v_index: usize,
    uv_index: Option<usize>,
}

impl std::str::FromStr for TriVertex {
    type Err = ObjParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parser_error = || ObjParseError::ParserError("Expected vertex index".into());
        let mut parts = s.split("/");
        let v_index = match parts.next() {
            None => return Err(parser_error()),
            Some(v) => v.parse().map_err(|_| parser_error())?,
        };
        let uv_index = match parts.next() {
            None => None,
            Some(v) => Some(v.parse().map_err(|_| parser_error())?),
        };
        Ok(TriVertex { v_index, uv_index })
    }
}

pub struct TriFace {
    a: TriVertex,
    b: TriVertex,
    c: TriVertex,
}

pub struct ObjFile {
    objects: HashMap<String, ObjObject>,
}

impl ObjFile {
    #[allow(unused)]
    pub fn read_from_string(s: &str) -> Result<Self, ObjParseError> {
        parse_obj_file(s.as_bytes())
    }

    #[allow(unused)]
    pub fn read_from_file(f: &std::fs::File) -> Result<Self, ObjParseError> {
        parse_obj_file(f)
    }

    pub fn make_triangle_list(&self, obj_name: &str) -> Result<Vec<(V3, V3, V3)>, String> {
        let obj = self.objects.get(obj_name).ok_or_else(|| format!("Could not find object {}", obj_name))?;
        make_triangles(obj)
    }
}

pub fn parse_elements<T, const N: usize>(line: &str) -> Result<[T; N], ObjParseError>
    where T: std::str::FromStr, T: Default, T: Copy
{
    let structure_error = || ObjParseError::ParserError(format!("expected {} values", N));
    let parse_error = |_| ObjParseError::ParserError(format!("error parsing {} values", N));

    let mut values = [Default::default(); N];
    let mut parts = line.split(char::is_whitespace);
    for i in 0..N {
        values[i] = parts.next().ok_or_else(structure_error)?.parse().map_err(parse_error)?;
    }
    if parts.next().is_some() {
        return Err(structure_error());
    }
    Ok(values)
}

pub fn parse_obj_file(source: impl Read) -> Result<ObjFile, ObjParseError> {
    
    let mut objects = HashMap::new();

    // Braindead OBJ parser, supports o, v & f directives only.
    let mut current_object = None;
    let mut current_vertices = Vec::new();
    let mut current_uv = Vec::new();
    let mut current_faces = Vec::new();

    for line in BufReader::new(source).lines() {
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
                if let Some(name) = current_object.take() {
                    objects.insert(name, ObjObject {
                        vertices: std::mem::replace(&mut current_vertices, Vec::new()),
                        faces: std::mem::replace(&mut current_faces, Vec::new()),
                        uv: std::mem::replace(&mut current_uv, Vec::new()),
                    });
                }
                let name = &line[2..];
                current_object = Some(name.to_string());
            },
            // Vertex
            Some("v") => {
                let [x, y, z] = parse_elements(&line[2..])?;
                current_vertices.push(V3(x, y, z));
            },
            // Texture vertex
            Some("vt") => {
                let [u, v] = parse_elements(&line[3..])?;
                current_uv.push((u, v));
            },
            // Face
            Some("f") => {
                let [a, b, c] = parse_elements(&line[2..])?;
                current_faces.push(TriFace { a, b, c });
            },
            _ => {}
        }
    }

    if let Some(name) = current_object.take() {
        objects.insert(name, ObjObject {
            vertices: current_vertices,
            faces: current_faces,
            uv: current_uv,
        });
    }

    // Ignore comments
    Ok(ObjFile { objects })
}

// Convert Obj face/vertex lists into a list of triangles
fn make_triangles(obj: &ObjObject) -> Result<Vec<(V3, V3, V3)>, String> {
    let vert_error = |i, v| format!("face {}: could not find vertex {}", i, v);
    let mut tris = Vec::new();
    for (i, face) in obj.faces.iter().enumerate() {
        let va = obj.vertices.get(face.a.v_index - 1).ok_or_else(|| vert_error(i, face.a.v_index))?;
        let vb = obj.vertices.get(face.b.v_index - 1).ok_or_else(|| vert_error(i, face.b.v_index))?;
        let vc = obj.vertices.get(face.c.v_index - 1).ok_or_else(|| vert_error(i, face.c.v_index))?;
        tris.push((va.clone(), vb.clone(), vc.clone()))
    }
    Ok(tris)
}