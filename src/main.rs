extern crate piston;
extern crate graphics;
extern crate glutin_window;
extern crate opengl_graphics;
extern crate image;
extern crate rand;
extern crate sha2;

use std::borrow::Borrow;
use std::path::{ Path };
use std::sync::{ Arc };
use std::sync::mpsc::{ channel, Sender, Receiver };
use std::thread::{ spawn, JoinHandle };
use std::time::{ Instant, Duration };

use piston::window::WindowSettings;
use piston::event_loop::*;
use piston::input::*;
use glutin_window::GlutinWindow as Window;
use opengl_graphics::{ GlGraphics, GlyphCache, OpenGL, Texture, TextureSettings };
use image::{ RgbaImage };
use rand::{ weak_rng };

mod raytracer;

use raytracer::{ Scene, RenderSettings, ViewChunk, Viewport, Rgb, RgbOutput };

const WIDTH: u32 = 1440;
const HEIGHT: u32 = 900;
const SAMPLES_PER_PIXEL: u32 = 1000;
const MAX_REFLECTIONS: u32 = 100;
const RENDER_THREAD_COUNT: u32 = 6;
const CHUNK_COUNT: u32 = 24;
const MAX_FRAMES_PER_SECOND: u64 = 5;
const UPDATES_PER_SECOND: u64 = 5;

struct App {
    buffer: RgbaImage,
    texture: Texture,
    render_args: Arc<(Scene, RenderSettings)>,
    pending_chunks: Vec<ViewChunk>,
    threads: Vec<RenderThread>,
}

// Adapt our generic "RGB" interface to Rust-Graphics' OpenGL implementation.
impl RgbOutput for RgbaImage {
    fn set_pixel(&mut self, x: u32, y: u32, value: Rgb) {
        let p = self.get_pixel_mut(x, y);
        p[0] = value[0];
        p[1] = value[1];
        p[2] = value[2];
        p[3] = 255; // Alpha
    }
}

impl App {
    fn new(scene: Scene, render_settings: RenderSettings, pending_chunks: Vec<ViewChunk>, threads: Vec<RenderThread>) -> App {
        let buffer = RgbaImage::new(WIDTH, HEIGHT);
        let texture = Texture::from_image(&buffer, &TextureSettings::new());
        let render_args = Arc::new((scene, render_settings));
        App { buffer, texture, render_args, pending_chunks, threads, }
    }

    fn render(&self, args: &RenderArgs, gl: &mut GlGraphics, font_cache: &mut GlyphCache<'_>) {
        use graphics::*;
        
        gl.draw(args.viewport(), |ctx, gl| {
            // Clear screen
            clear([0.0; 4], gl);
            // Draw the buffer texture
            image(&self.texture, ctx.transform, gl);
            // Draw thread status
            for (offset, thread) in self.threads.iter().enumerate() {
                let font_color = [0.0, 0.5, 0.0, 1.0];
                let font_size = 15;
                let transform = ctx.transform.trans(10.0, (offset * 20 + 25) as f64);
                let label = format!(
                    "Thread {}: {} chunks, {:.4} seconds (avg {:.4}s)",
                    thread.id, thread.total_chunks_rendered, thread.total_time_secs,
                    thread.total_time_secs / thread.total_chunks_rendered as f64
                );
                text(font_color, font_size, &label, font_cache, transform, gl).unwrap();
            }
        });
    }

    fn update(&mut self, _args: &UpdateArgs) {
        use RenderResult::*;

        // Poll each thread for completed work
        for thread in &mut self.threads {
            if let Ok(result) = thread.receiver.try_recv() {
                if let WorkCompleted(chunk, rgb, elapsed) = result {
                    // Render chunk to buffer
                    copy_chunk_to_buffer(&mut self.buffer, &chunk, &rgb);
                    // Update stats
                    thread.total_time_secs += elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9;
                    thread.total_chunks_rendered += 1;
                }
                // Send new work
                if let Some(chunk) = self.pending_chunks.pop() {
                    // Hack - paint in-progress chunks green
                    let mut chunk_buffer = RgbaImage::new(chunk.width, chunk.height);
                    for y in 0..chunk.height {
                        for x in 0..chunk.width {
                            chunk_buffer.set_pixel(x, y, [0, 150, 0]);
                        }
                    }
                    copy_chunk_to_buffer(&mut self.buffer, &chunk, &chunk_buffer);

                    let work = RenderWork(chunk, chunk_buffer, self.render_args.clone());
                    thread.sender.send(work).expect("Sending work");
                }
            }
        }

        self.texture.update(&self.buffer);
    }
}

