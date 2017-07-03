#[macro_use]
extern crate serde_derive;
extern crate serde_json;

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

#[derive(PartialEq, Eq, Hash)]
pub enum BlockEvent {
    Left,
    Right,
    Down,
    Drop,
    Rotate,
    None,
}

impl BlockEvent {
    pub fn from_event(evt: u8) -> BlockEvent {
        match evt {
            1 => BlockEvent::Left,
            2 => BlockEvent::Right,
            3 => BlockEvent::Down,
            4 => BlockEvent::Drop,
            5 => BlockEvent::Rotate,
            _ => BlockEvent::None,
        }
    }

    pub fn to_block_event(&self) -> u8 {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct TetrisEvent {
    pub worker_id: u8,
    pub tetris_idx: u8,
    pub events: Vec<u8>,
}

impl TetrisEvent {
    pub fn new(worker_id: u8, tetris_idx: u8, events: Vec<u8>) -> TetrisEvent {
        TetrisEvent {
            worker_id: worker_id,
            tetris_idx: tetris_idx,
            events: events,
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Msg {
    pub worker_id: u8,
    pub tetris_id: u8,
    pub points: Vec<Point>,
    pub grid: [[u8; COLUMNS as usize]; ROWS as usize],
}

impl Msg {
    pub fn new(worker_id: u8, tetris_id: u8, points: Vec<Point>, grid: [[u8; COLUMNS as usize]; ROWS as usize]) -> Msg {
        Msg {
            worker_id: worker_id,
            tetris_id: tetris_id,
            points: points,
            grid: grid,
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self)
    }
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug)]
pub struct Rect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Rect {
        Rect {
            x: x,
            y: y,
            width: width,
            height: height,
        }
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