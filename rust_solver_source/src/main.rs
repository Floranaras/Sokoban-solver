use rustc_hash::FxHashSet;
use smallvec::SmallVec;
use arrayvec::ArrayVec;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::env;
use std::fs;

// Direction constants
const DIR_UP: u8 = 0;
const DIR_DOWN: u8 = 1;
const DIR_LEFT: u8 = 2;
const DIR_RIGHT: u8 = 3;

const DIR_OFFSETS: [(i8, i8); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
const DIR_CHARS: [char; 4] = ['u', 'd', 'l', 'r'];

const ROTATION_PATTERNS: [[usize; 9]; 4] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8],
    [2, 5, 8, 1, 4, 7, 0, 3, 6],
    [8, 7, 6, 5, 4, 3, 2, 1, 0],
    [6, 3, 0, 7, 4, 1, 8, 5, 2],
];

// Compact point using i16 for better cache performance
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(C)]
struct Point {
    row: i16,
    col: i16,
}

impl Point {
    #[inline(always)]
    const fn new(row: i16, col: i16) -> Self {
        Point { row, col }
    }

    #[inline(always)]
    fn pack(self) -> u32 {
        ((self.row as u32) << 16) | (self.col as u32 & 0xFFFF)
    }

    #[inline(always)]
    fn unpack(packed: u32) -> Self {
        Point {
            row: (packed >> 16) as i16,
            col: (packed & 0xFFFF) as i16,
        }
    }
}

type BoxVec = SmallVec<[Point; 20]>;

struct State {
    boxes: BoxVec,
    player: Point,
    path: SmallVec<[u8; 256]>,
    heuristic: i32,
    hash: u64,
}

impl Eq for State {}
impl PartialEq for State {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.heuristic == other.heuristic
    }
}

impl Ord for State {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        other.heuristic.cmp(&self.heuristic)
            .then_with(|| other.path.len().cmp(&self.path.len()))
    }
}

impl PartialOrd for State {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct TranspositionTable {
    entries: Vec<(u64, i32, u8)>,
    size: usize,
    age: u8,
}

impl TranspositionTable {
    fn new(size: usize) -> Self {
        TranspositionTable {
            entries: vec![(0, 0, 0); size],
            size,
            age: 0,
        }
    }

    #[inline(always)]
    fn probe(&self, hash: u64) -> Option<i32> {
        let idx = (hash as usize) % self.size;
        let entry = unsafe { self.entries.get_unchecked(idx) };
        if entry.0 == hash && entry.2 == self.age {
            Some(entry.1)
        } else {
            None
        }
    }

    #[inline(always)]
    fn store(&mut self, hash: u64, heuristic: i32) {
        let idx = (hash as usize) % self.size;
        unsafe {
            *self.entries.get_unchecked_mut(idx) = (hash, heuristic, self.age);
        }
    }

    fn next_age(&mut self) {
        self.age = self.age.wrapping_add(1);
    }
}

struct SokobanSolver {
    width: i16,
    height: i16,
    map: Vec<u8>,
    goals: SmallVec<[Point; 20]>,
    goal_grid: Vec<u64>,
    dead_squares: Vec<u64>,
    room_ids: Vec<u8>,
    goal_counts_by_room: SmallVec<[i32; 8]>,
    zobrist_table: Vec<[u64; 2]>,
    tt: TranspositionTable,
}

impl SokobanSolver {
    fn new(puzzle: &str) -> Self {
        let lines: Vec<&str> = puzzle.lines().collect();
        let height = lines.len() as i16;
        let width = lines.iter().map(|l| l.len()).max().unwrap_or(0) as i16;

        let size = (width * height) as usize;
        let mut map = vec![0u8; size];
        let mut goals = SmallVec::new();

        for (row, line) in lines.iter().enumerate() {
            for (col, ch) in line.chars().enumerate() {
                let idx = row * width as usize + col;
                map[idx] = match ch {
                    '#' => 1,
                    '.' | '+' | '*' => {
                        goals.push(Point::new(row as i16, col as i16));
                        2
                    }
                    _ => 0,
                };
            }
        }

        let mut solver = SokobanSolver {
            width,
            height,
            map,
            goals,
            goal_grid: vec![0u64; (size + 63) / 64],
            dead_squares: vec![0u64; (size + 63) / 64],
            room_ids: vec![255u8; size],
            goal_counts_by_room: SmallVec::new(),
            zobrist_table: vec![[0u64; 2]; size],
            tt: TranspositionTable::new(1 << 20),
        };

        for goal in &solver.goals {
            let idx = (goal.row * width + goal.col) as usize;
            solver.goal_grid[idx / 64] |= 1u64 << (idx % 64);
        }

        solver.initialize_zobrist();
        solver.precompute_static_deadlocks();
        solver.precompute_rooms();
        solver
    }

