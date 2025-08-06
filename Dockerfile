# Cerberus v5.0 Production Dockerfile
FROM rust:1.83-slim as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml ./

# Copy source code
COPY src ./src
COPY config ./config

# Build for release
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false cerberus

# Create app directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/cerberus /usr/local/bin/cerberus

# Copy configuration
COPY --from=builder /app/config ./config

# Create data directory
RUN mkdir -p /app/data && chown cerberus:cerberus /app/data

# Switch to app user
USER cerberus

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Expose ports
EXPOSE 8080 9090

# Set environment
ENV RUST_LOG=cerberus=info
ENV CERBERUS_ENVIRONMENT=production

# Run the application
CMD ["cerberus"]
