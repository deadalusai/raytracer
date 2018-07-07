extern crate piston;
extern crate graphics;
extern crate glutin_window;
extern crate opengl_graphics;
extern crate image;
extern crate rand;
extern crate sha2;

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
use image::RgbaImage;
use rand::{ weak_rng };

mod raytracer;

use raytracer::{ Scene, Viewport, ViewChunk, Rgb };

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const SAMPLES_PER_PIXEL: u32 = 100;
const MAX_REFLECTIONS: u32 = 100;
const CHUNK_COUNT: u32 = 100;
const RENDER_THREAD_COUNT: u32 = 4;
const MAX_FRAMES_PER_SECOND: u64 = 30;
const UPDATES_PER_SECOND: u64 = 10;

struct App {
    buffer: RgbaImage,
    scene: Arc<Scene>,
    pending_chunks: Vec<ViewChunk>,
    threads: Vec<RenderThread>,
}

impl App {
    fn render(&self, args: &RenderArgs, gl: &mut GlGraphics, font_cache: &mut GlyphCache<'_>) {
        use graphics::*;

        let buffer_texture = Texture::from_image(&self.buffer, &TextureSettings::new());
        
        gl.draw(args.viewport(), |ctx, gl| {
            // Clear screen
            clear([0.0; 4], gl);
            // Draw the buffer texture
            image(&buffer_texture, ctx.transform, gl);
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
                if let WorkCompleted(chunk, elapsed) = result {
                    // Render chunk to buffer
                    copy_view_chunk_to_image_buffer(&mut self.buffer, &chunk);
                    // Update stats
                    thread.total_time_secs += elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9;
                    thread.total_chunks_rendered += 1;
                }
                // Send new work
                if let Some(mut chunk) = self.pending_chunks.pop() {

                    // Hack - paint in-progress chunks green
                    for chunk_y in 0..chunk.height {
                        for chunk_x in 0..chunk.width {
                            chunk.set_chunk_pixel(chunk_x, chunk_y, Rgb { r: 0, g: 150, b: 0 });
                        }
                    }
                    copy_view_chunk_to_image_buffer(&mut self.buffer, &chunk);

                    let work = RenderWork(chunk, self.scene.clone());
                    thread.sender.send(work).expect("Sending work");
                }
            }
        }
    }
}

fn copy_view_chunk_to_image_buffer (buffer: &mut RgbaImage, chunk: &ViewChunk) {
    for chunk_y in 0..chunk.height {
        for chunk_x in 0..chunk.width {
            let col = chunk.get_chunk_pixel(chunk_x, chunk_y);
            let (view_x, view_y) = chunk.get_view_relative_coords(chunk_x, chunk_y);
            let pixel = buffer.get_pixel_mut(view_x, view_y);
            pixel.data = [col.r, col.g, col.b, 255];
        }
    }
}

struct RenderWork (ViewChunk, Arc<Scene>);

enum RenderResult {
    Ready,
    WorkCompleted(ViewChunk, Duration)
}

fn start_render_thread (work_receiver: Receiver<RenderWork>, result_sender: Sender<RenderResult>) {
    let mut rng = weak_rng();
    result_sender.send(RenderResult::Ready).expect("Worker ready");
    loop {
        // Receive work
        let RenderWork (mut chunk, scene) = match work_receiver.recv() {
            Err(_) => break,
            Ok(work) => work
        };
        // Render
        let time = Instant::now();
        raytracer::cast_rays_into_scene(&mut chunk, &mut rng, &*scene, SAMPLES_PER_PIXEL, MAX_REFLECTIONS);
        let elapsed = time.elapsed();
        // Send result
        let result = RenderResult::WorkCompleted(chunk, elapsed);
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

fn start_background_render_threads () -> Vec<RenderThread>  {
    (0..RENDER_THREAD_COUNT)
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

fn make_chunks_list (viewport: &Viewport, chunk_count: u32) -> Vec<ViewChunk> {
    let divisions = (chunk_count as f32).sqrt();
    let h_count = (viewport.width as f32 / (viewport.width as f32 / divisions)) as u32;
    let v_count = (viewport.height as f32 / (viewport.height as f32 / divisions)) as u32;
    let mut chunks = viewport.iter_view_chunks(h_count, v_count).collect::<Vec<_>>();
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
    let scene = raytracer::samples::random_sphere_scene(&viewport);

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
    
    println!("Starting render threads");

    // Create a new game and run it.
    let mut app = App {
        buffer: RgbaImage::new(WIDTH, HEIGHT),
        scene: Arc::new(scene),
        pending_chunks: make_chunks_list(&viewport, CHUNK_COUNT),
        threads: start_background_render_threads(),
    };
    
    println!("Starting main event loop");
    
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

    println!("Writing rendered image to disk");
    
    app.buffer.save("test.png").unwrap();

    println!("Waiting for render threads to terminate");

    for thread in app.threads {
        drop(thread.sender);
        drop(thread.receiver);
        thread.handle.join().unwrap();
    }
}