    fn initialize_zobrist(&mut self) {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hash, Hasher};

        let random_state = RandomState::new();
        let size = (self.width * self.height) as usize;

        for i in 0..size {
            let mut hasher1 = random_state.build_hasher();
            (i, 0u8).hash(&mut hasher1);
            self.zobrist_table[i][0] = hasher1.finish();

            let mut hasher2 = random_state.build_hasher();
            (i, 1u8).hash(&mut hasher2);
            self.zobrist_table[i][1] = hasher2.finish();
        }
    }

    fn precompute_static_deadlocks(&mut self) {
        let size = (self.width * self.height) as usize;
        let mut live_squares = vec![false; size];
        let mut queue = std::collections::VecDeque::with_capacity(self.goals.len() * 4);

        for &goal in &self.goals {
            let idx = (goal.row * self.width + goal.col) as usize;
            live_squares[idx] = true;
            queue.push_back(goal);
        }

        while let Some(pull_target) = queue.pop_front() {
            for &(drow, dcol) in &DIR_OFFSETS {
                let pull_origin_row = pull_target.row + drow as i16;
                let pull_origin_col = pull_target.col + dcol as i16;
                let player_row = pull_origin_row + drow as i16;
                let player_col = pull_origin_col + dcol as i16;

                if self.is_valid(pull_origin_row, pull_origin_col)
                    && self.is_valid(player_row, player_col)
                {
                    let po_idx = (pull_origin_row * self.width + pull_origin_col) as usize;
                    let p_idx = (player_row * self.width + player_col) as usize;

                    if self.map[po_idx] != 1 && self.map[p_idx] != 1 && !live_squares[po_idx] {
                        live_squares[po_idx] = true;
                        queue.push_back(Point::new(pull_origin_row, pull_origin_col));
                    }
                }
            }
        }

        for i in 0..size {
            if self.map[i] != 1 && !live_squares[i] {
                self.dead_squares[i / 64] |= 1u64 << (i % 64);
            }
        }
    }

    fn precompute_rooms(&mut self) {
        self.goal_counts_by_room.clear();
        let mut current_room_id = 0u8;

        for row in 0..self.height {
            for col in 0..self.width {
                let idx = (row * self.width + col) as usize;
                if self.map[idx] != 1 && self.room_ids[idx] == 255 {
                    let goal_count = self.flood_fill_room(Point::new(row, col), current_room_id);
                    self.goal_counts_by_room.push(goal_count);
                    current_room_id += 1;
                }
            }
        }
    }

    #[inline(always)]
    fn is_solved_boxes(&self, boxes: &[Point]) -> bool {
        for &b in boxes {
            let idx = self.to_idx(b.row, b.col);
            if (self.goal_grid[idx / 64] & (1u64 << (idx % 64))) == 0 {
                return false;
            }
        }
        true
    }

    #[inline(always)]
    fn boxes_zobrist_key(&self, boxes: &[Point]) -> u64 {
        let mut key: u64 = 0;
        for &b in boxes {
            let idx = self.to_idx(b.row, b.col);
            key ^= self.zobrist_table[idx][1];
        }   
        key
    }

    #[inline(always)]
    fn find_goal_index(&self, row: i16, col: i16) -> Option<usize> {
        self.goals.iter().position(|g| g.row == row && g.col == col)
    }

    fn flood_fill_room(&mut self, start: Point, room_id: u8) -> i32 {
        let mut goal_count = 0;
        let mut queue = std::collections::VecDeque::with_capacity(100);
        queue.push_back(start);
        let start_idx = (start.row * self.width + start.col) as usize;
        self.room_ids[start_idx] = room_id;

        while let Some(current) = queue.pop_front() {
            let idx = (current.row * self.width + current.col) as usize;
            if (self.goal_grid[idx / 64] & (1u64 << (idx % 64))) != 0 {
                goal_count += 1;
            }

            for &(drow, dcol) in &DIR_OFFSETS {
                let new_row = current.row + drow as i16;
                let new_col = current.col + dcol as i16;

                if self.is_valid(new_row, new_col) {
                    let new_idx = (new_row * self.width + new_col) as usize;
                    if self.map[new_idx] != 1 && self.room_ids[new_idx] == 255 {
                        self.room_ids[new_idx] = room_id;
                        queue.push_back(Point::new(new_row, new_col));
                    }
                }
            }
        }

        goal_count
    }

