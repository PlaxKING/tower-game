#!/bin/sh
set -e

# Create the tower_game database for the Bevy server
# (Nakama uses the default 'nakama' DB, Bevy server uses 'tower_game')
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
    SELECT 'CREATE DATABASE tower_game'
    WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'tower_game')\gexec
EOSQL

echo "tower_game database ready"
