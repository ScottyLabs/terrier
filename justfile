#!/usr/bin/env -S just --justfile

set dotenv-load

# Database URL from .env or environment (required)
DATABASE_URL := env_var("DATABASE_URL")

# Show this help message
help:
    @just --list

# Start postgres database
db:
    docker-compose up -d postgres

# Stop all services
down:
    docker-compose down

# Run database migrations
migrate:
    sea-orm-cli migrate up --migration-dir ./migration -u {{DATABASE_URL}}

# Fresh database (drop all tables and reapply migrations)
fresh:
    sea-orm-cli migrate fresh --migration-dir ./migration -u {{DATABASE_URL}}

# Check migration status
status:
    sea-orm-cli migrate status --migration-dir ./migration -u {{DATABASE_URL}}

# Generate entities from database
generate-entities:
    sea-orm-cli generate entity -o ./src/entities --with-serde both --model-extra-derives "utoipa::ToSchema" -u {{DATABASE_URL}}

# Show logs for postgres
logs:
    docker-compose logs -f postgres

# Remove all containers, volumes, and images
clean:
    docker-compose down -v --rmi all

# Wait for postgres to be ready
wait-for-db:
    @echo "Waiting for postgres to be ready..."
    @until docker-compose exec -T postgres pg_isready -U terrier -d terrier > /dev/null 2>&1; do sleep 1; done
    @echo "Postgres is ready"

# Initialize database: start, wait, migrate, and generate entities
init: db wait-for-db migrate generate-entities

# Start development server
dev: db wait-for-db
    dx serve --platform web
