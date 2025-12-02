# Use cargo-chef for dependency caching
FROM lukemathwalker/cargo-chef:latest-rust-bookworm AS chef
WORKDIR /app

# Prepare the dependency recipe
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Build dependencies - this layer is cached when dependencies don't change
FROM chef AS builder

# Install wasm target and dioxus-cli for fullstack build
RUN rustup target add wasm32-unknown-unknown
RUN cargo install dioxus-cli

# Build dependencies (cached layer)
COPY --from=planner /app/recipe.json recipe.json
COPY --from=planner /app/dioxus-forms /app/dioxus-forms
COPY --from=planner /app/migration /app/migration
RUN cargo chef cook --release --features server --recipe-path recipe.json

# Copy source and build application with dx
COPY . .
RUN dx build --release --platform fullstack --features server

# Runtime image
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy the dist directory (binary + public assets) from builder
COPY --from=builder /app/dist /app

EXPOSE 3000

CMD ["/app/terrier"]
