use std::sync::Arc;
use std::io::{BufRead, BufReader, Read};

use raytracer_impl::types::{ V2, V3 };
use crate::ObjError;

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

#[derive(Default)]
pub struct ObjShared {
    pub vertices: Vec<V3>,
    pub uv: Vec<V2>,
}

pub struct ObjGroup {
    pub name: String,
    pub faces: Vec<ObjFace>,
    pub shared: Arc<ObjShared>,
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

fn clean(line: &str) -> String {
    line.trim().to_string()
}

pub struct ObjFile {
    pub mtllib: Option<String>,
    pub groups: Vec<ObjGroup>,
    pub shared: Arc<ObjShared>,
}

/// Braindead OBJ parser, supports o, v, vt & f directives only.
#[derive(Default)]
struct ObjFileParseState {
    // File-level directives
    mtllib: Option<String>,
    vertices: Vec<V3>,
    uv: Vec<V2>,

    // Object-level directives
    group_name: Option<String>,
    faces: Vec<ObjFace>,
    mtl: Option<String>,

    // A placeholder for shared vertex/uv data
    // while we collect all vertices + uv coordinates as we process the input.
    shared: Arc<ObjShared>,
    
    // Completed groups
    groups: Vec<ObjGroup>,
}

impl ObjFileParseState {

    fn try_push_group(&mut self) {
        if self.faces.len() == 0 {
            return;
        }

        self.groups.push(ObjGroup {
            name: self.group_name.take().unwrap_or_else(|| "default".to_string()),
            faces: std::mem::replace(&mut self.faces, Vec::new()),
            shared: self.shared.clone(),
        });
    }

    fn complete(self) -> ObjFile {
        let mtllib = self.mtllib;
        let mut groups = self.groups;
        
        // Fix shared data references
        let shared = Arc::new(ObjShared { vertices: self.vertices, uv: self.uv });
        for group in groups.iter_mut() {
            group.shared = shared.clone();
        }

        ObjFile { mtllib, groups, shared }
    }
}

pub fn parse_obj_file(source: &mut dyn Read) -> Result<ObjFile, ObjError> {

    let mut state = ObjFileParseState::default();

    for (line_no, line) in BufReader::new(source).lines().enumerate() {
        let line = line?;
        let line = line.trim();
        // Skip comments
        if line.starts_with("#") {
            continue;
        }
        let directive = line.split_once(' ');
        match directive {
            // mtllib directive
            Some(("mtllib", path)) => {
                state.mtllib = Some(clean(path));
            },
            // usemtl directive
            Some(("usemtl", name)) => {
                state.mtl = Some(clean(name));
            },
            // Face group
            Some(("g", name)) => {
                state.try_push_group();
                state.group_name = Some(clean(name))
            },
            // Vertex
            Some(("v", data)) => {
                let [x, y, z] = try_parse_elements(data)
                    .ok_or_else(|| ObjError::General(format!("Unable to parse vertex on line {line_no}: {data}")))?;
                state.vertices.push(V3(x, y, z));
            },
            // Texture vertex
            Some(("vt", data)) => {
                let [u, v] = try_parse_elements(data)
                    .ok_or_else(|| ObjError::General(format!("Unable to parse texture vertex on line {line_no}: {data}")))?;
                state.uv.push(V2(u, v));
            },
            // Vertex normals
            Some(("vn", _)) => {
                // TODO
            },
            // Face
            Some(("f", data)) => {
                let mtl = state.mtl.clone();
                let [a, b, c] = try_parse_elements(data)
                    .ok_or_else(|| ObjError::General(format!("Unable to parse face on line {line_no}: {data}")))?;
                state.faces.push(ObjFace { a, b, c, mtl });
            },
            _ => {}
        }
    }

    // Emit the last group
    state.try_push_group();

    Ok(state.complete())
}

pub struct MtlFile {
    pub materials: Vec<ObjMaterial>,
}

/// Braindead MTL parser, supports newmtl, Kd and Kd_map directives only.
#[derive(Default)]
struct MtlFileParseState {
    name: Option<String>,
    ambient_color: V3,
    specular_color: V3,
    diffuse_color: V3,
    diffuse_color_map: Option<String>,

    materials: Vec<ObjMaterial>,
}

impl MtlFileParseState {

    fn try_push_material(&mut self) {
        if self.name.is_none() {
            return;
        }

        self.materials.push(ObjMaterial {
            name: self.name.take().unwrap(),
            ambient_color: std::mem::replace(&mut self.ambient_color, V3::ZERO),
            specular_color: std::mem::replace(&mut self.specular_color, V3::ZERO),
            diffuse_color: std::mem::replace(&mut self.diffuse_color, V3::ZERO),
            diffuse_color_map: self.diffuse_color_map.take(),
        });
    }

    fn complete(self) -> MtlFile {
        MtlFile { materials: self.materials }
    }
}

pub fn parse_mtl_file(source: &mut dyn Read) -> Result<MtlFile, ObjError> {

    let mut state = MtlFileParseState::default();

    for (line_no, line) in BufReader::new(source).lines().enumerate() {
        let line = line?;
        let line = line.trim();
        // Skip comments
        if line.starts_with("#") {
            continue;
        }
        let directive = line.split_once(' ');
        match directive {
            // Object
            Some(("newmtl", name)) => {
                // Starting a new object?
                state.try_push_material();
                state.name = Some(clean(name));
            },
            // Ambient color
            Some(("Ka", data)) => {
                let [r, g, b] = try_parse_elements(data)
                    .ok_or_else(|| ObjError::General(format!("Unable to parse Ka on line {line_no}: {data}")))?;
                state.ambient_color = V3(r, g, b);
            },
            // Specular color
            Some(("Ks", data)) => {
                let [r, g, b] = try_parse_elements(data)
                    .ok_or_else(|| ObjError::General(format!("Unable to parse Ks on line {line_no}: {data}")))?;
                state.specular_color = V3(r, g, b);
            },
            // Diffuse color
            Some(("Kd", data)) => {
                let [r, g, b] = try_parse_elements(data)
                    .ok_or_else(|| ObjError::General(format!("Unable to parse Kd on line {line_no}: {data}")))?;
                state.diffuse_color = V3(r, g, b);
            },
            // Diffuse color map
            Some(("map_Kd", data)) => { 
                state.diffuse_color_map = Some(clean(data));
            },
            _ => {}
        }
    }

    // Emit the last object
    state.try_push_material();

    Ok(state.complete())
}
