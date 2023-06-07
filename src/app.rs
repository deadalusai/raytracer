use std::sync::{ Arc };
use std::thread::{ JoinHandle, spawn };
use std::time::{ Instant, Duration };

use cancellation::{ CancellationToken, CancellationTokenSource };
use eframe::egui::{self, Spinner};
use flume::{ Receiver, Sender };
use raytracer_impl::implementation::{ Scene, RenderSettings };
use raytracer_impl::viewport::{ RenderChunk, Viewport, create_render_chunks };
use raytracer_samples::{ CameraConfiguration };

use crate::rgba::{ RgbaBuffer, v3_to_rgba };
use crate::settings::{ SceneConfig };
use crate::frame_history::{ FrameHistory };
use crate::thread_stats::{ ThreadStats };
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
    worker_handle: RenderWorkerHandle,
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
    scene_configs: Vec<SceneConfig>,
    // Temporal state
    output_texture: Option<(egui::TextureHandle, [usize; 2])>,
    render_job_creating: Option<ConstructRenderJob>,
    render_job: Option<RenderJob>,
    frame_history: FrameHistory,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        
        // Persistent state
        let settings = match cc.storage {
            None => Settings::default(),
            Some(s) => eframe::get_value(s, eframe::APP_KEY).unwrap_or_default(),
        };

        let scene_configs = vec![
            SceneConfig { name: "Random Spheres", factory: raytracer_samples::samples::random_sphere_scene },
            SceneConfig { name: "Simple",         factory: raytracer_samples::samples::simple_scene },
            SceneConfig { name: "Planes",         factory: raytracer_samples::samples::planes_scene },
            SceneConfig { name: "Mirrors",        factory: raytracer_samples::samples::hall_of_mirrors },
            SceneConfig { name: "Triangles",      factory: raytracer_samples::samples::triangle_world },
            SceneConfig { name: "Mesh",           factory: raytracer_samples::samples::mesh_demo },
            SceneConfig { name: "Interceptor",    factory: raytracer_samples::samples::interceptor },
            SceneConfig { name: "Capsule",        factory: raytracer_samples::samples::capsule },
            SceneConfig { name: "Mesh Plane",     factory: raytracer_samples::samples::mesh_plane },
            SceneConfig { name: "Point Cloud",    factory: raytracer_samples::samples::point_cloud },
        ];

        App {
            settings,
            scene_configs,
            // Temporal state
            output_texture: None,
            render_job_creating: None,
            render_job: None,
            frame_history: FrameHistory::default(),
        }
    }

    fn start_job(&mut self) {

        // Stop running worker threads if an existing job is in progress
        if let Some(render_job) = self.render_job.take() {
            // Start shutdown of all threads
            render_job.worker_handle.cts.cancel();
        }

        let scene_factory = self.scene_configs[self.settings.scene].factory;

        let render_job_creating = start_background_construct_render_job(self.settings.clone(), scene_factory);
        self.render_job_creating = Some(render_job_creating);
    }

    fn update_pending_job(&mut self) -> bool {

        let is_creating_job = self.render_job_creating.is_some();
        if !is_creating_job {
            return false;
        }

        let is_job_finished = self.render_job_creating.as_ref().unwrap().handle.is_finished();
        if !is_job_finished {
            // Trigger immediate repaint to ensure we keep checking
            return true;
        }
            
        let job_creating = self.render_job_creating.take().unwrap();
        if let Ok(job) = job_creating.handle.join() {
            self.render_job = Some(job);
        }

        return false;
    }

    fn update_job(&mut self) -> bool {
        use RenderThreadMessage::*;

        let mut buffer_updated = false;

        if let Some(ref mut job) = self.render_job {

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
            if job.completed_chunk_count < job.total_chunk_count {
                job.render_time_secs = duration_total_secs(job.start_time.elapsed());
            }
        }

        buffer_updated
    }
}

const RNG_SEED: u64 = 12345;

