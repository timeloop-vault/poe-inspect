# rqe-server — HTTP Server for Reverse Query Engine

## Purpose

HTTP API wrapping the poe-rqe `QueryStore`. Allows external clients (poe-inspect,
web UI, curl) to register reverse queries, submit items, and receive matches.

## Status

Working prototype with SQLite persistence. Single-node.

## API

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/status` | Query count |
| `POST` | `/queries` | Add a reverse query. Body: `{ conditions: [...], labels: [...] }` |
| `GET` | `/queries/{id}` | Get query info by ID |
| `DELETE` | `/queries/{id}` | Remove a query |
| `POST` | `/match` | Submit an item entry. Returns matching query IDs |

## Running

```
cargo run -p rqe-server
# Listens on 0.0.0.0:8080
# Set RUST_LOG=rqe_server=debug for verbose logging
# Set RQE_DB_PATH=/path/to/rqe.db to change database location (default: ./rqe.db)
```

## Architecture

- **Axum** HTTP framework
- **Shared state**: `Arc<AppState>` with `Mutex<QueryStore>` + `Mutex<Db>` — simple, correct.
  Swap for `RwLock` if read-heavy profiling shows contention.
- **Persistence**: SQLite via `rusqlite` (bundled). Queries loaded on startup, mutations
  written through to DB. DB path configurable via `RQE_DB_PATH` env var.
- **Deploy target**: Cloud Run with `--min-instances` and `--cpu-always-allocated`

## Dependencies

- `poe-rqe` — QueryStore, predicate types, evaluation
- `axum` — HTTP framework
- `tokio` — async runtime
- `rusqlite` — SQLite persistence (bundled, no system dep)
- `tracing` / `tracing-subscriber` — structured logging

## Future

- Cloud SQL migration (swap SQLite for PostgreSQL)
- Authentication
- WebSocket/SSE for real-time match notifications
- Pub/Sub integration for multi-instance query sync
- Dockerfile for Cloud Run deployment
