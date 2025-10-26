#!/usr/bin/env bash
set -e  # Exit immediately on error

# --- Helper functions --------------------------------------------------------
show_usage() {
    echo "Usage: ./run.sh <map_name> [mode]"
    echo ""
    echo "Examples:"
    echo "  ./run.sh twoboxes1          (defaults to bot mode)"
    echo "  ./run.sh twoboxes1 bot      (bot mode)"
    echo "  ./run.sh twoboxes1 fp       (free play mode)"
    echo ""
    echo "Available maps:"
    list_maps
}

list_maps() {
    # Compatible sed pattern for both macOS (BSD) and Linux (GNU)
    find maps -maxdepth 1 -type f -name '*.txt' \
        | sed -E 's|^maps/||' \
        | sed -E 's|\.txt$||' \
        | sort \
        | awk '{print "  " $0}'
}

# --- Argument check ----------------------------------------------------------
if [ $# -eq 0 ]; then
    show_usage
    exit 1
fi

MAP_NAME=$1
MODE=${2:-bot}

# --- Validate map file -------------------------------------------------------
if [ ! -f "maps/${MAP_NAME}.txt" ]; then
    echo "Error: Map 'maps/${MAP_NAME}.txt' not found!"
    echo ""
    echo "Available maps:"
    list_maps
    exit 1
fi

# --- Validate mode -----------------------------------------------------------
if [ "$MODE" != "bot" ] && [ "$MODE" != "fp" ]; then
    echo "Error: Mode must be 'bot' or 'fp'"
    exit 1
fi

# --- Compile if needed -------------------------------------------------------
if [ ! -d "bin" ] || [ "$(find src -type f -newer bin -print -quit)" ]; then
    echo "Compiling Java sources..."
    mkdir -p bin
    javac -d bin $(find src -name "*.java")
fi

# --- Run ---------------------------------------------------------------------
echo "Running: $MAP_NAME in $MODE mode"
java -cp bin main.Driver "$MAP_NAME" "$MODE"

