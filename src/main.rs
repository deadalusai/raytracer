extern crate piston;
extern crate graphics;
extern crate glutin_window;
extern crate opengl_graphics;
extern crate image;
extern crate rand;

use std::sync::{ Arc };
use std::sync::mpsc::{ channel, Sender, Receiver };
use std::thread::{ spawn, JoinHandle };
use std::time::{ Instant, Duration };

use piston::window::WindowSettings;
use piston::event_loop::*;
use piston::input::*;
use glutin_window::GlutinWindow as Window;
use opengl_graphics::{ GlGraphics, OpenGL, Texture, TextureSettings };
use image::RgbaImage;
use rand::{ Rng, thread_rng };

mod raytracer;

use raytracer::{ Scene, Viewport, ViewChunk };

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const SAMPLES_PER_PIXEL: u32 = 10;
const CHUNK_COUNT: u32 = 100;
const RENDER_THREAD_COUNT: u32 = 3;

struct App {
    gl: GlGraphics,                 // OpenGL drawing backend
    buffer: RgbaImage,              // Buffer
    scene: Arc<Scene>,
    pending_chunks: Vec<ViewChunk>, // List of pending chunks
    threads: Vec<RenderThread>,     // List of thread handles
}

impl App {
    fn render(&mut self, args: &RenderArgs) {
        use graphics::*;

        let texture = Texture::from_image(&self.buffer, &TextureSettings::new());
        
        self.gl.draw(args.viewport(), |ctx, gl| {
            // Clear screen
            clear([0.0; 4], gl);
            // Apply transformations
            let transform = ctx.transform;
            // Draw the buffer texture
            image(&texture, transform, gl);
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
                    thread.total_time_secs += elapsed.as_secs();
                    thread.total_renders += 1;
                }
                // Send new work
                if let Some(chunk) = self.pending_chunks.pop() {
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
    result_sender.send(RenderResult::Ready).expect("Worker ready");
    loop {
        // Receive work
        let RenderWork (mut chunk, scene) = match work_receiver.recv() {
            Err(_) => return,
            Ok(work) => work
        };
        // Render
        let time = Instant::now();
        raytracer::cast_rays_into_scene(&mut chunk, &*scene, SAMPLES_PER_PIXEL);
        let elapsed = time.elapsed();
        // Send result
        let ok = result_sender.send(RenderResult::WorkCompleted(chunk, elapsed));
        // Main thread terminated?
        if ok.is_err() {
            break;
        }
    }
}

struct RenderThread {
    handle: JoinHandle<()>,
    sender: Sender<RenderWork>,
    receiver: Receiver<RenderResult>,
    total_time_secs: u64,
    total_renders: u64,
}

fn start_background_render_threads () -> Vec<RenderThread>  {

    (0..RENDER_THREAD_COUNT)
        .map(move |_| {
            let (work_sender, work_receiver) = channel::<RenderWork>();
            let (result_sender, result_receiver) = channel::<RenderResult>();
            let handle = spawn(move || start_render_thread(work_receiver, result_sender));
            RenderThread {
                handle: handle,
                sender: work_sender,
                receiver: result_receiver,
                total_time_secs: 0,
                total_renders: 0,
            }
        })
        .collect::<Vec<_>>()
}

fn make_chunks_list (viewport: &Viewport, chunk_count: u32) -> Vec<ViewChunk> {
    let divisions = (chunk_count as f32).sqrt();
    let h_count = (viewport.width as f32 / (viewport.width as f32 / divisions)) as u32;
    let v_count = (viewport.height as f32 / (viewport.height as f32 / divisions)) as u32;
    let mut chunks: Vec<_> =
        viewport.iter_view_chunks(h_count, v_count)
                .collect();
    // hax - randomly sort chunks
    let mut rng = thread_rng();
    chunks.sort_unstable_by_key(move |_| rng.next_u32());
    chunks
}

fn main() {
    // Change this to OpenGL::V2_1 if not working.
    let opengl = OpenGL::V3_2;

    println!("Creating scene");

    let viewport = Viewport::new(WIDTH, HEIGHT);
    let scene = raytracer::samples::random_sphere_scene(&viewport);

    let chunks = make_chunks_list(&viewport, CHUNK_COUNT);
    
    println!("Creating window");

    let mut window: Window =
        WindowSettings::new("raytracer", [WIDTH, HEIGHT])
            .opengl(opengl)
            .exit_on_esc(true)
            .build()
            .unwrap();
    
    println!("Starting render threads");

    let threads = start_background_render_threads();

    // Create a new game and run it.
    let mut app = App {
        gl: GlGraphics::new(opengl),
        buffer: RgbaImage::new(WIDTH, HEIGHT),
        scene: Arc::new(scene),
        pending_chunks: chunks,
        threads: threads,
    };
    
    println!("Starting main event loop");
    
    let mut events = Events::new(EventSettings::new());
    while let Some(e) = events.next(&mut window) {
        if let Some(r) = e.render_args() {
            app.render(&r);
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