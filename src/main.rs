extern crate emscripten_sys as asm;
#[macro_use]
extern crate lazy_static;
extern crate libc;
extern crate serde_json;
extern crate sdl2;
extern crate tetris_struct as ts;

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

const BORDER: u32 = 1;
const RIGHT_PANEL: u32 = 5;
const WINDOW_WIDTH: u32 = BORDER + ts::COLUMNS + BORDER + RIGHT_PANEL + BORDER;
const WINDOW_HEIGHT: u32 = BORDER + ts::ROWS + BORDER;
const TARGET_RENDER_WIDTH: u32 = 360;
const TARGET_RENDER_HEIGHT: u32 = 440;

const WORKER_COUNT: u8 = 2;
const TETRIS_COUNT: u32 = 128; // (4 as u32).pow(4) / WORKER_COUNT;

lazy_static!{
    static ref MESSAGE: Mutex<Vec<ts::Msg>> = Mutex::new(vec![]);
    static ref EVENT_Q: Mutex<Vec<ts::BlockEvent>> = Mutex::new(vec![]);
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
    let msg: ts::Msg = serde_json::from_str(msg.as_str()).unwrap();

    if msg.points.len() == 0 {
        println!("empty");
        return;
    }

    let mut messages = MESSAGE.lock().unwrap();
    messages.push(msg);
}

//
// cwrap('move_rotate', 'number')();
#[no_mangle]
pub fn move_rotate() -> u8 {
    println!("rotate");
    match EVENT_Q.lock() {
        Ok(mut v) => {
            v.push(ts::BlockEvent::Rotate);
            0
        }
        Err(_) => 1,
    }
}

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let events = sdl_context.event_pump().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("Tetris", TARGET_RENDER_WIDTH, TARGET_RENDER_WIDTH)
        .build()
        .unwrap();
    let canvas: WindowCanvas = window
        .into_canvas()
        .accelerated()
        .target_texture()
        .build()
        .unwrap();
    let texture_creator: TextureCreator<WindowContext> = canvas.texture_creator();

    let mut app = Box::new(App::new(
        canvas,
        events,
        &texture_creator,
        WORKER_COUNT,
        TETRIS_COUNT,
    ));
    let app_ptr = &mut *app as *mut App as *mut c_void;

    unsafe {
        asm::emscripten_set_main_loop_arg(Some(main_loop_callback), app_ptr, 0, 1);
    }

    mem::forget(app);
}

fn call_worker(handle: c_int, func_name: *const c_char, data: *mut c_char, len: c_int) {
    unsafe {
        asm::emscripten_call_worker(
            handle,
            func_name,
            data,
            len,
            Some(em_worker_callback_func),
            ptr::null_mut(),
        );
    }
}

fn create_and_init_workers(worker_count: u8, tetris_count_per_worker: u32) -> Vec<c_int> {
    (0..worker_count)
        .map(|i| {
            let resource = CString::new("tetriscore.js").unwrap();
            let handle = unsafe { asm::emscripten_create_worker(resource.as_ptr()) };
            let func_name = CString::new("init_event").unwrap();
            let func_name = func_name.as_ptr();

            let mut tetris_count: [u8; 4] =
                unsafe { mem::transmute(tetris_count_per_worker.to_be()) };

            let mut init_value: Vec<u8> = Vec::new();
            init_value.push(i);
            init_value.append(&mut tetris_count.to_vec());

            let len = init_value.len() as i32;
            let send_value = unsafe { CString::from_vec_unchecked(init_value) };
            let send_value = send_value.into_raw();

            call_worker(handle, func_name, send_value, len);

            handle
        })
        .collect()
}

fn send_event(tetris_event: ts::TetrisEvent, handle: c_int) {
    let func_name = CString::new("post_event").unwrap();
    let func_name = func_name.as_ptr();

    match tetris_event.to_json() {
        Ok(json) => {
            let send = CString::new(json).unwrap();
            let send = send.into_raw();
            let len = unsafe { libc::strlen(send) as i32 };

            call_worker(handle, func_name, send, len);
        }
        Err(e) => {
            panic!("{:?}", e);
        }
    };
}

struct Painter {
    ranges: Vec<ts::Rect>,
    tetris_count: u32,
}

