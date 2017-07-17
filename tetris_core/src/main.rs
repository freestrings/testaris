extern crate emscripten_sys as asm;
#[macro_use]
extern crate lazy_static;
extern crate libc;
extern crate serde_json;
extern crate tetris_struct;

use std::f32::consts::PI;
use std::ffi::CString;
use std::mem;
use std::os::raw::{c_char, c_int};
use std::sync::Mutex;
use std::slice;
use std::time::{SystemTime, Duration};

use tetris_struct::*;

type Points = Vec<Point>;
type Color = (u8, u8, u8);

fn main() {}

lazy_static! {
    static ref TETRIS: Mutex<Vec<Tetris>> = Mutex::new(vec![]);
    static ref IDX: Mutex<Option<u8>> = Mutex::new(None);
}

struct Ticker {
    fact: u32,
    elapsed: u32,
}

impl Ticker {
    pub fn new(fact: u32) -> Ticker {
        Ticker { fact: fact, elapsed: 0 }
    }

    pub fn tick(&mut self) -> bool {
        self.elapsed += 1;

        if self.elapsed >= self.fact {
            self.elapsed = 0;
            true
        } else {
            false
        }
    }
}

struct Tetris {
    pub block: Block,
    pub grid: Grid,
    pub ticker: Ticker,
}

impl Tetris {
    fn new() -> Tetris {
        Tetris {
            block: Block::new(BlockType::random()),
            grid: Grid::new(),
            ticker: Ticker::new(50),
        }
    }
}

mod log {
    use super::asm;

    pub fn debug<T>(msg: T) {
        unsafe {
            asm::emscripten_log(asm::EM_LOG_CONSOLE as i32, msg);
        }
    }

    pub fn error<T>(msg: T) {
        unsafe {
            asm::emscripten_log(asm::EM_LOG_ERROR as i32, msg);
        }
    }
}

fn into_raw<'a>(data: *mut c_char, size: c_int) -> &'a [u8] {
    unsafe {
        mem::transmute(slice::from_raw_parts(data, size as usize))
    }
}

fn send_back(msg: Msg) {
    let json = msg.to_json().expect("Error\0");
    let send_back = CString::new(json).unwrap();
    let send_back = send_back.into_raw();
    let len = unsafe { libc::strlen(send_back) as i32 };
    unsafe {
        asm::emscripten_worker_respond(send_back, len + 1);
    }
}

fn worker_guard(worker_id: u8) -> bool {
    match *IDX.lock().unwrap() {
        Some(ref idx) => worker_id.ne(idx),
        None => true,
    }
}

#[no_mangle]
pub fn on(data: *mut c_char, size: c_int) {
    let tetris_event = String::from_utf8(into_raw(data, size).to_vec()).unwrap();

    match serde_json::from_str(tetris_event.as_str()).unwrap() {
        TetrisEvent::InitWorker(worker_index, tetris_count) => {
            init_worker(worker_index, tetris_count)
        }
        TetrisEvent::InitTetris(worker_index, tetris_index) => {
            init_tetris(worker_index, tetris_index)
        }
        TetrisEvent::Tick(worker_index, tetris_index) => {
            tick_event(worker_index, tetris_index)
        }
        TetrisEvent::User(worker_index, tetris_index, block_event) => {
            user_event(worker_index, tetris_index, block_event)
        }
    }
}

pub fn init_worker(worker_index: u8, tetris_count: u32) {
    if let Some(idx) = *IDX.lock().unwrap() {
        log::error(format!("already initialized: {}\0", idx));
        return;
    }

    *IDX.lock().unwrap() = Some(worker_index);

    for _ in 0..tetris_count {
        TETRIS.lock().unwrap().push(Tetris::new());
    }

    let event = TetrisEvent::InitWorker(worker_index, tetris_count);
    send_back(Msg::new(event, None, None));
}

pub fn init_tetris(worker_index: u8, tetris_index: u32) {
    if worker_guard(worker_index) {
        return;
    }

    let ref mut tetris = TETRIS.lock().unwrap()[tetris_index as usize];
    let ref mut block = tetris.block;

    block.align_to_start();

    if block.next_ref().is_none() {
        block.load_next();
    }

    send_back(
        Msg::new(
            TetrisEvent::InitTetris(worker_index, tetris_index),
            Some(block.to_msg()),
            None
        )
    );
}

pub fn tick_event(worker_index: u8, tetris_index: u32) {
    if worker_guard(worker_index) {
        return;
    }

    let ref mut tetris = TETRIS.lock().unwrap()[tetris_index as usize];
    let ref mut ticker = tetris.ticker;
    let ref mut block = tetris.block;
    let ref mut grid = tetris.grid;

    if ticker.tick() {
        block.down(|points| !grid.is_empty(points));
    }

    send_back(
        Msg::new(
            TetrisEvent::Tick(worker_index, tetris_index),
            Some(block.to_msg()),
            None
        )
    );
}

