use rustc_hash::FxHashSet;
use smallvec::SmallVec;
use arrayvec::ArrayVec;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::env;
use std::fs;

// Direction constants
const DIR_UP: (i32, i32, char) = (-1, 0, 'u');
const DIR_DOWN: (i32, i32, char) = (1, 0, 'd');
const DIR_LEFT: (i32, i32, char) = (0, -1, 'l');
const DIR_RIGHT: (i32, i32, char) = (0, 1, 'r');
const DIRS: [(i32, i32, char); 4] = [DIR_UP, DIR_DOWN, DIR_LEFT, DIR_RIGHT];

const ROTATION_PATTERNS: [[usize; 9]; 4] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8],
    [2, 5, 8, 1, 4, 7, 0, 3, 6],
    [8, 7, 6, 5, 4, 3, 2, 1, 0],
    [6, 3, 0, 7, 4, 1, 8, 5, 2],
];

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(C)]
struct Point {
    row: i32,
    col: i32,
}

impl Point {
    #[inline(always)]
    const fn new(row: i32, col: i32) -> Self {
        Point { row, col }
    }
}

// Use SmallVec to avoid heap allocation for small box counts (most puzzles have < 20 boxes)
type BoxVec = SmallVec<[Point; 20]>;

struct State {
    boxes: BoxVec,
    player: Point,
    path: String,
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
    }
}

impl PartialOrd for State {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct SokobanSolver {
    width: i32,
    height: i32,
    map: Vec<Vec<char>>,
    goals: SmallVec<[Point; 20]>,
    goal_grid: Vec<Vec<bool>>,
    dead_squares: Vec<Vec<bool>>,
    room_ids: Vec<Vec<i32>>,
    goal_counts_by_room: Vec<i32>,
    zobrist_table: Vec<Vec<[u64; 2]>>,
}

impl SokobanSolver {
    fn new(puzzle: &str) -> Self {
        let lines: Vec<&str> = puzzle.lines().collect();
        let height = lines.len() as i32;
        let width = lines.iter().map(|l| l.len()).max().unwrap_or(0) as i32;

        let mut map = vec![vec![' '; width as usize]; height as usize];
        let mut goals = SmallVec::new();
        let mut goal_grid = vec![vec![false; width as usize]; height as usize];

        for (row, line) in lines.iter().enumerate() {
            for (col, ch) in line.chars().enumerate() {
                let cell = match ch {
                    '@' | '+' => ' ',
                    '$' | '*' => ' ',
                    _ => ch,
                };
                map[row][col] = cell;

                if matches!(ch, '.' | '+' | '*') {
                    goals.push(Point::new(row as i32, col as i32));
                    goal_grid[row][col] = true;
                }
            }
        }

        let mut solver = SokobanSolver {
            width,
            height,
            map,
            goals,
            goal_grid,
            dead_squares: vec![vec![false; width as usize]; height as usize],
            room_ids: vec![vec![-1; width as usize]; height as usize],
            goal_counts_by_room: Vec::new(),
            zobrist_table: vec![vec![[0u64; 2]; width as usize]; height as usize],
        };

        solver.initialize_zobrist();
        solver.dead_squares = solver.precompute_static_deadlocks();
        solver.precompute_rooms();
        solver
    }

    fn initialize_zobrist(&mut self) {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hash, Hasher};

        let random_state = RandomState::new();

        for row in 0..self.height as usize {
            for col in 0..self.width as usize {
                let mut hasher1 = random_state.build_hasher();
                (row, col, 0u8).hash(&mut hasher1);
                self.zobrist_table[row][col][0] = hasher1.finish();

                let mut hasher2 = random_state.build_hasher();
                (row, col, 1u8).hash(&mut hasher2);
                self.zobrist_table[row][col][1] = hasher2.finish();
            }
        }
    }

