//! Rate-limited HTTP client for the pathofexile.com trade API.
//!
//! Wraps `reqwest` with preemptive rate limiting (wait before sending)
//! and handles the search → fetch two-step flow.

use std::time::Duration;

use reqwest::header::HeaderMap;
use serde::Deserialize;

use crate::query::{self, TradeSearchBody};
use crate::rate_limit::{RateLimitPolicy, RateLimitTracker};
use crate::types::{
    ApiLeague, League, LeagueList, Price, PriceCheckResult, SearchResult, TradeQueryConfig,
    TradeStatsResponse,
};

/// Trade API base URL.
const POE1_TRADE_API: &str = "https://www.pathofexile.com/api/trade";

/// Main API base URL (leagues endpoint lives here, not under /trade).
const POE1_API: &str = "https://www.pathofexile.com/api";

/// User-Agent header value. GGG requires a descriptive User-Agent.
const USER_AGENT: &str = "poe-inspect-2/0.1 (contact: github.com/timeloop-vault/poe-inspect)";

/// Maximum listing IDs per fetch request (trade API limit).
const MAX_FETCH_IDS: usize = 10;

// ── Error ───────────────────────────────────────────────────────────────────

/// Errors from trade API operations.
#[derive(Debug, thiserror::Error)]
pub enum TradeApiError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Trade API returned {status}: {body}")]
    ApiError { status: u16, body: String },

    #[error("Rate limited — retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("Failed to parse API response: {0}")]
    Parse(String),
}

// ── Client ──────────────────────────────────────────────────────────────────

/// Rate-limited HTTP client for pathofexile.com/trade.
///
/// Maintains separate rate limit trackers for search and fetch endpoints.
/// Call methods with `&mut self` — the app should hold this behind a `Mutex`
/// in Tauri managed state.
#[derive(Debug)]
pub struct TradeClient {
    http: reqwest::Client,
    search_limiter: RateLimitTracker,
    fetch_limiter: RateLimitTracker,
    poesessid: Option<String>,
}

impl TradeClient {
    /// Create a new trade client.
    #[must_use]
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");

