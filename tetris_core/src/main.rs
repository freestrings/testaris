extern crate emscripten_sys as asm;
#[macro_use]
extern crate lazy_static;
extern crate rand;
extern crate libc;
extern crate tetris_struct;

use std::f32::consts::PI;
use std::ffi::CString;
use std::mem;
use std::sync::Mutex;
use std::os::raw::{c_char, c_int};
use std::cell::RefCell;

use std::slice;

use tetris_struct::*;

use rand::distributions::{IndependentSample, Range};

fn main() {}

type Points = Vec<Point>;
type Color = (u8, u8, u8);

// 웹 워커당 하나씩 있기 때문에 스레드 세이프하지만,
// 전역으로 Block, Grid 상태를 유지 할 방법이 없다.
lazy_static!{
    static ref BLOCK_OBJ: Mutex<Block> = Mutex::new(Block::new(BlockType::random()));
    static ref GRID_OBJ: Mutex<Grid> = Mutex::new(Grid::new());
    static ref ID: Mutex<RefCell<i8>> = Mutex::new(RefCell::new(-1));
}

mod log {
    use super::asm;

    pub fn debug<T>(msg: T) {
        // unsafe {
        //     asm::emscripten_log(asm::EM_LOG_CONSOLE as i32, msg);
        // }
    }

    pub fn error<T>(msg: T) {
        unsafe {
            asm::emscripten_log(asm::EM_LOG_ERROR as i32, msg);
        }
    }
}

fn send_back(msg: Msg) {
    match msg.to_json() {
        Ok(json) => {
            // log::debug(format!("{:?}\0", json));

            let send_back = CString::new(json).unwrap();
            let send_back_ptr = send_back.into_raw();
            let len = unsafe { libc::strlen(send_back_ptr) as i32 };

            unsafe {
                asm::emscripten_worker_respond(send_back_ptr, len + 1);
            }
        }
        Err(_) => {
            log::error(format!("Error\0"));
        }
    }
}

#[no_mangle]
pub fn init_event(data: *mut c_char, size: c_int) {

    let init_id: &[u8] = unsafe {
        let slice = slice::from_raw_parts(data, size as usize);
        mem::transmute(slice)
    };

    log::debug(format!("init id: {}\0", init_id[0]));

    let init_id = init_id[0];
    let mut id_ref = ID.lock().unwrap();

    if *id_ref.borrow() != -1 {
        return;
    }

    *id_ref.get_mut() = init_id as i8;

    let msg = Msg::new(
        *id_ref.borrow() as u8,
        vec![],
        [[0_u8; COLUMNS as usize]; ROWS as usize],
    );

    send_back(msg);
}

#[no_mangle]
pub fn post_event(data: *mut c_char, size: c_int) {

    let id_ref = ID.lock().unwrap();
    let id_val = *id_ref.borrow();

    log::debug(format!("('{}')))))... {}\0", id_val, size as i32));

    let raw_events: &[u8] = unsafe {
        let slice = slice::from_raw_parts(data, size as usize);
        mem::transmute(slice)
    };

    send_back(on_event(id_val as u8, &raw_events.to_vec()));

    log::debug(format!("...((((('{}')\0", id_val));
}

pub fn on_event(id_val: u8, events: &Vec<u8>) -> Msg {
    log::debug(format!("events {:?}\0", events));

    let mut block = BLOCK_OBJ.lock().unwrap();
    let grid = GRID_OBJ.lock().unwrap();

    for event in events {
        let event = tetris_struct::BlockEvent::from_event(*event);
        match event {
            BlockEvent::Rotate => {
                if block.type_ref() != &BlockType::O {
                    block.rotate();
                }
            }
            BlockEvent::Left => block.move_left(|points| grid.is_empty(points)),
            BlockEvent::Right => block.move_right(|points| grid.is_empty(points)),
            BlockEvent::Down => block.move_down(|points| grid.is_empty(points)),
            BlockEvent::Drop => block.drop_down(|points| grid.is_empty(points)),
            _ => (),
        };

        let range = block.range();
        block.check_left_bound(&range);
        block.check_right_bound(&range);
        block.check_bottom_bound(&range);
    }

    let points = block
        .get_points_ref()
        .iter()
        .map(|point| Point::new(point.x(), point.y()))
        .collect();

    Msg::new(id_val, points, [[0_u8; COLUMNS as usize]; ROWS as usize])
}

