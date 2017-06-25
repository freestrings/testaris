extern crate emscripten_sys as asm;
extern crate sdl2;

use std::ptr;
use std::os::raw::{c_char, c_int, c_void};
use std::ffi::CString;
use std::collections::HashSet;
use std::mem;

use sdl2::EventPump;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, Texture, TextureCreator, WindowCanvas};
use sdl2::video::{Window, WindowContext};

const COLUMNS: u32 = 10;
const ROWS: u32 = 20;
const BORDER: u32 = 1;
const WINDOW_WIDTH: u32 = BORDER + COLUMNS + BORDER + RIGHT_PANEL + BORDER;
const WINDOW_HEIGHT: u32 = BORDER + ROWS + BORDER;
const RIGHT_PANEL: u32 = 5;
const SCALE: u32 = 20;
const DEFAULT_GRAVITY: u8 = 20;

fn main() {

    let sdl_context = sdl2::init().unwrap();
    let events = sdl_context.event_pump().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Tetris", WINDOW_WIDTH * SCALE, WINDOW_HEIGHT * SCALE)
        .build()
        .unwrap();
    let canvas: WindowCanvas = window
        .into_canvas()
        .accelerated()
        .target_texture()
        .build()
        .unwrap();

    let texture_creator: TextureCreator<WindowContext> = canvas.texture_creator();

    let mut app = Box::new(App::new(canvas, events, &texture_creator));
    let app_ptr = &mut *app as *mut App as *mut c_void;

    unsafe {
        asm::emscripten_set_main_loop_arg(Some(main_loop_callback), app_ptr, 0, 1);
    }

    mem::forget(app);
}

extern "C" fn main_loop_callback(arg: *mut c_void) {
    unsafe {
        let mut app: &mut App = mem::transmute(arg);
        app.run();
    }
}

extern "C" fn em_worker_callback_func(data: *mut c_char, size: c_int, user_args: *mut c_void) {
    // 1.
    let msg = unsafe { std::ffi::CString::from_raw(data) };
    let bytes = msg.into_string();
    println!("{:?}", bytes);

    // 2.
    // let msg = unsafe { std::ffi::CString::from_raw(data) };
    // let bytes = msg.into_bytes();
    // println!("{:?}", bytes);
}

struct App<'a> {
    canvas: Canvas<Window>,
    texture: Texture<'a>,
    events: EventPump,
    worker_handle: c_int,
}

impl<'a> App<'a> {
    fn new(
        canvas: WindowCanvas,
        events: EventPump,
        texture_creator: &'a TextureCreator<WindowContext>,
    ) -> App {

        let texture = texture_creator
            .create_texture_target(None, WINDOW_WIDTH, WINDOW_HEIGHT)
            .unwrap();

        let resource = CString::new("tetriscore.js").unwrap();
        let worker_handle = unsafe { asm::emscripten_create_worker(resource.as_ptr()) };

        App {
            canvas: canvas,
            texture: texture,
            events: events,
            worker_handle: worker_handle,
        }
    }

    fn events(&mut self) -> HashSet<u8> {
        let mut events: HashSet<u8> = self.events
            .poll_iter()
            .map(|event| match event {
                Event::KeyDown { keycode: Some(Keycode::Up), .. } => {
                    BlockEvent::Rotate.to_block_event()
                }
                Event::KeyDown { keycode: Some(Keycode::Left), .. } => {
                    BlockEvent::Left.to_block_event()
                }
                Event::KeyDown { keycode: Some(Keycode::Right), .. } => {
                    BlockEvent::Right.to_block_event()
                }
                Event::KeyDown { keycode: Some(Keycode::Down), .. } => {
                    BlockEvent::Down.to_block_event()
                }
                Event::KeyDown { keycode: Some(Keycode::Space), .. } => {
                    BlockEvent::Drop.to_block_event()
                }
                _ => 0,
            })
            .filter(|e| e > &0)
            .collect();

        events
    }

    fn run(&mut self) {
        let events = self.events();

        if events.len() > 0 {
            let events: Vec<u8> = events.iter().map(|e| *e).collect();
            let len = events.len() as i32;
            let send_value = unsafe { std::ffi::CString::from_vec_unchecked(events) };
            let send_value_ptr = send_value.into_raw();

            let worker_func_name = CString::new("post_event").unwrap();
            let worker_func_name_ptr = worker_func_name.as_ptr();

            unsafe {
                asm::emscripten_call_worker(
                    self.worker_handle,
                    worker_func_name_ptr,
                    send_value_ptr,
                    len,
                    Some(em_worker_callback_func),
                    ptr::null_mut(),
                );
            }
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
enum BlockEvent {
    Left,
    Right,
    Down,
    Drop,
    Rotate,
    None,
}

impl BlockEvent {
    fn from_event(evt: u8) -> BlockEvent {
        match evt {
            1 => BlockEvent::Left,
            2 => BlockEvent::Right,
            3 => BlockEvent::Down,
            4 => BlockEvent::Drop,
            5 => BlockEvent::Rotate,
            _ => BlockEvent::None,
        }
    }

    fn to_block_event(&self) -> u8 {
        match *self {
            BlockEvent::Left => 1,
            BlockEvent::Right => 2,
            BlockEvent::Down => 3,
            BlockEvent::Drop => 4,
            BlockEvent::Rotate => 5,
            _ => 6,
        }
    }
}
