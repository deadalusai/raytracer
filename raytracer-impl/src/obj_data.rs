use std::collections::{HashMap, HashSet};
use std::sync::{Arc};

use crate::shapes::{Mesh, MeshFace};
use crate::texture::{MeshTexture, MeshTextureSet, ColorMap};
use crate::obj_format::{ObjObject, ObjMaterial};

#[derive(Default)]
pub struct ObjMeshBuilder {
    objects: HashMap<String, ObjObject>,
    materials: HashMap<String, ObjMaterial>,
    color_maps: HashMap<String, Arc<ColorMap>>,
}

impl ObjMeshBuilder {
    pub fn load_obj_from_string(&mut self, obj_source: &str) {
        let objects = crate::obj_format::parse_obj_file(obj_source.as_bytes()).expect("parse obj");
        for obj in objects.into_iter() {
            self.objects.insert(obj.name.clone(), obj);
        }
    }

    pub fn load_mtl_from_string(&mut self, mtl_source: &str) {
        let materials = crate::obj_format::parse_mtl_file(mtl_source.as_bytes()).expect("parse mtl");
        for mtl in materials.into_iter() {
            self.materials.insert(mtl.name.clone(), mtl);
        }
    }

    pub fn add_color_map(&mut self, name: &str, color_map: ColorMap) {
        self.color_maps.insert(name.to_string(), Arc::new(color_map));
    }

    pub fn build_mesh_and_texture(&self, object_name: &str) -> (Mesh, MeshTextureSet) {

        let obj = self.objects.get(object_name).expect("Selecting object");
        
        // Prepare materials as "texture" lookups
        let material_names = obj.faces.iter()
            .filter_map(|o| o.mtl.as_ref())
            .collect::<HashSet<_>>();

        let mut textures = Vec::new();
        for name in material_names {
            let mtl = match self.materials.get(name) {
                Some(mtl) => mtl,
                None => {
                    println!("WARNING: Unable to find material {name}");
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

        (Mesh { faces }, MeshTextureSet { textures })
    }
}