extern crate piston;
extern crate graphics;
extern crate glutin_window;
extern crate opengl_graphics;
extern crate image;
extern crate rand;
extern crate multiqueue;

use std::borrow::Borrow;
use std::path::{ Path };
use std::sync::{ Arc };
use std::thread::{ spawn, JoinHandle };
use std::time::{ Instant, Duration };
use std::sync::mpsc::{ Receiver, SendError, Sender, channel };

use piston::window::WindowSettings;
use piston::event_loop::*;
use piston::input::*;
use glutin_window::GlutinWindow as Window;
use opengl_graphics::{ GlGraphics, GlyphCache, OpenGL, Texture, TextureSettings };
use image::{ RgbaImage };
use rand::{ weak_rng };
use multiqueue::{ mpmc_queue, MPMCReceiver, MPMCSender };

mod raytracer;

use raytracer::{ Scene, RenderSettings, ViewChunk, Viewport };

const WIDTH: u32 = 1440;
const HEIGHT: u32 = 900;
const MAX_REFLECTIONS: u32 = 25;
const RENDER_THREAD_COUNT: u32 = 6;
const CHUNK_COUNT: u32 = 128;
const MAX_FRAMES_PER_SECOND: u64 = 10;
const UPDATES_PER_SECOND: u64 = 10;

fn duration_total_secs(elapsed: Duration) -> f64 {
    elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9
}

struct App {
    buffer: RgbaImage,
    texture: Texture,
    render_args: Arc<(Scene, RenderSettings)>,
    pending_chunks: Vec<ViewChunk>,
    worker_handle: RenderWorkerHandle,
    start_time: Instant,
    render_time_secs: f64,
    total_chunk_count: u32,
    completed_chunk_count: u32,
}

impl App {
    fn new(scene: Scene, render_settings: RenderSettings, pending_chunks: Vec<ViewChunk>, worker_handle: RenderWorkerHandle) -> App {
        let mut texture_settings = TextureSettings::new();
        texture_settings.set_convert_gamma(true);
        let buffer = RgbaImage::new(WIDTH, HEIGHT);
        let texture = Texture::from_image(&buffer, &texture_settings);
        App {
            buffer,
            texture,
            render_args: Arc::new((scene, render_settings)),
            total_chunk_count: pending_chunks.len() as u32,
            pending_chunks,
            worker_handle,
            start_time: Instant::now(),
            render_time_secs: 0_f64,
            completed_chunk_count: 0,
        }
    }

    fn render(&self, args: &RenderArgs, gl: &mut GlGraphics, font_cache: &mut GlyphCache<'_>) {
        use graphics::*;

        // Thread diagnostics
        let thread_diagnostics = self.worker_handle.thread_handles.iter()
            .map(|thread| format!(
                "Thread {}: {} chunks, {:.4} seconds (avg {:.4}s per chunk)",
                thread.id, thread.total_chunks_rendered, thread.total_time_secs,
                thread.total_time_secs / thread.total_chunks_rendered as f64
            ));

        // Overall diagnostics
        let percent_complete = (self.completed_chunk_count as f32 / self.total_chunk_count as f32) * 100.0_f32;
        let overall_diagnostic = format!("Progress: {:.0}% complete ({} chunks), {:.4} seconds", percent_complete, self.completed_chunk_count, self.render_time_secs);

        let diagnostic_strings = thread_diagnostics
            .chain(std::iter::once(overall_diagnostic));
        
        gl.draw(args.viewport(), |ctx, gl| {
            // Clear screen
            clear([0.0; 4], gl);
            // Draw the buffer texture
            image(&self.texture, ctx.transform, gl);
            // Draw diagnostics
            for (offset, label) in diagnostic_strings.enumerate() {
                let font_color = [0.0, 0.5, 0.0, 1.0];
                let font_size = 15;
                let transform = ctx.transform.trans(10.0, (offset * 20 + 25) as f64);
                text(font_color, font_size, &label, font_cache, transform, gl).unwrap();
            }
        });
    }

