extern crate piston;
extern crate graphics;
extern crate glutin_window;
extern crate opengl_graphics;
extern crate image;
extern crate rand;

use std::sync::{ Mutex, Arc };
use std::thread::{ spawn, JoinHandle };

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
const SAMPLES_PER_PIXEL: u32 = 100;
const CHUNK_COUNT: u32 = 1000;
const RENDER_THREAD_COUNT: u32 = 4;

struct Chunks {
    count: u32,
    ready: Vec<ViewChunk>,
    completed: Vec<ViewChunk>,
}

impl Chunks {
    fn from_viewport (viewport: &Viewport, chunk_count: u32) -> Chunks {
        let divisions = (chunk_count as f32).sqrt();
        let h_count = (viewport.width as f32 / (viewport.width as f32 / divisions)) as u32;
        let v_count = (viewport.height as f32 / (viewport.height as f32 / divisions)) as u32;
        let mut chunks: Vec<_> =
            viewport.iter_view_chunks(h_count, v_count)
                    .collect();

        // hax - randomly sort chunks
        let mut rng = thread_rng();
        chunks.sort_unstable_by_key(move |_| rng.next_u32());

        Chunks {
            count: chunks.len() as u32,
            ready: chunks,
            completed: vec!()
        }
    }
}

struct App {
    gl: GlGraphics,    // OpenGL drawing backend.
    buffer: RgbaImage, // Buffer
    chunks: Arc<Mutex<Chunks>>, // List of chunks completed and in progress
    t: f64,
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

    fn update(&mut self, args: &UpdateArgs) {
        self.t += args.dt;

        // Only update once every few seconds
        if self.t < 1.0 {
            return;
        }

        // Render chunks to buffer
        if let Ok(ref mut chunks) = self.chunks.lock() {
            // For each completed chunk...
            for chunk in chunks.completed.iter() {
                // Draw chunk to buffer
                for chunk_y in 0..chunk.height {
                    for chunk_x in 0..chunk.width {
                        let col = chunk.get_chunk_pixel(chunk_x, chunk_y);
                        let (view_x, view_y) = chunk.get_view_relative_coords(chunk_x, chunk_y);
                        let pixel = self.buffer.get_pixel_mut(view_x, view_y);
                        pixel.data = [col.r, col.g, col.b, 255];
                    }
                }
            }
        }

        self.t = 0.0;
    }
}

fn start_render_thread (chunks: &Mutex<Chunks>, scene: &Scene) {
    let mut working_chunk = None;
    let mut finished_chunk = None;
    loop {
        let mut chunk_count = 0;

        {
            if let Ok(ref mut chunks) = chunks.lock() {
                // Release old chunk
                if let Some(chunk) = finished_chunk.take() {
                    chunks.completed.push(chunk)
                }
                // Acquire a new chunk
                working_chunk = chunks.ready.pop();
                chunk_count = chunks.count;
            }
        }

        if let Some(chunk) = working_chunk.take() {
            // Render
            println!("Rendering chunk {} of {}", chunk.id, chunk_count);

            let mut chunk = chunk;
            raytracer::cast_rays_into_scene(&mut chunk, scene, SAMPLES_PER_PIXEL);

            // Done
            finished_chunk = Some(chunk);
        } else {
            // Work queue empty.
            break;
        }
    }
}

fn start_background_render_threads (chunks: Arc<Mutex<Chunks>>, scene: Arc<Scene>) -> Vec<JoinHandle<()>>  {
    (0..RENDER_THREAD_COUNT)
        .map(move |_| {
            let chunks = chunks.clone();
            let scene = scene.clone();
            spawn(move || start_render_thread(&*chunks, &*scene))
        })
        .collect::<Vec<_>>()
}

fn main() {
    // Change this to OpenGL::V2_1 if not working.
    let opengl = OpenGL::V3_2;

    println!("Creating scene");

    let viewport = Viewport::new(WIDTH, HEIGHT);
    let scene = raytracer::samples::random_sphere_scene(&viewport);

    let chunk_list = Chunks::from_viewport(&viewport, CHUNK_COUNT);
    
    println!("Creating window");

    let mut window: Window =
        WindowSettings::new("raytracer", [WIDTH, HEIGHT])
            .opengl(opengl)
            .exit_on_esc(true)
            .build()
            .unwrap();

    // Create a new game and run it.
    let mut app = App {
        gl: GlGraphics::new(opengl),
        buffer: RgbaImage::new(WIDTH, HEIGHT),
        chunks: Arc::new(Mutex::new(chunk_list)),
        t: 0.0,
    };

    let scene = Arc::new(scene);
    
    println!("Starting render threads");

    let threads = start_background_render_threads(app.chunks.clone(), scene.clone());
    
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

    for thread in threads {
        thread.join().unwrap();
    }
}