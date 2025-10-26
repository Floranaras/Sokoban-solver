# Sokoban Solver

A Sokoban puzzle solver with Java GUI and Rust solver backend. The system uses Greedy Best-First Search with Zobrist hashing, deadlock detection, and optimized state management.

## Demo

[Video demonstration](docs/demo.mov)

## System Requirements

- Java Development Kit 11+
- Rust 1.70+ (optional, for Rust solver)
- Unix-like environment (macOS, Linux) or Windows with bash

## Installation

### Clone Repository
```bash
git clone <repository-url>
cd Sokoban-solver
```

### Build Java Components
```bash
javac -d src src/main/*.java src/gui/*.java src/reader/*.java src/solver/*.java
```

### Build Rust Solver (Optional)
```bash
cd rust_solver_source
cargo build --release
cd ..
cp rust_solver_source/target/release/rust_solver ./
chmod +x rust_solver
```

## Usage

### Running the Application

```bash
./run.sh <map_name> <mode>
```

**Parameters:**
- `map_name`: Name of map file in `maps/` directory (without `.txt` extension)
- `mode`: Either `fp` (free play) or `bot` (automated solver)

**Examples:**
```bash
./run.sh twoboxes1 fp    # Free play mode
./run.sh twoboxes1 bot   # Bot solver mode
```

### Controls

**Free Play Mode:**
- Arrow keys: Move player

**Bot Mode:**
- Space: Start solver

### Map Format

Place `.txt` files in `maps/` directory using standard Sokoban notation:

| Character | Meaning |
|-----------|---------|
| `#` | Wall |
| `@` | Player |
| `$` | Box |
| `.` | Goal |
| `*` | Box on goal |
| `+` | Player on goal |
| ` ` | Empty space |

Example map:
```
#######
#     #
# .$. #
#  $  #
#  @  #
#     #
#######
```

## Project Structure

```
Sokoban-solver/
├── src/
│   ├── main/
│   │   └── Driver.java              # Application entry point
│   ├── gui/
│   │   ├── GameFrame.java           # Main window container
│   │   ├── GamePanel.java           # Rendering and game logic
│   │   └── BotThread.java           # Solver thread wrapper
│   ├── reader/
│   │   ├── FileReader.java          # Map file parser
│   │   └── MapData.java             # Map data container
│   ├── solver/
│   │   └── SokoBot.java             # Java A* solver implementation
│   └── graphics/                    # Sprite assets (32x32 PNG)
│       ├── brick.png
│       ├── goal.png
│       ├── crate.png
│       ├── crategoal.png
│       └── robot.png
├── rust_solver_source/
│   ├── src/
│   │   └── main.rs                  # Rust solver implementation
│   └── Cargo.toml                   # Rust dependencies
├── maps/                            # Puzzle files (.txt)
├── docs/                            # Documentation and media
├── run.sh                           # Execution script
└── README.md
```

## Architecture

### Java Components

**Driver.java**
- Parses command-line arguments
- Initializes FileReader and GameFrame
- Delegates to appropriate game mode

**GamePanel.java**
- Handles rendering (Swing)
- Manages player movement in free play mode
- Coordinates bot solver thread
- Displays game state and statistics

**SokoBot.java**
- Implements Greedy Best-First Search algorithm
- Zobrist hashing for state representation
- Deadlock detection (static, dynamic, frozen, room-based)
- Custom hash set for visited states

**FileReader.java**
- Reads map files from `maps/` directory
- Parses Sokoban notation into 2D char array
- Returns MapData object

### Rust Solver

**main.rs**
- Standalone executable solver
- Reads puzzle from file path argument
- Outputs solution string (e.g., "udlrr")
- Optimized for performance

## Algorithm Details

### Greedy Best-First Search Implementation

**State Representation:**
```
State {
    player_position: (row, col)
    box_positions: [(row, col), ...]
    zobrist_hash: u64
    path: String
    heuristic: i32
}
```

