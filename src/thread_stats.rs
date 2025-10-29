use eframe::{ egui };

#[derive(Clone)]
pub struct ThreadStats {
    pub id: u32,
    pub total_time_secs: f64,
    pub total_chunks_rendered: u32,
}

impl<'a> egui::Widget for ThreadStats {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let label = format!("Thread {}: {} chunks rendered in {:.1}s", self.id, self.total_chunks_rendered, self.total_time_secs);
        ui.horizontal(|ui| ui.label(label)).response
    }
}
