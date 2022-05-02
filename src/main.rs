mod app;
mod rgba;
mod frame_history;
mod settings;
mod thread_stats;

use app::App;

fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native("Raytracer", native_options, Box::new(|cc| Box::new(App::new(cc))));
}
