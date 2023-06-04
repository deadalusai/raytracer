use std::collections::{HashMap, HashSet};

use crate::shapes::{MeshFace};
use crate::texture::{MeshTexture};
use crate::implementation::{ColorMap};
use crate::types::{IntoArc};
use crate::obj_format::{ObjObject, ObjMaterial};

#[derive(Default)]
pub struct ObjMeshBuilder {
    objects: HashMap<String, ObjObject>,
    materials: HashMap<String, ObjMaterial>,
    color_maps: HashMap<String, std::sync::Arc<dyn ColorMap>>,
}

impl ObjMeshBuilder {
    pub fn load_objects_from_string(&mut self, obj_source: &str) {
        let objects = crate::obj_format::parse_obj_file(obj_source.as_bytes()).expect("parse obj");
        for obj in objects.into_iter() {
            self.objects.insert(obj.name.clone(), obj);
        }
    }

    pub fn load_materials_from_string(&mut self, mtl_source: &str) {
        let materials = crate::obj_format::parse_mtl_file(mtl_source.as_bytes()).expect("parse mtl");
        for mtl in materials.into_iter() {
            self.materials.insert(mtl.name.clone(), mtl);
        }
    }

    pub fn add_color_map(&mut self, name: &str, color_map: impl IntoArc<dyn ColorMap>) {
        self.color_maps.insert(name.to_string(), color_map.into_arc());
    }

    pub fn build_mesh_and_materials(&self, object_name: &str) -> (Vec<MeshFace>, Vec<MeshTexture>) {

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

            let mtl_id = face.mtl.as_ref().and_then(|name| textures.iter().position(|m| &m.name == name));
            
            faces.push(MeshFace {
                a: get_vertex(face.a.vertex_index),
                b: get_vertex(face.b.vertex_index),
                c: get_vertex(face.c.vertex_index),
                a_uv: get_uv_vertex(face.a.uv_index),
                b_uv: get_uv_vertex(face.b.uv_index),
                c_uv: get_uv_vertex(face.c.uv_index),
                material_id: mtl_id,
            });
        }

        (faces, textures)
    }
}