    #[inline(always)]
    fn is_valid(&self, row: i16, col: i16) -> bool {
        row >= 0 && row < self.height && col >= 0 && col < self.width
    }

    #[inline(always)]
    fn to_idx(&self, row: i16, col: i16) -> usize {
        (row * self.width + col) as usize
    }

    #[inline(always)]
    fn calculate_zobrist_hash_incremental(
        &self,
        old_hash: u64,
        old_player: Point,
        new_player: Point,
        old_box: Option<Point>,
        new_box: Option<Point>,
    ) -> u64 {
        let mut hash = old_hash;

        let old_p_idx = self.to_idx(old_player.row, old_player.col);
        hash ^= self.zobrist_table[old_p_idx][0];

        let new_p_idx = self.to_idx(new_player.row, new_player.col);
        hash ^= self.zobrist_table[new_p_idx][0];

        if let Some(old_b) = old_box {
            let old_b_idx = self.to_idx(old_b.row, old_b.col);
            hash ^= self.zobrist_table[old_b_idx][1];
        }

        if let Some(new_b) = new_box {
            let new_b_idx = self.to_idx(new_b.row, new_b.col);
            hash ^= self.zobrist_table[new_b_idx][1];
        }

        hash
    }

    #[inline(always)]
    fn calculate_zobrist_hash(&self, player: &Point, boxes: &[Point]) -> u64 {
        let p_idx = self.to_idx(player.row, player.col);
        let mut hash = self.zobrist_table[p_idx][0];

        for box_pos in boxes {
            let b_idx = self.to_idx(box_pos.row, box_pos.col);
            hash ^= self.zobrist_table[b_idx][1];
        }

        hash
    }


    fn calculate_heuristic(&self, boxes: &[Point]) -> i32 {
        let box_key = self.boxes_zobrist_key(boxes);
        if let Some(cached) = self.tt.probe(box_key) {
            return cached;
        }

        let mut total_dist = 0;
        let mut used_goal_mask: u64 = 0; // bitmask instead of ArrayVec<bool,32>
        let mut boxes_on_goals = 0;

        for &box_pos in boxes {
            let idx = self.to_idx(box_pos.row, box_pos.col);

            if (self.goal_grid[idx / 64] & (1u64 << (idx % 64))) != 0 {
                if let Some(goal_index) = self.find_goal_index(box_pos.row, box_pos.col) {
                    used_goal_mask |= 1u64 << goal_index;
                }
                boxes_on_goals += 1;
                continue;
            }

            if self.is_frozen_box_ultra_fast(boxes, box_pos.row, box_pos.col) {
                total_dist += 30;
            }

            let mut min_dist = i32::MAX;
            let mut best_idx: Option<usize> = None;

            for (i, goal) in self.goals.iter().enumerate() {
                if (used_goal_mask & (1u64 << i)) == 0 {
                    let dist = (box_pos.row - goal.row).abs() as i32
                        + (box_pos.col - goal.col).abs() as i32;
                    if dist < min_dist {
                        min_dist = dist;
                        best_idx = Some(i);
                    }
                }
            }

            if let Some(i) = best_idx {
                used_goal_mask |= 1u64 << i;
                total_dist += min_dist;
            }
        }

        if boxes_on_goals == boxes.len() {
            return 0;
        }

        total_dist
    }

    #[inline(always)]
    fn mark_goal_as_matched(&self, used_goals: &mut ArrayVec<bool, 32>, row: i16, col: i16) {
        for (i, goal) in self.goals.iter().enumerate() {
            if goal.row == row && goal.col == col {
                used_goals[i] = true;
                return;
            }
        }
    }

    #[inline(always)]
    fn is_frozen_box_ultra_fast(&self, boxes: &[Point], row: i16, col: i16) -> bool {
        let idx = self.to_idx(row, col);
        
        if (self.goal_grid[idx / 64] & (1u64 << (idx % 64))) != 0 {
            return false;
        }

        let has_obstacle = |r: i16, c: i16| -> bool {
            if !self.is_valid(r, c) {
                return true;
            }
            let i = self.to_idx(r, c);
            if self.map[i] == 1 {
                return true;
            }
            boxes.iter().any(|b| b.row == r && b.col == c)
        };

        let v_blocked = has_obstacle(row - 1, col) || has_obstacle(row + 1, col);
        let h_blocked = has_obstacle(row, col - 1) || has_obstacle(row, col + 1);

        v_blocked && h_blocked
    }

