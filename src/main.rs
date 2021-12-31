mod app;
mod rgba;
mod frame_history;
mod settings;
mod thread_stats;

use app::App;

fn main() {
    let app = App::default();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
