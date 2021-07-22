use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};

use super::{TriangleList, V3};

#[derive(Debug)]
pub enum ObjParseError {
    ParserError(&'static str),
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
//
// TODO(benf): Support other features of the OBJ format
// - Vertex normals
// - Materials
// - ???

pub struct ObjObject {
    vertices: Vec<V3>,
    faces: Vec<ObjFace>,
}

pub struct ObjFace(usize, usize, usize);

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

    pub fn make_triangle_list(&self, obj_name: &str) -> Option<TriangleList> {
        self.objects.get(obj_name).map(|obj| make_triangles(obj))
    }
}

pub fn parse_triple<T: std::str::FromStr>(line: &str) -> Result<(T, T, T), ObjParseError> {
    let structure_error = || ObjParseError::ParserError("expected triple");
    let parse_error = |_| ObjParseError::ParserError("expected triple");

    let mut parts = line.split(char::is_whitespace);
    let p0 = parts.next().ok_or_else(structure_error)?.parse().map_err(parse_error)?;
    let p1 = parts.next().ok_or_else(structure_error)?.parse().map_err(parse_error)?;
    let p2 = parts.next().ok_or_else(structure_error)?.parse().map_err(parse_error)?;
    if parts.next().is_some() {
        return Err(structure_error());
    }
    Ok((p0, p1, p2))
}

pub fn parse_obj_file(source: impl Read) -> Result<ObjFile, ObjParseError> {
    
    let mut objects = HashMap::new();

    // Braindead OBJ parser, supports o, v & f directives only.
    let mut current_object = None;
    let mut current_vertices = Vec::new();
    let mut current_faces = Vec::new();

    for line in BufReader::new(source).lines() {
        let line = line?;
        let line = line.trim();
        // Skip comments
        if line.starts_with("#") {
            continue;
        }
        let directive = line.chars().next();
        match directive {
            // Object
            Some('o') => {
                if let Some(name) = current_object.take() {
                    let vertices = std::mem::replace(&mut current_vertices, Vec::new());
                    let faces = std::mem::replace(&mut current_faces, Vec::new());
                    objects.entry(name).or_insert(ObjObject { vertices, faces });
                }
                let name = &line[2..];
                current_object = Some(name.to_string());
            },
            // Vertex
            Some('v') => {
                let (x, y, z) = parse_triple(&line[2..])?;
                current_vertices.push(V3(x, y, z));
            },
            // Face
            Some('f') => {
                let (a, b, c) = parse_triple(&line[2..])?;
                current_faces.push(ObjFace(a, b, c));
            },
            _ => {
                continue;
            }
        }
    }

    if let Some(name) = current_object.take() {
        let vertices = std::mem::replace(&mut current_vertices, Vec::new());
        let faces = std::mem::replace(&mut current_faces, Vec::new());
        objects.entry(name).or_insert(ObjObject { vertices, faces });
    }

    // Ignore comments
    Ok(ObjFile { objects })
}

// Convert Obj face/vertex lists into a list of triangles
fn make_triangles(obj: &ObjObject) -> TriangleList {
    let mut tris = Vec::new();

    for &ObjFace(a, b, c) in obj.faces.iter() {
        let va = obj.vertices.get(a - 1).expect(&format!("could not find vertex {}", a));
        let vb = obj.vertices.get(b - 1).expect(&format!("could not find vertex {}", b));
        let vc = obj.vertices.get(c - 1).expect(&format!("could not find vertex {}", c));
        tris.push((*va, *vb, *vc));
    }

    tris.into_boxed_slice()
}