    fn precompute_static_deadlocks(&self) -> Vec<Vec<bool>> {
        let mut live_squares = vec![vec![false; self.width as usize]; self.height as usize];
        let mut queue = std::collections::VecDeque::with_capacity(self.goals.len() * 4);

        for &goal in &self.goals {
            live_squares[goal.row as usize][goal.col as usize] = true;
            queue.push_back(goal);
        }

        while let Some(pull_target) = queue.pop_front() {
            for &(drow, dcol, _) in &DIRS {
                let pull_origin_row = pull_target.row - drow;
                let pull_origin_col = pull_target.col - dcol;
                let player_row = pull_origin_row - drow;
                let player_col = pull_origin_col - dcol;

                if self.is_valid(pull_origin_row, pull_origin_col)
                    && self.is_valid(player_row, player_col)
                    && unsafe {
                        *self.map.get_unchecked(pull_origin_row as usize).get_unchecked(pull_origin_col as usize) != '#'
                            && *self.map.get_unchecked(player_row as usize).get_unchecked(player_col as usize) != '#'
                    }
                    && !live_squares[pull_origin_row as usize][pull_origin_col as usize]
                {
                    live_squares[pull_origin_row as usize][pull_origin_col as usize] = true;
                    queue.push_back(Point::new(pull_origin_row, pull_origin_col));
                }
            }
        }

        let mut result = vec![vec![false; self.width as usize]; self.height as usize];
        for row in 0..self.height as usize {
            for col in 0..self.width as usize {
                if self.map[row][col] != '#' && !live_squares[row][col] {
                    result[row][col] = true;
                }
            }
        }

        result
    }

    fn precompute_rooms(&mut self) {
        self.goal_counts_by_room.clear();
        let mut current_room_id = 0;

        for row in 0..self.height {
            for col in 0..self.width {
                if self.map[row as usize][col as usize] != '#'
                    && self.room_ids[row as usize][col as usize] == -1
                {
                    let goal_count = self.flood_fill_room(Point::new(row, col), current_room_id);
                    self.goal_counts_by_room.push(goal_count);
                    current_room_id += 1;
                }
            }
        }
    }

    fn flood_fill_room(&mut self, start: Point, room_id: i32) -> i32 {
        let mut goal_count = 0;
        let mut queue = std::collections::VecDeque::with_capacity(100);
        queue.push_back(start);
        self.room_ids[start.row as usize][start.col as usize] = room_id;

        while let Some(current) = queue.pop_front() {
            if self.goal_grid[current.row as usize][current.col as usize] {
                goal_count += 1;
            }

            for &(drow, dcol, _) in &DIRS {
                let new_row = current.row + drow;
                let new_col = current.col + dcol;

                if self.is_valid(new_row, new_col)
                    && self.map[new_row as usize][new_col as usize] != '#'
                    && self.room_ids[new_row as usize][new_col as usize] == -1
                {
                    self.room_ids[new_row as usize][new_col as usize] = room_id;
                    queue.push_back(Point::new(new_row, new_col));
                }
            }
        }

        goal_count
    }

    #[inline(always)]
    fn is_valid(&self, row: i32, col: i32) -> bool {
        row >= 0 && row < self.height && col >= 0 && col < self.width
    }

    #[inline(always)]
    fn calculate_zobrist_hash(&self, player: &Point, boxes: &[Point]) -> u64 {
        let mut hash = unsafe {
            *self.zobrist_table
                .get_unchecked(player.row as usize)
                .get_unchecked(player.col as usize)
                .get_unchecked(0)
        };
        
        for box_pos in boxes {
            hash ^= unsafe {
                *self.zobrist_table
                    .get_unchecked(box_pos.row as usize)
                    .get_unchecked(box_pos.col as usize)
                    .get_unchecked(1)
            };
        }
        hash
    }

    fn calculate_heuristic(&self, boxes: &[Point]) -> i32 {
        let mut total_dist = 0;
        let mut used_goals = ArrayVec::<bool, 32>::new();
        used_goals.resize(self.goals.len(), false);
        let mut boxes_on_goals = 0;

        for &box_pos in boxes {
            if unsafe { *self.goal_grid.get_unchecked(box_pos.row as usize).get_unchecked(box_pos.col as usize) } {
                self.mark_goal_as_matched(&mut used_goals, box_pos.row, box_pos.col);
                boxes_on_goals += 1;
                continue;
            }

            if self.is_frozen_box_fast(boxes, box_pos.row, box_pos.col) {
                total_dist += 30;
            }

            let mut min_dist = i32::MAX;
            let mut best_idx = None;

            for (i, goal) in self.goals.iter().enumerate() {
                if !used_goals[i] {
                    let dist = (box_pos.row - goal.row).abs() + (box_pos.col - goal.col).abs();
                    if dist < min_dist {
                        min_dist = dist;
                        best_idx = Some(i);
                    }
                }
            }

            if let Some(idx) = best_idx {
                used_goals[idx] = true;
                total_dist += min_dist;
            }
        }

        if boxes_on_goals == boxes.len() {
            return 0;
        }

        total_dist
    }

