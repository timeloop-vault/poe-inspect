use poe_rqe::eval::Entry;
use poe_rqe::predicate::Condition;
use poe_rqe::store::QueryId;
use serde::{Deserialize, Serialize};

/// HTTP client for the RQE service.
pub struct RqeClient {
    http: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
}

/// Error type for RQE client operations.
#[derive(Debug, thiserror::Error)]
pub enum RqeError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("server returned {status}: {body}")]
    Server { status: u16, body: String },
}

#[derive(Serialize)]
struct AddQueryRequest {
    conditions: Vec<Condition>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    labels: Vec<String>,
}

#[derive(Deserialize)]
struct AddQueryResponse {
    id: QueryId,
}

#[derive(Debug, Deserialize)]
pub struct MatchResponse {
    pub matches: Vec<QueryId>,
    pub query_count: usize,
    /// Time spent in DAG matching on the server, in microseconds.
    #[serde(default)]
    pub match_us: u64,
}

#[derive(Debug, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub query_count: usize,
    pub node_count: usize,
}

#[allow(clippy::missing_errors_doc)]
impl RqeClient {
    /// Create a new client pointing at the given base URL.
    ///
    /// # Example
    /// ```no_run
    /// let client = poe_rqe_client::RqeClient::new("http://localhost:8080", None);
    /// ```
    #[must_use]
    pub fn new(base_url: &str, api_key: Option<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_owned(),
            api_key,
        }
    }

    /// Health check.
    pub async fn health(&self) -> Result<HealthResponse, RqeError> {
        let resp = self
            .http
            .get(format!("{}/health", self.base_url))
            .send()
            .await?;
        self.check_status(resp)
            .await?
            .json()
            .await
            .map_err(Into::into)
    }

    /// Register a reverse query. Returns its assigned ID.
    pub async fn add_query(
        &self,
        conditions: Vec<Condition>,
        labels: Vec<String>,
    ) -> Result<QueryId, RqeError> {
        let req = AddQueryRequest { conditions, labels };
        let resp = self
            .authed(self.http.post(format!("{}/queries", self.base_url)))
            .json(&req)
            .send()
            .await?;
        let parsed: AddQueryResponse = self.check_status(resp).await?.json().await?;
        Ok(parsed.id)
    }

    /// Get a stored query by ID. Returns the raw JSON (`StoredQuery`).
    pub async fn get_query(&self, id: QueryId) -> Result<serde_json::Value, RqeError> {
        let resp = self
            .http
            .get(format!("{}/queries/{id}", self.base_url))
            .send()
            .await?;
        self.check_status(resp)
            .await?
            .json()
            .await
            .map_err(Into::into)
    }

    /// Remove a query by ID. Returns true if it existed.
    pub async fn delete_query(&self, id: QueryId) -> Result<bool, RqeError> {
        let resp = self
            .authed(self.http.delete(format!("{}/queries/{id}", self.base_url)))
            .send()
            .await?;
        Ok(resp.status().as_u16() == 204)
    }

    /// Match an item entry against all stored queries.
    pub async fn match_item(&self, entry: &Entry) -> Result<MatchResponse, RqeError> {
        let resp = self
            .authed(self.http.post(format!("{}/match", self.base_url)))
            .json(entry)
            .send()
            .await?;
        self.check_status(resp)
            .await?
            .json()
            .await
            .map_err(Into::into)
    }

    /// Add API key header if configured.
    fn authed(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(key) = &self.api_key {
            builder.header("X-API-Key", key)
        } else {
            builder
        }
    }

    /// Check HTTP status and return an error for non-success responses.
    async fn check_status(&self, resp: reqwest::Response) -> Result<reqwest::Response, RqeError> {
        let status = resp.status().as_u16();
        if status >= 400 {
            let body = resp.text().await.unwrap_or_default();
            return Err(RqeError::Server { status, body });
        }
        Ok(resp)
    }
}
