use crate::app::AppStateUpdateResult;
use crate::render::{RenderJob, RenderJobUpdateResult};

pub struct RenderJobRunningState {
    pub job: RenderJob,
    pub output_tex: Option<eframe::egui::TextureHandle>,
}

impl RenderJobRunningState {
    pub fn new(job: RenderJob) -> Self {
        Self { job, output_tex: None }
    }

    pub fn update(&mut self, ctx: &eframe::egui::Context) -> AppStateUpdateResult {

        let v1 = self.job.buffer.version();
        let result = self.job.update();
        let v2 = self.job.buffer.version();

        if result == RenderJobUpdateResult::ErrorRenderThreadsStopped {
            return AppStateUpdateResult::TransitionToNewState(crate::app::AppState::Error("All render threads stopped".into()));
        }
    
        if v1 != v2 {
            // Update the working texture
            let rgba = self.job.buffer.get_raw_rgba_data();
            let tex_id = ctx.load_texture(
                "output_tex",
                eframe::egui::ColorImage::from_rgba_unmultiplied([rgba.width, rgba.height], rgba.data),
                eframe::egui::TextureOptions::LINEAR
            );
            self.output_tex = Some(tex_id);
        }

        if self.job.is_work_completed() {
            AppStateUpdateResult::None
        }
        else {
            AppStateUpdateResult::RequestRefresh
        }
    }

    pub fn stop(&self) {
        self.job.worker_handle.cts.cancel();
    }
}


impl Drop for RenderJobRunningState {
    fn drop(&mut self) {
        self.stop();
    }
}