    #[inline(always)]
    fn mark_goal_as_matched(&self, used_goals: &mut ArrayVec<bool, 32>, row: i32, col: i32) {
        for (i, goal) in self.goals.iter().enumerate() {
            if goal.row == row && goal.col == col {
                used_goals[i] = true;
                break;
            }
        }
    }

    // Optimized frozen box check using bit operations where possible
    #[inline]
    fn is_frozen_box_fast(&self, boxes: &[Point], row: i32, col: i32) -> bool {
        // Quick check: if not near walls or other boxes, can't be frozen
        let has_box_or_wall = |r: i32, c: i32| -> bool {
            if !self.is_valid(r, c) || self.map[r as usize][c as usize] == '#' {
                return true;
            }
            boxes.iter().any(|b| b.row == r && b.col == c)
        };

        let up_blocked = has_box_or_wall(row - 1, col);
        let down_blocked = has_box_or_wall(row + 1, col);
        let left_blocked = has_box_or_wall(row, col - 1);
        let right_blocked = has_box_or_wall(row, col + 1);

        (up_blocked || down_blocked) && (left_blocked || right_blocked)
    }

    fn is_room_deadlock(&self, boxes: &[Point]) -> bool {
        let mut box_counts = smallvec::smallvec![0i32; 8];
        box_counts.resize(self.goal_counts_by_room.len(), 0);

        for box_pos in boxes {
            let room_id = self.room_ids[box_pos.row as usize][box_pos.col as usize];
            if room_id >= 0 {
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

    fn solve(&self, start_player: Point, start_boxes: BoxVec) -> String {
        let start_hash = self.calculate_zobrist_hash(&start_player, &start_boxes);
        let start_heuristic = self.calculate_heuristic(&start_boxes);

        let start_state = State {
            boxes: start_boxes,
            player: start_player,
            path: String::with_capacity(500),
            heuristic: start_heuristic,
            hash: start_hash,
        };

        let mut open_set = BinaryHeap::with_capacity(10000);
        let mut visited: FxHashSet<u64> = FxHashSet::with_capacity_and_hasher(100000, Default::default());

        open_set.push(start_state);

        let goal_set: FxHashSet<Point> = self.goals.iter().copied().collect();

        while let Some(current) = open_set.pop() {
            // Early goal check
            if current.heuristic == 0 {
                return current.path;
            }

            if visited.contains(&current.hash) {
                continue;
            }
            visited.insert(current.hash);

            // Try all moves
            for &(drow, dcol, move_char) in &DIRS {
                let new_player_row = current.player.row + drow;
                let new_player_col = current.player.col + dcol;

                if !self.is_valid(new_player_row, new_player_col) {
                    continue;
                }

                if unsafe { *self.map.get_unchecked(new_player_row as usize).get_unchecked(new_player_col as usize) == '#' } {
                    continue;
                }

                let new_player = Point::new(new_player_row, new_player_col);
                
                let box_idx = current.boxes.iter().position(|b| b.row == new_player_row && b.col == new_player_col);

                let mut new_boxes = current.boxes.clone();

                if let Some(idx) = box_idx {
                    // Box push
                    let push_row = new_player_row + drow;
                    let push_col = new_player_col + dcol;

                    if !self.is_valid(push_row, push_col) {
                        continue;
                    }

                    if unsafe { *self.map.get_unchecked(push_row as usize).get_unchecked(push_col as usize) == '#' } {
                        continue;
                    }

                    let push_pos = Point::new(push_row, push_col);
                    if new_boxes.iter().any(|b| b.row == push_row && b.col == push_col) {
                        continue;
                    }

                    // Check deadlock
                    if self.dead_squares[push_row as usize][push_col as usize] {
                        continue;
                    }

                    new_boxes[idx] = push_pos;

                    // Check room deadlock
                    if self.is_room_deadlock(&new_boxes) {
                        continue;
                    }
                }

                let mut new_path = current.path.clone();
                new_path.push(move_char);
                
                let new_hash = self.calculate_zobrist_hash(&new_player, &new_boxes);

                if !visited.contains(&new_hash) {
                    let new_heuristic = self.calculate_heuristic(&new_boxes);

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
                '@' | '+' => player = Point::new(row as i32, col as i32),
                '$' | '*' => boxes.push(Point::new(row as i32, col as i32)),
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

    let (player, boxes, solver) = parse_puzzle(&puzzle);
    let solution = solver.solve(player, boxes);

    println!("{}", solution);
}
