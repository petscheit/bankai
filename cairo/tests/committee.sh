#!/bin/bash

# Define the log file path
LOG_FILE="full_flow_committee.log"

# Empty the log file at the beginning of each run
> "$LOG_FILE"

# Export the log file path so it is available in subshells
export LOG_FILE

run_tests() {
    local input_file="$1"
    local temp_output=$(mktemp)

    # Attempt to run the compiled program and capture output
    local start_time=$(date +%s)
    cairo-run --program="cairo/build/committee_update.json" --program_input="$input_file" --layout=all_cairo >> "$temp_output" 2>&1
    local status=$?
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))

    if [ $status -eq 0 ]; then
        echo "$(date '+%Y-%m-%d %H:%M:%S') - Successful test for $input_file: Duration ${duration} seconds"
    else
        echo "$(date '+%Y-%m-%d %H:%M:%S') - Failed test for $input_file"
        cat "$temp_output" >> "$LOG_FILE"
    fi

    return $status
}

# Export the functions so they're available in subshells
export -f run_tests

# Ensure the Cairo file is compiled before running parallel tests
echo "Compiling Bankai Cairo file..."
make build-committee

echo "Starting tests..."
# Use find to locate all input.json files in hdp-test/fixtures directory and run them in parallel
find ./cairo/tests/fixtures -name "committee_*.json" | parallel --halt soon,fail=1 run_tests {}

# Capture the exit status of parallel
exit_status=$?

# Print logs if tests failed
if [ $exit_status -ne 0 ]; then
    echo "Parallel execution exited with status: $exit_status. Some tests failed."
    echo "Printing logs for debugging:"
    cat "$LOG_FILE"
else
    echo "Parallel execution exited successfully. All tests passed."
fi

exit $exit_status