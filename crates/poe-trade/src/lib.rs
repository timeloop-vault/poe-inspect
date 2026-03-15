//! Trade API client for pathofexile.com.
//!
//! Provides stat index building, query construction, and rate-limited HTTP
//! client for `PoE`'s official trade search API.
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
pub mod filter_schema;
pub mod query;
pub mod rate_limit;
pub mod stats_index;
pub mod types;

pub use client::{TradeApiError, TradeClient, fetch_trade_stats};
pub use query::{QueryBuildResult, TradeSearchBody, build_query};
pub use stats_index::IndexBuildResult;
pub use types::{
    League, LeagueList, ListingStatus, MappedStat, Price, PriceCheckResult, SearchResult,
    SocketInfo, StatFilterOverride, TradeFilterConfig, TradeQueryConfig, TradeStatCategory,
    TradeStatEntry, TradeStatsIndex, TradeStatsResponse, TypeSearchScope, listing_statuses,
};
