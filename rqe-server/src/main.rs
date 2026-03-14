use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use poe_rqe::eval::Entry;
use poe_rqe::index::IndexedStore;
use poe_rqe::predicate::Condition;
use poe_rqe::store::QueryId;
use serde::{Deserialize, Serialize};

mod auth;
mod db;

use auth::ApiKey;

struct AppState {
    /// `RwLock`: match (read) runs in parallel, add/remove (write) gets exclusive access.
    store: RwLock<IndexedStore>,
    /// Mutex: all DB operations are sequential writes. rusqlite Connection is not Sync.
    db: Mutex<db::Db>,
}

type SharedState = Arc<AppState>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rqe_server=info".into()),
        )
        .init();

    let db_path = std::env::var("RQE_DB_PATH").ok();
    let db = db::Db::open(db_path.as_deref());

    let t0 = Instant::now();
    let store = db.load_all();
    let load_ms = t0.elapsed().as_secs_f64() * 1000.0;
    tracing::info!(
        queries = store.len(),
        nodes = store.node_count(),
        load_ms = format!("{load_ms:.1}"),
        "loaded queries into indexed store"
    );

    let state: SharedState = Arc::new(AppState {
        store: RwLock::new(store),
        db: Mutex::new(db),
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/queries", post(add_query))
        .route("/queries/{id}", get(get_query))
        .route("/queries/{id}", delete(delete_query))
        .route("/match", post(match_item))
        .with_state(state);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("rqe-server listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// --- Request/Response types ---

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    query_count: usize,
    node_count: usize,
}

#[derive(Deserialize)]
struct AddQueryRequest {
    conditions: Vec<Condition>,
    #[serde(default)]
    labels: Vec<String>,
}

#[derive(Serialize)]
struct AddQueryResponse {
    id: QueryId,
}

#[derive(Serialize)]
struct MatchResponse {
    matches: Vec<QueryId>,
    query_count: usize,
    /// Time spent in DAG matching, in microseconds.
    match_us: u64,
}

// --- Handlers ---

/// Health check — unauthenticated.
async fn health(State(state): State<SharedState>) -> Json<HealthResponse> {
    let store = state.store.read().expect("store lock poisoned");
    Json(HealthResponse {
        status: "ok",
        query_count: store.len(),
        node_count: store.node_count(),
    })
}

/// Register a reverse query.
async fn add_query(
    _auth: ApiKey,
    State(state): State<SharedState>,
    Json(req): Json<AddQueryRequest>,
) -> (StatusCode, Json<AddQueryResponse>) {
    let t0 = Instant::now();

    let (id, nodes) = {
        let mut store = state.store.write().expect("store lock poisoned");
        let id = store.add(req.conditions.clone(), req.labels.clone());
        (id, store.node_count())
    };

    {
        let db = state.db.lock().expect("db lock poisoned");
        db.insert(id, &req.conditions, &req.labels);
    }

    let elapsed_us = t0.elapsed().as_micros();
    tracing::info!(
        id,
        conditions = req.conditions.len(),
        nodes,
        elapsed_us = elapsed_us,
        "query added"
    );
    (StatusCode::CREATED, Json(AddQueryResponse { id }))
}

/// Get a stored query by ID — unauthenticated.
async fn get_query(State(state): State<SharedState>, Path(id): Path<QueryId>) -> impl IntoResponse {
    let store = state.store.read().expect("store lock poisoned");
    match store.get(id) {
        Some(q) => Ok(Json(q.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Remove a query by ID.
async fn delete_query(
    _auth: ApiKey,
    State(state): State<SharedState>,
    Path(id): Path<QueryId>,
) -> StatusCode {
    let t0 = Instant::now();

    let removed = {
        let mut store = state.store.write().expect("store lock poisoned");
        store.remove(id)
    };

    if removed {
        let db = state.db.lock().expect("db lock poisoned");
        db.delete(id);
        let elapsed_us = t0.elapsed().as_micros();
        tracing::info!(id, elapsed_us = elapsed_us, "query removed");
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

/// Match an item against all stored queries.
async fn match_item(
    _auth: ApiKey,
    State(state): State<SharedState>,
    Json(entry): Json<Entry>,
) -> Json<MatchResponse> {
    let t0 = Instant::now();

    let store = state.store.read().expect("store lock poisoned");
    let matches = store.match_item(&entry);
    let match_us = t0.elapsed().as_micros() as u64;
    let query_count = store.len();
    drop(store);

    tracing::info!(
        matched = matches.len(),
        total = query_count,
        match_us,
        "item matched"
    );

    Json(MatchResponse {
        matches,
        query_count,
        match_us,
    })
}
