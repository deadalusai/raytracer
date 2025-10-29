
use std::time::Instant;

use eframe::egui::{self, Align, Color32, FontSelection, RichText, Style};
use eframe::egui::text::LayoutJob;

use crate::logger::{LogEntry, LogSink};

pub fn logger_view(ui: &mut egui::Ui) {
    let mut sink = crate::logger::LOG_SINK.lock();

    ui.horizontal(|ui| {
        ui.label("Logs");

        if ui.button("Clear").clicked() {
            sink.entries.clear();
        }
    });

    ui.separator();
    egui::ScrollArea::vertical()
        .stick_to_bottom(true)
        .max_width(ui.available_width())
        .max_height(ui.available_height())
        .auto_shrink(false)
        .show(ui, |ui| {
            let Some(entry) = sink.entries.iter().last() else {
                return;
            };
            let time_width = format_instant(sink.start, entry.instant).len();
            for entry in sink.entries.iter() {
                ui.label(format_record(&sink, entry, time_width));
            }
        });
}

const WARN: Color32 = Color32::YELLOW;
const ERROR: Color32 = Color32::RED;
const HIGHLIGHT: Color32 = Color32::LIGHT_GRAY;

fn format_record(sink: &LogSink, entry: &LogEntry, time_width: usize) -> LayoutJob {
    let mut layout_job = LayoutJob::default();
    let style = Style::default();

    let timestamp = format_instant(sink.start, entry.instant);
    let timestamp = format!("{: >width$} ", timestamp, width = time_width);
    let timestamp = RichText::new(timestamp).monospace();
    let timestamp = match entry.level {
        log::Level::Warn => timestamp.color(WARN),
        log::Level::Error => timestamp.color(ERROR),
        _ => timestamp
    };
    timestamp.append_to(&mut layout_job, &style, FontSelection::Default, Align::LEFT);

    let prefix = format!("[{:5}] {: <width$}: ", entry.level, entry.target, width = 20);
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

fn format_instant(start: Instant, instant: Instant) -> String {
    let millis = instant.duration_since(start).as_millis();
    let h = millis / 1000 / 3600 % 24;
    let m = millis / 1000 / 60 % 60;
    let s = millis / 1000 % 60;
    let ms = millis % 1000;
    match (h, m, s, ms) {
        (0, 0, s, ms) => format!("{s}s {ms}ms"),
        (0, m, s, ms) => format!("{m}m {s}s {ms}ms"),
        (h, m, s, ms) => format!("{h}h {m}m {s}s {ms}ms"),
    }
}
