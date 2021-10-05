use std::sync::{ Arc };
use std::thread::{ spawn, JoinHandle };
use std::time::{ Instant, Duration };
use std::sync::mpsc::{ Receiver, Sender, channel };

use eframe::{ egui, epi };
use rand::{ weak_rng };
use multiqueue::{ mpmc_queue, MPMCReceiver, MPMCSender };
use raytracer::{ Scene, RenderSettings, RenderChunk, Viewport };

const WIDTH: u32 = 1440;
const HEIGHT: u32 = 900;
const RENDER_THREAD_COUNT: u32 = 6;
const CHUNK_COUNT: u32 = 128;
const MAX_FRAMES_PER_SECOND: u64 = 10;
const UPDATES_PER_SECOND: u64 = 10;

fn duration_total_secs(elapsed: Duration) -> f64 {
    elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9
}

type BoxError = Box<dyn std::error::Error + 'static>;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
enum RenderMode {
    Quality(u32),
    Fast,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
enum TestScene {
    RandomSpheres,
    Simple,
    Planes,
    Mirrors,
    Triangles,
    Mesh,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct State {
    render_mode: RenderMode,
    selected_scene: TestScene,
}

impl Default for State {
    fn default() -> Self {
        State {
            render_mode: RenderMode::Fast,
            selected_scene: TestScene::RandomSpheres,
        }
    }
}

#[derive(Clone)]
struct ColorBuffer {
    width: usize,
    height: usize,
    data: Vec<egui::Color32>,
}

impl ColorBuffer {
    fn new(width: u32, height: u32) -> ColorBuffer {
        ColorBuffer {
            width: width as usize,
            height: height as usize,
            data: vec![egui::Color32::BLACK; (width * height) as usize],
        }
    }

    fn put_pixel(&mut self, x: u32, y: u32, color: egui::Color32) {
        self.data[(y as usize) * self.width + (x as usize)] = color;
    }

    fn get_pixel(&self, x: u32, y: u32) -> egui::Color32 {
        self.data[(y as usize) * self.width + (x as usize)]
    }
}

fn v3_to_color32(v3: raytracer::V3) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(
        (255.0 * v3.0.sqrt()) as u8,
        (255.0 * v3.1.sqrt()) as u8,
        (255.0 * v3.2.sqrt()) as u8,
        255 // Alpha
    )
}

struct RenderJob {
    render_args: Arc<(Scene, RenderSettings)>,
    pending_chunks: Vec<RenderChunk>,
    start_time: Instant,
    render_time_secs: f64,
    total_chunk_count: u32,
    completed_chunk_count: u32,
    buffer: ColorBuffer,
    worker_handle: RenderWorkerHandle,
}

pub struct App {
    // Persistent state
    state: State,
    // Temporal state
    output_texture_id: Option<egui::TextureId>,
    render_job: Option<RenderJob>,
}

impl Default for App {
    fn default() -> Self {
        App {
            // Persistent state
            state: State::default(),
            // Temporal state
            output_texture_id: None,
            render_job: None,
        }
    }
}

impl App {

    // TODO
    fn shutdown(&mut self) {
        // Clean up worker threads
        if let Some(render_job) = self.render_job.take() {
            // TODO: Push "shutdown" message
            // Detatch work queues
            render_job.worker_handle.work_sender.unsubscribe();
            // Wait for threads to terminate
            for thread in render_job.worker_handle.thread_handles {
                drop(thread.result_receiver);
                thread.handle.join().expect("Waiting for worker to terminate");
            }
        }
    }

