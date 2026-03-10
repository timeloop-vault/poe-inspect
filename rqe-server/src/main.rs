use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use poe_rqe::eval::Entry;
use poe_rqe::predicate::Condition;
use poe_rqe::store::{QueryId, QueryStore};
use serde::{Deserialize, Serialize};

mod db;

struct AppState {
    store: Mutex<QueryStore>,
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
    let store = db.load_all();
    tracing::info!(queries = store.len(), "loaded queries from database");

    let state: SharedState = Arc::new(AppState {
        store: Mutex::new(store),
        db: Mutex::new(db),
    });

    let app = Router::new()
        .route("/status", get(status))
        .route("/queries", post(add_query))
        .route("/queries/{id}", get(get_query))
        .route("/queries/{id}", delete(delete_query))
        .route("/match", post(match_item))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("rqe-server listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// --- Request/Response types ---

#[derive(Serialize)]
struct StatusResponse {
    query_count: usize,
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
}

// --- Handlers ---

async fn status(State(state): State<SharedState>) -> Json<StatusResponse> {
    let store = state.store.lock().unwrap();
    Json(StatusResponse {
        query_count: store.len(),
    })
}

async fn add_query(
    State(state): State<SharedState>,
    Json(req): Json<AddQueryRequest>,
) -> (StatusCode, Json<AddQueryResponse>) {
    let mut store = state.store.lock().unwrap();
    let id = store.add(req.conditions.clone(), req.labels.clone());
    drop(store);

    let db = state.db.lock().unwrap();
    db.insert(id, &req.conditions, &req.labels);

    tracing::info!(id, "query added");
    (StatusCode::CREATED, Json(AddQueryResponse { id }))
}

async fn get_query(State(state): State<SharedState>, Path(id): Path<QueryId>) -> impl IntoResponse {
    let store = state.store.lock().unwrap();
    match store.get(id) {
        Some(q) => Ok(Json(q.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn delete_query(State(state): State<SharedState>, Path(id): Path<QueryId>) -> StatusCode {
    let mut store = state.store.lock().unwrap();
    if store.remove(id) {
        drop(store);
        let db = state.db.lock().unwrap();
        db.delete(id);
        tracing::info!(id, "query removed");
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn match_item(
    State(state): State<SharedState>,
    Json(entry): Json<Entry>,
) -> Json<MatchResponse> {
    let store = state.store.lock().unwrap();
    let matches = store.match_item(&entry);
    let query_count = store.len();
    tracing::info!(matched = matches.len(), total = query_count, "item matched");
    Json(MatchResponse {
        matches,
        query_count,
    })
}
