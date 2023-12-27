use std::collections::HashMap;

use raytracer_impl::types::V3;
use raytracer_impl::implementation::Camera;

use crate::util::deg_to_rad;

// Camera configuration

pub struct CameraConfiguration {
    pub width: f32,
    pub height: f32,
    pub lens_radius: f32,
    pub fov: f32,
    pub angle_adjust_v: f32,
    pub angle_adjust_h: f32,
    pub focus_dist_adjust: f32,
}

impl CameraConfiguration {
    pub fn aspect_ratio(&self) -> f32 {
        self.width / self.height
    }

    pub fn make_camera(&self, look_to: V3, default_look_from: V3) -> Camera {

        let look_from = {
            // Translate into rotation space
            let p = default_look_from - look_to;

            // The vertical axis (to rotate about horizontally)
            let v_axis = V3::POS_Y;
            let p = p.rotate_about_axis(v_axis, deg_to_rad(self.angle_adjust_h));
            
            // The horizontal axis (to rotate about vertically)
            let w = (V3::ZERO - p).unit();             // Vector to origin 
            let h_axis = V3::cross(v_axis, w).unit();  // Vector to camera right
            let p = p.rotate_about_axis(h_axis, deg_to_rad(self.angle_adjust_v));

            // Translate into world space
            p + look_to
        };
        let dist_to_focus = (look_from - look_to).length() + self.focus_dist_adjust;
        
        Camera::new(look_from, look_to, self.fov, self.aspect_ratio(), self.lens_radius, dist_to_focus)
    }
}


// Scene Configuration Controls

// TODO: Error type
#[derive(Debug)]
pub struct CreateSceneError(String);

pub trait SceneFactory {
    fn create_controls(&self) -> SceneControlCollection;
    fn create_scene(&self, camera_config: &CameraConfiguration, config: &SceneConfiguration) -> Result<raytracer_impl::implementation::Scene, CreateSceneError>;
}

pub struct SceneConfiguration {
    values: HashMap<String, f32>
}

impl SceneConfiguration {
    pub fn get(&self, key: &str) -> f32 {
        self.values.get(key).expect("scene control value").clone()
    }
}

pub struct SceneControlCollection {
    pub name: String,
    pub controls: Vec<SceneControl>,
}

impl SceneControlCollection {
    pub fn collect_configuration(&self) -> SceneConfiguration {
        SceneConfiguration { 
            values: self.controls.iter().map(|c| (c.name.clone(), c.value)).collect()
        }
    }

    pub fn reset(&mut self) {
        for c in self.controls.iter_mut() {
            c.reset();
        }
    }
}

pub enum SceneControlType {
    Range(f32, f32),
}

pub struct SceneControl {
    pub name: String,
    pub control_type: SceneControlType,
    pub default: f32,
    pub value: f32,
}

impl SceneControl {
    pub fn range(name: &str, min: f32, max: f32) -> Self {
        Self {
            name: name.into(),
            control_type: SceneControlType::Range(min, max),
            default: 0.0,
            value: 0.0,
        }
    }

    pub fn with_default(self, default: f32) -> Self {
        Self { default, value: default, ..self }
    }

    pub fn reset(&mut self) {
        self.value = self.default;
    }
}

// Wrapper implementation

type BasicSceneFactoryFn = fn(&CameraConfiguration) -> raytracer_impl::implementation::Scene;

pub struct BasicSceneFactory {
    name: String,
    factory: BasicSceneFactoryFn,
}

impl BasicSceneFactory {
    pub fn new(name: impl Into<String>, factory: BasicSceneFactoryFn) -> Self {
        BasicSceneFactory { name: name.into(), factory }
    }
}

impl SceneFactory for BasicSceneFactory {
    fn create_controls(&self) -> SceneControlCollection {
        SceneControlCollection {
            name: self.name.clone(),
            controls: Vec::default(),
        }
    }

    fn create_scene(&self, camera_config: &CameraConfiguration, _config: &SceneConfiguration) -> Result<raytracer_impl::implementation::Scene, CreateSceneError> {
        Ok((self.factory)(camera_config))
    }
}

