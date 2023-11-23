#!/bin/bash

if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <filename>"
    exit 1
fi

# Check if $1 exists
if [ ! -f "cairo_programs/$1" ]; then
    echo "Error: cairo_programs/$1 does not exist."
    exit 1
fi

# Creates symlink for Garaga dependencies
ln -s "$(pwd)/cairo_programs/deps/garaga/src" cairo_programs/src
source venv/bin/activate

FILENAME=$(basename $1 .cairo)  # Removing the .cairo extension, if present

echo "Compiling Cairo programs..."
cd cairo_programs

# Ensure the build directory exists
mkdir -p build
touch "build/${FILENAME}.json"

cairo-compile $1 --proof_mode --output build/${FILENAME}.json

# Ensure the output file exists or create it

# Removes symlink
rm "src"

echo "Success!"
