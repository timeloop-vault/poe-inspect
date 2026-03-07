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

type SharedStore = Arc<Mutex<QueryStore>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rqe_server=info".into()),
        )
        .init();

    let store: SharedStore = Arc::new(Mutex::new(QueryStore::new()));

    let app = Router::new()
        .route("/status", get(status))
        .route("/queries", post(add_query))
        .route("/queries/{id}", get(get_query))
        .route("/queries/{id}", delete(delete_query))
        .route("/match", post(match_item))
        .with_state(store);

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
struct QueryResponse {
    id: QueryId,
    labels: Vec<String>,
    condition_count: usize,
}

#[derive(Serialize)]
struct MatchResponse {
    matches: Vec<QueryId>,
    query_count: usize,
}

// --- Handlers ---

async fn status(State(store): State<SharedStore>) -> Json<StatusResponse> {
    let store = store.lock().unwrap();
    Json(StatusResponse {
        query_count: store.len(),
    })
}

async fn add_query(
    State(store): State<SharedStore>,
    Json(req): Json<AddQueryRequest>,
) -> (StatusCode, Json<AddQueryResponse>) {
    let mut store = store.lock().unwrap();
    let id = store.add(req.conditions, req.labels);
    tracing::info!(id, "query added");
    (StatusCode::CREATED, Json(AddQueryResponse { id }))
}

async fn get_query(
    State(store): State<SharedStore>,
    Path(id): Path<QueryId>,
) -> impl IntoResponse {
    let store = store.lock().unwrap();
    match store.get(id) {
        Some(q) => Ok(Json(QueryResponse {
            id: q.id,
            labels: q.labels.clone(),
            condition_count: q.conditions.len(),
        })),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn delete_query(
    State(store): State<SharedStore>,
    Path(id): Path<QueryId>,
) -> StatusCode {
    let mut store = store.lock().unwrap();
    if store.remove(id) {
        tracing::info!(id, "query removed");
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn match_item(
    State(store): State<SharedStore>,
    Json(entry): Json<Entry>,
) -> Json<MatchResponse> {
    let store = store.lock().unwrap();
    let matches = store.match_item(&entry);
    let query_count = store.len();
    tracing::info!(
        matched = matches.len(),
        total = query_count,
        "item matched"
    );
    Json(MatchResponse {
        matches,
        query_count,
    })
}
