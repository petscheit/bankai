#!/bin/bash
# Enable debug output
set -x

# Run migrations first
echo "[$(date)] Starting database migrations..."
/usr/src/app/client-rs/scripts/run-migrations.sh
MIGRATION_STATUS=$?
echo "[$(date)] Migration completed with status: $MIGRATION_STATUS"

# Start daemon
echo "Starting the daemon..."
exec /usr/local/bin/daemon
