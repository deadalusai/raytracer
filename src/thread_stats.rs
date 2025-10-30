use std::time::Duration;

use eframe::{ egui };

use crate::format::FormattedDuration;

#[derive(Clone)]
pub struct ThreadStats {
    pub id: u32,
    pub total_time: Duration,
    pub total_chunks_rendered: u32,
}

impl<'a> egui::Widget for ThreadStats {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let label = format!(
            "Thread {}: {} chunks rendered in {}",
            self.id,
            self.total_chunks_rendered,
            FormattedDuration(self.total_time)
        );
        ui.horizontal(|ui| ui.label(label)).response
    }
}
