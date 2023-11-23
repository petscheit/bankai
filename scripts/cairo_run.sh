#!/bin/bash

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <filename> <program_input>"
    exit 1
fi

# Check if the program and program_input files exist
if [ ! -f "cairo_programs/build/$1" ]; then
    echo "Error: cairo_programs/build/$1 does not exist."
    exit 1
fi

if [ ! -f "$2" ]; then
    echo "Error: $2 does not exist."
    exit 1
fi

mkdir -p cairo_programs/build/inputs
mkdir -p cairo_programs/build/trace
mkdir -p cairo_programs/build/memory

# Create empty files for public input, private input, trace, and memory if they don't exist
touch "cairo_programs/build/inputs/private_$1" \
      "cairo_programs/build/inputs/public_$1" \
      "cairo_programs/build/trace/$1" \
      "cairo_programs/build/memory/$1"

source venv/bin/activate

echo "Running Cairo program..."

cairo-run --program=cairo_programs/build/$1 \
    --program_input=$2 \
    --air_private_input cairo_programs/build/inputs/private_$1 \
    --air_public_input cairo_programs/build/inputs/public_$1 \
    --trace_file=cairo_programs/build/trace/$1 \
    --memory_file=cairo_programs/build/memory/$1 \
    --print_output \
    --proof_mode \
    --min_steps=128 \
    --layout=starknet \

echo "Success!"
