extern crate rand;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use rand::distributions::{IndependentSample, Range};

type Points = Vec<Point>;
type Color = (u8, u8, u8);

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

pub const COLUMNS: usize = 10;
pub const ROWS: usize = 20;

pub const COLOR_PURPLE: (u8, u8, u8) = (128, 0, 128);
pub const COLOR_BLUE: (u8, u8, u8) = (0, 0, 255);
pub const COLOR_ORANGE: (u8, u8, u8) = (255, 165, 0);
pub const COLOR_LIME: (u8, u8, u8) = (128, 255, 0);
pub const COLOR_RED: (u8, u8, u8) = (255, 0, 0);
pub const COLOR_YELLOW: (u8, u8, u8) = (255, 255, 0);
pub const COLOR_CYAN: (u8, u8, u8) = (0, 255, 255);
pub const COLOR_BLACK: (u8, u8, u8) = (0, 0, 0);

pub const DEFAULT_GRAVITY: u8 = 20;

pub struct Ticker {
    fact: u32,
    elapsed: u32,
}

impl Ticker {
    pub fn new(fact: u32) -> Ticker {
        Ticker {
            fact: fact,
            elapsed: 0,
        }
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

pub struct Tetris {
    pub block: Block,
    pub grid: Grid,
    pub ticker: Ticker,
}

impl Tetris {
    pub fn new() -> Tetris {
        Tetris {
            block: Block::new(BlockType::random()),
            grid: Grid::new(),
            ticker: Ticker::new(10),
        }
    }

    pub fn init(&mut self) {
        self.block.align_to_start();

        if self.block.next_ref().is_none() {
            self.block.load_next();
        }
    }

    pub fn tick(&mut self) {
        let ref mut ticker = self.ticker;
        let ref mut block = self.block;
        let ref mut grid = self.grid;

        if ticker.tick() {
            block.down(|points| !grid.is_empty(points));
        }

        if !grid.is_empty_below(block.points_ref()) {
            grid.fill(&block);
            grid.erase_full_row(&block);
            block.apply_next();
        }
    }

    pub fn event(&mut self, block_events: Option<Vec<BlockEvent>>) {
        if block_events.is_none() {
            return;
        }

        let block_events = block_events.unwrap();

        let ref mut block = self.block;
        let ref mut grid = self.grid;

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

            if !grid.is_empty_below(block.points_ref()) {
                grid.fill(block);
                grid.erase_full_row(block);
                break;
            }

            block.adjust_bound();
        }
    }

    pub fn get_block(&self) -> Block {
        self.block.clone()
    }

    pub fn get_grid(&self) -> Grid {
        self.grid.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    block_type: BlockType,
    color: Color,
    points: Points,
    next: Option<Box<Block>>,
}

impl Block {
    pub fn new(block_type: BlockType) -> Block {
        let points = block_type.points();
        let color = block_type.color();

        Block {
            block_type: block_type,
            points: points,
            color: color,
            next: None,
        }
    }

    pub fn load_next(&mut self) {
        self.next = Some(Box::new(Block::new(BlockType::random())));
    }

    pub fn apply_next(&mut self) {
        let mut block = self.next.take().expect("Can not apply a next block!");
        self.block_type = block.block_type.clone();
        self.color = block.color;
        self.update(block.points_ref_mut());
        self.align_to_start();
        self.load_next();
    }

    pub fn align_to_start(&mut self) {
        let range = self.range();
        let x = (COLUMNS / 2 - range.width() / 2) as i32;
        let y = range.height() as i32 * -1;
        self.shift(|| (x, y));
    }

    pub fn next_ref(&self) -> &Option<Box<Block>> {
        &self.next
    }

    pub fn next_type(&self) -> Option<BlockType> {
        if let Some(ref next) = self.next {
            Some(next.type_ref().clone())
        } else {
            None
        }
    }

    pub fn type_ref(&self) -> &BlockType {
        &self.block_type
    }

    pub fn color_ref(&self) -> &Color {
        &self.color
    }

    pub fn points_ref(&self) -> &Points {
        &self.points
    }

    pub fn points_ref_mut(&mut self) -> &mut Points {
        &mut self.points
    }

    pub fn points(&self) -> Points {
        self.points.clone()
    }

    pub fn update(&mut self, target_points: &mut Points) {
        self.points.truncate(0);
        self.points.append(target_points);
    }

    //
    // https://www.youtube.com/watch?v=Atlr5vvdchY
    //
    pub fn rotate(&mut self) {
        let angle = std::f32::consts::PI * 0.5_f32;
        let center = Point::new(self.points_ref()[2].x(), self.points_ref()[2].y());
        let mut points = self.points_ref()
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

    pub fn shift<F>(&mut self, mut f: F)
    where
        F: FnMut() -> (i32, i32),
    {
        let mut points = self.points_ref()
            .iter()
            .map(|point| {
                let raw_point = f();
                Point::new(point.x() + raw_point.0, point.y() + raw_point.1)
            })
            .collect();

        self.update(&mut points);
    }

    pub fn left<GARD>(&mut self, rollback_gard: GARD)
    where
        GARD: Fn(&Points) -> bool,
    {
        self.shift(|| (-1, 0));
        if rollback_gard(self.points_ref()) {
            self.shift(|| (1, 0));
        }
    }

    pub fn right<GARD>(&mut self, rollback_gard: GARD)
    where
        GARD: Fn(&Points) -> bool,
    {
        self.shift(|| (1, 0));
        if rollback_gard(self.points_ref()) {
            self.shift(|| (-1, 0));
        }
    }

    pub fn down<GARD>(&mut self, rollback_gard: GARD)
    where
        GARD: Fn(&Points) -> bool,
    {
        self.shift(|| (0, 1));
        if rollback_gard(self.points_ref()) {
            self.shift(|| (0, -1));
        }
    }

    pub fn drop<GARD>(&mut self, rollback_gard: GARD)
    where
        GARD: Fn(&Points) -> bool,
    {
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

    pub fn range(&self) -> Rect {
        let mut min_x = i32::max_value();
        let mut max_x = i32::min_value();
        let mut min_y = i32::max_value();
        let mut max_y = i32::min_value();

        let points = self.points_ref();
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

        let width = (max_x - min_x).abs() as usize + 1;
        let height = (max_y - min_y).abs() as usize + 1;
        Rect::new(min_x, min_y, width, height)
    }

    pub fn adjust_bound(&mut self) {
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grid {
    data: Vec<Vec<u8>>,
}

impl Grid {
    pub fn new() -> Grid {
        Grid { data: vec![vec![0_u8; COLUMNS]; ROWS] }
    }

    pub fn get_data(&self) -> &Vec<Vec<u8>> {
        &self.data
    }

    fn _check_index_range(&self, point: &Point) -> bool {
        point.y() >= 0 && point.y() < ROWS as i32 && point.x() >= 0 && point.x() < COLUMNS as i32
    }

    pub fn fill(&mut self, block: &Block) {
        for point in block.points_ref() {
            if self._check_index_range(point) {
                self.data[point.y() as usize][point.x() as usize] = block.block_type.index();
            }
        }
    }

    pub fn is_empty_below(&self, points: &Points) -> bool {
        for point in points {
            if point.y() + 1 == ROWS as i32 {
                return false;
            }

            if self._check_index_range(point) && point.y() + 1 < ROWS as i32 &&
                self.data[point.y() as usize + 1][point.x() as usize] > 0
            {
                return false;
            }
        }
        true
    }

    pub fn is_empty(&self, points: &Points) -> bool {
        points
            .iter()
            .filter(|point| self._check_index_range(point))
            .filter(|point| {
                self.data[point.y() as usize][point.x() as usize] > 0
            })
            .collect::<Vec<&Point>>()
            .len() == 0
    }

    fn _is_full(&self, r_index: usize) -> bool {
        !self.data[r_index].contains(&0)
    }

    pub fn remove_row(&mut self, r_index: usize) {
        self.data.remove(r_index);
        self.data.insert(0, vec![0_u8; COLUMNS]);
    }

    pub fn erase_full_row(&mut self, block: &Block) {
        let range = block.range();

        for r in range.y()..range.y() + range.height() as i32 {
            if r < 0 || r >= ROWS as i32 {
                continue;
            }

            let r_index = r as usize;

            if self._is_full(r_index) {
                self.remove_row(r_index);
            }
        }
    }
}


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
    InitWorker(u8 /*worker index*/, u32 /*tetris count*/),
    InitTetris(u8 /*worker index*/, u32 /*tetris id*/),
    Tick(u8 /*worker index*/, u32 /*tetris id*/),
    User(u8 /*worker index*/, u32 /*tetris id*/, Option<Vec<BlockEvent>>),
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
            AppEvent::User(worker_index, _, _) => worker_index,
        }
    }

    pub fn tetris_id(&self) -> u32 {
        match *self {
            AppEvent::InitWorker(_, tetris_id) |
            AppEvent::InitTetris(_, tetris_id) |
            AppEvent::Tick(_, tetris_id) |
            AppEvent::User(_, tetris_id, _) => tetris_id,
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
        Msg {
            event: event,
            block: block,
            grid: grid,
        }
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
    width: usize,
    height: usize,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: usize, height: usize) -> Rect {
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

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_fill() {
        let mut grid = Grid::new();
        let block = Block::new(BlockType::J);
        grid.fill(&block);

        let data = grid.get_data();
        assert_eq!(data[0][0], 2);
        assert_eq!(data[1][0..3], [2_u8, 2_u8, 2_u8]);
    }

    #[test]
    fn grid_remove() {
        let mut grid = Grid::new();
        let mut block = Block::new(BlockType::I);

        block.down(|_| false);
        grid.fill(&block);

        grid.remove_row(1);

        assert_eq!(grid.get_data().len(), 20);
        assert_eq!(
            grid.get_data().iter().fold(0, |acc, r| if r.iter()
                .filter(|c| c > &&0_u8)
                .collect::<Vec<&u8>>()
                .len() == 0
            {
                acc + 1
            } else {
                acc
            }),
            20
        );
    }

    fn right(block: &mut Block, iter: i32) {
        for _ in 0..iter {
            block.right(|_| false);
        }
    }

    fn left(block: &mut Block, iter: i32) {
        for _ in 0..iter {
            block.left(|_| false);
        }
    }

    #[test]
    fn grid_erase1() {
        let mut grid = Grid::new();

        let mut block = Block::new(BlockType::Z);
        block.drop(|_| false);
        grid.fill(&block);

        let mut block = Block::new(BlockType::Z);
        right(&mut block, 2);
        block.drop(|_| false);
        grid.fill(&block);

        let mut block = Block::new(BlockType::Z);
        right(&mut block, 4);
        block.drop(|_| false);
        grid.fill(&block);

        let mut block = Block::new(BlockType::Z);
        right(&mut block, 6);
        block.drop(|_| false);
        grid.fill(&block);

        let mut block = Block::new(BlockType::T);
        block.rotate();
        right(&mut block, 8);
        block.drop(|_| false);
        grid.fill(&block);

        let data = grid.get_data().clone();

        assert_eq!(
            data[17][0..10],
            [0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 1_u8]
        );
        assert_eq!(
            data[18][0..10],
            [5_u8, 5_u8, 5_u8, 5_u8, 5_u8, 5_u8, 5_u8, 5_u8, 1_u8, 1_u8]
        );
        assert_eq!(
            data[19][0..10],
            [0_u8, 5_u8, 5_u8, 5_u8, 5_u8, 5_u8, 5_u8, 5_u8, 5_u8, 1_u8]
        );

        grid.erase_full_row(&block);

        let data = grid.get_data();
        assert_eq!(
            data[18][0..10],
            [0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 1_u8]
        );
        assert_eq!(
            data[19][0..10],
            [0_u8, 5_u8, 5_u8, 5_u8, 5_u8, 5_u8, 5_u8, 5_u8, 5_u8, 1_u8]
        );

    }

    #[test]
    fn grid_erase2() {
        let mut grid = Grid::new();

        let mut block = Block::new(BlockType::I);
        block.drop(|p| !grid.is_empty(p));
        grid.fill(&block);

        let mut block = Block::new(BlockType::I);
        right(&mut block, 4);
        block.drop(|p| !grid.is_empty(p));
        grid.fill(&block);

        let mut block = Block::new(BlockType::I);
        right(&mut block, 1);
        block.drop(|p| !grid.is_empty(p));
        grid.fill(&block);

        let mut block = Block::new(BlockType::I);
        right(&mut block, 5);
        block.drop(|p| !grid.is_empty(p));
        grid.fill(&block);

        let mut block = Block::new(BlockType::I);
        right(&mut block, 1);
        block.drop(|p| !grid.is_empty(p));
        grid.fill(&block);

        let mut block = Block::new(BlockType::L);
        right(&mut block, 6);
        block.drop(|p| !grid.is_empty(p));
        grid.fill(&block);

        let mut block = Block::new(BlockType::I);
        right(&mut block, 1);
        block.drop(|p| !grid.is_empty(p));
        grid.fill(&block);

        let mut block = Block::new(BlockType::L);
        right(&mut block, 5);
        block.drop(|p| !grid.is_empty(p));
        grid.fill(&block);

        let mut block = Block::new(BlockType::I);
        block.rotate();
        right(&mut block, 7);
        block.drop(|p| !grid.is_empty(p));
        grid.fill(&block);

        let mut block = Block::new(BlockType::I);
        block.rotate();
        left(&mut block, 2);
        block.drop(|p| !grid.is_empty(p));
        grid.fill(&block);

        let data = grid.get_data().clone();
        assert_eq!(
            data[15][0..10],
            [7_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 3_u8, 0_u8, 0_u8]
        );
        assert_eq!(
            data[16][0..10],
            [7_u8, 7_u8, 7_u8, 7_u8, 7_u8, 3_u8, 3_u8, 3_u8, 3_u8, 7_u8]
        );
        assert_eq!(
            data[17][0..10],
            [7_u8, 7_u8, 7_u8, 7_u8, 7_u8, 0_u8, 3_u8, 3_u8, 3_u8, 7_u8]
        );
        assert_eq!(
            data[18][0..10],
            [7_u8, 7_u8, 7_u8, 7_u8, 7_u8, 7_u8, 7_u8, 7_u8, 7_u8, 7_u8]
        );
        assert_eq!(
            data[19][0..10],
            [7_u8, 7_u8, 7_u8, 7_u8, 7_u8, 7_u8, 7_u8, 7_u8, 0_u8, 7_u8]
        );

        grid.erase_full_row(&block);

        let data = grid.get_data().clone();
        assert_eq!(
            data[17][0..10],
            [7_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 3_u8, 0_u8, 0_u8]
        );
        assert_eq!(
            data[18][0..10],
            [7_u8, 7_u8, 7_u8, 7_u8, 7_u8, 0_u8, 3_u8, 3_u8, 3_u8, 7_u8]
        );
        assert_eq!(
            data[19][0..10],
            [7_u8, 7_u8, 7_u8, 7_u8, 7_u8, 7_u8, 7_u8, 7_u8, 0_u8, 7_u8]
        );

    }

}
