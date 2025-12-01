#!/usr/bin/env -S just --justfile

set dotenv-load

# Environment variables
DATABASE_URL := env_var("DATABASE_URL")
MINIO_ROOT_USER := env_var("MINIO_ROOT_USER")
MINIO_ROOT_PASSWORD := env_var("MINIO_ROOT_PASSWORD")
MINIO_BUCKET := env_var("MINIO_BUCKET")

# Show this help message
help:
    @just --list

# Start postgres database
db:
    docker-compose up -d postgres

# Start valkey
valkey:
    docker-compose up -d valkey

# Start minio
minio:
    docker-compose up -d minio

# Wait for postgres to be ready
wait-for-db:
    @echo "Waiting for postgres to be ready..."
    @until docker-compose exec -T postgres pg_isready -U terrier -d terrier > /dev/null 2>&1; do sleep 1; done
    @echo "Postgres is ready"

# Wait for valkey to be ready
wait-for-valkey:
    @echo "Waiting for valkey to be ready..."
    @until docker-compose exec -T valkey valkey-cli ping > /dev/null 2>&1; do sleep 1; done
    @echo "Valkey is ready"

# Wait for minio to be ready
wait-for-minio:
    @echo "Waiting for minio to be ready..."
    @until curl -s http://localhost:9000/minio/health/live > /dev/null 2>&1; do sleep 1; done
    @echo "MinIO is ready"

# Stop all services
down:
    docker-compose down

# Create new migration
new-migration NAME:
    sea-orm-cli migrate generate {{NAME}} --migration-dir ./migration

# Run database migrations
migrate: db wait-for-db
    sea-orm-cli migrate up --migration-dir ./migration -u {{DATABASE_URL}}

# Fresh database (drop all tables and reapply migrations)
fresh: db wait-for-db minio wait-for-minio
    @echo "Clearing MinIO bucket..."
    @docker-compose exec -T minio sh -c "mc alias set local http://localhost:9000 {{MINIO_ROOT_USER}} {{MINIO_ROOT_PASSWORD}} && mc rm --recursive --force local/{{MINIO_BUCKET}}/" || true
    sea-orm-cli migrate fresh --migration-dir ./migration -u {{DATABASE_URL}}

# Check migration status
status: db wait-for-db
    sea-orm-cli migrate status --migration-dir ./migration -u {{DATABASE_URL}}

# Generate entities from database
generate-entities: migrate
    sea-orm-cli generate entity -o ./src/entities --with-serde both --model-extra-derives "utoipa::ToSchema" -u {{DATABASE_URL}}

# Remove all containers, volumes, and images
clean:
    docker-compose down -v --rmi all

# Initialize database: start, wait, migrate, and generate entities
init: db wait-for-db migrate generate-entities

# Start development server
dev: db wait-for-db valkey wait-for-valkey minio wait-for-minio
    dx serve --platform web
