# Use cargo-chef for dependency caching
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

# Prepare the dependency recipe
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Build dependencies - this layer is cached when dependencies don't change
FROM chef AS builder

# Build dependencies (cached layer)
COPY --from=planner /app/recipe.json recipe.json
COPY --from=planner /app/dioxus-forms /app/dioxus-forms
RUN cargo chef cook --release --features server --recipe-path recipe.json

# Copy source and build application
COPY . .
RUN cargo build --release --features server

# Runtime image
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/terrier /usr/local/bin/terrier

EXPOSE 3000

CMD ["terrier"]
