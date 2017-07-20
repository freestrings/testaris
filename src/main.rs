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
//const TETRIS_COUNT: u32 = 256; // (4 as u32).pow(4) / WORKER_COUNT;

lazy_static! {
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
        mem::transmute(std::slice::from_raw_parts(data, size as usize - 1))
    };

    let msg = String::from_utf8(raw_msg.to_vec()).unwrap();
    let msg = serde_json::from_str::<ts::Msg>(msg.as_str()).unwrap();

    MESSAGE.lock().unwrap().push(msg);
}

//
// cwrap('move_rotate', 'number')();
#[no_mangle]
pub fn move_rotate() -> u8 {
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
    let window = video_subsystem.window("Tetris", TARGET_RENDER_WIDTH, TARGET_RENDER_HEIGHT).build().unwrap();
    let canvas: WindowCanvas = window.into_canvas().accelerated().target_texture().build().unwrap();
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
        asm::emscripten_call_worker(handle, func_name, data, len, Some(em_worker_callback_func), ptr::null_mut());
    }
}

struct Sender {
    worker_handles: Vec<c_int>,
}

impl Sender {
    fn new(worker_count: u8) -> Sender {
        let worker_handles = (0..worker_count)
            .map(|_| {
                let resource = CString::new("tetriscore.js").unwrap();
                unsafe { asm::emscripten_create_worker(resource.as_ptr()) }
            })
            .collect();

        Sender {
            worker_handles: worker_handles,
        }
    }

    fn init(&mut self, tetris_per_worker: u32) {
        for worker_index in 0..self.worker_handles.len() {
            self.send(ts::AppEvent::InitWorker(worker_index as u8, tetris_per_worker));
        }

        for worker_index in 0..self.worker_handles.len() as u8 {
            for tetris_index in 0..tetris_per_worker {
                self.send(ts::AppEvent::InitTetris(worker_index, tetris_index));
            }
        }
    }

    fn send(&mut self, event: ts::AppEvent) {
        let json = serde_json::to_string(&event).expect("[main] Serialize error");
        let send = CString::new(json).unwrap();
        let send = send.into_raw();
        let len = unsafe { libc::strlen(send) as i32 };
        let method = CString::new("on").unwrap();
        call_worker(self.worker_handles[event.worker_id() as usize], method.as_ptr(), send, len);
    }
}

struct Painter {
    starts: Vec<ts::Point>,
    width: u32,
    height: u32,
    scale: i32,
    tetris_count: u32,
}

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
        if let Some((ref current_type, ref current_points, _)) = message.block {
            let (r, g, b) = current_type.color();
            let points: Vec<Point> = current_points.iter()
                .map(|point| {
                    Point::new(point.x() + BORDER as i32, point.y())
                })
                .collect();

            canvas.set_draw_color(Color::RGB(r, g, b));
            canvas.draw_points(points.as_slice()).unwrap();
        }
    }

    fn paint_scoreboard(&self, message: &ts::Msg, canvas: &mut Canvas<Window>) {
        if let Some((_, _, Some(ref next_type))) = message.block {
            let (r, g, b) = next_type.color();
            let points: Vec<Point> = next_type.points().iter()
                .map(|point| {
                    Point::new(point.x() + 3 + MAIN_WIDTH as i32, point.y() + 2)
                })
                .collect();

            canvas.set_draw_color(Color::RGB(10, 10, 10));
            canvas.fill_rect(Rect::new(MAIN_WIDTH as i32, 0, WINDOW_WIDTH - MAIN_WIDTH, WINDOW_HEIGHT)).unwrap();

            canvas.set_draw_color(Color::RGB(r, g, b));
            canvas.draw_points(points.as_slice()).unwrap();
        }
    }

    fn paint(&self, message: &ts::Msg, canvas: &mut Canvas<Window>, texture: &mut Texture) {
        canvas.with_texture_canvas(texture, |texture_canvas| {
            texture_canvas.set_draw_color(Color::RGB(0, 0, 0));
            texture_canvas.clear();

            self.paint_main(message, texture_canvas);
            self.paint_scoreboard(message, texture_canvas);
        }).unwrap();

        match message.event {
            ts::AppEvent::InitTetris(worker_index, tetris_index) |
            ts::AppEvent::Tick(worker_index, tetris_index) |
            ts::AppEvent::User(worker_index, tetris_index, _) => {
                let index = (worker_index as u32 * TETRIS_COUNT + tetris_index) as usize;
                let start = &self.starts[index];
                canvas.copy(texture,
                            Some(Rect::new(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT)),
                            Some(Rect::new(
                                start.x(),
                                start.y(),
                                WINDOW_WIDTH * self.scale as u32,
                                WINDOW_HEIGHT * self.scale as u32
                            ))).unwrap();
            }
            _ => ()
        }
    }
}

