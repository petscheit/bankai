#!/bin/bash
# Enable debug output
set -x

# Check for required tools
echo "[$(date)] Checking required dependencies..."

# if ! command -v cairo-run >/dev/null; then
#     echo "Error: cairo-run is not installed. Please ensure Cairo is properly installed."
#     exit 1
# fi

echo "Cairo installation:"
cairo-run --version || echo "Warning: cairo-run found but version check failed"
echo "----------------------------------------"

# Run migrations first
echo "[$(date)] Starting database migrations..."
/usr/src/app/client-rs/scripts/run-migrations.sh
MIGRATION_STATUS=$?
echo "[$(date)] Migration completed with status: $MIGRATION_STATUS"

# Start daemon
echo "Starting the daemon..."
exec /usr/local/bin/daemon
