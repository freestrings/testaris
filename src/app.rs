extern crate tetris_core as tc;

use events::*;

use sdl2::EventPump;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, Texture, TextureCreator, WindowCanvas};
use sdl2::video::{Window, WindowContext};

pub const BORDER: u32 = 1;
pub const RIGHT_PANEL: u32 = 9;
pub const MAIN_WIDTH: u32 = BORDER + tc::COLUMNS as u32 + BORDER;
pub const WINDOW_WIDTH: u32 = MAIN_WIDTH + RIGHT_PANEL + BORDER;
pub const WINDOW_HEIGHT: u32 = BORDER + tc::ROWS as u32 + BORDER;
pub const TARGET_RENDER_WIDTH: u32 = 440;
pub const TARGET_RENDER_HEIGHT: u32 = 440;

pub const WORKER_COUNT: u8 = 1;
pub const TETRIS_COUNT: u32 = 4; // (4 as u32).pow(4) / WORKER_COUNT;

pub struct App<'a> {
    canvas: Canvas<Window>,
    texture: Texture<'a>,
    events: EventPump,
    worker_count: u8,
    tetris_per_worker: u32,
    painter: Painter,
    op_event: Box<OpEvent + 'a>,
}

impl<'a> App<'a> {
    pub fn new(
        canvas: WindowCanvas,
        events: EventPump,
        texture_creator: &'a TextureCreator<WindowContext>,
        worker_count: u8,
        tetris_per_worker: u32,
    ) -> App {
        let mut op_event = EventMgr::new();
        op_event.create(worker_count);
        op_event.init(tetris_per_worker);

        App {
            canvas: canvas,
            texture: texture_creator
                .create_texture_target(None, WINDOW_WIDTH, WINDOW_HEIGHT)
                .unwrap(),
            events: events,
            worker_count: worker_count,
            tetris_per_worker: tetris_per_worker,
            painter: Painter::new(worker_count as u32 * tetris_per_worker as u32),
            op_event: Box::new(op_event),
        }
    }

    fn handle_events(&mut self) {
        let events: Vec<tc::BlockEvent> = self.events
            .poll_iter()
            .map(|event| match event {
                Event::KeyDown { keycode: Some(Keycode::Up), .. } => tc::BlockEvent::Rotate,
                Event::KeyDown { keycode: Some(Keycode::Left), .. } => tc::BlockEvent::Left,
                Event::KeyDown { keycode: Some(Keycode::Right), .. } => tc::BlockEvent::Right,
                Event::KeyDown { keycode: Some(Keycode::Down), .. } => tc::BlockEvent::Down,
                Event::KeyDown { keycode: Some(Keycode::Space), .. } => tc::BlockEvent::Drop,
                _ => tc::BlockEvent::None,
            })
            .filter(|e| *e != tc::BlockEvent::None)
            .collect();

        for worker_index in 0..self.worker_count {
            for tetris_index in 0..self.tetris_per_worker {
                self.op_event.send_app_event(tc::AppEvent::User(
                    worker_index,
                    tetris_index,
                    Some(events.clone()),
                ));
            }
        }

    }

    fn check_gravity(&mut self) {
        for worker_index in 0..self.worker_count {
            for tetris_index in 0..self.tetris_per_worker {
                self.op_event.send_app_event(
                    tc::AppEvent::Tick(worker_index, tetris_index),
                );
            }
        }
    }

    fn handle_messages(&mut self) {
        for message in self.op_event.received() {
            self.painter.paint(
                &message,
                &mut self.canvas,
                &mut self.texture,
            );
        }
    }

    pub fn run(&mut self) {
        self.check_gravity();
        self.handle_events();
        self.handle_messages();
    }
}


struct Painter {
    starts: Vec<tc::Point>,
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
        let mut starts: Vec<tc::Point> = Vec::new();

