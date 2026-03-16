//! Rate limit tracking for the trade API.
//!
//! GGG's trade API returns rate limit policy and state via response headers.
//! This module parses those headers and provides preemptive blocking — we
//! wait before sending rather than reacting to 429 responses.
//!
//! Header format:
//! - `X-Rate-Limit-Ip: 12:6:60,16:12:300` — policy: `max:period_secs:penalty_secs`
//! - `X-Rate-Limit-Ip-State: 1:6:0,1:12:0` — state: `current:period_secs:penalty_remaining`

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Safety margin added to all delays. Accounts for network latency and
/// clock skew between our tracking and GGG's server.
const SAFETY_MARGIN: Duration = Duration::from_millis(1500);

/// Conservative default policy used before any GGG response arrives.
/// Based on observed limits (`12:6:60,16:12:300`) but at half capacity
/// to be safe during the unknown initial period.
const DEFAULT_POLICY_STR: &str = "6:6:60,8:12:300";

/// A single rate limit rule (e.g., "12 requests per 6 seconds, 60s penalty").
#[derive(Debug, Clone)]
pub struct RateLimitRule {
    pub max_hits: u32,
    pub period: Duration,
    pub penalty: Duration,
}

/// Parsed rate limit policy from response headers.
#[derive(Debug, Clone)]
pub struct RateLimitPolicy {
    pub rules: Vec<RateLimitRule>,
}

impl RateLimitPolicy {
    /// Parse from `X-Rate-Limit-Ip` header value: `"12:6:60,16:12:300"`.
    pub fn parse(header: &str) -> Option<Self> {
        let rules: Vec<RateLimitRule> = header
            .split(',')
            .filter_map(|part| {
                let mut fields = part.trim().split(':');
                let max_hits = fields.next()?.parse().ok()?;
                let period_secs: u64 = fields.next()?.parse().ok()?;
                let penalty_secs: u64 = fields.next()?.parse().ok()?;
                Some(RateLimitRule {
                    max_hits,
                    period: Duration::from_secs(period_secs),
                    penalty: Duration::from_secs(penalty_secs),
                })
            })
            .collect();

        if rules.is_empty() {
            None
        } else {
            Some(Self { rules })
        }
    }
}

/// Tracks request history and enforces rate limits across all trade API
/// endpoints. GGG's `X-Rate-Limit-Ip` is per-IP across all endpoints,
/// so a single tracker covers search, fetch, stats, filters, and leagues.
#[derive(Debug)]
pub struct RateLimitTracker {
    policy: Option<RateLimitPolicy>,
    request_times: VecDeque<Instant>,
    blocked_until: Option<Instant>,
}

impl RateLimitTracker {
    /// Create a new tracker with a conservative default policy.
    ///
    /// The default limits requests to half the observed GGG limits,
    /// protecting the user even before the first API response reveals
    /// the actual policy.
    #[must_use]
    pub fn new() -> Self {
        Self {
            policy: RateLimitPolicy::parse(DEFAULT_POLICY_STR),
            request_times: VecDeque::new(),
            blocked_until: None,
        }
    }

    /// Update the rate limit policy from response headers.
    ///
    /// Called after every API response that includes `X-Rate-Limit-Ip`.
    /// Replaces the conservative default with the real server policy.
    pub fn update_policy(&mut self, policy: RateLimitPolicy) {
        self.policy = Some(policy);
    }

    /// Sync local request count with server-reported state.
    ///
    /// Parses the `X-Rate-Limit-Ip-State` header (`"current:period:penalty,..."`)
    /// and adjusts local tracking to match. If the server reports a penalty,
    /// blocks for that duration.
    pub fn sync_server_state(&mut self, state_header: &str) {
        let now = Instant::now();

        for part in state_header.split(',') {
            let mut fields = part.trim().split(':');
            let Some(current_str) = fields.next() else {
                continue;
            };
            let Ok(current) = current_str.parse::<u32>() else {
                continue;
            };
            // Skip period field.
            let _ = fields.next();
            // Parse penalty remaining.
            if let Some(penalty_str) = fields.next() {
                if let Ok(penalty_secs) = penalty_str.parse::<u64>() {
                    if penalty_secs > 0 {
                        let block_until = now + Duration::from_secs(penalty_secs) + SAFETY_MARGIN;
                        self.blocked_until = Some(
                            self.blocked_until
                                .map_or(block_until, |existing| existing.max(block_until)),
                        );
                    }
                }
            }

            // Adjust local request count to match server's view.
            // This is conservative: we assume all server-counted requests
            // happened just now (recent), so the limiter will be cautious.
            let local_count = self.request_times.len() as u32;
            if current > local_count {
                // Server knows about more requests than us (e.g., after restart).
                // Inject synthetic timestamps at `now` to match.
                for _ in 0..(current - local_count) {
                    self.request_times.push_back(now);
                }
            }
        }
    }

    /// Record that a request was just sent.
    pub fn record_request(&mut self) {
        let now = Instant::now();
        self.request_times.push_back(now);
        self.prune_old_entries(now);
    }

    /// Set a penalty block (e.g., from `Retry-After` header on a 429).
    pub fn block_for(&mut self, duration: Duration) {
        self.blocked_until = Some(Instant::now() + duration);
    }

