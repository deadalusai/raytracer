use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use raytracer_impl::shapes::{Mesh, MeshFace};
use raytracer_impl::texture::{MeshTexture, MeshTextureSet, ColorMap};
use super::format::{ObjObject, ObjMaterial, MtlFile, ObjFile};
use crate::ObjError;

use std::path::{PathBuf, Path};

pub trait ObjLoader {
    fn load_obj(&self, file_name: &str) -> Result<ObjFile, ObjError>;
    fn load_mtl(&self, file_name: &str) -> Result<MtlFile, ObjError>;
    fn load_color_map(&self, file_name: &str) -> Result<ColorMap, ObjError>;
}

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

    pub fn load_obj_data(&mut self, file_name: &str, loader: &dyn ObjLoader) -> Result<(), ObjError> {
        // Load objects
        let ObjFile { objects, mtllib } = loader.load_obj(file_name)?;
        for obj in objects.into_iter() {
            self.objects.insert(obj.name.clone(), obj);
        }

        // Load associated materials
        if let Some(mtllib) = mtllib {
            let MtlFile { materials } = loader.load_mtl(&mtllib)?;

            for mtl in materials.into_iter() {
                self.materials.insert(mtl.name.clone(), mtl);

                // Load associated color maps
                for bmp_name in self.materials.iter().filter_map(|(_, v)| v.diffuse_color_map.as_ref()) {
                    let map = loader.load_color_map(bmp_name)?;
                    self.color_maps.insert(bmp_name.clone(), Arc::new(map));
                }
            }
        }
        
        Ok(())
    }

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
                    println!("WARNING: Unable to find material {} while loading object {}", name, obj.name);
                    continue;
                },
            };
            let diffuse_color_map = match mtl.diffuse_color_map.as_ref() {
                None => None,
                Some(name) => match self.color_maps.get(name) {
                    Some(map) => Some(map.clone()),
                    None => {
                        println!("WARNING: Unable to find color map {} while loading material {}", name, mtl.name);
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

#[derive(Clone)]
pub struct FileObjLoader {
    root_path: PathBuf,
}

impl FileObjLoader {
    pub fn with_root_path(root_path: &Path) -> FileObjLoader {
        FileObjLoader {
            root_path: root_path.to_path_buf(),
        }
    }
}

impl ObjLoader for FileObjLoader {

    fn load_obj(&self, file_name: &str) -> Result<ObjFile, ObjError> {
        let mut obj_path = self.root_path.clone();
        obj_path.push(file_name);

        let mut file = std::fs::File::open(obj_path)?;
        let obj_file = super::format::parse_obj_file(&mut file)?;
        Ok(obj_file)
    }

    fn load_mtl(&self, file_name: &str) -> Result<MtlFile, ObjError> {
        let mut mtl_path = self.root_path.clone();
        mtl_path.push(file_name);

        let mut file = std::fs::File::open(mtl_path)?;
        let obj_file = super::format::parse_mtl_file(&mut file)?;
        Ok(obj_file)
    }

    fn load_color_map(&self, file_name: &str) -> Result<ColorMap, ObjError> {
        let mut image_path = self.root_path.clone();
        image_path.push(file_name);

        let mut file = std::fs::File::open(image_path)?;
        let color_map = super::color_map::load_bitmap_color_map(&mut file)?;
        Ok(color_map)
    }
}