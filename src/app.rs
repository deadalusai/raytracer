use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{ Instant, Duration };

use cancellation::{ CancellationToken, CancellationTokenSource };
use eframe::egui::{self, Spinner};
use flume::{ Receiver, Sender };
use raytracer_impl::implementation::{ Scene, RenderSettings };
use raytracer_impl::viewport::{ RenderChunk, Viewport, create_render_chunks };
use raytracer_samples::scene::{ CameraConfiguration, SceneFactory, SceneControlCollection, SceneConfiguration, CreateSceneError };

use crate::rgba::{ RgbaBuffer, v3_to_rgba };
use crate::frame_history::FrameHistory;
use crate::thread_stats::ThreadStats;
use crate::settings::{ SettingsWidget, Settings };

fn duration_total_secs(elapsed: Duration) -> f64 {
    elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9
}

type BoxError = Box<dyn std::error::Error + 'static>;

struct RenderJob {
    render_args: Arc<(Scene, RenderSettings)>,
    pending_chunks: Vec<RenderChunk>,
    start_time: Instant,
    render_time_secs: f64,
    total_chunk_count: u32,
    completed_chunk_count: u32,
    buffer: RgbaBuffer,
    worker_handle: RenderJobWorkerHandle,
}

// A message from the master thread to a worker
#[derive(Clone)]
struct RenderWork(RenderChunk, Arc<(Scene, RenderSettings)>);

// A message from a worker thread to the master thread
#[derive(Clone)]
enum RenderThreadMessage {
    Ready(ThreadId),
    FrameUpdated(ThreadId, RenderChunk, RgbaBuffer),
    FrameCompleted(ThreadId, Duration),
    Terminated(ThreadId)
}

pub struct App {
    // Persistent state
    settings: Settings,
    // Configuration
    scene_factories: Vec<Arc<dyn SceneFactory + Send + Sync>>,
    scene_configs: Vec<SceneControlCollection>,
    // Temporal state
    frame_history: FrameHistory,
    state: AppState,
    output_texture: Option<egui::TextureHandle>,
}

enum AppState {
    None,
    RenderJobConstructing(RenderJobConstructingState),
    RenderJobRunning(RenderJob),
    Error(String),
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        
        // Persistent state
        let settings = match cc.storage {
            None => Settings::default(),
            Some(s) => eframe::get_value(s, eframe::APP_KEY).unwrap_or_default(),
        };

        let scene_factories = raytracer_samples::make_sample_scene_factories();
        let scene_configs = scene_factories.iter().map(|f| f.create_controls()).collect();

        App {
            settings,
            // Configuration
            scene_factories,
            scene_configs,
            // Temporal state
            frame_history: FrameHistory::default(),
            state: AppState::None,
            output_texture: None,
        }
    }

    fn start_new_job(&mut self) {

        // Stop running worker threads if an existing job is in progress
        if let AppState::RenderJobRunning(state) = &self.state {
            state.worker_handle.cts.cancel();
        }

        let scene_factory = self.scene_factories[self.settings.scene].clone();
        let scene_config = self.scene_configs[self.settings.scene].collect_configuration();

        let state = start_background_construct_render_job(ConstructRenderJobArgs {
            settings: self.settings.clone(),
            scene_config,
            scene_factory,
        });

        self.state = AppState::RenderJobConstructing(state);
    }

    /// Runs internal state update logic, and may transition
    /// the app into a new state.
    /// 
    /// The return value indicates if work is still running on a background thread.
    fn update_state(&mut self, ctx: &eframe::egui::Context) -> bool {
        self.state = match &mut self.state {
            AppState::RenderJobConstructing(state) => {
                if !state.is_finished() {
                    // Background work still in progress
                    return true;
                }
                // Background work completed,
                // transition to appropriate next state
                let handle = state.handle.take().unwrap();
                match handle.join() {
                    Ok(Ok(job)) => {
                        AppState::RenderJobRunning(job)
                    },
                    Ok(Err(CreateSceneError(err))) => {
                        AppState::Error(err)
                    },
                    Err(panic) => {
                        AppState::Error(try_extract_panic_argument(&panic).unwrap_or("Unknown error").to_string())
                    },
                }
            },
            AppState::RenderJobRunning(job) => {
                use RenderThreadMessage::*;

                let mut buffer_updated = false;

                // Poll for completed work
                while let Ok(result) = job.worker_handle.result_receiver.try_recv() {
                    match result {
                        Ready(_) => {}, // Worker thread ready to go.
                        FrameUpdated(_, chunk, buf) => {
                            // Copy chunk to buffer
                            job.buffer.copy_from_sub_buffer(chunk.left, chunk.top, &buf);
                            buffer_updated = true;
                        },
                        FrameCompleted(id, elapsed) => {
                            // Update stats
                            let thread = &mut job.worker_handle.thread_handles[id as usize];
                            thread.total_time_secs += duration_total_secs(elapsed);
                            thread.total_chunks_rendered += 1;
                            job.completed_chunk_count += 1;
                        },
                        Terminated(_) => {}, // Worker halted
                    }
                }

                // Update the working texture?
                if buffer_updated {
                    let rgba = job.buffer.get_raw_rgba_data();
                    let tex_id = ctx.load_texture(
                        "output_tex",
                        egui::ColorImage::from_rgba_unmultiplied([rgba.width, rgba.height], rgba.data),
                        egui::TextureOptions::LINEAR
                    );
                    self.output_texture = Some(tex_id);
                }
        
                // Refill the the work queue
                use flume::TrySendError;
                while let Some(chunk) = job.pending_chunks.pop() {
                    let work = RenderWork(chunk, job.render_args.clone());
                    if let Err(err) = job.worker_handle.work_sender.try_send(work) {
                        match err {
                            TrySendError::Full(RenderWork(chunk, _)) => {
                                // Queue full, try again later
                                job.pending_chunks.push(chunk);
                            }
                            TrySendError::Disconnected(_) => {
                                println!("All render threads stopped!");
                            }
                        }
                        break;
                    }
                }
            
                // Update timer, as long as we have outstanding work
                let working = job.completed_chunk_count < job.total_chunk_count;
                if working {
                    job.render_time_secs = duration_total_secs(job.start_time.elapsed());
                }

                return true;
            },
            _ => {
                return false;
            }
        };

        false
    }
}

