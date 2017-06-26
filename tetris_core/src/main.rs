extern crate emscripten_sys as asm;
#[macro_use]
extern crate lazy_static;
extern crate rand;
extern crate time;
extern crate libc;

use std::collections::HashSet;
use std::f32::consts::PI;
use std::ffi::CString;
use std::mem;
use std::sync::Mutex;
use std::os::raw::{c_char, c_int};

use std::slice;

use rand::distributions::{IndependentSample, Range};

fn main() {}

//
//    #
// #, @, #
const BLOCK_T: &[(u8, u8)] = &[(1, 0), (0, 1), (1, 1), (2, 1)];
//
// #
// #, @, #
const BLOCK_J: &[(u8, u8)] = &[(0, 0), (0, 1), (1, 1), (2, 1)];
//
//       #
// #, @, #
const BLOCK_L: &[(u8, u8)] = &[(2, 0), (2, 1), (1, 1), (0, 1)];
//
//    #, #
// #, @
const BLOCK_S: &[(u8, u8)] = &[(2, 0), (1, 0), (1, 1), (0, 1)];
//
// #, #
//    @, #
const BLOCK_Z: &[(u8, u8)] = &[(0, 0), (1, 0), (1, 1), (2, 1)];
//
// #, #
// #, #
const BLOCK_O: &[(u8, u8)] = &[(0, 0), (1, 0), (0, 1), (1, 1)];
//
// #, #, @, #
const BLOCK_I: &[(u8, u8)] = &[(0, 0), (1, 0), (2, 0), (3, 0)];

const COLUMNS: u32 = 10;
const ROWS: u32 = 20;

const COLOR_PURPLE: (u8, u8, u8) = (128, 0, 128);
const COLOR_BLUE: (u8, u8, u8) = (0, 0, 255);
const COLOR_ORANGE: (u8, u8, u8) = (255, 165, 0);
const COLOR_LIME: (u8, u8, u8) = (128, 255, 0);
const COLOR_RED: (u8, u8, u8) = (255, 0, 0);
const COLOR_YELLOW: (u8, u8, u8) = (255, 255, 0);
const COLOR_CYAN: (u8, u8, u8) = (0, 255, 255);
const COLOR_BLACK: (u8, u8, u8) = (0, 0, 0);

const DEFAULT_GRAVITY: u8 = 20;

type Points = Vec<Point>;
type Color = (u8, u8, u8);

trait ToData {
    fn to_data(&self) -> *mut c_char;
}

// lazy_static!{
//     static ref blockObj: Mutex<Block> = Mutex::new(Block::new(BlockType::random()));
//     static ref gridObj: Mutex<Grid> = Mutex::new(Grid::new());
// }


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

use std::borrow::BorrowMut;

#[derive(Debug)]
struct Msg {
    data: Vec<u8>,
}

fn _print<T>(msg: T) {
    unsafe {
        asm::emscripten_log(asm::EM_LOG_CONSOLE as i32, msg);
    }
}