**Heuristic Function:**
```
h(state) = sum of min Manhattan distances from each box to nearest unassigned goal
         + penalty for frozen boxes (30 points)
         + penalty for room deadlocks (infinite/pruned)

Note: This is Greedy Best-First Search, not A*. 
States are ordered by h(state) only, without path cost g(state).
Path length is used as tiebreaker but not part of evaluation function.
```

**Zobrist Hashing:**
- Pre-compute random 64-bit values for each (position, entity_type) pair
- State hash = XOR of all entity position hashes
- Incremental updates: `new_hash = old_hash XOR old_pos XOR new_pos`

### Deadlock Detection

**Static Deadlocks:**
- Reverse BFS from goal positions
- Mark squares that cannot reach any goal
- Pre-computed during initialization

**Frozen Boxes:**
- Check if box blocked vertically and horizontally
- Use 3x3 neighborhood with rotation patterns
- Skip if box already on goal

**Room Deadlocks:**
- Partition map into connected regions (rooms)
- Count goals per room
- Prune if any room has more boxes than goals

### Optimization Techniques

**Java:**
- Custom LongHashSet with open addressing
- Pre-computed Zobrist table with fixed seed
- Reusable temporary buffers
- Flat array representation for box positions

**Rust:**
- Incremental Zobrist hashing (O(n) to O(1))
- Flat memory layout (Vec<u8> instead of Vec<Vec<T>>)
- Bitsets for goal/deadlock lookups
- SmallVec for stack allocation
- Transposition table for heuristic caching
- Unsafe array access in hot paths

## Performance Characteristics

**Time Complexity:**
- Best case: O(d) where d is solution depth
- Average case: O(b^d) where b is branching factor
- Worst case: Unbounded (greedy search not guaranteed to find optimal path)
- Note: Does not guarantee shortest solution unlike A*

**Space Complexity:**
- O(|visited states|) for hash set
- O(|open set|) for priority queue
- Dominated by visited state storage

**Measured Performance (Apple Silicon M-series):**

| Puzzle Type | Boxes | Java Time | Rust Time |
|-------------|-------|-----------|-----------|
| Simple | 2-3 | 0.5s | 0.05s |
| Medium | 4-6 | 5s | 0.3s |
| Complex | 7-10 | 30s | 0.8s |
| Very Hard | 10+ | Timeout (50s) | 3s |

## Dependencies

### Java
Standard Library only (Java 11+)

### Rust
```toml
[dependencies]
rustc-hash = "2.0"      # FxHashSet implementation
smallvec = "1.13"       # Stack-allocated vectors
arrayvec = "0.7"        # Fixed-capacity arrays
```

## Development

### Commit Convention

| Type | Purpose |
|------|---------|
| `feat` | Add new feature (functions, logic) |
| `fix` | Fix bug (incorrect output, logic errors) |
| `refactor` | Improve code without changing behavior |
| `perf` | Optimize performance (faster loops, better memory usage) |
| `style` | Formatting changes (indentation, comments) |
| `test` | Add or update test cases |
| `build` | Modify Cargo.toml or compilation setup |
| `docs` | Update README, specs, or comments |
| `chore` | Non-code maintenance (renaming files, updating .gitignore) |

### Building for Development

**Java:**
```bash
# Compile all sources
javac -d src src/**/*.java

# Run with debugging
java -cp src main.Driver <map> <mode>
```

**Rust:**
```bash
# Debug build (faster compilation)
cd rust_solver_source
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Check without building
cargo check
```

### Adding New Maps

1. Create `.txt` file in `maps/` directory
2. Use standard Sokoban notation
3. Ensure exactly one player (`@` or `+`)
4. Equal number of boxes (`$` or `*`) and goals (`.`, `+`, or `*`)
5. Test with `./run.sh <map_name> bot`

## Known Limitations

- Maximum map size: 100x100 (hardcoded buffer in FileReader)
- Solution timeout: 50 seconds
- No support for multiple players
- GUI performance degrades with >100 moves/second playback

