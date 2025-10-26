#!/bin/bash

if [ $# -eq 0 ]; then
    echo "Usage: ./run.sh <map_name> [mode]"
    echo ""
    echo "Examples:"
    echo "  ./run.sh twoboxes1          (defaults to bot mode)"
    echo "  ./run.sh twoboxes1 bot      (bot mode)"
    echo "  ./run.sh twoboxes1 fp       (free play mode)"
    echo ""
    echo "Available maps:"
    ls maps/*.txt | sed 's/maps\//  /' | sed 's/.txt//'
    exit 1
fi

MAP_NAME=$1
MODE=${2:-bot}

if [ ! -f "maps/${MAP_NAME}.txt" ]; then
    echo "Error: Map 'maps/${MAP_NAME}.txt' not found!"
    echo ""
    echo "Available maps:"
    ls maps/*.txt | sed 's/maps\//  /' | sed 's/.txt//'
    exit 1
fi

if [ "$MODE" != "bot" ] && [ "$MODE" != "fp" ]; then
    echo "Error: Mode must be 'bot' or 'fp'"
    exit 1
fi

echo "Running: $MAP_NAME in $MODE mode"
java -cp src main.Driver "$MAP_NAME" "$MODE"
