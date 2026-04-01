FROM rust:1.85-bookworm AS builder
WORKDIR /build

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main(){}" > src/main.rs && cargo build --release 2>/dev/null; rm -rf src

# Build application
COPY src/ src/
COPY templates/ templates/
COPY static/ static/
COPY migrations/ migrations/
RUN touch src/main.rs && cargo build --release

# Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /build/target/release/evcc-dashboard ./
COPY --from=builder /build/templates/ ./templates/
COPY --from=builder /build/static/ ./static/
COPY --from=builder /build/migrations/ ./migrations/
COPY config.example.toml ./config.toml

EXPOSE 3000
VOLUME ["/app/data"]
ENTRYPOINT ["./evcc-dashboard"]
CMD ["--config", "/app/config.toml"]
