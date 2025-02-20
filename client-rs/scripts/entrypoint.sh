#!/usr/bin/env bash
set -e

su postgres -c "/usr/lib/postgresql/14/bin/pg_ctl -D /var/lib/postgresql/data -l logfile start"

sleep 5

su postgres -c "psql -c \"CREATE USER postgres WITH SUPERUSER PASSWORD 'postgres';\"" || true
su postgres -c "psql -c \"CREATE DATABASE bankai_sepolia;\"" || true

# We need to do migration here, create initial DB structure form DB file

echo "PostgreSQL is running. Starting the daemon..."

exec "$@"
