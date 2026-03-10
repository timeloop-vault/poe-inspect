//! Trade API client for pathofexile.com.
//!
//! Provides stat index building, query construction, and rate-limited HTTP
//! client for PoE's official trade search API.
//!
//! # Architecture
//!
//! ```text
//! poe-item::ResolvedItem
//!     │
//!     ▼
//! TradeStatsIndex (template text → trade stat IDs)
//!     │
//!     ▼
//! query builder (ResolvedItem → TradeSearchBody)
//!     │
//!     ▼
//! TradeClient (rate-limited HTTP: search + fetch)
//!     │
//!     ▼
//! PriceCheckResult
//! ```

pub mod client;
pub mod query;
pub mod stats_index;
pub mod types;

pub use client::{fetch_trade_stats, TradeApiError};
pub use query::{build_query, QueryBuildResult, TradeSearchBody};
pub use stats_index::IndexBuildResult;
pub use types::{
    Price, PriceCheckResult, SearchResult, TradeQueryConfig, TradeStatCategory, TradeStatEntry,
    TradeStatsIndex, TradeStatsResponse,
};
