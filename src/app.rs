use std::sync::{ Arc, atomic::{ AtomicBool, Ordering } };
use std::thread::{ spawn, JoinHandle };
use std::time::{ Instant, Duration };
use std::sync::mpsc::{ Receiver, Sender, channel };

use eframe::{ egui, epi };
use rand::{ weak_rng };
use multiqueue::{ mpmc_queue, MPMCReceiver, MPMCSender };
use raytracer::{ Scene, RenderSettings, RenderChunk, Viewport, create_render_chunks };

use crate::frame_history::{ FrameHistory };
use crate::thread_stats::{ ThreadStats };
use crate::settings::{ SettingsWidget, Settings, TestScene };

fn duration_total_secs(elapsed: Duration) -> f64 {
    elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9
}

type BoxError = Box<dyn std::error::Error + 'static>;

// Unmultiplied RGBA data
type Rgba = [u8; 4];

fn v3_to_rgba(v3: raytracer::V3) -> Rgba {
    let r = (255.0 * v3.0.sqrt()) as u8;
    let g = (255.0 * v3.1.sqrt()) as u8;
    let b = (255.0 * v3.2.sqrt()) as u8;
    let a = 255;
    [r, g, b, a]
}

#[derive(Clone)]
struct RgbaBuffer {
    width: usize,
    height: usize,
    data: Vec<u8>,
}

impl RgbaBuffer {
    pub fn new(width: u32, height: u32) -> RgbaBuffer {
        RgbaBuffer {
            width: width as usize,
            height: height as usize,
            data: vec![0; (width * height * 4) as usize],
        }
    }

    fn index(&self, x: u32, y: u32) -> usize {
        ((y as usize) * self.width + (x as usize)) * 4
    }

    pub fn put_pixel(&mut self, x: u32, y: u32, rgba: Rgba) {
        let i = self.index(x, y);
        self.data[i + 0] = rgba[0];
        self.data[i + 1] = rgba[1];
        self.data[i + 2] = rgba[2];
        self.data[i + 3] = rgba[3];
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> Rgba {
        let i = self.index(x, y);
        [
            self.data[i + 0],
            self.data[i + 1],
            self.data[i + 2],
            self.data[i + 3]
        ]
    }
}

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

pub struct App {
    // Persistent state
    settings: Settings,
    // Temporal state
    output_texture: Option<(egui::TextureId, [usize; 2])>,
    render_job: Option<RenderJob>,
    frame_history: FrameHistory,
}

impl Default for App {
    fn default() -> Self {
        App {
            // Persistent state
            settings: Settings::default(),
            // Temporal state
            output_texture: None,
            render_job: None,
            frame_history: FrameHistory::default(),
        }
    }
}

impl App {

    fn start_job(&mut self) {

        // Stop running worker threads if an existing job is in progress
        if let Some(render_job) = self.render_job.take() {
            // Start shutdown of all threads
            render_job.worker_handle.halt_flag.store(true, Ordering::Relaxed);
            render_job.worker_handle.work_sender.unsubscribe();
        }

        let st = &self.settings;

        // Create render work arguments
        let viewport = Viewport::new(st.width, st.height);
        let camera_aperture = st.camera_aperture;
        let scene = match st.scene {
            TestScene::RandomSpheres => raytracer::samples::random_sphere_scene(&viewport, camera_aperture),
            TestScene::Simple        => raytracer::samples::simple_scene(&viewport, camera_aperture),
            TestScene::Planes        => raytracer::samples::planes_scene(&viewport, camera_aperture),
            TestScene::Mirrors       => raytracer::samples::hall_of_mirrors(&viewport, camera_aperture),
            TestScene::Triangles     => raytracer::samples::triangle_world(&viewport, camera_aperture),
            TestScene::Mesh          => raytracer::samples::mesh_demo(&viewport, camera_aperture),
            TestScene::Interceptor   => raytracer::samples::interceptor(&viewport, camera_aperture),
        };
        let settings = RenderSettings {
            max_reflections: st.max_reflections,
            samples_per_pixel: st.samples_per_pixel,
        };

        // Chunks are popped from this list as they are rendered.
        // Reverse the list so the top of the image is rendered first.
        let mut chunks = create_render_chunks(&viewport, st.chunk_count);
        chunks.reverse();

        self.render_job = Some(RenderJob {
            render_args: Arc::new((scene, settings)),
            total_chunk_count: chunks.len() as u32,
            completed_chunk_count: 0,
            pending_chunks: chunks,
            start_time: Instant::now(),
            render_time_secs: 0_f64,
            buffer: RgbaBuffer::new(st.width, st.height),
            worker_handle: start_background_render_threads(st.thread_count),
        });
    }