#[derive(Clone, Debug, PartialEq)]
enum BlockType {
    T,
    J,
    L,
    S,
    Z,
    O,
    I,
}

impl BlockType {
    fn new(index: u8) -> BlockType {
        match index {
            1 => BlockType::T,
            2 => BlockType::J,
            3 => BlockType::L,
            4 => BlockType::S,
            5 => BlockType::Z,
            6 => BlockType::O,
            7 => BlockType::I,
            _ => BlockType::T,
        }
    }

    fn random() -> BlockType {
        let mut rng = rand::thread_rng();
        let between = Range::new(1, 8);
        BlockType::new(between.ind_sample(&mut rng))
    }

    fn index(&self) -> u8 {
        match *self {
            BlockType::T => 1,
            BlockType::J => 2,
            BlockType::L => 3,
            BlockType::S => 4,
            BlockType::Z => 5,
            BlockType::O => 6,
            BlockType::I => 7,
        }
    }

    fn color(&self) -> (u8, u8, u8) {
        match *self {
            BlockType::T => COLOR_PURPLE,
            BlockType::J => COLOR_BLUE,
            BlockType::L => COLOR_ORANGE,
            BlockType::S => COLOR_LIME,
            BlockType::Z => COLOR_RED,
            BlockType::O => COLOR_YELLOW,
            BlockType::I => COLOR_CYAN,
        }
    }