pub fn user_event(worker_index: u8, tetris_index: u32, block_events: Vec<BlockEvent>) {
    if worker_guard(worker_index) {
        return;
    }

    let ref mut tetris = TETRIS.lock().unwrap()[tetris_index as usize];
    let ref mut block = tetris.block;
    let ref mut grid = tetris.grid;
    let ref mut ticker = tetris.ticker;

    for event in block_events {
        match event {
            BlockEvent::Rotate => {
                if block.type_ref() != &BlockType::O {
                    block.rotate();
                }
            }
            BlockEvent::Left => block.left(|points| !grid.is_empty(points)),
            BlockEvent::Right => block.right(|points| !grid.is_empty(points)),
            BlockEvent::Down => block.down(|points| !grid.is_empty(points)),
            BlockEvent::Drop => block.drop(|points| !grid.is_empty(points)),
            _ => (),
        };

        block.adjust_bound();
    }

    send_back(
        Msg::new(
            TetrisEvent::User(worker_index, tetris_index, Vec::new()),
            Some(block.to_msg()),
            None
        )
    );
}

struct Block {
    block_type: BlockType,
    color: Color,
    points: Points,
    next: Option<Box<Block>>,
}

impl Block {
    fn new(block_type: BlockType) -> Block {
        let points = block_type.points();
        let color = block_type.color();

        Block {
            block_type: block_type,
            points: points,
            color: color,
            next: None,
        }
    }

    fn load_next(&mut self) {
        self.next = Some(Box::new(Block::new(BlockType::random())));
    }

    fn apply_next(&mut self) {
        let block = self.next.take().expect("Can not apply a next block!");
        self.block_type = block.block_type.clone();
        self.color = block.color;
        self.align_to_start();
        self.load_next();
    }

    fn align_to_start(&mut self) {
        let range = self.range();
        let x = (COLUMNS / 2 - range.width() / 2) as i32;
        let y = range.height() as i32 * -1;
        self.shift(|| (x, y));
    }

    fn next_ref(&self) -> &Option<Box<Block>> {
        &self.next
    }

    fn next_type(&self) -> Option<BlockType> {
        if let Some(ref next) = self.next {
            Some(next.type_ref().clone())
        } else {
            None
        }
    }

    fn type_ref(&self) -> &BlockType {
        &self.block_type
    }

    fn color_ref(&self) -> &Color {
        &self.color
    }

    fn points_ref(&self) -> &Points {
        &self.points
    }

    fn points(&self) -> Points {
        self.points.clone()
    }

    fn update(&mut self, target_points: &mut Points) {
        self.points.truncate(0);
        self.points.append(target_points);
    }

    //
    // https://www.youtube.com/watch?v=Atlr5vvdchY
    //
    fn rotate(&mut self) {
        let angle = PI * 0.5_f32;
        let center = Point::new(self.points_ref()[2].x(), self.points_ref()[2].y());
        let mut points = self.points_ref().iter().map(|point| {
            let x = point.x() - center.x();
            let y = point.y() - center.y();
            let y = y * -1;

            let rotated_x = angle.cos() * x as f32 - angle.sin() * y as f32;
            let rotated_x = rotated_x.round() as i32 + center.x();
            let rotated_y = angle.sin() * x as f32 + angle.cos() * y as f32;
            let rotated_y = rotated_y.round() as i32 * -1 + center.y();

            Point::new(rotated_x, rotated_y)
        }).collect();

        self.update(&mut points);
    }

    fn shift<F>(&mut self, mut f: F) where F: FnMut() -> (i32, i32) {
        let mut points = self.points_ref().iter().map(|point| {
            let raw_point = f();
            Point::new(point.x() + raw_point.0, point.y() + raw_point.1)
        }).collect();

        self.update(&mut points);
    }

    fn left<GARD>(&mut self, rollback_gard: GARD) where GARD: Fn(&Points) -> bool {
        self.shift(|| (-1, 0));
        if rollback_gard(self.points_ref()) {
            self.shift(|| (1, 0));
        }
    }

    fn right<GARD>(&mut self, rollback_gard: GARD) where GARD: Fn(&Points) -> bool {
        self.shift(|| (1, 0));
        if rollback_gard(self.points_ref()) {
            self.shift(|| (-1, 0));
        }
    }

    fn down<GARD>(&mut self, rollback_gard: GARD) where GARD: Fn(&Points) -> bool {
        self.shift(|| (0, 1));
        if rollback_gard(self.points_ref()) {
            self.shift(|| (0, -1));
        }
    }

