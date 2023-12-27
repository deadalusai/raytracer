use eframe::egui;
use raytracer_samples::scene::SceneControlCollection;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Settings {
    pub scene: usize,
    pub width: usize,
    pub height: usize,
    pub chunk_count: u32,
    pub thread_count: u32,
    pub samples_per_pixel: u32,
    pub camera_fov: f32,
    pub camera_lens_radius: f32,
    pub camera_angle_adjust_v: f32,
    pub camera_angle_adjust_h: f32,
    pub camera_focus_dist_adjust: f32,
    pub max_reflections: u32,
    pub scale_render_to_window: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            scene: Default::default(),
            width: 1024,
            height: 768,
            chunk_count: 128,
            thread_count: 4,
            samples_per_pixel: 1,
            camera_fov: 45.0,
            camera_lens_radius: 0.1,
            camera_angle_adjust_v: 0.0,
            camera_angle_adjust_h: 0.0,
            camera_focus_dist_adjust: 0.0,
            max_reflections: 5,
            scale_render_to_window: true,
        }
    }
}

pub struct SettingsWidget<'a> {
    settings: &'a mut Settings,
    scene_configs: &'a mut [SceneControlCollection],
}

impl<'a> SettingsWidget<'a> {
    pub fn new(settings: &'a mut Settings, scene_configs: &'a mut [SceneControlCollection]) -> SettingsWidget<'a> {
        SettingsWidget { settings, scene_configs }
    }
}

impl<'a> egui::Widget for SettingsWidget<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        egui::Grid::new("settings_grid")
            .num_columns(2)
            .spacing([10.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                
                let st = self.settings;
                let configs = self.scene_configs;
                
                // Scene
                ui.label("Scene");
                egui::ComboBox::from_id_source("scene_combo")
                    .selected_text(configs[st.scene].name.to_string())
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        for (i, c) in configs.iter().enumerate() {
                            ui.selectable_value(&mut st.scene, i, c.name.to_string());
                        }
                    });
                ui.end_row();

                // Scene-specific controls
                for c in configs[st.scene].controls.iter_mut() {
                    ui.label(&c.name);
                    use raytracer_samples::scene::SceneControlType::*;
                    ui.add(match c.control_type {
                        Range(min, max) => egui::DragValue::new(&mut c.value).clamp_range(min..=max),
                    });
                    ui.end_row();
                }

                // Render size
                ui.label("Render size");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut st.width).clamp_range(0..=2048).suffix("px"));
                    ui.add(egui::DragValue::new(&mut st.height).clamp_range(0..=2048).suffix("px"));
                });
                ui.end_row();
                
                // Render threads
                ui.label("Render threads");
                ui.add(egui::DragValue::new(&mut st.thread_count).clamp_range(1..=16));
                ui.end_row();
                
                // Render chunks
                ui.label("Render chunks");
                ui.add(egui::DragValue::new(&mut st.chunk_count).clamp_range(1..=256));
                ui.end_row();
                
                // Samples per pixel
                ui.label("Samples per pixel");
                ui.add(egui::DragValue::new(&mut st.samples_per_pixel).clamp_range(1..=1000));
                ui.end_row();
                
                // Camera aperture
                ui.label("Camera FOV");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut st.camera_fov)
                        .clamp_range(1.0..=100.0)
                        .speed(0.05)
                        .max_decimals(2)
                        .suffix("°"));
                });
                ui.end_row();
                
                // Camera aperture
                ui.label("Camera lens radius");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut st.camera_lens_radius)
                        .clamp_range(0.0..=1.5)
                        .speed(0.005)
                        .max_decimals(3));

                    if st.samples_per_pixel == 1 {
                        ui.add(egui::Label::new(egui::RichText::new("⚠").color(egui::Color32::RED)))
                            .on_hover_text("Camera lens diameter ignored with 1 sample per ray");
                    }
                });
                ui.end_row();
                
                // Camera focus
                ui.label("Camera focus");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut st.camera_focus_dist_adjust)
                        .clamp_range(-10.0..=10.0)
                        .speed(0.05)
                        .max_decimals(3));
                });
                ui.end_row();
                
                // Camera angle
                ui.label("Camera angle");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut st.camera_angle_adjust_h)
                        .clamp_range(-180..=180)
                        .speed(0.5)
                        .max_decimals(3)
                        .suffix("°"));

                    ui.add(egui::DragValue::new(&mut st.camera_angle_adjust_v)
                        .clamp_range(-90.0..=90.0)
                        .speed(0.5)
                        .max_decimals(3)
                        .suffix("°"));
                });
                ui.end_row();

                // Max reflections
                ui.label("Max reflections");
                ui.add(egui::DragValue::new(&mut st.max_reflections)
                    .clamp_range(1..=25));
                ui.end_row();

                // Scale to window
                ui.label("Scaling");
                ui.add(egui::Checkbox::new(&mut st.scale_render_to_window, "Scale to window"));
                ui.end_row();
                
                // Reset button
                ui.label("Reset");
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
                    if ui.button("Reset").clicked() {
                        *st = Settings::default();
                        for c in configs.iter_mut() {
                            c.reset()
                        }
                    }
                });
                ui.end_row();
            })
            .response
    }
}