/// Tries to get the value passed to [panic!]
fn try_extract_panic_argument(panic: &dyn std::any::Any) -> Option<&str> {
    panic.downcast_ref::<String>().map(|s| s.as_ref())
        .ok_or_else(|| panic.downcast_ref::<&str>())
        .ok()
}

//
// RenderJob factory worker
//

struct ConstructRenderJobArgs {
    settings: Settings,
    scene_config: SceneConfiguration,
    scene_factory: Arc<dyn SceneFactory + Send + Sync>,
}

struct RenderJobConstructingState {
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
}

fn start_background_construct_render_job(args: ConstructRenderJobArgs) -> RenderJobConstructingState {
    let work = move || {
    
        // Create render work arguments
        let viewport = Viewport::new(args.settings.width, args.settings.height);
        let camera_config = CameraConfiguration {
            width: args.settings.width as f32,
            height: args.settings.height as f32,
            fov: args.settings.camera_fov,
            lens_radius: args.settings.camera_lens_radius,
            angle_adjust_v: args.settings.camera_angle_adjust_v,
            angle_adjust_h: args.settings.camera_angle_adjust_h,
            focus_dist_adjust: args.settings.camera_focus_dist_adjust,
        };

        let start = Instant::now();

        let mut scene = args.scene_factory.create_scene(&camera_config, &args.scene_config)?;

        println!("Constructed Scene in {}ms", start.elapsed().as_millis());

        let start = Instant::now();

        scene.reorganize_objects_into_bvh();

        println!("Constructed Bounding Volume Hierachy in {}ms", start.elapsed().as_millis());
        
        let render_settings = RenderSettings {
            max_reflections: args.settings.max_reflections,
            samples_per_pixel: args.settings.samples_per_pixel,
        };

        // Chunks are popped from this list as they are rendered.
        // Reverse the list so the top of the image is rendered first.
        let mut chunks = create_render_chunks(&viewport, args.settings.chunk_count);
        chunks.reverse();

        Ok(RenderJob {
            render_args: Arc::new((scene, render_settings)),
            total_chunk_count: chunks.len() as u32,
            completed_chunk_count: 0,
            pending_chunks: chunks,
            start_time: Instant::now(),
            render_time_secs: 0_f64,
            buffer: RgbaBuffer::new(args.settings.width, args.settings.height),
            worker_handle: start_background_render_threads(args.settings.thread_count),
        })
    };

    let handle = std::thread::Builder::new()
        .name("Construct Render Job".into())
        .spawn(work)
        .expect("failed to spawn background thread");

    RenderJobConstructingState { handle: Some(handle) }
}

//
// Render workers
//

const RNG_SEED: u64 = 12345;

