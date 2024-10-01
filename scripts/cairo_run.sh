#!/bin/bash

# Check if input file path is provided, otherwise use default
if [ -z "$1" ]; then
    INPUT_FILE="input.json"
else
    INPUT_FILE="$1"
fi

echo "Running Cairo program..."

cairo-run --program=cairo/build/main.json --program_input="$INPUT_FILE" --layout=all_cairo --print_output --print_info

if [ $? -eq 0 ]; then
    echo "Cairo program execution successful!"
fi

