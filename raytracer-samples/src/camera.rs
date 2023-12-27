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