    #[inline]
    fn is_room_deadlock(&self, boxes: &[Point]) -> bool {
        let mut box_counts: SmallVec<[i32; 8]> = SmallVec::new();
        box_counts.resize(self.goal_counts_by_room.len(), 0);

        for box_pos in boxes {
            let idx = self.to_idx(box_pos.row, box_pos.col);
            let room_id = self.room_ids[idx];
            if room_id != 255 {
                box_counts[room_id as usize] += 1;
            }
        }

        for (room_id, &goal_count) in self.goal_counts_by_room.iter().enumerate() {
            if box_counts[room_id] > goal_count {
                return true;
            }
        }

        false
    }

    fn solve(&mut self, start_player: Point, start_boxes: BoxVec) -> String {
        let start_hash = self.calculate_zobrist_hash(&start_player, &start_boxes);
        let start_heuristic = self.calculate_heuristic(&start_boxes);

        let start_state = State {
            boxes: start_boxes,
            player: start_player,
            path: SmallVec::new(),
            heuristic: start_heuristic,
            hash: start_hash,
        };

        let mut open_set = BinaryHeap::with_capacity(10000);
        let mut visited: FxHashSet<u64> = FxHashSet::with_capacity_and_hasher(200000, Default::default());

        open_set.push(start_state);

        while let Some(current) = open_set.pop() {
            if self.is_solved_boxes(&current.boxes) {
                return current.path.iter().map(|&dir| DIR_CHARS[dir as usize]).collect();
            }

            if !visited.insert(current.hash) {
                continue;
            }

            for dir in 0..4 {
                let (drow, dcol) = DIR_OFFSETS[dir];
                let new_player_row = current.player.row + drow as i16;
                let new_player_col = current.player.col + dcol as i16;

                if !self.is_valid(new_player_row, new_player_col) {
                    continue;
                }

                let new_p_idx = self.to_idx(new_player_row, new_player_col);
                if self.map[new_p_idx] == 1 {
                    continue;
                }

                let new_player = Point::new(new_player_row, new_player_col);
                
                let box_idx = current.boxes.iter().position(|b| b.row == new_player_row && b.col == new_player_col);

                let mut new_boxes = current.boxes.clone();
                let mut old_box = None;
                let mut pushed_box = None;

                if let Some(idx) = box_idx {
                    let push_row = new_player_row + drow as i16;
                    let push_col = new_player_col + dcol as i16;

                    if !self.is_valid(push_row, push_col) {
                        continue;
                    }

                    let push_idx = self.to_idx(push_row, push_col);
                    if self.map[push_idx] == 1 {
                        continue;
                    }

                    let push_pos = Point::new(push_row, push_col);
                    if new_boxes.iter().any(|b| b.row == push_row && b.col == push_col) {
                        continue;
                    }

                    if (self.dead_squares[push_idx / 64] & (1u64 << (push_idx % 64))) != 0 {
                        continue;
                    }

                    old_box = Some(new_boxes[idx]);
                    new_boxes[idx] = push_pos;
                    pushed_box = Some(push_pos);

                    if self.is_room_deadlock(&new_boxes) {
                        continue;
                    }
                }

                let new_hash = self.calculate_zobrist_hash_incremental(
                    current.hash,
                    current.player,
                    new_player,
                    old_box,
                    pushed_box,
                );

                if !visited.contains(&new_hash) {
                    let new_heuristic = self.calculate_heuristic(&new_boxes);

                    let mut new_path = current.path.clone();
                    new_path.push(dir as u8);

                    let next_state = State {
                        boxes: new_boxes,
                        player: new_player,
                        path: new_path,
                        heuristic: new_heuristic,
                        hash: new_hash,
                    };

                    open_set.push(next_state);
                }
            }
        }

        String::new()
    }
}

fn parse_puzzle(puzzle: &str) -> (Point, BoxVec, SokobanSolver) {
    let lines: Vec<&str> = puzzle.lines().collect();
    let mut player = Point::new(0, 0);
    let mut boxes = BoxVec::new();

    for (row, line) in lines.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            match ch {
                '@' | '+' => player = Point::new(row as i16, col as i16),
                '$' | '*' => boxes.push(Point::new(row as i16, col as i16)),
                _ => {}
            }
        }
    }

    let solver = SokobanSolver::new(puzzle);
    (player, boxes, solver)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: rust_solver <puzzle_file>");
        std::process::exit(1);
    }

    let puzzle_path = &args[1];
    let puzzle = fs::read_to_string(puzzle_path).expect("Failed to read puzzle file");

    let (player, boxes, mut solver) = parse_puzzle(&puzzle);
    let solution = solver.solve(player, boxes);

    println!("{}", solution);
}