    fn points(&self) -> Points {
        match *self {
            BlockType::T => BLOCK_T,
            BlockType::J => BLOCK_J,
            BlockType::L => BLOCK_L,
            BlockType::S => BLOCK_S,
            BlockType::Z => BLOCK_Z,
            BlockType::O => BLOCK_O,
            BlockType::I => BLOCK_I,
        }.iter()
            .map(|raw_point| {
                Point::new(raw_point.0 as i32, raw_point.1 as i32)
            })
            .collect()
    }
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
        if let Some(block) = self.next.take() {
            self.block_type = block.block_type.clone();
            self.color = block.color;
            self.align_to_start();
            self.load_next();
        } else {
            panic!("Does not loaded the next block.");
        }
    }

    // 다른 곳으로 옮김
    fn align_to_start(&mut self) {
        let points: Points = self.block_type.points();
        let range = self.range();
        let center = range.width() / 2;
        self.shift(|| {
            (
                (COLUMNS / 2) as i32 - center as i32,
                range.height() as i32 * -1,
            )
        });
    }

    fn type_ref(&self) -> &BlockType {
        &self.block_type
    }

    fn color_ref(&self) -> &Color {
        &self.color
    }

    fn get_points_ref(&self) -> &Points {
        &self.points
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
        let center = Point::new(self.get_points_ref()[2].x(), self.get_points_ref()[2].y());
        let mut points = self.get_points_ref()
            .iter()
            .map(|point| {
                let x = point.x() - center.x();
                let y = point.y() - center.y();
                let y = y * -1;

                let rotated_x = angle.cos() * x as f32 - angle.sin() * y as f32;
                let rotated_x = rotated_x.round() as i32 + center.x();
                let rotated_y = angle.sin() * x as f32 + angle.cos() * y as f32;
                let rotated_y = rotated_y.round() as i32 * -1 + center.y();

                Point::new(rotated_x, rotated_y)
            })
            .collect();

        self.update(&mut points);
    }

    fn shift<F>(&mut self, mut f: F)
    where
        F: FnMut() -> (i32, i32),
    {
        let mut points = self.get_points_ref()
            .iter()
            .map(|point| {
                let raw_point = f();
                Point::new(point.x() + raw_point.0, point.y() + raw_point.1)
            })
            .collect();

        self.update(&mut points);
    }

    fn move_left<GARD>(&mut self, rollback_gard: GARD)
    where
        GARD: Fn(&Points) -> bool,
    {
        self.shift(|| (-1, 0));
        if rollback_gard(self.get_points_ref()) {
            self.shift(|| (1, 0));
        }
    }

    fn move_right<GARD>(&mut self, rollback_gard: GARD)
    where
        GARD: Fn(&Points) -> bool,
    {
        self.shift(|| (1, 0));
        if rollback_gard(self.get_points_ref()) {
            self.shift(|| (-1, 0));
        }
    }

    fn move_down<GARD>(&mut self, rollback_gard: GARD)
    where
        GARD: Fn(&Points) -> bool,
    {
        self.shift(|| (0, 1));
        if rollback_gard(self.get_points_ref()) {
            self.shift(|| (0, -1));
        }
    }

    fn drop_down<GARD>(&mut self, rollback_gard: GARD)
    where
        GARD: Fn(&Points) -> bool,
    {
        let range = self.range();
        let start_y = range.y() + range.height() as i32;
        for _ in start_y..ROWS as i32 {
            self.shift(|| (0, 1));
            if rollback_gard(self.get_points_ref()) {
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

        let points = self.get_points_ref();
        for b in points {
            if b.x().gt(&max_x) {
                max_x = b.x();
            }
            if b.x().lt(&min_x) {
                min_x = b.x();
            }
            if b.y().gt(&max_y) {
                max_y = b.y();
            }
            if b.y().lt(&min_y) {
                min_y = b.y();
            }
        }

        Rect::new(
            min_x,
            min_y,
            (max_x - min_x).abs() as u32 + 1,
            (max_y - min_y).abs() as u32 + 1,
        )
    }

    fn check_left_bound(&mut self, range: &Rect) {
        if range.x() < 0 {
            self.shift(|| (range.x().abs(), 0));
        }
    }

    fn check_right_bound(&mut self, range: &Rect) {
        let right = range.x() + range.width() as i32;
        if right >= COLUMNS as i32 {
            self.shift(|| (COLUMNS as i32 - right, 0));
        }
    }

    fn check_bottom_bound(&mut self, range: &Rect) {
        let bottom = range.y() + range.height() as i32;
        if bottom >= ROWS as i32 {
            self.shift(|| (0, ROWS as i32 - bottom));
        }
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
        point.y() >= 0 && point.y() < ROWS as i32 && point.x() >= 0 && point.x() < COLUMNS as i32
    }

    fn fill(&mut self, block: &Block) {
        let points: Vec<&Point> = block
            .get_points_ref()
            .iter()
            .filter(|point| self._check_index_rage(point))
            .collect();
        for point in points {
            self.data[point.y() as usize][point.x() as usize] = block.block_type.index();
        }
    }

    fn is_reach_to_end(&self, points: &Points) -> bool {
        let c: Vec<&Point> = points
            .iter()
            .filter(|point| point.y() == ROWS as i32 - 1)
            .collect();

        c.len() > 0
    }

    fn is_empty_below(&self, points: &Points) -> bool {
        let mut dummy_block = Block::new(BlockType::I);
        dummy_block.move_down(|_| false);
        self.is_empty(dummy_block.get_points_ref())
    }

    fn is_empty(&self, points: &Points) -> bool {
        let c: Vec<&Point> = points
            .iter()
            .filter(|point| self._check_index_rage(point))
            .filter(|point| {
                self.data[point.y() as usize][point.x() as usize] > 0
            })
            .collect();

        c.len() == 0
    }

    fn for_each_cell<F>(&self, mut func: F)
    where
        F: FnMut(i32, i32, u8),
    {
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
