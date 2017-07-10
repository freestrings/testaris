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
use std::collections::{HashSet, HashMap};
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
const RIGHT_PANEL: u32 = 9;
const MAIN_WIDTH: u32 = BORDER + ts::COLUMNS + BORDER;
const WINDOW_WIDTH: u32 = MAIN_WIDTH + RIGHT_PANEL + BORDER;
const WINDOW_HEIGHT: u32 = BORDER + ts::ROWS + BORDER;
const TARGET_RENDER_WIDTH: u32 = 440;
const TARGET_RENDER_HEIGHT: u32 = 440;

const WORKER_COUNT: u8 = 1;
const TETRIS_COUNT: u32 = 4; // (4 as u32).pow(4) / WORKER_COUNT;

lazy_static! {
    static ref MESSAGE: Mutex<Vec<ts::Msg>> = Mutex::new(vec![]);
    static ref EVENT_Q: Mutex<Vec<ts::BlockEvent>> = Mutex::new(vec![]);
    static ref APP_STATUS: Mutex<u8> = Mutex::new(0);
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
    let msg = msg.as_str();
    let msg: ts::Msg = serde_json::from_str(msg).unwrap();

    if msg.event_name.as_str() == "init_worker" {
        *APP_STATUS.lock().unwrap() = 1;
    } else if msg.event_name.as_str() == "init_tetris" {
        *APP_STATUS.lock().unwrap() = 2;
    } else {
        let mut messages = MESSAGE.lock().unwrap();
        messages.push(msg);
    }
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
        .window("Tetris", TARGET_RENDER_WIDTH, TARGET_RENDER_HEIGHT)
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
            let c = CString::new("tetriscore.js").unwrap();
            let handle = unsafe { asm::emscripten_create_worker(c.as_ptr()) };
            let tetris_count: [u8; 4] = unsafe { mem::transmute(tetris_count_per_worker.to_be()) };

            let mut init_value: Vec<u8> = Vec::new();
            init_value.push(i);
            init_value.append(&mut tetris_count.to_vec());

            let len = init_value.len() as i32;
            let send_value = unsafe { CString::from_vec_unchecked(init_value) };
            let send_value = send_value.into_raw();

            let c = CString::new("init_worker").unwrap();
            call_worker(handle, c.as_ptr(), send_value, len);

            handle
        })
        .collect()
}

fn init_tetris(worker_handles: &Vec<c_int>, tetris_count_per_worker: u32) {
    for worker_idx in 0..worker_handles.len() as u8 {
        for tetris_idx in 0..tetris_count_per_worker {
            let tetris_event = ts::TetrisEvent::new(worker_idx, tetris_idx, Vec::new());
            match tetris_event.to_json() {
                Ok(json) => {
                    let send = CString::new(json).unwrap();
                    let send = send.into_raw();
                    let len = unsafe { libc::strlen(send) as i32 };

                    let c = CString::new("init_tetris").unwrap();
                    call_worker(worker_handles[worker_idx as usize], c.as_ptr(), send, len);
                }
                Err(e) => {
                    panic!("{:?}", e);
                }
            };
        }
    }
}

fn send_event(tetris_event: ts::TetrisEvent, handle: c_int) {
    match tetris_event.to_json() {
        Ok(json) => {
            let send = CString::new(json).unwrap();
            let send = send.into_raw();
            let len = unsafe { libc::strlen(send) as i32 };

            let c = CString::new("post_event").unwrap();
            call_worker(handle, c.as_ptr(), send, len);
        }
        Err(e) => {
            panic!("{:?}", e);
        }
    };
}

struct Painter {
    starts: Vec<ts::Point>,
    width: u32,
    height: u32,
    scale: i32,
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
        let scale = width / WINDOW_WIDTH;

        println!(
            "width: {} heigth:{}, room_count: {}, scale: {}",
            width,
            height,
            room_count,
            scale,
        );

        let mut w = 0;
        let mut h = 0;
        let mut starts: Vec<ts::Point> = Vec::new();