    fn start_job(&mut self) {

        // TODO: Stop running worker threads if an existing job is in progress

        // Create render work arguments
        let viewport = Viewport::new(WIDTH, HEIGHT);
        let camera_aperture = match self.state.render_mode {
            RenderMode::Fast => 0.0,
            RenderMode::Quality(_) => 0.1,
        };
        // let scene = match self.state.test_scene {
        //     TestScene::RandomSpheres => raytracer::samples::random_sphere_scene(&viewport, camera_aperture),
        //     TestScene::Simple => raytracer::samples::simple_scene(&viewport, camera_aperture),
        //     TestScene::Planes => raytracer::samples::planes_scene(&viewport, camera_aperture),
        //     TestScene::Mirrors => raytracer::samples::hall_of_mirrors(&viewport, camera_aperture),
        //     TestScene::Triangles => raytracer::samples::triangle_world(&viewport, camera_aperture),
        //     TestScene::Mesh => raytracer::samples::mesh_demo(&viewport, camera_aperture),
        // };
        let scene = raytracer::samples::random_sphere_scene(&viewport, camera_aperture);

        let settings = RenderSettings {
            max_reflections: match self.state.render_mode {
                RenderMode::Fast => 5,
                RenderMode::Quality(_) => 25
            },
            anti_alias: match self.state.render_mode {
                RenderMode::Fast => false,
                RenderMode::Quality(_) => true
            },
        };

        // Chunks are popped from this list as they are rendered.
        // Reverse the list so the top of the image is rendered first.
        let mut chunks = viewport.create_render_chunks(CHUNK_COUNT);
        chunks.reverse();

        self.render_job = Some(RenderJob {
            render_args: Arc::new((scene, settings)),
            total_chunk_count: chunks.len() as u32,
            completed_chunk_count: 0,
            pending_chunks: chunks,
            start_time: Instant::now(),
            render_time_secs: 0_f64,
            buffer: ColorBuffer::new(WIDTH, HEIGHT),
            worker_handle: start_background_render_threads(RENDER_THREAD_COUNT),
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
                match job.worker_handle.work_sender.try_send(ThreadMessage::Work(work)) {
                    Ok(_) => {
                        // Move to next thread
                        continue;
                    },
                    Err(TrySendError::Full(ThreadMessage::Work(RenderWork(v, _)))) => {
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
enum ThreadMessage {
    Work(RenderWork),
    Stop
}

#[derive(Clone)]
enum RenderResult {
    Ready,
    Frame(RenderChunk, ColorBuffer),
    Done(Duration),
}

fn start_render_thread(work_receiver: &MPMCReceiver<ThreadMessage>, result_sender: &Sender<RenderResult>) -> Result<(), BoxError> {
    let mut rng = weak_rng();
    result_sender.send(RenderResult::Ready)?;
    // Receive messages
    loop {
        let work = match work_receiver.recv()? {
            ThreadMessage::Stop => break,
            ThreadMessage::Work(work) => work
        };
        render_thread_handler(work, result_sender, &mut rng)?;
    }
    // Done
    Ok(())
}

fn render_thread_handler(work: RenderWork, result_sender: &Sender<RenderResult>, rng: &mut impl rand::Rng) -> Result<(), BoxError> {
    use RenderResult::*;
    let RenderWork(chunk, args) = work;
    // Paint in-progress chunks green
    let mut buf = ColorBuffer::new(chunk.width, chunk.height);
    for p in chunk.iter_pixels() {
        buf.put_pixel(p.chunk_x, p.chunk_y, v3_to_color32(raytracer::V3(0.0, 0.58, 0.0)));
    }
    result_sender.send(Frame(chunk.clone(), buf.clone()))?;
    // Render the scene chunk
    let (scene, render_settings) = args.as_ref();
    let time = Instant::now();
    // For each x, y coordinate in this view chunk, cast a ray.
    for p in chunk.iter_pixels() {
        // Convert to view-relative coordinates
        let color = raytracer::cast_ray_into_scene(render_settings, scene, &chunk.viewport, p.viewport_x, p.viewport_y, rng);
        buf.put_pixel(p.chunk_x, p.chunk_y, v3_to_color32(color));
    }
    let elapsed = time.elapsed();
    // Send final frame and results
    result_sender.send(Frame(chunk.clone(), buf))?;
    result_sender.send(Done(elapsed))?;
    Ok(())
}

struct RenderThread {
    id: u32,
    handle: JoinHandle<()>,
    result_receiver: Receiver<RenderResult>,
    total_time_secs: f64,
    total_chunks_rendered: u32,
}

struct RenderWorkerHandle {
    work_sender: MPMCSender<ThreadMessage>,
    thread_handles: Vec<RenderThread>,
}

fn start_background_render_threads(render_thread_count: u32) -> RenderWorkerHandle {
    let (work_sender, work_receiver) = mpmc_queue::<ThreadMessage>(render_thread_count as u64 * 2);

    let thread_handles = (0..render_thread_count)
        .map(move |id| {
            let work_receiver = work_receiver.clone();
            let (result_sender, result_receiver) = channel::<RenderResult>();
            let handle = spawn(move || {
                if let Err(err) = start_render_thread(&work_receiver, &result_sender) {
                    // TODO: Error logs
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
        work_sender,
        thread_handles
    }
}

impl epi::App for App {

    fn name(&self) -> &str {
        "Raytracer"
    }

    /// Called once before the first frame.
    fn setup(&mut self, _ctx: &egui::CtxRef, _frame: &mut epi::Frame<'_>, storage: Option<&dyn epi::Storage>) {

        if let Some(storage) = storage {
            self.state = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
        }
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {

        let buffer_updated = self.update();

        if let Some(ref mut job) = self.render_job {
            // Update the texture
            if buffer_updated {
                if let Some(texture_id) = self.output_texture_id.take() {
                    frame.tex_allocator().free(texture_id);
                }
                let texture_id = frame.tex_allocator().alloc_srgba_premultiplied(
                    (job.buffer.width, job.buffer.height),
                    &job.buffer.data
                );
                self.output_texture_id = Some(texture_id);
            }
        }

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.heading("Raytracer");

            if ui.button("Start").clicked() {
                self.start_job();
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {

            if let Some(output_texture_id) = self.output_texture_id {

                // TODO: Stretch over Width or Height
                ui.image(output_texture_id, [ui.available_width(), ui.available_height()]);
            }
        });
    }
}