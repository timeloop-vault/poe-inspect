//! HTTP client for the pathofexile.com trade API.
//!
//! Phase 1: stats endpoint only. Search + fetch + rate limiting in Phase 3.

use crate::types::TradeStatsResponse;

/// Trade API base URLs.
const POE1_TRADE_API: &str = "https://www.pathofexile.com/api/trade";

/// User-Agent header value. GGG requires a descriptive User-Agent.
const USER_AGENT: &str = "poe-inspect-2/0.1 (contact: github.com/timeloop-vault/poe-inspect)";

/// Errors from trade API operations.
#[derive(Debug, thiserror::Error)]
pub enum TradeApiError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Trade API returned {status}: {body}")]
    ApiError { status: u16, body: String },
}

/// Fetch the trade stats dictionary from the trade API.
///
/// Returns all searchable stats with their IDs, text patterns, and categories.
/// This is the foundation for building the `TradeStatsIndex`.
pub async fn fetch_trade_stats() -> Result<TradeStatsResponse, TradeApiError> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()?;

    let url = format!("{POE1_TRADE_API}/data/stats");
    let response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(TradeApiError::ApiError {
            status: status.as_u16(),
            body,
        });
    }

    let stats: TradeStatsResponse = response.json().await?;
    Ok(stats)
}