fn start_render_thread(
    id: ThreadId,
    cancellation_token: &CancellationToken,
    work_receiver: &Receiver<RenderWork>,
    result_sender: &Sender<RenderThreadMessage>
) -> Result<(), BoxError> {
    use RenderThreadMessage::*;
    use rand::SeedableRng;
    use rand_xorshift::XorShiftRng;

    result_sender.send(Ready(id))?;

    // Receive messages
    for RenderWork(chunk, args) in work_receiver.into_iter() {
        if cancellation_token.is_canceled() {
            return Ok(());
        }
        // Paint in-progress chunks green
        let mut buffer = RgbaBuffer::new(chunk.width, chunk.height);
        let green = v3_to_rgba(raytracer_impl::types::V3(0.0, 0.58, 0.0));
        for p in chunk.iter_pixels() {
            buffer.put_pixel(p.chunk_x, p.chunk_y, green);
        }
        result_sender.send(FrameUpdated(id, chunk.clone(), buffer.clone()))?;
        // Using the same seeded RNG for every frame makes every run repeatable
        let mut rng = XorShiftRng::seed_from_u64(RNG_SEED);
        // Render the scene chunk
        let (scene, render_settings) = args.as_ref();
        let time = Instant::now();
        // For each x, y coordinate in this view chunk, cast a ray.
        for p in chunk.iter_pixels() {
            if cancellation_token.is_canceled() {
                return Ok(());
            }
            // Convert to view-relative coordinates
            let color = raytracer_impl::implementation::cast_rays_into_scene(render_settings, scene, &chunk.viewport, p.viewport_x, p.viewport_y, &mut rng);
            buffer.put_pixel(p.chunk_x, p.chunk_y, v3_to_rgba(color));
        }
        let elapsed = time.elapsed();
        // Send final frame and results
        result_sender.send(FrameUpdated(id, chunk, buffer))?;
        result_sender.send(FrameCompleted(id, elapsed))?;
    }

    Ok(())
}

type ThreadId = u32;

#[allow(unused)]
struct RenderThread {
    id: ThreadId,
    handle: JoinHandle<()>,
    total_time_secs: f64,
    total_chunks_rendered: u32,
}

struct RenderJobWorkerHandle {
    cts: CancellationTokenSource,
    work_sender: Sender<RenderWork>,
    result_receiver: Receiver<RenderThreadMessage>,
    thread_handles: Vec<RenderThread>,
}

fn start_background_render_threads(render_thread_count: u32) -> RenderJobWorkerHandle {
    let cts = CancellationTokenSource::new();
    let (work_sender, work_receiver) = flume::bounded(render_thread_count as usize);
    let (result_sender, result_receiver) = flume::unbounded();

    let thread_handles = (0..render_thread_count)
        .map(|id| {
            let cancellation_token = cts.token().clone();
            let work_receiver = work_receiver.clone();
            let result_sender = result_sender.clone();
            let work = move || {
                if let Err(err) = start_render_thread(id, &cancellation_token, &work_receiver, &result_sender) {
                    println!("Thread {id} terminated due to error: {err}");
                }
                // Notify master thread that we've terminated.
                // NOTE: There may be nobody listening...
                result_sender.send(RenderThreadMessage::Terminated(id)).ok();
            };
            let handle = std::thread::Builder::new()
                .name(format!("Render Thread {id}"))
                .spawn(work)
                .expect("failed to spawn render thread");

            RenderThread {
                id,
                handle,
                total_time_secs: 0.0,
                total_chunks_rendered: 0,
            }
        })
        .collect::<Vec<_>>();

    RenderJobWorkerHandle {
        cts,
        work_sender,
        result_receiver,
        thread_handles,
    }
}

impl eframe::App for App {

    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.settings);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {

        ctx.set_visuals(egui::Visuals::dark());

        self.frame_history.on_new_frame(ctx.input(|s| s.time), frame.info().cpu_usage);

        // Ensure we keep updating the UI as long as there's work on a background thread,
        // as we rely on the update loop to keep checking for progress.
        
        let work_pending = self.update_state(ctx); 
        if work_pending {
            ctx.request_repaint();
        }
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                match &self.state {
                    AppState::None => {
                        ui.label("Press 'Start render' to start");
                    },
                    AppState::RenderJobConstructing(_) => {
                        ui.add(Spinner::new());
                    },
                    AppState::RenderJobRunning(_) => {
                        match self.output_texture.as_ref() {
                            Some(output_texture) => {
                                ui.add(
                                    if self.settings.scale_render_to_window {
                                        egui::Image::new(output_texture).fit_to_fraction(egui::vec2(1.0, 1.0))
                                    }
                                    else {
                                        egui::Image::new(output_texture).fit_to_original_size(1.0)
                                    }
                                );
                            }
                            None => {
                                ui.spinner();
                            }
                        }
                    },
                    AppState::Error(error) => {
                        ui.label(error);
                    },
                }
            });
        });

        // Settings UI
        egui::Window::new("Settings")
            .resizable(false)
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.add(SettingsWidget::new(&mut self.settings, &mut self.scene_configs));
                ui.separator();

                ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
                    if ui.button("Start render").clicked() {
                        self.start_new_job();
                    }
                });

                if let AppState::RenderJobRunning(job) = &self.state {
                    for thread in job.worker_handle.thread_handles.iter() {
                        ui.add(ThreadStats {
                            id: thread.id,
                            total_chunks_rendered: thread.total_chunks_rendered,
                            total_time_secs: thread.total_time_secs,
                        });
                    }
                }

                self.frame_history.ui(ui);
            });
    }
}
