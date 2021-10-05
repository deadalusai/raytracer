use eframe::{ egui };

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize)]
pub enum TestScene {
    RandomSpheres,
    Simple,
    Planes,
    Mirrors,
    Triangles,
    Mesh,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Settings {
    pub scene: TestScene,
    pub width: u32,
    pub height: u32,
    pub chunk_count: u32,
    pub thread_count: u32,
    pub samples_per_ray: u32,
    pub camera_aperture: f32,
    pub max_reflections: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            scene: TestScene::RandomSpheres,
            width: 1024,
            height: 768,
            chunk_count: 128,
            thread_count: 8,
            samples_per_ray: 1,
            camera_aperture: 0.1,
            max_reflections: 5,
        }
    }
}

pub struct SettingsWidget<'a> {
    settings: &'a mut Settings,
}

impl<'a> SettingsWidget<'a> {
    pub fn new(settings: &'a mut Settings) -> SettingsWidget<'a> {
        SettingsWidget { settings }
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
                
                // Scene
                ui.label("Scene");
                egui::ComboBox::from_id_source("scene_combo")
                    .selected_text(format!("{:?}", st.scene))
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        let values = [
                            TestScene::RandomSpheres,
                            TestScene::Simple,
                            TestScene::Planes,
                            TestScene::Mirrors,
                            TestScene::Triangles,
                            TestScene::Mesh,
                        ];
                        for v in values {
                            ui.selectable_value(&mut st.scene, v, format!("{:?}", v));
                        }
                    });
                ui.end_row();
                
                // Render size
                ui.label("Render size");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut st.width).clamp_range(0..=2048).suffix("px"));
                    ui.label("x");
                    ui.add(egui::DragValue::new(&mut st.height).clamp_range(0..=2048).suffix("px"));
                });
                ui.end_row();
                
                // Render threads
                ui.label("Render threads");
                ui.add(egui::DragValue::new(&mut st.thread_count).clamp_range(1..=8));
                ui.end_row();
                
                // Render chunks
                ui.label("Render chunks");
                ui.add(egui::DragValue::new(&mut st.chunk_count).clamp_range(1..=256));
                ui.end_row();
                
                // Samples per ray
                ui.label("Samples per ray");
                ui.add(egui::DragValue::new(&mut st.samples_per_ray).clamp_range(1..=25));
                ui.end_row();
                
                // Camera aperture
                ui.label("Camera aperture");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut st.camera_aperture)
                        .clamp_range(0.0..=15.0)
                        .speed(0.05)
                        .max_decimals(2));

                    if st.samples_per_ray == 1 {
                        ui.add(egui::Label::new("⚠")
                            .text_color(egui::Color32::RED))
                            .on_hover_text("Camera aperture ignored with 1 sample per ray");
                    }
                });
                ui.end_row();

                // Max reflections
                ui.label("Max reflections");
                ui.add(egui::DragValue::new(&mut st.max_reflections)
                    .clamp_range(1..=25));
                ui.end_row();
            })
            .response
    }
}