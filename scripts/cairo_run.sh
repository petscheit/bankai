#!/bin/bash

INPUT_FILE="epoch_input.json"
PIE_FLAG=""
PROGRAM="epoch_update"  # Default program

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
         --pie)
            # Set PIE name based on program type
            if [ "$PROGRAM" = "committee_update" ]; then
                PIE_FLAG="--cairo_pie_output=committee_pie.zip"
            else
                PIE_FLAG="--cairo_pie_output=epoch_pie.zip"
            fi
            shift
            ;;
        --committee)
            PROGRAM="committee_update"
            INPUT_FILE="committee_input.json"  # Default committee input file
            shift
            ;;
        *)
            INPUT_FILE="$1"
            shift
            ;;
    esac
done

echo "Running Cairo program: $PROGRAM..."

# Start timing
start_time=$(date +%s.%N)

cairo-run --program=cairo/build/${PROGRAM}.json \
    --program_input="$INPUT_FILE" \
    --layout=all_cairo \
    --print_info \
    $PIE_FLAG

# End timing
end_time=$(date +%s.%N)

# Calculate execution time
execution_time=$(echo "$end_time - $start_time" | bc)

echo "Trace Generation time: $execution_time seconds"