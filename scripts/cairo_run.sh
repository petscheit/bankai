#!/bin/bash

INPUT_FILE="input.json"
PIE_FLAG=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --pie)
            PIE_FLAG="--cairo_pie_output=pie.zip"
            shift
            ;;
        *)
            INPUT_FILE="$1"
            shift
            ;;
    esac
done

echo "Running Cairo program..."

# Start timing
start_time=$(date +%s.%N)

cairo-run --program=cairo/build/main.json \
    --program_input="$INPUT_FILE" \
    --layout=all_cairo \
    --print_info \
    $PIE_FLAG \
    # --cairo_layout_params_file=dynamic_params.json \
    # --proof_mode \

# End timing
end_time=$(date +%s.%N)

# Calculate execution time
execution_time=$(echo "$end_time - $start_time" | bc)

echo "Trace Generation time: $execution_time seconds"