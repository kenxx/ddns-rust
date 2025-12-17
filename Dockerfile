# Build stage
FROM rust:1.83-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy main to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src

# Copy actual source
COPY src ./src

# Build the real application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM alpine:3.21

RUN apk add --no-cache ca-certificates tzdata

COPY --from=builder /app/target/release/ddns-rust /usr/local/bin/ddns-rust

# Create config directory
RUN mkdir -p /etc/ddns-rust

WORKDIR /etc/ddns-rust

ENTRYPOINT ["ddns-rust"]
CMD ["--config", "/etc/ddns-rust/config.toml"]

