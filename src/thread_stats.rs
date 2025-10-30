use std::time::Duration;
use std::fmt::Display;

use eframe::egui;

use crate::format::FormattedDuration;

#[derive(Clone)]
pub struct ThreadStats {
    pub id: u32,
    pub total_time: Duration,
    pub total_chunks_rendered: u32,
}

impl egui::Widget for ThreadStats {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let style = egui::Style::default();
        let mut layout_job = egui::text::LayoutJob::default();

        append_text(&mut layout_job, &style, rich_text("Thread "));
        append_text(&mut layout_job, &style, rich_text(self.id).color(HIGHLIGHT));
        append_text(&mut layout_job, &style, rich_text(": "));
        append_text(&mut layout_job, &style, rich_text(self.total_chunks_rendered).color(HIGHLIGHT));
        append_text(&mut layout_job, &style, rich_text(" chunks rendered in "));
        append_text(&mut layout_job, &style, rich_text(FormattedDuration(self.total_time)).color(HIGHLIGHT));

        ui.label(layout_job)
    }
}

const HIGHLIGHT: egui::Color32 = egui::Color32::LIGHT_GRAY;

fn append_text(layout: &mut egui::text::LayoutJob, style: &egui::Style, rich: egui::RichText) {
    rich.append_to(layout, &style, egui::FontSelection::Default, egui::Align::LEFT);
}

fn rich_text(element: impl Display) -> egui::RichText {
    let formatted = format!("{}", element);
    egui::RichText::new(formatted)
}
