extern crate emscripten_sys as asm;
#[macro_use]
extern crate lazy_static;
extern crate libc;
extern crate serde_json;
extern crate sdl2;
extern crate tetris_struct;

use std::ptr;
use std::os::raw::{c_char, c_int, c_void};
use std::ffi::CString;
use std::collections::HashSet;
use std::sync::Mutex;
use std::mem;

use sdl2::EventPump;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, Texture, TextureCreator, WindowCanvas};
use sdl2::video::{Window, WindowContext};

use tetris_struct::*;

const BORDER: u32 = 1;
const WINDOW_WIDTH: u32 = BORDER + COLUMNS + BORDER + RIGHT_PANEL + BORDER;
const WINDOW_HEIGHT: u32 = BORDER + ROWS + BORDER;
const RIGHT_PANEL: u32 = 5;
const SCALE: u32 = 20;

lazy_static!{
    static ref MESSAGE: Mutex<Vec<Msg>> = Mutex::new(vec![]);
}

extern "C" fn main_loop_callback(arg: *mut c_void) {
    unsafe {
        let mut app: &mut App = mem::transmute(arg);
        app.run();
    }
}

extern "C" fn em_worker_callback_func(data: *mut c_char, size: c_int, _user_args: *mut c_void) {

    let raw_msg: &[u8] = unsafe {
        let slice = std::slice::from_raw_parts(data, size as usize - 1);
        mem::transmute(slice)
    };

    let msg = String::from_utf8(raw_msg.to_vec()).unwrap();
    let msg: Msg = serde_json::from_str(msg.as_str()).unwrap();

    if msg.points.len() == 0 {
        println!("empty");
        return;
    }

    let mut messages = MESSAGE.lock().unwrap();
    messages.push(msg);
}

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

    let mut app = Box::new(App::new(canvas, events, &texture_creator, 4, 100));
    let app_ptr = &mut *app as *mut App as *mut c_void;

    unsafe {
        asm::emscripten_set_main_loop_arg(Some(main_loop_callback), app_ptr, 0, 1);
    }

    mem::forget(app);
}

fn create_and_init_workers(worker_count: u8, tetris_count_per_worker: u8) -> Vec<c_int> {

    (0..worker_count)
        .map(|i| {
            let resource = CString::new("tetriscore.js").unwrap();
            let handle = unsafe { asm::emscripten_create_worker(resource.as_ptr()) };

            let worker_func_name = CString::new("init_event").unwrap();
            let worker_func_name_ptr = worker_func_name.as_ptr();

            let init_value: Vec<u8> = vec![i, tetris_count_per_worker];
            let len = init_value.len() as i32;
            let send_value = unsafe { CString::from_vec_unchecked(init_value) };
            let send_value_ptr = send_value.into_raw();

            unsafe {
                asm::emscripten_call_worker(
                    handle,
                    worker_func_name_ptr,
                    send_value_ptr,
                    len,
                    Some(em_worker_callback_func),
                    ptr::null_mut(),
                );
            }

            handle
        })
        .collect()
}

struct App<'a> {
    canvas: Canvas<Window>,
    texture: Texture<'a>,
    events: EventPump,
    worker_handles: Vec<c_int>,
    worker_count: u8,
    tetris_count_per_worker: u8,
}

impl<'a> App<'a> {
    fn new(
        canvas: WindowCanvas,
        events: EventPump,
        texture_creator: &'a TextureCreator<WindowContext>,
        worker_count: u8,
        tetris_count_per_worker: u8,
    ) -> App {

        let texture = texture_creator
            .create_texture_target(None, WINDOW_WIDTH, WINDOW_HEIGHT)
            .unwrap();

        App {
            canvas: canvas,
            texture: texture,
            events: events,
            worker_handles: create_and_init_workers(worker_count, tetris_count_per_worker),
            worker_count: worker_count,
            tetris_count_per_worker: tetris_count_per_worker
        }
    }

    fn events(&mut self) -> HashSet<u8> {
        let events: HashSet<u8> = self.events
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

    fn handle_events(&mut self) {
        let events = self.events();

        if events.len() == 0 {
            return;
        }

        let events: Vec<u8> = events.iter().map(|e| *e).collect();

        let worker_func_name = CString::new("post_event").unwrap();
        let worker_func_name_ptr = worker_func_name.as_ptr();

        let ref worker_handles = self.worker_handles;
        for i in 0..self.worker_count {
            let worker_handle = worker_handles[i as usize];

            for j in 0..self.tetris_count_per_worker {

                let tetris_event = TetrisEvent::new(i, j, events.clone());

                // println!("send, {}:{}", i, j);

                match tetris_event.to_json() {
                    Ok(json) => {
                        let send = CString::new(json).unwrap();
                        let send_ptr = send.into_raw();
                        let len = unsafe { libc::strlen(send_ptr) as i32 };
                        unsafe {
                            asm::emscripten_call_worker(
                                worker_handle,
                                worker_func_name_ptr,
                                send_ptr,
                                len,
                                Some(em_worker_callback_func),
                                ptr::null_mut(),
                            );
                        }
                    }
                    Err(e) => {
                        panic!("{:?}", e);
                    }
                }
            }
        }
    }

    fn handle_messages(&mut self) {
        let mut messages = MESSAGE.lock().unwrap();
        if messages.is_empty() {
            return;
        }

        let len = messages.len();
        while !messages.is_empty() {
            if let Some(message) = messages.pop() {
                // println!("{}:{} -> {:?}", message.worker_id, message.tetris_id, message.points);
            }
        }
        println!("done: {}", len);
    }

    fn run(&mut self) {
        self.handle_events();
        self.handle_messages();
    }
}
