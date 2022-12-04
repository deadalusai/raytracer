use eframe::{ egui };

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize)]
pub enum TestScene {
    RandomSpheres,
    Simple,
    Planes,
    Mirrors,
    Triangles,
    Mesh,
    Interceptor,
}
const TEST_SCENES: [TestScene; 7] = [
    TestScene::RandomSpheres,
    TestScene::Simple,
    TestScene::Planes,
    TestScene::Mirrors,
    TestScene::Triangles,
    TestScene::Mesh,
    TestScene::Interceptor,
];

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Settings {
    pub scene: TestScene,
    pub width: usize,
    pub height: usize,
    pub chunk_count: u32,
    pub thread_count: u32,
    pub samples_per_pixel: u32,
    pub camera_aperture: f32,
    pub max_reflections: u32,
    pub scale_render_to_window: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            scene: TestScene::RandomSpheres,
            width: 1024,
            height: 768,
            chunk_count: 128,
            thread_count: 4,
            samples_per_pixel: 1,
            camera_aperture: 0.1,
            max_reflections: 5,
            scale_render_to_window: true,
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
                        for v in TEST_SCENES {
                            ui.selectable_value(&mut st.scene, v, format!("{:?}", v));
                        }
                    });
                ui.end_row();

                // Render size
                ui.label("Render size");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut st.width).clamp_range(0..=2048).suffix("px"));
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
                
                // Samples per pixel
                ui.label("Samples per pixel");
                ui.add(egui::DragValue::new(&mut st.samples_per_pixel).clamp_range(1..=1000));
                ui.end_row();
                
                // Camera aperture
                ui.label("Camera aperture");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut st.camera_aperture)
                        .clamp_range(0.0..=15.0)
                        .speed(0.05)
                        .max_decimals(2));

                    if st.samples_per_pixel == 1 {
                        ui.add(egui::Label::new(egui::RichText::new("⚠").color(egui::Color32::RED)))
                            .on_hover_text("Camera aperture ignored with 1 sample per ray");
                    }
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
                    }
                });
                ui.end_row();
            })
            .response
    }
}