    fn update(&mut self) -> bool {
        use RenderResult::*;

        let mut buffer_updated = false;

        if let Some(ref mut job) = self.render_job {

            // Poll for completed work
            for thread in job.worker_handle.thread_handles.iter_mut() {
                while let Ok(result) = thread.result_receiver.try_recv() {
                    match result {
                        Frame(chunk, buf) => {
                            // Copy chunk to buffer
                            for p in chunk.iter_pixels() {
                                job.buffer.put_pixel(p.viewport_x, p.viewport_y, buf.get_pixel(p.chunk_x, p.chunk_y));
                            }
                            buffer_updated = true;
                        },
                        Ready => {}, // Worker thread ready to go.
                        Done(elapsed) => {
                            // Update stats
                            thread.total_time_secs += duration_total_secs(elapsed);
                            thread.total_chunks_rendered += 1;
                            job.completed_chunk_count += 1;
                        },
                    }
                }
            }
    
            // Refill the the work queue
            use std::sync::mpsc::TrySendError;
            while let Some(chunk) = job.pending_chunks.pop() {
                let work = RenderWork(chunk, job.render_args.clone());
                match job.worker_handle.work_sender.try_send(work) {
                    Ok(_) => {
                        // Move to next thread
                        continue;
                    },
                    Err(TrySendError::Full(RenderWork(v, _))) => {
                        // Queue full, try again later
                        job.pending_chunks.push(v);
                        break;
                    },
                    Err(_) => unreachable!(),
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

#[derive(Clone)]
struct RenderWork (RenderChunk, Arc<(Scene, RenderSettings)>);


#[derive(Clone)]
enum RenderResult {
    Ready,
    Frame(RenderChunk, RgbaBuffer),
    Done(Duration),
}

fn start_render_thread(halt_flag: Arc<AtomicBool>, work_receiver: &MPMCReceiver<RenderWork>, result_sender: &Sender<RenderResult>) -> Result<(), BoxError> {
    use RenderResult::*;
    let mut rng = weak_rng();
    result_sender.send(RenderResult::Ready)?;
    // Receive messages
    loop {
        if halt_flag.load(Ordering::Relaxed) {
            return Ok(());
        }
        let RenderWork(chunk, args) = work_receiver.recv()?;
        // Paint in-progress chunks green
        let mut buffer = RgbaBuffer::new(chunk.width, chunk.height);
        for p in chunk.iter_pixels() {
            buffer.put_pixel(p.chunk_x, p.chunk_y, v3_to_rgba(raytracer::V3(0.0, 0.58, 0.0)));
        }
        result_sender.send(Frame(chunk.clone(), buffer.clone()))?;
        // Render the scene chunk
        let (scene, render_settings) = args.as_ref();
        let time = Instant::now();
        // For each x, y coordinate in this view chunk, cast a ray.
        for p in chunk.iter_pixels() {
            if halt_flag.load(Ordering::Relaxed) {
                return Ok(());
            }
            // Convert to view-relative coordinates
            let color = raytracer::cast_rays_into_scene(render_settings, scene, &chunk.viewport, p.viewport_x, p.viewport_y, &mut rng);
            buffer.put_pixel(p.chunk_x, p.chunk_y, v3_to_rgba(color));
        }
        let elapsed = time.elapsed();
        // Send final frame and results
        result_sender.send(Frame(chunk.clone(), buffer))?;
        result_sender.send(Done(elapsed))?;
    }
}

#[allow(unused)]
struct RenderThread {
    id: u32,
    handle: JoinHandle<()>,
    result_receiver: Receiver<RenderResult>,
    total_time_secs: f64,
    total_chunks_rendered: u32,
}

struct RenderWorkerHandle {
    halt_flag: Arc<AtomicBool>,
    work_sender: MPMCSender<RenderWork>,
    thread_handles: Vec<RenderThread>,
}

fn start_background_render_threads(render_thread_count: u32) -> RenderWorkerHandle {
    let halt_flag = Arc::new(AtomicBool::new(false));
    let (work_sender, work_receiver) = mpmc_queue::<RenderWork>(render_thread_count as u64 * 2);

    let thread_handles = (0..render_thread_count)
        .map(|id| {
            let halt_flag = halt_flag.clone();
            let work_receiver = work_receiver.clone();
            let (result_sender, result_receiver) = channel::<RenderResult>();
            let handle = spawn(move || {
                if let Err(err) = start_render_thread(halt_flag, &work_receiver, &result_sender) {
                    println!("Worker thread {} terminated: {}", id, err);
                }
                work_receiver.unsubscribe();
            });
            RenderThread {
                id: id,
                handle: handle,
                result_receiver: result_receiver,
                total_time_secs: 0.0,
                total_chunks_rendered: 0,
            }
        })
        .collect::<Vec<_>>();

    RenderWorkerHandle {
        halt_flag,
        work_sender,
        thread_handles
    }
}

impl epi::App for App {

    fn name(&self) -> &str {
        "Raytracer"
    }

    /// Called once before the first frame.
    fn setup(&mut self, _ctx: &egui::CtxRef, _frame: &epi::Frame, storage: Option<&dyn epi::Storage>) {
        if let Some(storage) = storage {
            self.settings = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
        }
    }

    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, &self.settings);
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &epi::Frame) {

        self.frame_history.on_new_frame(ctx.input().time, frame.info().cpu_usage);

        let buffer_updated = self.update();
        if buffer_updated {
            let job = self.render_job.as_ref().unwrap();
            // Update the output texture
            if let Some((texture_id, _)) = self.output_texture.take() {
                frame.free_texture(texture_id);
            }
            let tex_dim = [job.buffer.width, job.buffer.height];
            let tex_data = eframe::epi::Image::from_rgba_unmultiplied(tex_dim, &job.buffer.data);
            let tex_id = frame.alloc_texture(tex_data);
            self.output_texture = Some((tex_id, tex_dim));
        }

        // Ensure we keep updating the UI as long as there's an active job,
        // as we rely on the update loop to keep feeding the worker threads and updating the in-progress image
        if let Some(job) = self.render_job.as_ref() {
            if job.completed_chunk_count != job.total_chunk_count {
                // Tell the backend to repaint as soon as possible
                ctx.request_repaint();
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // Output image
            if let Some((id, dim)) = self.output_texture {
                ui.centered_and_justified(|ui| {
                    // Scale the output texture to fit in the container
                    let container_dim = (ui.available_width(), ui.available_height());
                    let image_dim = (dim[0] as f32, dim[1] as f32);
                    let (width, height) = scale_to_container_dimensions(image_dim, container_dim);
                    ui.image(id, [width, height]);
                });
            }
        });

        // Settings UI
        egui::Window::new("Settings")
            .resizable(false)
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.add(SettingsWidget::new(&mut self.settings));
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