#!/bin/bash

set -e

echo "Building Rust Sokoban Solver..."

cd rust_solver_source
cargo build --release
cp target/release/rust_solver ../rust_solver
chmod +x ../rust_solver
echo "Done!"