struct App<'a> {
    canvas: Canvas<Window>,
    texture: Texture<'a>,
    events: EventPump,
    worker_count: u8,
    tetris_per_worker: u32,
    painter: Painter,
    sender: Sender,
}

impl<'a> App<'a> {
    fn new(
        canvas: WindowCanvas,
        events: EventPump,
        texture_creator: &'a TextureCreator<WindowContext>,
        worker_count: u8,
        tetris_per_worker: u32,
    ) -> App {
        let mut sender = Sender::new(worker_count);
        sender.init(tetris_per_worker);
        let painter = Painter::new(worker_count as u32 * tetris_per_worker as u32);

        App {
            canvas: canvas,
            texture: texture_creator.create_texture_target(None, WINDOW_WIDTH, WINDOW_HEIGHT).unwrap(),
            events: events,
            worker_count: worker_count,
            tetris_per_worker: tetris_per_worker,
            painter: painter,
            sender: sender,
        }
    }

    fn events(&mut self) -> Vec<ts::BlockEvent> {
        let mut events: Vec<ts::BlockEvent> = self.events.poll_iter()
            .map(|event| match event {
                Event::KeyDown { keycode: Some(Keycode::Up), .. } => ts::BlockEvent::Rotate,
                Event::KeyDown { keycode: Some(Keycode::Left), .. } => ts::BlockEvent::Left,
                Event::KeyDown { keycode: Some(Keycode::Right), .. } => ts::BlockEvent::Right,
                Event::KeyDown { keycode: Some(Keycode::Down), .. } => ts::BlockEvent::Down,
                Event::KeyDown { keycode: Some(Keycode::Space), .. } => ts::BlockEvent::Drop,
                _ => ts::BlockEvent::None,
            })
            .filter(|e| *e != ts::BlockEvent::None)
            .collect();

        if let Ok(mut v) = EVENT_Q.lock() {
            while let Some(e) = v.pop() {
                events.push(e);
            }
        }

        events
    }

    fn check_gravity(&mut self) {
        for worker_index in 0..self.worker_count {
            for tetris_index in 0..self.tetris_per_worker {
                self.sender.send(ts::AppEvent::Tick(worker_index, tetris_index));
            }
        }
    }

    fn handle_events(&mut self) {
        let events = self.events();

        if events.len() == 0 {
            return;
        }

        for worker_index in 0..self.worker_count {
            for tetris_index in 0..self.tetris_per_worker {
                self.sender.send(ts::AppEvent::User(worker_index, tetris_index, events.clone()));
            }
        }
    }

    fn handle_messages(&mut self) {
        let mut messages = MESSAGE.lock().unwrap();
        while !messages.is_empty() {
            if let Some(ref message) = messages.pop() {
                self.painter.paint(message, &mut self.canvas, &mut self.texture);
            }
        }
    }

    fn run(&mut self) {
        self.check_gravity();
        self.handle_events();
        self.handle_messages();
    }
}