        Self {
            http,
            search_limiter: RateLimitTracker::new(),
            fetch_limiter: RateLimitTracker::new(),
            poesessid: None,
        }
    }

    /// Set the POESESSID cookie for authenticated requests.
    ///
    /// Enables "online only" filtering and shows own listings.
    /// Pass `None` to clear.
    pub fn set_session_id(&mut self, poesessid: Option<String>) {
        self.poesessid = poesessid;
    }

    // ── High-level API ──────────────────────────────────────────────────────

    /// Full price check: search → fetch listings → extract prices.
    ///
    /// Returns prices from the cheapest listings (up to 10).
    pub async fn price_check(
        &mut self,
        query_body: &TradeSearchBody,
        config: &TradeQueryConfig,
    ) -> Result<PriceCheckResult, TradeApiError> {
        let search = self.search(query_body, &config.league).await?;

        let trade_url = query::trade_url(&config.league, &search.id);

        if search.listing_ids.is_empty() || search.total == 0 {
            return Ok(PriceCheckResult {
                search_id: search.id,
                total: search.total,
                prices: vec![],
                trade_url,
            });
        }

        // Fetch first batch of listings (max 10).
        let fetch_ids: Vec<_> = search
            .listing_ids
            .iter()
            .take(MAX_FETCH_IDS)
            .cloned()
            .collect();

        let listings = self.fetch_listings(&search.id, &fetch_ids).await?;

        let prices: Vec<Price> = listings
            .into_iter()
            .filter_map(|entry| {
                entry.listing.price.map(|p| Price {
                    amount: p.amount,
                    currency: p.currency,
                })
            })
            .collect();

        Ok(PriceCheckResult {
            search_id: search.id,
            total: search.total,
            prices,
            trade_url,
        })
    }

    // ── Search ──────────────────────────────────────────────────────────────

    /// Execute a trade search.
    ///
    /// `POST /api/trade/search/{league}` with the query body.
    /// Returns a search ID and the first batch of listing IDs.
    pub async fn search(
        &mut self,
        query_body: &TradeSearchBody,
        league: &str,
    ) -> Result<SearchResult, TradeApiError> {
        self.search_limiter.wait_for_capacity().await;
        self.search_limiter.record_request();

        let url = format!("{POE1_TRADE_API}/search/{league}");
        let mut request = self.http.post(&url).json(query_body);
        if let Some(ref sessid) = self.poesessid {
            request = request.header("Cookie", format!("POESESSID={sessid}"));
        }

        let response = request.send().await?;
        let status = response.status();
        let headers = response.headers().clone();
        let body = response.text().await?;

        // Update rate limits from response headers.
        self.update_limiter_from_headers(&headers, LimiterKind::Search);

        // Handle rate limit response.
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = parse_retry_after(&headers);
            self.search_limiter.block_for(Duration::from_secs(retry_after));
            return Err(TradeApiError::RateLimited {
                retry_after_secs: retry_after,
            });
        }

        if !status.is_success() {
            return Err(TradeApiError::ApiError {
                status: status.as_u16(),
                body,
            });
        }

        let api_response: SearchApiResponse = serde_json::from_str(&body)
            .map_err(|e| TradeApiError::Parse(format!("search response: {e}")))?;

        tracing::info!(
            search_id = %api_response.id,
            total = api_response.total,
            results = api_response.result.len(),
            "trade search complete"
        );

        Ok(SearchResult {
            id: api_response.id,
            total: api_response.total,
            listing_ids: api_response.result,
        })
    }

    // ── Fetch Listings ──────────────────────────────────────────────────────

    /// Fetch listing details by IDs.
    ///
    /// `GET /api/trade/fetch/{id1,id2,...}?query={search_id}`
    /// Maximum 10 IDs per request (trade API limit).
    pub async fn fetch_listings(
        &mut self,
        search_id: &str,
        listing_ids: &[String],
    ) -> Result<Vec<FetchResultEntry>, TradeApiError> {
        if listing_ids.is_empty() {
            return Ok(vec![]);
        }

        let ids_to_fetch = if listing_ids.len() > MAX_FETCH_IDS {
            &listing_ids[..MAX_FETCH_IDS]
        } else {
            listing_ids
        };

        self.fetch_limiter.wait_for_capacity().await;
        self.fetch_limiter.record_request();

        let ids_csv = ids_to_fetch.join(",");
        let url = format!("{POE1_TRADE_API}/fetch/{ids_csv}?query={search_id}");
        let mut request = self.http.get(&url);
        if let Some(ref sessid) = self.poesessid {
            request = request.header("Cookie", format!("POESESSID={sessid}"));
        }

        let response = request.send().await?;
        let status = response.status();
        let headers = response.headers().clone();
        let body = response.text().await?;

        self.update_limiter_from_headers(&headers, LimiterKind::Fetch);

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = parse_retry_after(&headers);
            self.fetch_limiter.block_for(Duration::from_secs(retry_after));
            return Err(TradeApiError::RateLimited {
                retry_after_secs: retry_after,
            });
        }

        if !status.is_success() {
            return Err(TradeApiError::ApiError {
                status: status.as_u16(),
                body,
            });
        }

        let api_response: FetchApiResponse = serde_json::from_str(&body)
            .map_err(|e| TradeApiError::Parse(format!("fetch response: {e}")))?;

        tracing::info!(listings = api_response.result.len(), "fetched listings");

        Ok(api_response.result)
    }

    // ── Stats ───────────────────────────────────────────────────────────────

    /// Fetch the trade stats dictionary from the trade API.
    ///
    /// Returns all searchable stats with their IDs, text patterns, and categories.
    /// This is the foundation for building the `TradeStatsIndex`.
    pub async fn fetch_stats(&self) -> Result<TradeStatsResponse, TradeApiError> {
        let url = format!("{POE1_TRADE_API}/data/stats");
        let response = self.http.get(&url).send().await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(TradeApiError::ApiError {
                status: status.as_u16(),
                body,
            });
        }

        let result: TradeStatsResponse = response.json().await?;
        Ok(result)
    }

    // ── Leagues ──────────────────────────────────────────────────────────────

    /// Fetch the list of trade-eligible leagues from the GGG API.
    ///
    /// Filters out SSF leagues (which have the `NoParties` rule — no trading).
    /// Returns remaining leagues grouped into public and private.
    /// Private leagues are detected by the `(PLnnnn)` suffix pattern.
    pub async fn fetch_leagues(&self) -> Result<LeagueList, TradeApiError> {
        let url = format!("{POE1_API}/leagues?type=main&realm=pc");
        let response = self.http.get(&url).send().await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(TradeApiError::ApiError {
                status: status.as_u16(),
                body,
            });
        }

        let api_leagues: Vec<ApiLeague> = response.json().await?;

        let mut leagues = Vec::new();
        let mut private_leagues = Vec::new();
        let mut skipped_ssf = 0u32;

        for api in api_leagues {
            // SSF leagues have the "NoParties" rule — no trading possible.
            if api.rules.iter().any(|r| r.id == "NoParties") {
                skipped_ssf += 1;
                continue;
            }

            let is_private = is_private_league(&api.id);
            let league = League {
                id: api.id,
                private: is_private,
            };
            if is_private {
                private_leagues.push(league);
            } else {
                leagues.push(league);
            }
        }

        tracing::info!(
            public = leagues.len(),
            private = private_leagues.len(),
            skipped_ssf,
            "fetched trade-eligible leagues"
        );

        Ok(LeagueList {
            leagues,
            private_leagues,
        })
    }

    // ── Internals ───────────────────────────────────────────────────────────

    fn update_limiter_from_headers(&mut self, headers: &HeaderMap, kind: LimiterKind) {
        // Try X-Rate-Limit-Ip first (most common), then generic X-Rate-Limit.
        let policy_header = headers
            .get("x-rate-limit-ip")
            .or_else(|| headers.get("x-rate-limit"))
            .and_then(|v| v.to_str().ok());

        if let Some(header_str) = policy_header {
            if let Some(policy) = RateLimitPolicy::parse(header_str) {
                let limiter = match kind {
                    LimiterKind::Search => &mut self.search_limiter,
                    LimiterKind::Fetch => &mut self.fetch_limiter,
                };
                limiter.update_policy(policy);
            }
        }
    }
}

