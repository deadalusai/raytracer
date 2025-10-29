use eframe::egui::{TextureHandle};

use crate::thread_stats::ThreadStats;

pub struct RenderJobCompleteState {
    pub output_tex: TextureHandle,
    pub thread_stats: Vec<ThreadStats>,
}