    fn drop<GARD>(&mut self, rollback_gard: GARD) where GARD: Fn(&Points) -> bool {
        let range = self.range();
        let start_y = range.y() + range.height() as i32;
        for _ in start_y..ROWS as i32 {
            self.shift(|| (0, 1));
            if rollback_gard(self.points_ref()) {
                self.shift(|| (0, -1));
                break;
            }
        }
    }

    fn range(&mut self) -> Rect {
        let mut min_x = i32::max_value();
        let mut max_x = i32::min_value();
        let mut min_y = i32::max_value();
        let mut max_y = i32::min_value();

        let points = self.points_ref();
        for b in points {
            if b.x().gt(&max_x) { max_x = b.x(); }
            if b.x().lt(&min_x) { min_x = b.x(); }
            if b.y().gt(&max_y) { max_y = b.y(); }
            if b.y().lt(&min_y) { min_y = b.y(); }
        }

        let width = (max_x - min_x).abs() as u32;
        let height = (max_y - min_y).abs() as u32;
        Rect::new(min_x, min_y, width, height)
    }

    fn adjust_bound(&mut self) {
        let range = self.range();
        self._adjust_left_bound(&range);
        self._adjust_right_bound(&range);
        self._adjust_bottom_bound(&range);
    }

    fn _adjust_left_bound(&mut self, range: &Rect) {
        if range.x() < 0 {
            self.shift(|| (range.x().abs(), 0));
        }
    }

    fn _adjust_right_bound(&mut self, range: &Rect) {
        let right = range.x() + range.width() as i32;
        if right >= COLUMNS as i32 {
            self.shift(|| (COLUMNS as i32 - right, 0));
        }
    }

    fn _adjust_bottom_bound(&mut self, range: &Rect) {
        let bottom = range.y() + range.height() as i32;
        if bottom >= ROWS as i32 {
            self.shift(|| (0, ROWS as i32 - bottom));
        }
    }

    fn to_msg(&mut self) -> (BlockType/*current*/, Vec<Point>/*current*/, Option<BlockType>/*next*/) {
        (self.type_ref().clone(), self.points(), self.next_type())
    }
}

struct Grid {
    data: [[u8; COLUMNS as usize]; ROWS as usize],
}

impl Grid {
    fn new() -> Grid {
        Grid { data: [[0; COLUMNS as usize]; ROWS as usize] }
    }

    fn _check_index_rage(&self, point: &Point) -> bool {
        point.y() >= 0
            && point.y() < ROWS as i32
            && point.x() >= 0
            && point.x() < COLUMNS as i32
    }

    fn fill(&mut self, block: &Block) {
        let points: Vec<&Point> = block.points_ref().iter()
            .filter(|point| {
                self._check_index_rage(point)
            })
            .collect();

        for point in points {
            self.data[point.y() as usize][point.x() as usize] = block.block_type.index();
        }
    }

    fn is_reach_to_end(&self, points: &Points) -> bool {
        points.iter()
            .filter(|point| point.y() == ROWS as i32 - 1)
            .collect::<Vec<&Point>>()
            .len() > 0
    }

    fn is_empty_below(&self, points: &Points) -> bool {
        let mut dummy_block = Block::new(BlockType::I);
        dummy_block.down(|_| false);
        self.is_empty(dummy_block.points_ref())
    }

    fn is_empty(&self, points: &Points) -> bool {
        points.iter()
            .filter(|point| self._check_index_rage(point))
            .filter(|point| {
                self.data[point.y() as usize][point.x() as usize] > 0
            })
            .collect::<Vec<&Point>>()
            .len() == 0
    }

    fn foreach<F>(&self, mut func: F) where F: FnMut(i32, i32, u8) {
        for r in 0..self.data.len() {
            let row = self.data[r];
            for c in 0..row.len() {
                func(c as i32, r as i32, row[c]);
            }
        }
    }

    fn find_full_row(&self) -> Vec<i32> {
        let mut rows: Vec<i32> = Vec::new();
        for r in (0..self.data.len()).rev() {
            let row = self.data[r];
            let mut filled = 0;
            for c in 0..row.len() {
                if row[c] > 0 {
                    filled += 1;
                }
            }
            if filled == row.len() {
                rows.push(r as i32);
            }
        }
        rows
    }

    fn remove_row(&mut self, row_index: usize) {
        for r in (0..row_index).rev() {
            for c in 0..self.data[r].len() {
                self.data[r + 1][c] = self.data[r][c];
            }
        }

        for c in 0..self.data[0].len() {
            self.data[0][c] = 0;
        }
    }
}
