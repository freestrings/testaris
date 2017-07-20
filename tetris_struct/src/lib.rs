extern crate rand;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use rand::distributions::{IndependentSample, Range};

type Points = Vec<Point>;
type Block = (BlockType/*current*/, Vec<Point>/*current*/, Option<BlockType>/*next*/);
type Grid = [[u8; COLUMNS as usize]; ROWS as usize];

//
//    #
// #, @, #
pub const BLOCK_T: &[(u8, u8)] = &[(1, 0), (0, 1), (1, 1), (2, 1)];
//
// #
// #, @, #
pub const BLOCK_J: &[(u8, u8)] = &[(0, 0), (0, 1), (1, 1), (2, 1)];
//
//       #
// #, @, #
pub const BLOCK_L: &[(u8, u8)] = &[(2, 0), (2, 1), (1, 1), (0, 1)];
//
//    #, #
// #, @
pub const BLOCK_S: &[(u8, u8)] = &[(2, 0), (1, 0), (1, 1), (0, 1)];
//
// #, #
//    @, #
pub const BLOCK_Z: &[(u8, u8)] = &[(0, 0), (1, 0), (1, 1), (2, 1)];
//
// #, #
// #, #
pub const BLOCK_O: &[(u8, u8)] = &[(0, 0), (1, 0), (0, 1), (1, 1)];
//
// #, #, @, #
pub const BLOCK_I: &[(u8, u8)] = &[(0, 0), (1, 0), (2, 0), (3, 0)];

pub const COLUMNS: u32 = 10;
pub const ROWS: u32 = 20;

pub const COLOR_PURPLE: (u8, u8, u8) = (128, 0, 128);
pub const COLOR_BLUE: (u8, u8, u8) = (0, 0, 255);
pub const COLOR_ORANGE: (u8, u8, u8) = (255, 165, 0);
pub const COLOR_LIME: (u8, u8, u8) = (128, 255, 0);
pub const COLOR_RED: (u8, u8, u8) = (255, 0, 0);
pub const COLOR_YELLOW: (u8, u8, u8) = (255, 255, 0);
pub const COLOR_CYAN: (u8, u8, u8) = (0, 255, 255);
pub const COLOR_BLACK: (u8, u8, u8) = (0, 0, 0);

pub const DEFAULT_GRAVITY: u8 = 20;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BlockType {
    T,
    J,
    L,
    S,
    Z,
    O,
    I,
}

impl BlockType {
    pub fn new(index: u8) -> BlockType {
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

    pub fn random() -> BlockType {
        let mut rng = rand::thread_rng();
        let between = Range::new(1, 8);
        BlockType::new(between.ind_sample(&mut rng))
    }

    pub fn index(&self) -> u8 {
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

    pub fn color(&self) -> (u8, u8, u8) {
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

    pub fn points(&self) -> Points {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockEvent {
    Left,
    Right,
    Down,
    Drop,
    Rotate,
    None,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AppEvent {
    InitWorker(u8/*worker index*/, u32/*tetris count*/),
    InitTetris(u8/*worker index*/, u32/*tetris id*/),
    Tick(u8/*worker index*/, u32/*tetris id*/),
    User(u8/*worker index*/, u32/*tetris id*/, Vec<BlockEvent>),
}

impl AppEvent {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self)
    }

    pub fn worker_id(&self) -> u8 {
        match *self {
            AppEvent::InitWorker(worker_index, _) |
            AppEvent::InitTetris(worker_index, _) |
            AppEvent::Tick(worker_index, _) |
            AppEvent::User(worker_index, _, _) => worker_index
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Msg {
    pub event: AppEvent,
    pub block: Option<Block>,
    pub grid: Option<Grid>,
}

impl Msg {
    pub fn new(event: AppEvent, block: Option<Block>, grid: Option<Grid>) -> Msg {
        Msg { event: event, block: block, grid: grid }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    x: i32,
    y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Point {
        Point { x: x, y: y }
    }

    pub fn x(&self) -> i32 {
        self.x
    }

    pub fn y(&self) -> i32 {
        self.y
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Rect {
        Rect { x: x, y: y, width: width, height: height }
    }

    pub fn x(&self) -> i32 {
        self.x
    }

    pub fn y(&self) -> i32 {
        self.y
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}