    fn update(&mut self, _args: &UpdateArgs) {
        use RenderResult::*;

        let mut buffer_updated = false;

        // Poll for completed work
        for thread in self.worker_handle.thread_handles.iter_mut() {
            while let Ok(result) = thread.result_receiver.try_recv() {
                match result {
                    Frame(chunk, buf) => {
                        // Copy chunk to buffer
                        for p in chunk.iter_pixels() {
                            self.buffer.put_pixel(p.viewport_x, p.viewport_y, *buf.get_pixel(p.chunk_x, p.chunk_y));
                        }
                        buffer_updated = true;
                    },
                    Ready => {}, // Worker thread ready to go.
                    Done(elapsed) => {
                        // Update stats
                        thread.total_time_secs += duration_total_secs(elapsed);
                        thread.total_chunks_rendered += 1;
                        self.completed_chunk_count += 1;
                    },
                }
            }
        }

        if buffer_updated {
            self.texture.update(&self.buffer);
        }

        // Refill the the work queue
        use std::sync::mpsc::TrySendError;
        while let Some(chunk) = self.pending_chunks.pop() {
            let work = RenderWork(chunk, self.render_args.clone());
            match self.worker_handle.work_sender.try_send(work) {
                Ok(_) => {
                    // Move to next thread
                    continue;
                },
                Err(TrySendError::Full(RenderWork(v, _))) => {
                    // Queue full, try again later
                    self.pending_chunks.push(v);
                    break;
                },
                Err(_) => unreachable!(),
            }
        }

        // Update timer, as long as we have outstanding work
        if self.completed_chunk_count < self.total_chunk_count {
            self.render_time_secs = duration_total_secs(self.start_time.elapsed());
        }
    }
}

#[derive(Clone)]
struct RenderWork (ViewChunk, Arc<(Scene, RenderSettings)>);

#[derive(Clone)]
enum RenderResult {
    Ready,
    Frame(ViewChunk, Arc<RgbaImage>),
    Done(Duration),
}

fn start_render_thread(work_receiver: &MPMCReceiver<RenderWork>, result_sender: &Sender<RenderResult>) -> Result<(), SendError<RenderResult>> {
    use RenderResult::*;
    let mut rng = weak_rng();
    result_sender.send(Ready)?;
    // Receive work
    while let Ok(RenderWork(chunk, args)) = work_receiver.recv() {
        // Paint in-progress chunks green
        let mut buf = RgbaImage::new(chunk.width, chunk.height);
        for p in chunk.iter_pixels() {
            write_pixel_color(&mut buf, p.chunk_x, p.chunk_y, raytracer::V3(0.0, 0.58, 0.0));
        }
        result_sender.send(Frame(chunk.clone(), Arc::new(buf.clone())))?;
        // Render the scene chunk
        let (scene, render_settings) = args.borrow();
        let time = Instant::now();
        // For each x, y coordinate in this view chunk, cast a ray.
        for p in chunk.iter_pixels() {
            // Convert to view-relative coordinates
            let color = raytracer::cast_ray_into_scene(render_settings, scene, &chunk.viewport, p.viewport_x, p.viewport_y, &mut rng);
            write_pixel_color(&mut buf, p.chunk_x, p.chunk_y, color);
        }
        let elapsed = time.elapsed();
        // Send final frame and results
        result_sender.send(Frame(chunk.clone(), Arc::new(buf)))?;
        result_sender.send(Done(elapsed))?;
    }
    Ok(())
}

// Wtite a generic "RGB" value to an OpenGL image
fn write_pixel_color(buf: &mut RgbaImage, x: u32, y: u32, color: raytracer::V3) {
    // Convert from RGB in sRGB color space to linear color space
    let color = graphics::color::gamma_srgb_to_linear([color.0, color.1, color.2, 1.0]);
    let p = buf.get_pixel_mut(x, y);
    p[0] = (255.0 * color[0].sqrt()) as u8;
    p[1] = (255.0 * color[1].sqrt()) as u8;
    p[2] = (255.0 * color[2].sqrt()) as u8;
    p[3] = (255.0 * color[3].sqrt()) as u8;
}

