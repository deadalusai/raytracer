use eframe::egui::{TextureHandle};

pub struct RenderJobCompleteState {
    pub output_tex: TextureHandle,
}

impl RenderJobCompleteState {
    pub fn new(output_tex: TextureHandle) -> Self {
        Self { output_tex }
    }
}
