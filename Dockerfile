# Use cargo-chef for dependency caching
FROM lukemathwalker/cargo-chef:latest-rust-bookworm AS chef
WORKDIR /app

# Prepare the dependency recipe
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Build stage
FROM chef AS builder

# Install wasm target and dioxus-cli
RUN rustup target add wasm32-unknown-unknown
RUN cargo install dioxus-cli

# Build dependencies
COPY --from=planner /app/recipe.json recipe.json
COPY --from=planner /app/dioxus-forms /app/dioxus-forms
COPY --from=planner /app/migration /app/migration
RUN cargo chef cook --release --features server --recipe-path recipe.json

# Copy source and build
COPY . .
RUN dx build --release --platform server --features server

# Runtime image
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy the build output (binary + public assets) from builder
COPY --from=builder /app/target/dx/terrier/release/web /app

EXPOSE 3000

CMD ["/app/terrier"]
