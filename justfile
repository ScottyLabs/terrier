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

# Create new migration
new-migration NAME:
    sea-orm-cli migrate generate {{NAME}} --migration-dir ./migration

# Run database migrations
migrate:
    devenv up postgres
    sea-orm-cli migrate up --migration-dir ./migration -u {{DATABASE_URL}}

# Fresh database (drop all tables and reapply migrations)
fresh:
    devenv up postgres minio
    @echo "Clearing MinIO bucket..."
    mc alias set local http://localhost:9000 {{MINIO_ROOT_USER}} {{MINIO_ROOT_PASSWORD}} 2>/dev/null || true
    mc rm --recursive --force local/{{MINIO_BUCKET}}/ 2>/dev/null || true
    sea-orm-cli migrate fresh --migration-dir ./migration -u {{DATABASE_URL}}

# Check migration status
status:
    devenv up postgres
    sea-orm-cli migrate status --migration-dir ./migration -u {{DATABASE_URL}}

# Generate entities from database
generate-entities:
    devenv up postgres
    sea-orm-cli generate entity -o ./src/entities --with-serde both --model-extra-derives "utoipa::ToSchema" -u {{DATABASE_URL}}

# Start database, run migrations, and generate entities
init: migrate generate-entities

# Start development server
dev:
    process-compose up -D postgres redis minio
    dx serve --platform web

# Start iOS development server
ios:
    process-compose up -D postgres redis minio
    xcrun simctl boot "iPhone 17 Pro" 2>/dev/null || true
    dx serve --platform ios

# Start Android development server
android:
    process-compose up -D postgres redis minio
    dx serve --platform android

# Display service logs
attach:
    process-compose attach

# Stop development server
down:
    process-compose down

# Clean devenv state (removes all service data)
clean: down
    rm -rf .devenv/state
