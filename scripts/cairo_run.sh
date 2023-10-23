#!/bin/bash

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <filename> <program_input>"
    exit 1
fi

source venv/bin/activate

echo "Running Cairo program..."

cairo-run --program=cairo_programs/build/$1 --program_input=$2 --layout=small --print_output --print_info

echo "Success!"
