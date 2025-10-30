mod app;
mod logger;
mod logger_view;
mod job_constructing;
mod job_running;
mod job_complete;
mod render;
mod rgba;
mod frame_history;
mod settings;
mod thread_stats;
mod format;
mod timer;

use app::App;

fn main() -> eframe::Result<()> {
    logger::init().expect("Error initializing logger");
    let native_options = eframe::NativeOptions::default();
    eframe::run_native("Raytracer", native_options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
