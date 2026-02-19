#!/usr/bin/env -S just --justfile

set dotenv-load

# Show this help message
help:
    @just --list

# Start infrastructure services (postgres, redis, minio)
services:
    devenv up -d

# Start backend server
server: services
    dx serve --hotpatch --package terrier-server

# Generate OpenAPI specs for app and docs
generate-api:
    cd app && bun run generate-api
    cd sites/docs && bun run generate-api

# Start app dev server
app:
    cd app && bun dev

# Start docs dev server
docs:
    cd sites/docs && bun dev

# Attach to process-compose interface
attach:
    process-compose attach

# Stop infrastructure services
down:
    process-compose down

# Clean devenv state (removes all service data)
clean: down
    rm -rf .devenv/state
