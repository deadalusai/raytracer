use std::io::{BufRead, Read};

use super::V3;

#[derive(Debug)]
pub enum ParseMeshErr {
    InvalidFace(&'static str),
    InvalidVertex(&'static str),
    IoError(std::io::Error),
    InvalidIndex(std::num::ParseIntError),
    InvalidFloat(std::num::ParseFloatError)
}

impl std::convert::From<std::io::Error> for ParseMeshErr {
    fn from(err: std::io::Error) -> Self {
        ParseMeshErr::IoError(err)
    }
}

impl std::convert::From<std::num::ParseIntError> for ParseMeshErr {
    fn from(err: std::num::ParseIntError) -> Self {
        ParseMeshErr::InvalidIndex(err)
    }
}

impl std::convert::From<std::num::ParseFloatError> for ParseMeshErr {
    fn from(err: std::num::ParseFloatError) -> Self {
        ParseMeshErr::InvalidFloat(err)
    }
}

pub struct MeshFile {
    faces: Vec<(usize, usize, usize)>,
    vertices: Vec<V3>,
}

impl MeshFile {
    #[allow(unused)]
    pub fn read_from_string(s: &str) -> Result<MeshFile, ParseMeshErr> {
        parse_mesh_file(s.as_bytes())
    }

    #[allow(unused)]
    pub fn read_from_file(f: &std::fs::File) -> Result<MeshFile, ParseMeshErr> {
        parse_mesh_file(f)
    }

    pub fn get_triangles(&self) -> Vec<(V3, V3, V3)> {
        let mut triangles = Vec::new();
        for face in &self.faces {
            let v1 = self.vertices.get(face.0).expect(&format!("unable to find vertex 0 for face {:?}", face));
            let v2 = self.vertices.get(face.1).expect(&format!("unable to find vertex 1 for face {:?}", face));
            let v3 = self.vertices.get(face.2).expect(&format!("unable to find vertex 2 for face {:?}", face));
            triangles.push((*v1, *v2, *v3));
        }
        triangles
    }
}

pub fn parse_face_line(line: &str) -> Result<(usize, usize, usize), ParseMeshErr> {
    use self::ParseMeshErr::*;
    let mut parts = line.trim().split(char::is_whitespace);
    let a = parts.next().ok_or(InvalidFace("expected vertex 0"))?.parse()?;
    let b = parts.next().ok_or(InvalidFace("expected vertex 1"))?.parse()?;
    let c = parts.next().ok_or(InvalidFace("expected vertex 2"))?.parse()?;
    if parts.next().is_some() {
        return Err(InvalidFace("unexpected extra vertex"));
    }
    Ok((a, b, c))
}

pub fn parse_vertex_line(line: &str) -> Result<V3, ParseMeshErr> {
    use self::ParseMeshErr::*;
    let mut parts = line.trim().split(char::is_whitespace);
    let a = parts.next().ok_or(InvalidVertex("expected x component"))?.parse()?;
    let b = parts.next().ok_or(InvalidVertex("expected y component"))?.parse()?;
    let c = parts.next().ok_or(InvalidVertex("expected z component"))?.parse()?;
    if parts.next().is_some() {
        return Err(InvalidVertex("unexpected extra component"));
    }
    Ok(V3(a, b, c))
}

pub fn parse_mesh_file(source: impl Read) -> Result<MeshFile, ParseMeshErr> {
    let lines = std::io::BufReader::new(source)
        .lines()
        .collect::<Result<Vec<_>, _>>()?;

    // Clean up the input
    let lines = lines.iter()
        .map(|line| line.trim())
        .filter(|line| line.len() > 0);

    // Find the `faces` section and consume it
    let face_lines = lines.clone()
        .skip_while(|x| *x != "faces").skip(1)
        .take_while(|x| *x != "vertices");

    let mut faces = Vec::new();
    for line in face_lines {
        faces.push(parse_face_line(line)?);
    }

    // Find the `vertices` section and consume it
    let vertex_lines = lines
        .skip_while(|x| *x != "vertices").skip(1)
        .take_while(|x| *x != "faces");

    let mut vertices = Vec::new();
    for line in vertex_lines {
        vertices.push(parse_vertex_line(line)?);
    }

    Ok(MeshFile { vertices, faces })
}