        for _ in 0..room_count {
            for _ in 0..room_count {
                starts.push(tc::Point::new(w as i32, h as i32));
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

    fn _as_point(&self, x: i32, y: i32) -> Point {
        Point::new(x + BORDER as i32, y + BORDER as i32)
    }

    fn paint_border(&self, canvas: &mut Canvas<Window>) {
        canvas.set_draw_color(Color::RGB(100, 100, 100));
        canvas
            .fill_rect(Rect::new(0, 0, MAIN_WIDTH, WINDOW_HEIGHT))
            .unwrap();
    }

    fn paint_main(&self, message: &tc::Msg, canvas: &mut Canvas<Window>) {
        canvas.set_draw_color(Color::RGB(0, 100, 100));
        canvas
            .fill_rect(Rect::new(1, 1, tc::COLUMNS as u32, tc::ROWS as u32))
            .unwrap();

        if let Some(ref block) = message.block {
            let &(r, g, b) = block.color_ref();
            let points: Vec<Point> = block
                .points_ref()
                .iter()
                .filter(|point| point.y() >= 0)
                .map(|point| self._as_point(point.x(), point.y()))
                .collect();

            canvas.set_draw_color(Color::RGB(r, g, b));
            canvas.draw_points(points.as_slice()).unwrap();
        }
    }

    fn paint_scoreboard(&self, message: &tc::Msg, canvas: &mut Canvas<Window>) {
        if let Some(ref block) = message.block {
            if let &Some(ref next) = block.next_ref() {
                let &(r, g, b) = next.color_ref();
                let points: Vec<Point> = next.points_ref()
                    .iter()
                    .map(|point| {
                        Point::new(point.x() + 3 + MAIN_WIDTH as i32, point.y() + 2)
                    })
                    .collect();

                canvas.set_draw_color(Color::RGB(10, 10, 10));
                canvas
                    .fill_rect(Rect::new(
                        MAIN_WIDTH as i32,
                        0,
                        WINDOW_WIDTH - MAIN_WIDTH,
                        WINDOW_HEIGHT,
                    ))
                    .unwrap();

                canvas.set_draw_color(Color::RGB(r, g, b));
                canvas.draw_points(points.as_slice()).unwrap();
            }
        }
    }

    fn paint_grid(&self, message: &tc::Msg, canvas: &mut Canvas<Window>) {
        if let Some(ref grid) = message.grid {
            let data = grid.get_data();
            for r_index in 0..data.len() {
                for c_index in 0..data[r_index].len() {
                    let piece = data[r_index][c_index];
                    if piece == 0 {
                        continue;
                    }

                    let (r, g, b) = tc::BlockType::new(piece).color();
                    canvas.set_draw_color(Color::RGB(r, g, b));
                    canvas
                        .draw_point(self._as_point(c_index as i32, r_index as i32))
                        .unwrap();
                }
            }
        }
    }

    fn paint(&self, message: &tc::Msg, canvas: &mut Canvas<Window>, texture: &mut Texture) {
        canvas
            .with_texture_canvas(texture, |texture_canvas| {
                texture_canvas.set_draw_color(Color::RGB(0, 0, 0));
                texture_canvas.clear();

                self.paint_border(texture_canvas);
                self.paint_main(message, texture_canvas);
                self.paint_grid(message, texture_canvas);
                self.paint_scoreboard(message, texture_canvas);
            })
            .unwrap();

        match message.event {
            tc::AppEvent::InitTetris(worker_index, tetris_index) |
            tc::AppEvent::Tick(worker_index, tetris_index) |
            tc::AppEvent::User(worker_index, tetris_index, _) => {
                let index = (worker_index as u32 * TETRIS_COUNT + tetris_index) as usize;
                let start = &self.starts[index];
                canvas
                    .copy(
                        texture,
                        Some(Rect::new(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT)),
                        Some(Rect::new(
                            start.x(),
                            start.y(),
                            WINDOW_WIDTH * self.scale as u32,
                            WINDOW_HEIGHT * self.scale as u32,
                        )),
                    )
                    .unwrap();
            }
            _ => (),
        }
    }
}