fn copy_chunk_to_buffer(buffer: &mut RgbaImage, chunk: &ViewChunk, chunk_buffer: &RgbaImage) {
    for x in 0..chunk.width {
        for y in 0..chunk.height {
            buffer.put_pixel(chunk.left + x, chunk.top + y, *chunk_buffer.get_pixel(x, y));
        }
    }
}

struct RenderWork (ViewChunk, RgbaImage, Arc<(Scene, RenderSettings)>);

enum RenderResult {
    Ready,
    WorkCompleted(ViewChunk, RgbaImage, Duration)
}

fn start_render_thread(work_receiver: Receiver<RenderWork>, result_sender: Sender<RenderResult>) {
    let mut rng = weak_rng();
    result_sender.send(RenderResult::Ready).expect("Worker ready");
    loop {
        // Receive work
        let RenderWork (chunk, mut rgb, args) = match work_receiver.recv() {
            Err(_) => break,
            Ok(work) => work
        };
        // Render
        let (scene, render_settings) = args.borrow();
        let time = Instant::now();
        raytracer::cast_rays_into_scene(scene, render_settings, &chunk, &mut rgb, &mut rng);
        let elapsed = time.elapsed();
        // Send result
        let result = RenderResult::WorkCompleted(chunk, rgb, elapsed);
        if let Err(_) = result_sender.send(result) {
            break;
        }
    }
}

struct RenderThread {
    id: u32,
    handle: JoinHandle<()>,
    sender: Sender<RenderWork>,
    receiver: Receiver<RenderResult>,
    total_time_secs: f64,
    total_chunks_rendered: u32,
}

fn start_background_render_threads(render_thread_count: u32) -> Vec<RenderThread>  {
    (0..render_thread_count)
        .map(move |id| {
            let (work_sender, work_receiver) = channel::<RenderWork>();
            let (result_sender, result_receiver) = channel::<RenderResult>();
            let handle = spawn(move || start_render_thread(work_receiver, result_sender));
            RenderThread {
                id: id,
                handle: handle,
                sender: work_sender,
                receiver: result_receiver,
                total_time_secs: 0.0,
                total_chunks_rendered: 0,
            }
        })
        .collect()
}

fn make_chunks_list(viewport: &Viewport, chunk_count: u32) -> Vec<ViewChunk> {
    let mut chunks = viewport.create_view_chunks(chunk_count);
    
    // Chunks are popped from this list as they are rendered.
    // Reverse the list so the top of the image is rendered first.
    chunks.reverse();
    chunks
}

fn main() {
    // Change this to OpenGL::V2_1 if not working.
    let opengl = OpenGL::V3_2;

    println!("Creating scene");
    let viewport = Viewport::new(WIDTH, HEIGHT);
    let scene = raytracer::samples::simple_scene(&viewport);

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
    let render_threads = start_background_render_threads(RENDER_THREAD_COUNT);
    
    println!("Starting main event loop");
    let render_settings = RenderSettings {
        max_reflections: MAX_REFLECTIONS,
        samples_per_pixel: SAMPLES_PER_PIXEL
    };
    let mut app = App::new(scene, render_settings, chunk_list, render_threads);
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

    println!("Waiting for render threads to terminate");
    for thread in app.threads {
        drop(thread.sender);
        drop(thread.receiver);
        thread.handle.join().unwrap();
    }

    println!("Writing rendered image to disk");
    app.buffer.save("test.png").unwrap();
}