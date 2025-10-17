use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use raytracer_impl::shapes::{Mesh, MeshTri};
use raytracer_impl::texture::{MeshTexture, MeshTextureSet, ColorMap};
use super::format::{ObjGroup, ObjMaterial, MtlFile, ObjFile};
use crate::ObjError;

use std::path::Path;

pub struct MeshAndTextureData {
    pub mesh: Arc<Mesh>,
    pub texture_set: Arc<MeshTextureSet>,
}

#[derive(Default)]
pub struct ObjMeshBuilder {
    groups: Vec<ObjGroup>,
    materials: HashMap<String, ObjMaterial>,
    color_maps: HashMap<String, Arc<ColorMap>>,
}

impl ObjMeshBuilder {

    pub fn group_names(&self) -> impl Iterator<Item=&str> {
        self.groups.iter().map(|k| k.name.as_str())
    }

    pub fn build_mesh(&self) -> MeshAndTextureData {
        self.inner_build_mesh(&|_| true)
    }

    pub fn build_mesh_group(&self, group_name: &str) -> MeshAndTextureData {
        self.inner_build_mesh(&|g| g.name == group_name)
    }

    /// Build Mesh and Texture data.
    /// If {group_name} is specified, filter mesh and texture data for that group only.
    fn inner_build_mesh(&self, group_filter: &dyn Fn(&ObjGroup) -> bool) -> MeshAndTextureData {

        let groups = self.groups.iter().filter(|g| group_filter(g));
        
        // Prepare materials as "texture" lookups
        let material_names = groups.clone()
            .flat_map(|g| g.faces.iter())
            .filter_map(|o| o.mtl.as_ref())
            .collect::<HashSet<_>>();

        let mut textures = Vec::new();
        for name in material_names {
            let mtl = match self.materials.get(name) {
                Some(mtl) => mtl,
                None => {
                    println!("WARNING: Unable to find material {}", name);
                    continue;
                },
            };
            let diffuse_color_map = match mtl.diffuse_color_map.as_ref() {
                None => None,
                Some(name) => match self.color_maps.get(name) {
                    Some(map) => Some(map.clone()),
                    None => {
                        println!("WARNING: Unable to find color map {}", name);
                        None
                    },
                }
            };

            textures.push(MeshTexture {
                name: mtl.name.clone(),
                ambient_color: mtl.ambient_color,
                diffuse_color: mtl.diffuse_color,
                diffuse_color_map,
            });
        }

        // Prepare mesh tris
        let mut tris = Vec::new();
        for group in groups {
            for face in group.faces.iter() {
                let tex_key = face.mtl.as_ref().and_then(|name| textures.iter().position(|m| &m.name == name));
                let get_vertex = |i: usize| group.shared.vertices.get(i - 1).cloned().expect("vertex by index");
                let get_uv_vertex = |oi: Option<usize>| oi.and_then(|i| group.shared.uv.get(i - 1).cloned()).unwrap_or_default();
                tris.push(MeshTri {
                    a: get_vertex(face.a.vertex_index),
                    b: get_vertex(face.b.vertex_index),
                    c: get_vertex(face.c.vertex_index),
                    a_uv: get_uv_vertex(face.a.uv_index),
                    b_uv: get_uv_vertex(face.b.uv_index),
                    c_uv: get_uv_vertex(face.c.uv_index),
                    tex_key,
                });
            }
        }

        if tris.len() == 0 {
            panic!("[ObjMeshBuilder::inner_build_mesh] expected at least one face (are you building a vertex group with the wrong name?)");
        }

        MeshAndTextureData {
            mesh: Arc::new(Mesh { tris }),
            texture_set: Arc::new(MeshTextureSet { textures }),
        }
    }
}

pub fn load_obj_builder(path: impl AsRef<Path>) -> Result<ObjMeshBuilder, ObjError> {
    let mut builder = ObjMeshBuilder::default();

    // Load objects
    let obj_path = path.as_ref();
    let obj_file = load_obj(&obj_path)?;
    for group in obj_file.groups.into_iter() {
        builder.groups.push(group);
    }

    // Load associated materials
    if let Some(mtllib) = obj_file.mtllib {
        let mtl_path = obj_path.parent().unwrap().join(mtllib);
        let mtl_file = load_mtl(&mtl_path)?;
        for mtl in mtl_file.materials.into_iter() {

            // Load associated color map
            if let Some(ref colormap) = mtl.diffuse_color_map {
                let path = mtl_path.parent().unwrap().join(colormap);
                let data = load_color_map(&path)?;
                builder.color_maps.insert(colormap.clone(), Arc::new(data));
            }
            
            builder.materials.insert(mtl.name.clone(), mtl);
        }
    }

    Ok(builder)
}

pub fn load_obj(path: impl AsRef<Path>) -> Result<ObjFile, ObjError> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(ObjError::General(format!("load_obj: expected obj file at path {}", path.display())));
    }

    let mut file = std::fs::File::open(path)?;
    let obj_file = super::format::parse_obj_file(&mut file)?;
    Ok(obj_file)
}

pub fn load_mtl(path: impl AsRef<Path>) -> Result<MtlFile, ObjError> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(ObjError::General(format!("load_mtl: expected mtl file at path {}", path.display())));
    }

    let mut file = std::fs::File::open(path)?;
    let mtl_file = super::format::parse_mtl_file(&mut file)?;
    Ok(mtl_file)
}

pub fn load_color_map(path: impl AsRef<Path>) -> Result<ColorMap, ObjError> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(ObjError::General(format!("load_color_map: expected file at path {}", path.display())));
    }

    let format = match path.extension().and_then(image::ImageFormat::from_extension) {
        Some(ext) => ext,
        None      => Err(ObjError::General(format!("load_color_map: Color map type unknown")))?,
    };
    let file = std::fs::File::open(path)?;
    let color_data = crate::color_map::load_color_map(file, format)?;
    Ok(color_data)
}
