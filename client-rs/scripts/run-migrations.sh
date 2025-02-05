#!/bin/bash
set -e

# Disable command printing for the entire script
set +x

# Add psql version check and store result
PSQL_VERSION=$(psql --version)
echo "Using PostgreSQL client version: $PSQL_VERSION"

MIGRATIONS_DIR="usr/src/app/client-rs/migrations"
DB_NAME="${POSTGRESQL_DB_NAME}"

# Check required environment variables
if [ -z "$POSTGRESQL_HOST" ] || [ -z "$POSTGRESQL_PORT" ] || [ -z "$POSTGRESQL_USER" ] || [ -z "$POSTGRESQL_PASSWORD" ]; then
    echo "Error: Required PostgreSQL environment variables are not set"
    echo "Please ensure POSTGRESQL_HOST, POSTGRESQL_PORT, POSTGRESQL_USER, and POSTGRESQL_PASSWORD are set"
    exit 1
fi

# Export PostgreSQL connection environment variables (without printing them)
export PGHOST="${POSTGRESQL_HOST}"
export PGPORT="${POSTGRESQL_PORT}"
export PGUSER="${POSTGRESQL_USER}"
export PGPASSWORD="${POSTGRESQL_PASSWORD}"
export PGDATABASE="${POSTGRESQL_DB_NAME}"

# Check if DB_NAME is set
if [ -z "$DB_NAME" ]; then
    echo "Error: POSTGRESQL_DB_NAME environment variable is not set"
    exit 1
fi

# Check if migrations directory exists
if [ ! -d "$MIGRATIONS_DIR" ]; then
    echo "Error: Migrations directory $MIGRATIONS_DIR does not exist"
    exit 1
fi

# Function to execute a migration
run_migration() {
    local file="$1"
    local version
    version=$(basename "$file" | cut -d'_' -f1)
    local name
    name=$(basename "$file" | cut -d'_' -f2- | sed 's/\.sql$//')
    
    echo "Checking migration ${version}: ${name}"
    
    # Modified version check to handle leading zeros and ensure proper numeric comparison
    if ! psql -v ON_ERROR_STOP=1 -d "$DB_NAME" -t -c "SELECT 1 FROM schema_migrations WHERE version = '$version'::bigint" | grep -q 1; then
        echo "Applying migration ${version}: ${name}"
        
        # Run the migration inside a transaction with error checking
        if ! psql -v ON_ERROR_STOP=1 -d "$DB_NAME" <<EOF
        BEGIN;
        \i $file
        INSERT INTO schema_migrations (version, name) VALUES ('$version'::bigint, '$name');
        COMMIT;
EOF
        then
            echo "Error: Migration ${version} failed"
            exit 1
        fi
        
        echo "Migration ${version} completed successfully"
    else
        echo "Migration ${version} already applied"
    fi
}

# Create migrations table if it doesn't exist
if ! psql -v ON_ERROR_STOP=1 -d "$DB_NAME" -f "${MIGRATIONS_DIR}/000_create_migrations_table.sql"; then
    echo "Error: Failed to create migrations table"
    exit 1
fi

# Check if any .sql files exist
if ! ls ${MIGRATIONS_DIR}/*.sql >/dev/null 2>&1; then
    echo "No migration files found in $MIGRATIONS_DIR"
    exit 1
fi

# Run all migrations in order (modified to use more reliable sorting)
for f in $(find ${MIGRATIONS_DIR} -name "*.sql" | sort -V); do
    # Skip the migrations table creation
    if [[ $(basename "$f") == "000_create_migrations_table.sql" ]]; then
        continue
    fi
    run_migration "$f"
done

echo "All migrations completed"
