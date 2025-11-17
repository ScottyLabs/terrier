#!/usr/bin/env bash
set -euo pipefail

cleanup() {
    echo "Cleaning up..."
    kill $(jobs -p) 2>/dev/null || true
    docker-compose stop postgres minio pgadmin nginx-dev
    exit
}

trap cleanup SIGINT SIGTERM

ROOT_DIR=$(dirname "$(dirname "$(realpath "$0")")")
cd "$ROOT_DIR"

if [ ! -f ".env" ]; then
    echo ".env file not found, running setup script..."
    ./scripts/setup.sh --dev
    echo "Run the dev script again when you've finished configuring the environment"
    exit 0
fi

if [ ! -d "terrier-client/node_modules" ]; then
    echo "terrier-client/node_modules not found, checking for bun..."

    if ! command -v bun &> /dev/null; then
        echo "Bun is not installed. Please install it from: https://bun.sh/"
        exit 0
    fi

    echo "Installing dependencies with bun..."
    cd terrier-client && bun i && cd ..
fi

# Start Docker services
docker-compose up -d postgres minio pgadmin nginx-dev

# Run migrations
cd "$ROOT_DIR/terrier-backend/migration"
cargo run

# Start backend in background and wait for it to be ready
cd "$ROOT_DIR/terrier-backend"
cargo run &

echo "Waiting for backend to be ready..."
until curl -s http://localhost:3000/api/openapi.json > /dev/null; do
    sleep 1
done

# Sync OpenAPI spec
cd "$ROOT_DIR/terrier-client"
bun run generate-types

# Start frontend in background
bun run dev &

# Wait for all background jobs
wait
