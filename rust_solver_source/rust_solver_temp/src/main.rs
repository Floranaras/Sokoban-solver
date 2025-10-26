use rustc_hash::FxHashSet;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::env;
use std::fs;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn new(x: i32, y: i32) -> Self {
        Point { x, y }
    }
}

#[derive(Clone, Eq, PartialEq)]
struct State {
    player: Point,
    crates: Vec<Point>,
    path: String,
    heuristic: i32,
    hash: u64,
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other.heuristic.cmp(&self.heuristic)
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct SokobanSolver {
    width: i32,
    height: i32,
    map: Vec<Vec<char>>,
    targets: Vec<Point>,
    zobrist_table: Vec<Vec<u64>>,
    player_zobrist_table: Vec<Vec<u64>>,
}

const DIRS: [(i32, i32, char); 4] = [(0, -1, 'u'), (0, 1, 'd'), (-1, 0, 'l'), (1, 0, 'r')];

impl SokobanSolver {
    fn new(puzzle: &str) -> Self {
        let lines: Vec<&str> = puzzle.lines().collect();
        let height = lines.len() as i32;
        let width = lines.iter().map(|l| l.len()).max().unwrap_or(0) as i32;

        let mut map = vec![vec![' '; width as usize]; height as usize];
        let mut targets = Vec::new();

        for (y, line) in lines.iter().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                map[y][x] = match ch {
                    '@' | '+' => ' ',
                    '$' | '*' => ' ',
                    '.' | '+' | '*' => {
                        targets.push(Point::new(x as i32, y as i32));
                        '.'
                    }
                    _ => ch,
                };
            }
        }

        let mut solver = SokobanSolver {
            width,
            height,
            map,
            targets,
            zobrist_table: vec![vec![0; width as usize]; height as usize],
            player_zobrist_table: vec![vec![0; width as usize]; height as usize],
        };

        solver.init_zobrist_table();
        solver
    }

    fn init_zobrist_table(&mut self) {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hash, Hasher};

        let random_state = RandomState::new();

        for y in 0..self.height as usize {
            for x in 0..self.width as usize {
                let mut hasher1 = random_state.build_hasher();
                (x, y, 0).hash(&mut hasher1);
                self.zobrist_table[y][x] = hasher1.finish();

                let mut hasher2 = random_state.build_hasher();
                (x, y, 1).hash(&mut hasher2);
                self.player_zobrist_table[y][x] = hasher2.finish();
            }
        }
    }

    fn calculate_zobrist_hash(&self, player: &Point, crates: &[Point]) -> u64 {
        let mut hash = self.player_zobrist_table[player.y as usize][player.x as usize];
        for crate_pos in crates {
            hash ^= self.zobrist_table[crate_pos.y as usize][crate_pos.x as usize];
        }
        hash
    }

    fn calculate_heuristic(&self, crates: &[Point]) -> i32 {
        let mut total_dist = 0;
        let mut used_targets = vec![false; self.targets.len()];

        for crate_pos in crates {
            let mut min_dist = i32::MAX;
            let mut best_target_idx = None;

            for (i, target) in self.targets.iter().enumerate() {
                if !used_targets[i] {
                    let dist = (crate_pos.x - target.x).abs() + (crate_pos.y - target.y).abs();
                    if dist < min_dist {
                        min_dist = dist;
                        best_target_idx = Some(i);
                    }
                }
            }

            if let Some(idx) = best_target_idx {
                used_targets[idx] = true;
                total_dist += min_dist;
            }
        }

        total_dist
    }

    fn is_deadlock(&self, x: i32, y: i32) -> bool {
        if self.targets.iter().any(|t| t.x == x && t.y == y) {
            return false;
        }

        let up_blocked = self.map[y as usize - 1][x as usize] == '#';
        let down_blocked = self.map[y as usize + 1][x as usize] == '#';
        let left_blocked = self.map[y as usize][x as usize - 1] == '#';
        let right_blocked = self.map[y as usize][x as usize + 1] == '#';

        (up_blocked || down_blocked) && (left_blocked || right_blocked)
    }

    fn solve(&self, start_player: Point, start_crates: Vec<Point>) -> String {
        let start_state = State {
            player: start_player,
            crates: start_crates.clone(),
            path: String::new(),
            heuristic: self.calculate_heuristic(&start_crates),
            hash: self.calculate_zobrist_hash(&start_player, &start_crates),
        };

        let mut open_set = BinaryHeap::new();
        let mut visited: FxHashSet<u64> = FxHashSet::default();

        open_set.push(start_state);

        let target_set: FxHashSet<Point> = self.targets.iter().copied().collect();

        while let Some(current) = open_set.pop() {
            let current_crate_set: FxHashSet<Point> = current.crates.iter().copied().collect();
            if current_crate_set == target_set {
                return current.path;
            }

            if visited.contains(&current.hash) {
                continue;
            }
            visited.insert(current.hash);

            for &(dx, dy, move_char) in &DIRS {
                let nx = current.player.x + dx;
                let ny = current.player.y + dy;

                if nx < 0 || nx >= self.width || ny < 0 || ny >= self.height {
                    continue;
                }

                if self.map[ny as usize][nx as usize] == '#' {
                    continue;
                }

                let next_pos = Point::new(nx, ny);
                let has_crate = current.crates.iter().any(|c| c.x == nx && c.y == ny);

                let mut new_crates = current.crates.clone();

                if has_crate {
                    let push_x = nx + dx;
                    let push_y = ny + dy;

                    if push_x < 0 || push_x >= self.width || push_y < 0 || push_y >= self.height {
                        continue;
                    }

                    if self.map[push_y as usize][push_x as usize] == '#' {
                        continue;
                    }

                    let push_pos = Point::new(push_x, push_y);
                    if new_crates.iter().any(|c| c.x == push_x && c.y == push_y) {
                        continue;
                    }

                    if self.is_deadlock(push_x, push_y) {
                        continue;
                    }

                    if let Some(idx) = new_crates.iter().position(|c| c.x == nx && c.y == ny) {
                        new_crates[idx] = push_pos;
                    }
                }

                let new_path = format!("{}{}", current.path, move_char);
                let new_heuristic = self.calculate_heuristic(&new_crates);
                let new_hash = self.calculate_zobrist_hash(&next_pos, &new_crates);

                if !visited.contains(&new_hash) {
                    let next_state = State {
                        player: next_pos,
                        crates: new_crates,
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

fn parse_puzzle(puzzle: &str) -> (Point, Vec<Point>, SokobanSolver) {
    let lines: Vec<&str> = puzzle.lines().collect();
    let mut player = Point::new(0, 0);
    let mut crates = Vec::new();

    for (y, line) in lines.iter().enumerate() {
        for (x, ch) in line.chars().enumerate() {
            match ch {
                '@' | '+' => player = Point::new(x as i32, y as i32),
                '$' => crates.push(Point::new(x as i32, y as i32)),
                '*' => crates.push(Point::new(x as i32, y as i32)),
                _ => {}
            }
        }
    }

    let solver = SokobanSolver::new(puzzle);
    (player, crates, solver)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: rust_solver <puzzle_file>");
        std::process::exit(1);
    }

    let puzzle_path = &args[1];
    let puzzle = fs::read_to_string(puzzle_path).expect("Failed to read puzzle file");

    let (player, crates, solver) = parse_puzzle(&puzzle);
    let solution = solver.solve(player, crates);

    println!("{}", solution);
}
