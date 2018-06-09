extern crate piston;
extern crate graphics;
extern crate glutin_window;
extern crate opengl_graphics;
extern crate image;
extern crate rand;

use piston::window::WindowSettings;
use piston::event_loop::*;
use piston::input::*;
use glutin_window::GlutinWindow as Window;
use opengl_graphics::{ GlGraphics, OpenGL, Texture, TextureSettings };
use image::RgbaImage;

mod raytracer;

pub struct App {
    gl: GlGraphics,    // OpenGL drawing backend.
    buffer: RgbaImage, // Buffer
    rot: f64           // Rotation
}

impl App {
    fn render(&mut self, args: &RenderArgs) {
        use graphics::*;

        let settings = TextureSettings::new();
        let texture = Texture::from_image(&self.buffer, &settings);

        // let rot_rads = self.rot;
        // let rot_offset_x = args.width as f64 / 2.0;
        // let rot_offset_y = args.height as f64 / 2.0;
        
        self.gl.draw(args.viewport(), |ctx, gl| {
            // Clear screen
            clear([0.0; 4], gl);
            
            // Apply rotation
            let transform = ctx.transform;
            //let transform = ctx.transform
            //    .trans(rot_offset_x, rot_offset_y)
            //    .rot_rad(rot_rads)
            //    .trans(-rot_offset_x, -rot_offset_y);

            // Draw the buffer texture
            image(&texture, transform, gl);
        });
    }

    fn update(&mut self, args: &UpdateArgs) {
        // Rotate 2 radians per second.
        self.rot += 2.0 * args.dt;
    }
}

fn main() {
    // Change this to OpenGL::V2_1 if not working.
    let opengl = OpenGL::V3_2;

    const WIDTH: u32 = 200;
    const HEIGHT: u32 = 200;
    const SAMPLES_PER_PIXEL: u32 = 1;

    // Create an Glutin window.
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
        rot: 0 as f64
    };

    let viewport = raytracer::Viewport::new(WIDTH, HEIGHT);
    let world = raytracer::samples::random_shpere_scene(&viewport);

    let mut chunks: Vec<_> = viewport.iter_view_chunks(2, 2).collect();
    let len = chunks.len();
    for (i, chunk) in chunks.iter_mut().enumerate() {
        println!("Rendering chunk {} of {}", i + 1, len);
        raytracer::cast_rays_into_world(chunk, &world, SAMPLES_PER_PIXEL);
    }

    println!("Drawing chunks to buffer");

    for (i, chunk) in chunks.iter().enumerate() {
        println!("Drawing chunk {} of {}", i + 1, len);
        for chunk_y in 0..chunk.height {
            for chunk_x in 0..chunk.width {
                let col = chunk.get_chunk_pixel(chunk_x, chunk_y);
                let (view_x, view_y) = chunk.get_view_relative_coords(chunk_x, chunk_y);
                let pixel = app.buffer.get_pixel_mut(view_x, view_y);
                pixel.data = [col.r, col.g, col.b, 255];
            }
        }
    }

    // HAX
    app.buffer.save("test.png").unwrap();

    let mut events = Events::new(EventSettings::new());
    while let Some(e) = events.next(&mut window) {
        if let Some(r) = e.render_args() {
            app.render(&r);
        }

        if let Some(u) = e.update_args() {
            app.update(&u);
        }
    }
}