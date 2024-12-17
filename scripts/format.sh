#!/bin/bash

format_file() {
    local file="$1"
    
    echo "Formatting file: $file"
    
    # Attempt to format the file
    if cairo-format -i "$file"; then
        echo "$(date '+%Y-%m-%d %H:%M:%S') - Successfully formatted: $file"
    else
        echo "$(date '+%Y-%m-%d %H:%M:%S') - Failed to format: $file"
        return 1
    fi
}

# Export only the format_file function
export -f format_file

# Find all .cairo files under src/ and tests/ directories and format them in parallel
echo "Formatting .cairo files..."
find ./cairo/src ./cairo/tests -name '*.cairo' | parallel --halt soon,fail=1 format_file {}

# Capture the exit status of parallel for .cairo files
exit_status_cairo_files=$?

# Format Scarb workspace
echo "Formatting Scarb workspace..."
cd contract
scarb fmt
cd ..

# Capture the exit status of parallel for Scarb projects
exit_status_scarb_projects=$?

# Determine the final exit status
if [ $exit_status_cairo_files -ne 0 ] || [ $exit_status_scarb_projects -ne 0 ]; then
    final_exit_status=1
else
    final_exit_status=0
fi

# Exit with the determined status
echo "Parallel execution exited with status: $final_exit_status"
exit $final_exit_status
