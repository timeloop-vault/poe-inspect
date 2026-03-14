# rqe-server — HTTP Server for Reverse Query Engine

## Purpose

HTTP API wrapping the poe-rqe `IndexedStore` (decision DAG). Allows external clients
(poe-inspect, web UI, curl) to register reverse queries, submit items, and receive matches.

## Status

Working prototype with SQLite persistence, indexed matching (decision DAG with threshold
groups), `RwLock`-based concurrency, and optional API key auth.

## API

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/health` | No | Health check: status, query count, node count |
| `POST` | `/queries` | Yes | Add a reverse query. Body: `{ conditions: [...], labels: [...] }` |
| `GET` | `/queries/{id}` | No | Get query info by ID |
| `DELETE` | `/queries/{id}` | Yes | Remove a query |
| `POST` | `/match` | Yes | Submit an item entry. Returns matching query IDs |

## Running

```
cargo run -p rqe-server
# Listens on 0.0.0.0:8080 (override with PORT env var)
# Set RUST_LOG=rqe_server=debug for verbose logging
# Set RQE_DB_PATH=/path/to/rqe.db to change database location (default: ./rqe.db)
# Set RQE_API_KEY=<secret> to enable API key auth (disabled when unset)
```

## Authentication

Extractor-based: handlers that need auth include `_auth: ApiKey` in their parameters.

- **`RQE_API_KEY` unset/empty**: Auth disabled, all requests pass (development mode)
- **`RQE_API_KEY` set**: Requires `X-API-Key: <key>` header on protected endpoints
- **Missing header**: 401 Unauthorized
- **Wrong key**: 403 Forbidden

Designed for future swap: replace `ApiKey` extractor internals with JWT/OAuth
without changing handler signatures.

## Architecture

- **Axum** HTTP framework
- **`RwLock<IndexedStore>`**: Match requests (read) run in parallel. Add/remove (write)
  gets exclusive access. Match is the hot path — never blocked by other reads.
- **`Mutex<Db>`**: SQLite via `rusqlite`. All DB ops are sequential writes.
  `rusqlite::Connection` is not `Sync`.
- **Persistence**: Queries loaded into `IndexedStore` on startup from SQLite.
  Mutations written through to DB. DB path configurable via `RQE_DB_PATH`.
- **Deploy target**: Cloud Run with `--min-instances` and `--cpu-always-allocated`

## Dependencies

- `poe-rqe` — IndexedStore, predicate types, evaluation
- `axum` — HTTP framework
- `tokio` — async runtime
- `rusqlite` — SQLite persistence (bundled, no system dep)
- `tracing` / `tracing-subscriber` — structured logging

## Future

- Cloud SQL migration (swap SQLite for PostgreSQL)
- JWT/OAuth authentication (swap ApiKey extractor)
- WebSocket/SSE for real-time match notifications
- Pub/Sub integration for multi-instance query sync
- Dockerfile for Cloud Run deployment
