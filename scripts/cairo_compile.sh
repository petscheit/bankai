#!/bin/bash

if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <filename>"
    exit 1
fi

#creates symlink for Garaga dependencies
ln -s "$(pwd)/cairo_programs/deps/garaga/src" cairo_programs/src
source venv/bin/activate

FILENAME=$(basename $1 .cairo)  # Removing the .cairo extension, if present

echo "Compiling Cairo programs..."
cd cairo_programs
cairo-compile $1 --proof_mode --output build/${FILENAME}.json

# removes symlink
rm "src"

echo "Success!"