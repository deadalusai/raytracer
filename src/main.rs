extern crate piston;
extern crate graphics;
extern crate glutin_window;
extern crate opengl_graphics;
extern crate image;

use image::RgbaImage;
use piston::window::WindowSettings;
use piston::event_loop::*;
use piston::input::*;
use glutin_window::GlutinWindow as Window;
use opengl_graphics::{ GlGraphics, OpenGL, Texture, TextureSettings };

mod raytracer;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

pub struct App {
    gl: GlGraphics,    // OpenGL drawing backend.
    buffer: RgbaImage, // Buffer
    rot: f64           // Rotation
}

impl App {
    fn render(&mut self, args: &RenderArgs) {
        use graphics::*;

        const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

        let settings = TextureSettings::new();
        let texture = Texture::from_image(&self.buffer, &settings);
        
        let rot_rad = self.rot;
        let center_x = (args.width / 2) as f64;
        let center_y = (args.height / 2) as f64;
        
        self.gl.draw(args.viewport(), |ctx, gl| {
            // Clear the screen.
            clear(BLACK, gl);
        
            // Rotate the texture from the center..
            let transform = ctx.transform
                .trans(center_x, center_y)
                .rot_rad(rot_rad)
                .trans(-center_x, -center_y);

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

    // Create an Glutin window.
    let mut window: Window =
        WindowSettings::new("spinning-square", [WIDTH, HEIGHT])
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

    raytracer::draw_gradient(&mut app.buffer);

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