#[no_mangle]
pub fn post_event(data: *mut c_char, size: c_int) {

    _print("<<\0");
    _print(format!("{}\0", size as i32));

    // let len = libc::strlen(data) + 1; // Including the NUL byte
    let slice = unsafe { slice::from_raw_parts(data, size as usize) };
    let s: &[u8] = unsafe { mem::transmute(slice) };
    _print(format!("{:?}\0", s));
    
    let ss = unsafe { CString::from_vec_unchecked(s.to_vec()) };
    
    // let mut s: &mut Vec<u8> = unsafe { mem::transmute(data) };
    // let ss = unsafe { CString::from_vec_unchecked(s.to_vec()) };
    // let s1 = unsafe { CString::from_raw(data) };
    if let Ok(s) = ss.into_string() {
        _print(format!("{:?}\0", s));
    }
    // _print(">>\0");


    // let s = s.into_string().unwrap();
    
    // let mut block = blockObj.lock().unwrap();
    // let grid = gridObj.lock().unwrap();
    // let events = unsafe { CString::from_raw(data) };
    // let bytes = events.into_bytes();

    // for byte in bytes.clone() {
    //     let event = BlockEvent::from_event(byte);
    //     match event {
    //         BlockEvent::Rotate => {
    //             if block.type_ref() != &BlockType::O {
    //                 block.rotate();
    //             }
    //         }
    //         BlockEvent::Left => block.move_left(|points| grid.is_empty(points)),
    //         BlockEvent::Right => block.move_right(|points| grid.is_empty(points)),
    //         BlockEvent::Down => block.move_down(|points| grid.is_empty(points)),
    //         BlockEvent::Drop => block.drop_down(|points| grid.is_empty(points)),
    //         _ => (),
    //     };

    //     let range = block.range();
    //     block.check_left_bound(&range);
    //     block.check_right_bound(&range);
    //     block.check_bottom_bound(&range);
    // }

    // unsafe {
    //     asm::emscripten_log(asm::EM_LOG_ERROR as i32, "#2\n");
    // }

    // let mut msg = block.get_points_ref().iter().fold(
    //     String::new(),
    //     |acc, point| {
    //         let point_string = format!("({},{})", point.x(), point.y());
    //         let point_str = point_string.as_str();
    //         acc + point_str
    //     },
    // );

    // unsafe {
    //     asm::emscripten_log(asm::EM_LOG_ERROR as i32, "#3\n");
    // }

    // unsafe {
    //     asm::emscripten_log(asm::EM_LOG_ERROR as i32, "#3-1: %d\n", bytes.len());
    // }

    // let mut points = bytes;
    // points.clear();
    // // if msg.len() > points.len() {
    //     points.append(&mut msg.into_bytes());
    // // } else {
    // //     points.append(&mut msg.into_bytes());
    // //     points.shrink_to_fit();
    // // }
    // points.shrink_to_fit();

    // unsafe {
    //     asm::emscripten_log(asm::EM_LOG_ERROR as i32, "#4\n");
    // }

    // unsafe {
    //     asm::emscripten_log(asm::EM_LOG_ERROR as i32, "#4-1: %d\n", points.len());
    // }

    // let len = points.len() as i32;
    // let send_back_value = unsafe { CString::from_vec_unchecked(points) };
    // let send_back_value_ptr = send_back_value.into_raw();

    // unsafe {
    //     asm::emscripten_log(asm::EM_LOG_ERROR as i32, "#5\n");
    // }

    // unsafe {
    //     asm::emscripten_worker_respond(send_back_value_ptr, len + 1);
    // }

    // 2.
    // let len = points.len() as i32;
    // let send_back_value = unsafe { std::ffi::CString::from_vec_unchecked(points) };
    // let send_back_value_ptr = send_back_value.into_raw();
    // unsafe {
    //     asm::emscripten_worker_respond(send_back_value_ptr, len);
    // }
}

#[derive(Debug)]
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn new(x: i32, y: i32) -> Point {
        Point { x: x, y: y }
    }

    fn x(&self) -> i32 {
        self.x
    }

    fn y(&self) -> i32 {
        self.y
    }
}

#[derive(Debug)]
struct Rect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl Rect {
    fn new(x: i32, y: i32, width: u32, height: u32) -> Rect {
        Rect {
            x: x,
            y: y,
            width: width,
            height: height,
        }
    }

    fn x(&self) -> i32 {
        self.x
    }

    fn y(&self) -> i32 {
        self.y
    }

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }
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
            if b.x.gt(&max_x) {
                max_x = b.x;
            }
            if b.x.lt(&min_x) {
                min_x = b.x;
            }
            if b.y.gt(&max_y) {
                max_y = b.y;
            }
            if b.y.lt(&min_y) {
                min_y = b.y;
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
