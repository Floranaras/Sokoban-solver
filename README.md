# Sokoban Rust Solver Setup

This setup replaces the Java Sokoban solver with a high-performance Rust implementation.

## File Structure

```
your-project/
├── src/
│   ├── main/
│   │   └── Driver.java
│   ├── gui/
│   │   ├── GameFrame.java
│   │   ├── GamePanel.java
│   │   └── BotThread.java
│   ├── reader/
│   │   ├── FileReader.java
│   │   └── MapData.java
│   ├── solver/
│   │   └── SokoBot.java          ← Replace with SokoBot_rust_caller.java
│   └── graphics/
│       └── (sprite files)
│
├── maps/
│   └── (level files)
│
├── rust_solver/                   ← New Rust project
│   ├── Cargo.toml
│   ├── src/
│   │   └── main.rs
│   └── target/
│       └── release/
│           └── rust_solver        ← Compiled executable
│
└── rust_solver                    ← Copy of executable (or symlink)
```

## Setup Instructions

### Step 1: Build the Rust Solver

```bash
# Navigate to the rust_solver directory
cd rust_solver

# Build in release mode (optimized)
cargo build --release

# The executable will be at: target/release/rust_solver
```

Or use the provided build script:
```bash
./build_rust.sh
```

### Step 2: Copy the Rust Executable

Copy the compiled Rust executable to your Java project root:

```bash
# From the rust_solver directory
cp target/release/rust_solver /path/to/your/java/project/

# Make sure it's executable (Linux/Mac)
chmod +x /path/to/your/java/project/rust_solver
```

### Step 3: Replace SokoBot.java

Replace your current `SokoBot.java` with the new version that calls Rust:

```bash
cp SokoBot_rust_caller.java /path/to/your/java/project/src/solver/SokoBot.java
```

### Step 4: Adjust the Executable Path (if needed)

Open `SokoBot.java` and check line 64:

```java
command.add("./rust_solver");  // Default: executable in project root
```

Adjust if your executable is elsewhere:
- Windows: `command.add("rust_solver.exe");`
- Full path: `command.add("/full/path/to/rust_solver");`

### Step 5: Test It

Run your Java application:

```bash
# Compile Java (adjust paths as needed)
javac -d bin src/**/*.java

# Run with a test map
java -cp bin main.Driver level1 bot
```

## How It Works

1. **Java calls Rust**: When `solveSokobanPuzzle()` is called, Java:
   - Creates a temporary file with the puzzle
   - Spawns the Rust executable as a subprocess
   - Passes the puzzle file path as an argument
   - Reads the solution from stdout

2. **Rust solves the puzzle**: The Rust solver:
   - Reads the puzzle file
   - Parses the Sokoban format
   - Runs a Greedy Best-First Search with:
     - Zobrist hashing for fast state comparison
     - Manhattan distance heuristic
     - Deadlock detection
   - Outputs the solution string (e.g., "udlrr")

3. **Java animates the solution**: The solution string is returned to Java and animated in the GUI

## Performance Benefits

- **Speed**: Rust is typically 5-10x faster than Java for this type of algorithm
- **Memory**: More efficient memory usage with Rust's ownership system
- **Optimization**: Compiled with aggressive optimizations (`--release`)

## Troubleshooting

### "rust_solver not found"
- Make sure the executable is in the correct location
- Check that it's executable: `chmod +x rust_solver`
- Verify the path in `SokoBot.java`

### "Permission denied"
```bash
chmod +x rust_solver
```

### Rust not installed
```bash
# Install Rust from https://rustup.rs/
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Solution times out
- Increase timeout in `SokoBot.java` line 82
- Current timeout: 30 seconds

## Rebuilding After Changes

If you modify the Rust code:

```bash
cd rust_solver
cargo build --release
cp target/release/rust_solver ../
```

## Solution Format

The Rust solver outputs a string of moves:
- `u` = up
- `d` = down
- `l` = left
- `r` = right

Example: `"uurrdllddrr"`