struct RenderThread {
    id: u32,
    handle: JoinHandle<()>,
    result_receiver: Receiver<RenderResult>,
    total_time_secs: f64,
    total_chunks_rendered: u32,
}

struct RenderWorkerHandle {
    work_sender: MPMCSender<RenderWork>,
    thread_handles: Vec<RenderThread>,
}

fn start_background_render_threads(render_thread_count: u32) -> RenderWorkerHandle  {
    let (work_sender, work_receiver) = mpmc_queue::<RenderWork>(render_thread_count as u64 * 2);

    let thread_handles = (0..render_thread_count)
        .map(move |id| {
            let work_receiver = work_receiver.clone();
            let (result_sender, result_receiver) = channel::<RenderResult>();
            let handle = spawn(move || {
                if let Err(err) = start_render_thread(&work_receiver, &result_sender) {
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

fn make_chunks_list(viewport: &Viewport, chunk_count: u32) -> Vec<ViewChunk> {
    let mut chunks = viewport.create_view_chunks(chunk_count);
    
    // Chunks are popped from this list as they are rendered.
    // Reverse the list so the top of the image is rendered first.
    chunks.reverse();
    chunks
}

#[derive(Debug)]
enum Mode {
    Quality(u32),
    Fast,
}

fn main() {
    let mode = match std::env::args().skip(1).next().map(|s| s.parse()) {
        Some(Ok(quality)) => Mode::Quality(quality),
        _ => Mode::Fast,
    };

    println!("Running in mode {:?}", mode);

    // Change this to OpenGL::V2_1 if not working.
    let opengl = OpenGL::V3_2;

    println!("Creating scene");
    let viewport = Viewport::new(WIDTH, HEIGHT);
    let camera_aperture = match mode {
        Mode::Fast => 0.0,
        Mode::Quality(_) => 0.1,
    };
    let scene = raytracer::samples::simple_scene(&viewport, camera_aperture);

    println!("Creating window");
    let mut window: Window =
        WindowSettings::new("raytracer", [WIDTH, HEIGHT])
            .opengl(opengl)
            .exit_on_esc(true)
            .build()
            .unwrap();

    println!("Preparing graphics");
    let font_path = Path::new("fonts/FiraSans-Regular.ttf");
    let mut font_cache = GlyphCache::new(font_path, (), TextureSettings::new()).expect("Loading font");
    let mut gl = GlGraphics::new(opengl);
    
    println!("Making chunk list");
    let chunk_list = make_chunks_list(&viewport, CHUNK_COUNT);

    println!("Starting render threads");
    let worker_handle = start_background_render_threads(RENDER_THREAD_COUNT);
    
    println!("Starting main event loop");
    let render_settings = RenderSettings {
        max_reflections: MAX_REFLECTIONS,
        samples_per_pixel: match mode {
            Mode::Fast => 1,
            Mode::Quality(quality) => quality
        },
    };
    let mut app = App::new(scene, render_settings, chunk_list, worker_handle);
    let mut events =
        Events::new(EventSettings::new())
            .max_fps(MAX_FRAMES_PER_SECOND)
            .ups(UPDATES_PER_SECOND);

    while let Some(e) = events.next(&mut window) {
        if let Some(r) = e.render_args() {
            app.render(&r, &mut gl, &mut font_cache);
        }
        if let Some(u) = e.update_args() {
            app.update(&u);
        }
    }

    println!("Stopping work queues");
    app.worker_handle.work_sender.unsubscribe();

    println!("Waiting for render threads to terminate");
    for thread in app.worker_handle.thread_handles {
        drop(thread.result_receiver);
        thread.handle.join().expect("Waiting for worker to terminate");
    }

    println!("Writing rendered image to disk");
    app.buffer.save("test.png").expect("Writing render to disk");
}