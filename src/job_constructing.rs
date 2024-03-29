use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Instant;

use raytracer_impl::implementation::RenderSettings;
use raytracer_impl::viewport::{ Viewport, create_render_chunks };
use raytracer_samples::scene::{ CameraConfiguration, SceneFactory, SceneConfiguration, CreateSceneError };

use crate::app::{AppStateUpdateResult, AppState};
use crate::job_running::RenderJobRunningState;
use crate::render::{RenderJob, start_background_render_threads};
use crate::rgba::RgbaBuffer;
use crate::settings::Settings;

pub struct RenderJobConstructingState {
    // NOTE: Wrap the thread handle in an Option
    // to allow us to move ownership out of a mut reference as part of [App::update].
    handle: Option<JoinHandle<Result<RenderJob, CreateSceneError>>>,
}

impl RenderJobConstructingState {
    fn is_finished(&self) -> bool {
        match &self.handle {
            Some(handle) => handle.is_finished(),
            None => false,
        }
    }

    pub fn update(&mut self) -> AppStateUpdateResult {
        if !self.is_finished() {
            // Background work still in progress
            return AppStateUpdateResult::RequestRefresh;
        }
        // Background work completed,
        // transition to appropriate next state
        let handle = self.handle.take().unwrap();
        AppStateUpdateResult::TransitionToNewState(match handle.join() {
            Ok(Ok(job)) => {
                AppState::RenderJobRunning(RenderJobRunningState::new(job))
            },
            Ok(Err(CreateSceneError(err))) => {
                AppState::Error(err)
            },
            Err(panic) => {
                AppState::Error(try_extract_panic_argument(&panic).unwrap_or("Unknown error").to_string())
            },
        })
    }
}

/// Tries to get the value passed to [panic!]
fn try_extract_panic_argument(panic: &dyn std::any::Any) -> Option<&str> {
    panic.downcast_ref::<String>().map(|s| s.as_ref())
        .ok_or_else(|| panic.downcast_ref::<&str>())
        .ok()
}

pub fn start_render_job_construction(
    settings: Settings,
    scene_config: SceneConfiguration,
    scene_factory: Arc<dyn SceneFactory + Send + Sync>
) -> RenderJobConstructingState {
    let work = move || {
    
        // Create render work arguments
        let camera_config = CameraConfiguration {
            width: settings.width as f32,
            height: settings.height as f32,
            fov: settings.camera_fov,
            lens_radius: settings.camera_lens_radius,
            angle_adjust_v: settings.camera_angle_adjust_v,
            angle_adjust_h: settings.camera_angle_adjust_h,
            focus_dist_adjust: settings.camera_focus_dist_adjust,
        };

        let start = Instant::now();

        let mut scene = scene_factory.create_scene(&camera_config, &scene_config)?;

        println!("Constructed Scene in {}ms", start.elapsed().as_millis());

        let start = Instant::now();

        scene.reorganize_objects_into_bvh();

        println!("Constructed Bounding Volume Hierachy in {}ms", start.elapsed().as_millis());
        
        let render_settings = RenderSettings {
            max_reflections: settings.max_reflections,
            samples_per_pixel: settings.samples_per_pixel,
        };

        // Chunks are popped from this list as they are rendered.
        // Reverse the list so the top of the image is rendered first.
        let viewport = Viewport::new(settings.width, settings.height);
        let mut chunks = create_render_chunks(&viewport, settings.chunk_count);
        chunks.reverse();

        Ok(RenderJob {
            render_args: Arc::new((scene, render_settings)),
            total_chunk_count: chunks.len() as u32,
            completed_chunk_count: 0,
            pending_chunks: chunks,
            start_time: Instant::now(),
            render_time_secs: 0_f64,
            buffer: RgbaBuffer::new(settings.width, settings.height),
            worker_handle: start_background_render_threads(settings.thread_count),
        })
    };

    let handle = std::thread::Builder::new()
        .name("Construct Render Job".into())
        .spawn(work)
        .expect("failed to spawn background thread");

    RenderJobConstructingState { handle: Some(handle) }
}