    /// Calculate delay needed before the next request is allowed.
    ///
    /// Includes a safety margin to account for network latency.
    #[must_use]
    pub fn delay_needed(&self) -> Duration {
        let now = Instant::now();

        // Check penalty block first.
        if let Some(until) = self.blocked_until {
            if until > now {
                return until - now;
            }
        }

        let Some(policy) = &self.policy else {
            return Duration::ZERO;
        };

        let mut max_delay = Duration::ZERO;

        for rule in &policy.rules {
            let window_start = now.checked_sub(rule.period).unwrap_or(now);
            let hits_in_window = self
                .request_times
                .iter()
                .filter(|&&t| t > window_start)
                .count() as u32;

            if hits_in_window >= rule.max_hits {
                // Need to wait until the oldest request in this window expires.
                if let Some(&oldest) = self.request_times.iter().find(|&&t| t > window_start) {
                    let expires_at = oldest + rule.period;
                    if expires_at > now {
                        max_delay = max_delay.max(expires_at - now);
                    }
                }
            }
        }

        if max_delay > Duration::ZERO {
            max_delay += SAFETY_MARGIN;
        }

        max_delay
    }

    /// Wait until a request can be made. Returns immediately if no delay needed.
    pub async fn wait_for_capacity(&self) {
        let delay = self.delay_needed();
        if delay > Duration::ZERO {
            tracing::info!(delay_ms = delay.as_millis(), "rate limit: waiting");
            tokio::time::sleep(delay).await;
        }
    }

    /// Prune request timestamps older than 5 minutes.
    fn prune_old_entries(&mut self, now: Instant) {
        let cutoff = now.checked_sub(Duration::from_secs(300)).unwrap_or(now);
        while self.request_times.front().is_some_and(|&t| t < cutoff) {
            self.request_times.pop_front();
        }
    }
}

impl Default for RateLimitTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_policy_single_rule() {
        let policy = RateLimitPolicy::parse("12:6:60").unwrap();
        assert_eq!(policy.rules.len(), 1);
        assert_eq!(policy.rules[0].max_hits, 12);
        assert_eq!(policy.rules[0].period, Duration::from_secs(6));
        assert_eq!(policy.rules[0].penalty, Duration::from_secs(60));
    }

    #[test]
    fn parse_policy_multiple_rules() {
        let policy = RateLimitPolicy::parse("12:6:60,16:12:300").unwrap();
        assert_eq!(policy.rules.len(), 2);
        assert_eq!(policy.rules[0].max_hits, 12);
        assert_eq!(policy.rules[1].max_hits, 16);
        assert_eq!(policy.rules[1].penalty, Duration::from_secs(300));
    }

    #[test]
    fn parse_policy_empty_returns_none() {
        assert!(RateLimitPolicy::parse("").is_none());
        assert!(RateLimitPolicy::parse("garbage").is_none());
    }

    #[test]
    fn new_tracker_has_default_policy() {
        let tracker = RateLimitTracker::new();
        assert!(
            tracker.policy.is_some(),
            "new tracker should have default policy"
        );
    }

    #[test]
    fn default_policy_limits_requests() {
        let mut tracker = RateLimitTracker::new();
        // Default policy: "6:6:60,8:12:300"
        // Fill to limit (6 requests).
        for _ in 0..6 {
            tracker.record_request();
        }
        let delay = tracker.delay_needed();
        assert!(
            delay > Duration::ZERO,
            "should need delay at limit: {delay:?}"
        );
    }

    #[test]
    fn under_limit_no_delay() {
        let mut tracker = RateLimitTracker::new();
        tracker.update_policy(RateLimitPolicy::parse("5:10:60").unwrap());
        tracker.record_request();
        tracker.record_request();
        assert_eq!(tracker.delay_needed(), Duration::ZERO);
    }

    #[test]
    fn at_limit_needs_delay_with_margin() {
        let mut tracker = RateLimitTracker::new();
        // 2 requests per 10 seconds
        tracker.update_policy(RateLimitPolicy::parse("2:10:60").unwrap());
        tracker.record_request();
        tracker.record_request();
        let delay = tracker.delay_needed();
        // Should need ~10s + 1.5s margin
        assert!(delay > Duration::from_secs(10), "delay was {delay:?}");
        assert!(
            delay <= Duration::from_millis(11_600),
            "delay was {delay:?}"
        );
    }

    #[test]
    fn penalty_block() {
        let mut tracker = RateLimitTracker::new();
        tracker.block_for(Duration::from_secs(30));
        let delay = tracker.delay_needed();
        assert!(delay > Duration::from_secs(29));
        assert!(delay <= Duration::from_secs(30));
    }

    #[test]
    fn sync_server_state_adds_requests() {
        let mut tracker = RateLimitTracker::new();
        tracker.update_policy(RateLimitPolicy::parse("5:10:60").unwrap());
        // Server says 4 requests in window.
        tracker.sync_server_state("4:10:0");
        assert_eq!(tracker.request_times.len(), 4);
    }

    #[test]
    fn sync_server_state_penalty_blocks() {
        let mut tracker = RateLimitTracker::new();
        // Server says penalty of 60 seconds.
        tracker.sync_server_state("12:6:60");
        let delay = tracker.delay_needed();
        assert!(delay > Duration::from_secs(59), "delay was {delay:?}");
    }
}
