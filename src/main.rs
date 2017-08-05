extern crate emscripten_sys as asm;
extern crate tetris_core as tc;

#[macro_use]
extern crate lazy_static;
extern crate libc;
extern crate serde_json;
extern crate sdl2;

mod app;
mod events;

use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let events = sdl_context.event_pump().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window(
            "Tetris",
            app::TARGET_RENDER_WIDTH,
            app::TARGET_RENDER_HEIGHT,
        )
        .build()
        .unwrap();
    let canvas: WindowCanvas = window
        .into_canvas()
        .accelerated()
        .target_texture()
        .build()
        .unwrap();
    let texture_creator: TextureCreator<WindowContext> = canvas.texture_creator();
    events::event_loop(Box::new(app::App::new(
        canvas,
        events,
        &texture_creator,
        app::WORKER_COUNT,
        app::TETRIS_COUNT,
    )));
}
