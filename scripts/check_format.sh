#!/bin/bash

format_file() {
    local file="$1"
    cairo-format -c "$file"
    local status=$?
    if [ $status -eq 0 ]; then
        echo "$(date '+%Y-%m-%d %H:%M:%S') - File $file is formatted correctly"
    else
        echo "$(date '+%Y-%m-%d %H:%M:%S') - File $file is not formatted correctly"
        return $status
    fi
}

# Export functions so they're available in subshells
export -f format_file

# Find all .cairo files and format them in parallel
echo "Finding and formatting .cairo files..."
find ./cairo/src ./cairo/tests -name '*.cairo'| parallel --halt soon,fail=1 format_file {}

# Capture the exit status of parallel for .cairo files
exit_status_cairo_files=$?

# Format Scarb workspace
echo "Checking Scarb workspace..."
cd contract
scarb fmt --check
cd ..

# Capture the exit status of parallel for Scarb projects
exit_status_scarb_projects=$?

# Check cargo formatting
echo "Checking cargo formatting..."
cd client-rs
cargo fmt --check
exit_status_cargo=$?

# Run clippy
echo "Running cargo clippy..."
cargo clippy -- -D warnings
exit_status_clippy=$?

# Determine the final exit status
if [ $exit_status_cairo_files -ne 0 ] || [ $exit_status_scarb_projects -ne 0 ] || [ $exit_status_cargo -ne 0 ] || [ $exit_status_clippy -ne 0 ]; then
    final_exit_status=1
else
    final_exit_status=0
fi

# Exit with the determined status
echo "Parallel execution exited with status: $final_exit_status"
exit $final_exit_status
