# Multi-stage build: compile the static binary, then ship it on distroless.
FROM rust:1.85-slim AS build
WORKDIR /app

# Cache dependencies first.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && echo "" > src/lib.rs \
    && cargo build --release 2>/dev/null || true
RUN rm -rf src

# Build the real binary.
COPY . .
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12
COPY --from=build /app/target/release/toran /toran
COPY --from=build /app/policies /policies

ENV TORAN_POLICY_DIR=/policies \
    TORAN_DATABASE_PATH=/data/toran.db \
    TORAN_API_BIND=0.0.0.0:7878

EXPOSE 7878
VOLUME ["/data"]
ENTRYPOINT ["/toran", "start"]
