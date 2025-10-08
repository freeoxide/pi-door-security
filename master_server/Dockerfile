# Build stage
FROM rust:1.83-bookworm as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY migration ./migration

# Create dummy src to cache dependencies
RUN mkdir -p src/cli && \
    echo "fn main() {}" > src/main.rs && \
    echo "fn main() {}" > src/cli/main.rs

# Build dependencies
RUN cargo build --release && \
    rm -rf src

# Copy actual source
COPY src ./src

# Build application
RUN touch src/main.rs src/cli/main.rs && \
    cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binaries from builder
COPY --from=builder /app/target/release/master_server /usr/local/bin/
COPY --from=builder /app/target/release/masterctl /usr/local/bin/

EXPOSE 8080

CMD ["master_server"]
