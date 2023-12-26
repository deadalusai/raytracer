use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use raytracer_impl::shapes::{Mesh, MeshFace};
use raytracer_impl::texture::{MeshTexture, MeshTextureSet, ColorMap};
use super::format::{ObjObject, ObjMaterial, MtlFile, ObjFile};
use crate::ObjError;

use std::path::Path;

pub struct MeshAndTextureData {
    pub mesh: Mesh,
    pub texture_set: MeshTextureSet,
}

#[derive(Default)]
pub struct ObjMeshBuilder {
    objects: HashMap<String, ObjObject>,
    materials: HashMap<String, ObjMaterial>,
    color_maps: HashMap<String, Arc<ColorMap>>,
}

impl ObjMeshBuilder {

    pub fn build_mesh_data(&self, object_name: &str) -> MeshAndTextureData {

        let obj = self.objects.get(object_name).expect("Unable to find object");
        
        // Prepare materials as "texture" lookups
        let material_names = obj.faces.iter()
            .filter_map(|o| o.mtl.as_ref())
            .collect::<HashSet<_>>();

        let mut textures = Vec::new();
        for name in material_names {
            let mtl = match self.materials.get(name) {
                Some(mtl) => mtl,
                None => {
                    println!("WARNING: Unable to find material {} while building object {}", name, obj.name);
                    continue;
                },
            };
            let diffuse_color_map = match mtl.diffuse_color_map.as_ref() {
                None => None,
                Some(name) => match self.color_maps.get(name) {
                    Some(map) => Some(map.clone()),
                    None => {
                        println!("WARNING: Unable to find color map {} while building material {}", name, mtl.name);
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

        // Prepare mesh faces
        let get_vertex = |i: usize| obj.vertices.get(i - 1).cloned().unwrap();
        let get_uv_vertex = |oi: Option<usize>| oi.and_then(|i| obj.uv.get(i - 1).cloned()).unwrap_or_default();
        let mut faces = Vec::new();
        for face in obj.faces.iter() {

            let tex_key = face.mtl.as_ref().and_then(|name| textures.iter().position(|m| &m.name == name));
            
            faces.push(MeshFace {
                a: get_vertex(face.a.vertex_index),
                b: get_vertex(face.b.vertex_index),
                c: get_vertex(face.c.vertex_index),
                a_uv: get_uv_vertex(face.a.uv_index),
                b_uv: get_uv_vertex(face.b.uv_index),
                c_uv: get_uv_vertex(face.c.uv_index),
                tex_key,
            });
        }

        MeshAndTextureData {
            mesh: Mesh { faces },
            texture_set: MeshTextureSet { textures },
        }
    }
}

pub fn load_obj_builder(path: impl AsRef<Path>) -> Result<ObjMeshBuilder, ObjError> {
    let mut builder = ObjMeshBuilder::default();

    // Load objects
    let obj_path = path.as_ref();
    let obj_file = load_obj(&obj_path)?;
    for obj in obj_file.objects.into_iter() {
        builder.objects.insert(obj.name.clone(), obj);
    }

    // Load associated materials
    if let Some(mtllib) = obj_file.mtllib {
        let mtl_path = obj_path.parent().unwrap().join(mtllib);
        let mtl_file = load_mtl(&mtl_path)?;
        for mtl in mtl_file.materials.into_iter() {

            // Load associated color map
            if let Some(ref colormap) = mtl.diffuse_color_map {
                let bmp_path = mtl_path.parent().unwrap().join(colormap);
                let bmp_data = load_bmp(&bmp_path)?;
                builder.color_maps.insert(colormap.clone(), Arc::new(bmp_data));
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

pub fn load_bmp(path: impl AsRef<Path>) -> Result<ColorMap, ObjError> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(ObjError::General(format!("load_color_map: expected bmp file at path {}", path.display())));
    }

    let mut file = std::fs::File::open(path)?;
    let bmp_data = super::color_map::load_bitmap_color_map(&mut file)?;
    Ok(bmp_data)
}