impl Default for TradeClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
enum LimiterKind {
    Search,
    Fetch,
}

// ── Standalone function (backwards compat) ──────────────────────────────────

/// Fetch trade stats without a persistent client.
///
/// Convenience wrapper — creates a one-shot client. For repeated use,
/// prefer `TradeClient::fetch_stats()`.
pub async fn fetch_trade_stats() -> Result<TradeStatsResponse, TradeApiError> {
    TradeClient::new().fetch_stats().await
}

// ── API response types (internal) ───────────────────────────────────────────

/// Raw response from `POST /api/trade/search/{league}`.
#[derive(Debug, Deserialize)]
struct SearchApiResponse {
    id: String,
    total: u32,
    result: Vec<String>,
}

/// Raw response from `GET /api/trade/fetch/{ids}?query={search_id}`.
#[derive(Debug, Deserialize)]
struct FetchApiResponse {
    result: Vec<FetchResultEntry>,
}

/// A single listing from the fetch response.
#[derive(Debug, Deserialize)]
pub struct FetchResultEntry {
    pub id: String,
    pub listing: ListingData,
}

/// Listing metadata (price, account, indexed time).
#[derive(Debug, Deserialize)]
pub struct ListingData {
    pub price: Option<ListingPrice>,
    #[serde(default)]
    pub indexed: Option<String>,
    #[serde(default)]
    pub account: Option<AccountInfo>,
}

/// Price on a trade listing.
#[derive(Debug, Deserialize)]
pub struct ListingPrice {
    pub amount: f64,
    pub currency: String,
    #[serde(rename = "type")]
    pub price_type: Option<String>,
}

/// Account info on a trade listing.
#[derive(Debug, Deserialize)]
pub struct AccountInfo {
    pub name: String,
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Parse `Retry-After` header, defaulting to 60 seconds.
fn parse_retry_after(headers: &HeaderMap) -> u64 {
    headers
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(60)
}

/// Check if a league ID is a private league.
///
/// Private leagues have a `(PLnnnn)` suffix, e.g., `"My League (PL12345)"`.
fn is_private_league(id: &str) -> bool {
    let Some(rest) = id.strip_suffix(')') else {
        return false;
    };
    let Some(idx) = rest.rfind("(PL") else {
        return false;
    };
    rest[idx + 3..].chars().all(|c| c.is_ascii_digit()) && !rest[idx + 3..].is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_league_detection() {
        assert!(is_private_league("My League (PL12345)"));
        assert!(is_private_league("(PL1)"));
        assert!(!is_private_league("Mirage"));
        assert!(!is_private_league("Standard"));
        assert!(!is_private_league("Hardcore Mirage"));
        assert!(!is_private_league("(PL)")); // no digits
        assert!(!is_private_league("(PLabc)")); // non-digits
    }
}