//
//
impl Painter {
    fn new(tetris: u32) -> Painter {
        let tetris_count: u32 = WORKER_COUNT as u32 * TETRIS_COUNT;
        let room_count = 2_u32.pow((tetris_count as f32).log(4.0) as u32);
        let width = TARGET_RENDER_WIDTH / room_count;
        let height = TARGET_RENDER_HEIGHT / room_count;

        println!(
            "{}:{}, {}:{}",
            width,
            height,
            width as f32 / ts::COLUMNS as f32,
            height as f32 / ts::ROWS as f32
        );

        Painter {
            ranges: vec![],
            tetris_count: tetris_count,
        }
    }

    fn create_texture<'a>(
        &self,
        texture_creator: &'a TextureCreator<WindowContext>,
    ) -> Vec<Texture<'a>> {
        (0..self.tetris_count).fold(Vec::new(), |mut textures, _| {
            textures.push(
                texture_creator
                    .create_texture_target(None, WINDOW_WIDTH, WINDOW_HEIGHT)
                    .unwrap(),
            );
            textures
        })
    }

    fn paint(&self, message: ts::Msg, canvas: &mut Canvas<Window>, textures: &mut Vec<Texture>) {
        let idx = message.worker_id as usize * TETRIS_COUNT as usize + message.tetris_id as usize;

        canvas
            .with_texture_canvas(&mut textures[idx], |texture_canvas| {
                texture_canvas.clear();
            })
            .unwrap();
    }
}

struct App<'a> {
    canvas: Canvas<Window>,
    textures: Vec<Texture<'a>>,
    events: EventPump,
    worker_handles: Vec<c_int>,
    worker_count: u8,
    tetris_count_per_worker: u32,
    painter: Painter,
}

impl<'a> App<'a> {
    fn new(
        canvas: WindowCanvas,
        events: EventPump,
        texture_creator: &'a TextureCreator<WindowContext>,
        worker_count: u8,
        tetris_count_per_worker: u32,
    ) -> App {
        let painter = Painter::new(WORKER_COUNT as u32 * TETRIS_COUNT as u32);

        App {
            canvas: canvas,
            textures: painter.create_texture(texture_creator),
            events: events,
            worker_handles: create_and_init_workers(worker_count, tetris_count_per_worker),
            worker_count: worker_count,
            tetris_count_per_worker: tetris_count_per_worker,
            painter: painter,
        }
    }

    fn events(&mut self) -> Vec<u8> {
        let mut events: HashSet<u8> = self.events
            .poll_iter()
            .map(|event| match event {
                Event::KeyDown { keycode: Some(Keycode::Up), .. } => {
                    ts::BlockEvent::Rotate.to_block_event()
                }
                Event::KeyDown { keycode: Some(Keycode::Left), .. } => {
                    ts::BlockEvent::Left.to_block_event()
                }
                Event::KeyDown { keycode: Some(Keycode::Right), .. } => {
                    ts::BlockEvent::Right.to_block_event()
                }
                Event::KeyDown { keycode: Some(Keycode::Down), .. } => {
                    ts::BlockEvent::Down.to_block_event()
                }
                Event::KeyDown { keycode: Some(Keycode::Space), .. } => {
                    ts::BlockEvent::Drop.to_block_event()
                }
                _ => 0,
            })
            .filter(|e| e > &0)
            .collect();

        match EVENT_Q.lock() {
            Ok(mut v) => {
                while let Some(e) = v.pop() {
                    events.insert(e.to_block_event());
                }
            }
            Err(_) => (),
        }

        events.iter().map(|e| *e).collect()
    }

    fn handle_events(&mut self) {
        let events = self.events();

        if events.len() == 0 {
            return;
        }

        let ref worker_handles = self.worker_handles;
        for worker_idx in 0..self.worker_count {
            for tetris_idx in 0..self.tetris_count_per_worker {
                send_event(
                    ts::TetrisEvent::new(worker_idx, tetris_idx, events.clone()),
                    worker_handles[worker_idx as usize],
                );
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
                self.painter.paint(
                    message,
                    &mut self.canvas,
                    &mut self.textures,
                );
            }
        }
        println!("done: {}", len);
    }

    fn run(&mut self) {
        self.handle_events();
        self.handle_messages();
    }
}
