#!/bin/bash

# Check if input file path is provided, otherwise use default
if [ -z "$1" ]; then
    INPUT_FILE="input.json"
else
    INPUT_FILE="$1"
fi

echo "Running Cairo program..."

# Start timing
start_time=$(date +%s.%N)

cairo-run --program=cairo/build/main.json --program_input="$INPUT_FILE" --layout=all_cairo --print_output --print_info

# End timing
end_time=$(date +%s.%N)

# Calculate execution time
execution_time=$(echo "$end_time - $start_time" | bc)

echo "Trace Generation time: $execution_time seconds"