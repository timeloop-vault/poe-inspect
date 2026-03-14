# rqe-server — multi-stage Docker build
#
# Build: docker build -t rqe-server -f build/package/Dockerfile .
# Run:   docker run -p 8080:8080 -v rqe-data:/data -e RQE_API_KEY=secret rqe-server
#
# Must be built from the repo root (needs workspace Cargo.toml + crates/poe-rqe).

# ── Build stage ───────────────────────────────────────────────────────────

FROM rust:1.85-bookworm AS builder

WORKDIR /src

# Copy workspace root and dependency crates first (layer caching)
COPY Cargo.toml Cargo.lock ./
COPY crates/poe-rqe crates/poe-rqe

# Copy rqe-server source
COPY rqe-server rqe-server

# Create stub crates so cargo resolves the workspace without copying everything.
# These are workspace members that rqe-server doesn't depend on.
RUN mkdir -p crates/poe-dat/src && echo "" > crates/poe-dat/src/lib.rs \
    && mkdir -p crates/poe-data/src && echo "" > crates/poe-data/src/lib.rs \
    && mkdir -p crates/poe-item/src && echo "" > crates/poe-item/src/lib.rs \
    && mkdir -p crates/poe-eval/src && echo "" > crates/poe-eval/src/lib.rs \
    && mkdir -p crates/poe-trade/src && echo "" > crates/poe-trade/src/lib.rs \
    && mkdir -p crates/poe-rqe-client/src && echo "" > crates/poe-rqe-client/src/lib.rs \
    && mkdir -p crates/rqe-cli/src && echo "fn main(){}" > crates/rqe-cli/src/main.rs

# Copy stub Cargo.toml files for workspace members
COPY crates/poe-dat/Cargo.toml crates/poe-dat/
COPY crates/poe-data/Cargo.toml crates/poe-data/
COPY crates/poe-item/Cargo.toml crates/poe-item/
COPY crates/poe-eval/Cargo.toml crates/poe-eval/
COPY crates/poe-trade/Cargo.toml crates/poe-trade/
COPY crates/poe-rqe-client/Cargo.toml crates/poe-rqe-client/
COPY crates/rqe-cli/Cargo.toml crates/rqe-cli/

RUN cargo build --release -p rqe-server

# ── Runtime stage ─────────────────────────────────────────────────────────

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -r -s /bin/false rqe
RUN mkdir -p /data && chown rqe:rqe /data

COPY --from=builder /src/target/release/rqe_server /usr/local/bin/rqe_server

USER rqe

ENV PORT=8080
ENV RQE_DB_PATH=/data/rqe.db
ENV RUST_LOG=rqe_server=info

EXPOSE 8080

VOLUME /data

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

ENTRYPOINT ["/usr/local/bin/rqe_server"]