        for _ in 0..room_count {
            for _ in 0..room_count {
                starts.push(ts::Point::new(w as i32, h as i32));
                w += width;
            }
            w = 0;
            h += height;
        }

        Painter {
            starts: starts,
            width: width,
            height: height,
            scale: scale as i32,
            tetris_count: tetris_count,
        }
    }

    fn paint_main(&self, message: &ts::Msg, canvas: &mut Canvas<Window>) {
        let (r, g, b) = message.block.0.color();
        let ref points = message.block.1;

        let points: Vec<Point> = points.iter()
            .map(|point| {
                Point::new(point.x(), point.y())
            })
            .collect();

        canvas.set_draw_color(Color::RGB(r, g, b));
        canvas.draw_points(points.as_slice()).unwrap();
    }

    fn paint_scoreboard(&self, message: &ts::Msg, canvas: &mut Canvas<Window>) {
        let ref next_block_type = message.block.2;
        let (r, g, b) = next_block_type.color();
        let points = next_block_type.points();

        let points: Vec<Point> = points.iter()
            .map(|point| {
                Point::new(point.x() + 2 + MAIN_WIDTH as i32, point.y() + 1)
            })
            .collect();

        canvas.set_draw_color(Color::RGB(10, 10, 10));
        canvas.fill_rect(Rect::new(MAIN_WIDTH as i32, 0, WINDOW_WIDTH - MAIN_WIDTH, WINDOW_HEIGHT)).unwrap();
        canvas.set_draw_color(Color::RGB(r, g, b));
        canvas.draw_points(points.as_slice()).unwrap();
    }

    fn paint(
        &self,
        messages: &HashMap<u32, ts::Msg>,
        canvas: &mut Canvas<Window>,
        texture: &mut Texture,
    ) {
        // println!("{}", messages.len());

        for idx in 0..messages.len() {
            if let Some(message) = messages.get(&(idx as u32)) {
                let ref start = self.starts[idx];

                canvas.with_texture_canvas(texture, |texture_canvas| {
                    texture_canvas.set_draw_color(Color::RGB(0, 0, 0));
                    texture_canvas.clear();
                    self.paint_main(&message, texture_canvas);
                    self.paint_scoreboard(&message, texture_canvas);
                }).unwrap();

                canvas.copy(texture, Some(Rect::new(
                    0,
                    0,
                    WINDOW_WIDTH,
                    WINDOW_HEIGHT,
                )), Some(Rect::new(
                    start.x(),
                    start.y(),
                    WINDOW_WIDTH * self.scale as u32,
                    WINDOW_HEIGHT * self.scale as u32,
                ))).unwrap();
            }
        }
    }
}

struct App<'a> {
    canvas: Canvas<Window>,
    texture: Texture<'a>,
    events: EventPump,
    worker_handles: Vec<c_int>,
    worker_count: u8,
    tetris_count_per_worker: u32,
    painter: Painter,
    messages: HashMap<u32, ts::Msg>,
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
        let texture = texture_creator
            .create_texture_target(None, WINDOW_WIDTH, WINDOW_HEIGHT)
            .unwrap();
        let worker_handles = create_and_init_workers(worker_count, tetris_count_per_worker);

        App {
            canvas: canvas,
            texture: texture,
            events: events,
            worker_handles: worker_handles,
            worker_count: worker_count,
            tetris_count_per_worker: tetris_count_per_worker,
            painter: painter,
            messages: HashMap::new(),
        }
    }

    fn check_app_status(&mut self) -> bool {
        match *APP_STATUS.lock().unwrap() {
            1 => {
                init_tetris(&self.worker_handles, self.tetris_count_per_worker);
                false
            }
            2 => true,
            _ => false,
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

        while !messages.is_empty() {
            if let Some(message) = messages.pop() {
                let idx = message.worker_id as u32 * TETRIS_COUNT + message.tetris_id;
                self.messages.insert(idx, message);
            }
        }
    }

    fn run(&mut self) {
        if !self.check_app_status() {
            return;
        }

        self.handle_events();
        self.handle_messages();

        self.painter.paint(&self.messages, &mut self.canvas, &mut self.texture);
    }
}