fn start_render_thread(
    id: ThreadId,
    cancellation_token: &CancellationToken,
    work_receiver: &Receiver<RenderWork>,
    result_sender: &Sender<RenderThreadMessage>
) -> Result<(), BoxError> {
    use RenderThreadMessage::*;
    use rand::{ SeedableRng };
    use rand_xorshift::{ XorShiftRng };

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

struct RenderWorkerHandle {
    cts: CancellationTokenSource,
    work_sender: Sender<RenderWork>,
    result_receiver: Receiver<RenderThreadMessage>,
    thread_handles: Vec<RenderThread>,
}

fn start_background_render_threads(render_thread_count: u32) -> RenderWorkerHandle {
    let cts = CancellationTokenSource::new();
    let (work_sender, work_receiver) = flume::bounded(render_thread_count as usize);
    let (result_sender, result_receiver) = flume::unbounded();

    let thread_handles = (0..render_thread_count)
        .map(|id| {
            let cancellation_token = cts.token().clone();
            let work_receiver = work_receiver.clone();
            let result_sender = result_sender.clone();
            let handle = spawn(move || {
                if let Err(err) = start_render_thread(id, &cancellation_token, &work_receiver, &result_sender) {
                    println!("Thread {id} terminated due to error: {err}");
                }
                // Notify master thread that we've terminated.
                // NOTE: There may be nobody listening...
                result_sender.send(RenderThreadMessage::Terminated(id)).ok();
            });
            RenderThread {
                id: id,
                handle: handle,
                total_time_secs: 0.0,
                total_chunks_rendered: 0,
            }
        })
        .collect::<Vec<_>>();

    RenderWorkerHandle {
        cts,
        work_sender,
        result_receiver,
        thread_handles,
    }
}

struct ConstructRenderJob {
    handle: JoinHandle<RenderJob>,
}

fn start_background_construct_render_job(st: Settings, factory: fn(&CameraConfiguration) -> Scene) -> ConstructRenderJob {
    let handle = std::thread::spawn(move || {
    
        // Create render work arguments
        let viewport = Viewport::new(st.width, st.height);
        let camera_config = CameraConfiguration {
            width: st.width as f32,
            height: st.height as f32,
            fov: st.camera_fov,
            aperture: st.camera_aperture,
            angle_adjust_v: st.camera_angle_adjust_v,
            angle_adjust_h: st.camera_angle_adjust_h,
            focus_dist_adjust: st.camera_focus_dist_adjust,
        };

        let start = Instant::now();

        let mut scene = factory(&camera_config);

        println!("Constructed Scene in {}ms", start.elapsed().as_millis());

        let start = Instant::now();

        scene.reorganize_objects_into_bvh();

        println!("Constructed Bounding Volume Hierachy in {}ms", start.elapsed().as_millis());
        
        let settings = RenderSettings {
            max_reflections: st.max_reflections,
            samples_per_pixel: st.samples_per_pixel,
        };

        // Chunks are popped from this list as they are rendered.
        // Reverse the list so the top of the image is rendered first.
        let mut chunks = create_render_chunks(&viewport, st.chunk_count);
        chunks.reverse();

        RenderJob {
            render_args: Arc::new((scene, settings)),
            total_chunk_count: chunks.len() as u32,
            completed_chunk_count: 0,
            pending_chunks: chunks,
            start_time: Instant::now(),
            render_time_secs: 0_f64,
            buffer: RgbaBuffer::new(st.width, st.height),
            worker_handle: start_background_render_threads(st.thread_count),
        }
    });

    ConstructRenderJob { handle }
}

impl eframe::App for App {

    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.settings);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {

        ctx.set_visuals(egui::Visuals::dark());

        self.frame_history.on_new_frame(ctx.input().time, frame.info().cpu_usage);

        // Ensure we keep updating the UI as long as there's a job being created,
        // as we rely on the update loop to keep checking the factory thread.
        
        let job_pending = self.update_pending_job(); 
        if job_pending {
            ctx.request_repaint();
        }

        let buffer_updated = self.update_job();
        if buffer_updated {
            // Update the output texture
            if let Some(job) = self.render_job.as_ref() {
                let (tex_dim, tex_data) = job.buffer.get_raw_rgba_data();
                let tex_data = egui::ColorImage::from_rgba_unmultiplied(tex_dim, tex_data);
                let tex_id = ctx.load_texture("output_tex", tex_data, egui::TextureOptions::LINEAR);
                self.output_texture = Some((tex_id, tex_dim));
            }
        }
        
        // Ensure we keep updating the UI as long as there's an active job,
        // as we rely on the update loop to keep feeding the worker threads and updating the in-progress image.
        
        if let Some(job) = self.render_job.as_ref() {
            if job.completed_chunk_count != job.total_chunk_count {
                // Tell the backend to repaint as soon as possible
                ctx.request_repaint();
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.render_job_creating.is_some() {
                ui.centered_and_justified(|ui| {
                    ui.add(Spinner::new());
                });
            }
            // Output image
            else if let Some((id, dim)) = &self.output_texture {
                ui.centered_and_justified(|ui| {
                    let (width, height) = match dim {
                        // Scale the output texture to fit in the container
                        &[w, h] if self.settings.scale_render_to_window => scale_to_container_dimensions((w as f32, h as f32), (ui.available_width(), ui.available_height())),
                        &[w, h] => (w as f32, h as f32),
                    };
                    ui.image(id, [width, height]);
                });
            }
        });

        // Settings UI
        egui::Window::new("Settings")
            .resizable(false)
            .default_width(200.0)
            .default_height(500.0)
            .show(ctx, |ui| {
                ui.add(SettingsWidget::new(&mut self.settings, &self.scene_configs));
                ui.separator();

                ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
                    if ui.button("Start render").clicked() {
                        self.start_job();
                    }
                });

                if let Some(job) = self.render_job.as_ref() {
                    for thread in job.worker_handle.thread_handles.iter() {
                        ui.add(ThreadStats {
                            id: thread.id,
                            total_chunks_rendered: thread.total_chunks_rendered,
                            total_time_secs: thread.total_time_secs,
                        });
                    }
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    self.frame_history.ui(ui);
                });
            });
    }
}

/// Scales the given `i` dimensions to fit the given `c` dimensions
fn scale_to_container_dimensions((iw, ih): (f32, f32), (cw, ch): (f32, f32)) -> (f32, f32) {
    let iratio = iw / ih;
    let cratio = cw / ch;
    match (cw < ch, iratio > cratio) {
        | (true,  true)  => (cw, cw * (1.0 / iratio)),
        | (false, true)  => (cw, cw * (1.0 / iratio)),
        | (true,  false) => (ch * iratio, ch),
        | (false, false) => (ch * iratio, ch),
    }
}