#!/bin/bash

if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <filename.json>"
    exit 1
fi

FILENAME=$(basename $1 .json)  # Removing the .json extension, if present

# Check if the toml, program, and public/private input files exist
if [ ! -f "./sandstorm/Cargo.toml" ]; then
    echo "Error: ./sandstorm/Cargo.toml does not exist."
    exit 1
fi

if [ ! -f "cairo_programs/build/$1" ]; then
    echo "Error: cairo_programs/build/$1 does not exist."
    exit 1
fi

if [ ! -f "cairo_programs/build/inputs/public_$1" ]; then
    echo "Error: cairo_programs/build/inputs/public_$1 does not exist."
    exit 1
fi

if [ ! -f "cairo_programs/proofs/$FILENAME.proof" ]; then
    echo "Error: cairo_programs/proofs/$FILENAME.proof does not exist."
    exit 1
fi

# Create output file if it doesn't exist
mkdir -p cairo_programs/proofs
touch "cairo_programs/proofs/$FILENAME.proof"

source venv/bin/activate

cargo +nightly run --manifest-path ./sandstorm/Cargo.toml -p sandstorm-cli -r -F parallel -- \
    --program cairo_programs/build/$1 \
    --air-public-input cairo_programs/build/inputs/public_$1 \
    verify --proof cairo_programs/proofs/$FILENAME.proof

