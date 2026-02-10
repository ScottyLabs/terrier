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

# Start frontend dev server
web:
    cd web && bun dev

# Attach to process-compose interface
attach:
    process-compose attach

# Stop infrastructure services
down:
    process-compose down

# Clean devenv state (removes all service data)
clean: down
    rm -rf .devenv/state
