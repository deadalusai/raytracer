use eframe::egui::{Color32, ColorImage, TextureOptions};
use log::info;

use crate::app::AppStateUpdateResult;
use crate::job_complete::RenderJobCompleteState;
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

        if self.output_tex.is_none() {
            // Initialise the output texture
            let settings = &self.job.render_args.1;
            let img = ColorImage::filled([settings.width, settings.height], Color32::BLACK);
            self.output_tex = Some(ctx.load_texture("output_tex", img, TextureOptions::LINEAR));
        }

        let result = self.job.update();
        if result == RenderJobUpdateResult::ErrorRenderThreadsStopped {
            return AppStateUpdateResult::TransitionToNewState(
                crate::app::AppState::Error("All render threads stopped".into())
            );
        }

        // Update the output texture
        let tex = self.output_tex.as_mut().unwrap();
        for (pos, buf) in self.job.updates.drain(..) {
            let raw = buf.get_raw_rgba_data();
            let img = ColorImage::from_rgba_unmultiplied(raw.size, raw.rgba);
            tex.set_partial(pos, img, TextureOptions::LINEAR);
        }

        if self.job.is_work_completed() {
            info!("Render complete");
            return AppStateUpdateResult::TransitionToNewState(
                crate::app::AppState::RenderJobComplete(RenderJobCompleteState {
                    output_tex: self.output_tex.take().unwrap(),
                    thread_stats: self.job.thread_stats().collect(),
                })
            );
        }

        AppStateUpdateResult::RequestRefresh
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
