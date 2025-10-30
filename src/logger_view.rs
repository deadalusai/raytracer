use eframe::egui::{self, Align, Color32, FontSelection, RichText, Style};
use eframe::egui::text::LayoutJob;

use crate::logger::LogEntry;

pub fn logger_view(ui: &mut egui::Ui) {
    let Ok(mut sink) = crate::logger::LOG_SINK.lock() else {
        return;
    };

    ui.horizontal(|ui| {
        ui.label("Logs");

        if ui.button("Clear").clicked() {
            sink.entries.clear();
        }
    });

    let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
    let row_count = sink.entries.len();

    ui.separator();
    egui::ScrollArea::vertical()
        .stick_to_bottom(true)
        .max_width(ui.available_width())
        .max_height(ui.available_height())
        .auto_shrink(false)
        .show_rows(ui, row_height, row_count, |ui, range| {
            for entry in sink.entries[range].iter() {
                ui.label(format_record(entry));
            }
        });
}

fn format_timestamp(entry: &LogEntry) -> String {
    let format = time_format::DateFormat::Custom("%H:%M:%S.{ms}");
    time_format::format_common_ms_local(entry.time, format).unwrap_or_else(|_| "???".to_string())
}

const WARN: Color32 = Color32::YELLOW;
const ERROR: Color32 = Color32::RED;
const HIGHLIGHT: Color32 = Color32::LIGHT_GRAY;

fn format_record(entry: &LogEntry) -> LayoutJob {
    let mut layout_job = LayoutJob::default();
    let style = Style::default();

    let timestamp = format_timestamp(entry);
    let timestamp = RichText::new(timestamp).monospace();
    timestamp.append_to(&mut layout_job, &style, FontSelection::Default, Align::LEFT);

    let prefix = format!(" [{:5}] {} ", entry.level, entry.target);
    let prefix = RichText::new(prefix).monospace();
    let prefix = match entry.level {
        log::Level::Warn => prefix.color(WARN),
        log::Level::Error => prefix.color(ERROR),
        _ => prefix.color(HIGHLIGHT),
    };
    prefix.append_to(&mut layout_job, &style, FontSelection::Default, Align::LEFT);

    let message = RichText::new(&entry.message).monospace();
    let message = match entry.level {
        log::Level::Warn => message.color(WARN),
        log::Level::Error => message.color(ERROR),
        _ => message
    };
    message.append_to(&mut layout_job, &style, FontSelection::Default, Align::LEFT);

    layout_job
}
