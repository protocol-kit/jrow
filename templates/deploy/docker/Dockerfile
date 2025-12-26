# Multi-stage Dockerfile for JROW examples
FROM rust:1.75 as builder

WORKDIR /usr/src/jrow

# Copy workspace files (excluding templates to avoid recursive copy issues)
COPY Cargo.toml Cargo.lock ./
COPY jrow-core ./jrow-core
COPY jrow-server ./jrow-server
COPY jrow-client ./jrow-client
COPY jrow-macros ./jrow-macros
COPY src ./src
COPY examples ./examples

# Build all examples in release mode
RUN cargo build --release --examples

# Runtime stage
FROM debian:bookworm-slim

# Install necessary runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the built examples
COPY --from=builder /usr/src/jrow/target/release/examples/* /usr/local/bin/

# Expose common ports
EXPOSE 8080 9000 9001 9002 9003 9004

# Default to running the simple server example
CMD ["simple_server"]

