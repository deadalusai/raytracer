use std::time::Duration;

use crate::app::AppStateUpdateResult;
use crate::render::{RenderJob, RenderThreadMessage, RenderWork};

fn duration_total_secs(elapsed: Duration) -> f64 {
    elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9
}

pub struct RenderJobRunningState {
    pub job: RenderJob,
    pub output_tex: Option<eframe::egui::TextureHandle>,
}

impl RenderJobRunningState {
    pub fn new(job: RenderJob) -> Self {
        Self { job, output_tex: None }
    }

    pub fn update(&mut self, ctx: &eframe::egui::Context) -> AppStateUpdateResult {
        use RenderThreadMessage::*;

        let mut buffer_updated = false;

        // Poll for completed work
        while let Ok(result) = self.job.worker_handle.result_receiver.try_recv() {
            match result {
                Ready(_) => {}, // Worker thread ready to go.
                FrameUpdated(_, chunk, buf) => {
                    // Copy chunk to buffer
                    self.job.buffer.copy_from_sub_buffer(chunk.left, chunk.top, &buf);
                    buffer_updated = true;
                },
                FrameCompleted(id, elapsed) => {
                    // Update stats
                    let thread = &mut self.job.worker_handle.thread_handles[id as usize];
                    thread.total_time_secs += duration_total_secs(elapsed);
                    thread.total_chunks_rendered += 1;
                    self.job.completed_chunk_count += 1;
                },
                Terminated(_) => {}, // Worker halted
            }
        }

        // Update the working texture?
        if buffer_updated {
            let rgba = self.job.buffer.get_raw_rgba_data();
            let tex_id = ctx.load_texture(
                "output_tex",
                eframe::egui::ColorImage::from_rgba_unmultiplied([rgba.width, rgba.height], rgba.data),
                eframe::egui::TextureOptions::LINEAR
            );
            self.output_tex = Some(tex_id);
        }

        // Refill the the work queue
        use flume::TrySendError;
        while let Some(chunk) = self.job.pending_chunks.pop() {
            let work = RenderWork(chunk, self.job.render_args.clone());
            if let Err(err) = self.job.worker_handle.work_sender.try_send(work) {
                match err {
                    TrySendError::Full(RenderWork(chunk, _)) => {
                        // Queue full, try again later
                        self.job.pending_chunks.push(chunk);
                    }
                    TrySendError::Disconnected(_) => {
                        println!("All render threads stopped!");
                    }
                }
                break;
            }
        }
    
        let working = self.job.completed_chunk_count < self.job.total_chunk_count;
        if working {
            // Update timer
            self.job.render_time_secs = duration_total_secs(self.job.start_time.elapsed());
            AppStateUpdateResult::RequestRefresh
        }
        else {
            AppStateUpdateResult::None
        }
    }
}
