//! Trade stats index: maps between GGPK stat IDs and trade API stat IDs.
//!
//! The join key is template text (e.g., `"+# to maximum Life"`) which appears
//! in both our `ReverseIndex` (from stat_descriptions.txt) and the trade API's
//! `/data/stats` endpoint.

use std::collections::HashMap;

use poe_data::GameData;

use crate::types::{TradeStatEntry, TradeStatsIndex, TradeStatsResponse};

impl TradeStatsIndex {
    /// Build an index from a raw trade API response.
    ///
    /// Cross-references with `GameData.reverse_index` to build the bidirectional
    /// GGPK stat ID ↔ trade stat number mapping.
    pub fn from_response(response: &TradeStatsResponse, game_data: &GameData) -> Self {
        let mut by_template: HashMap<String, Vec<TradeStatEntry>> = HashMap::new();
        let mut by_trade_id: HashMap<String, TradeStatEntry> = HashMap::new();
        let mut ggpk_to_trade: HashMap<String, u64> = HashMap::new();
        let mut trade_to_ggpk: HashMap<u64, Vec<String>> = HashMap::new();

        let mut total_entries = 0u32;
        let mut matched = 0u32;
        let mut unmatched_templates: Vec<String> = Vec::new();

        for category in &response.result {
            for entry in &category.entries {
                total_entries += 1;

                // Index by full trade ID
                by_trade_id.insert(entry.id.clone(), entry.clone());

                // Index by normalized template text
                let normalized = normalize_template(&entry.text);
                by_template
                    .entry(normalized.clone())
                    .or_default()
                    .push(entry.clone());

                // Cross-reference with GGPK stat IDs via ReverseIndex
                if let Some(trade_num) = extract_stat_number(&entry.id) {
                    if let Some(ri) = &game_data.reverse_index {
                        if let Some(stat_ids) = ri.stat_ids_for_template(&normalized) {
                            matched += 1;
                            for stat_id in &stat_ids {
                                ggpk_to_trade.insert(stat_id.clone(), trade_num);
                            }
                            trade_to_ggpk
                                .entry(trade_num)
                                .or_default()
                                .extend(stat_ids);
                        } else {
                            unmatched_templates.push(entry.text.clone());
                        }
                    }
                }
                // Skip pseudo/named stats for cross-referencing (they don't have stat_ numbers)
            }
        }

        tracing::info!(
            total_entries,
            matched,
            unmatched = unmatched_templates.len(),
            "Trade stats index built"
        );

        if !unmatched_templates.is_empty() && unmatched_templates.len() <= 20 {
            tracing::debug!(
                ?unmatched_templates,
                "Trade stats without reverse index match"
            );
        }

        Self {
            by_template,
            by_trade_id,
            ggpk_to_trade,
            trade_to_ggpk,
        }
    }

    /// Look up trade stat entries by normalized template text.
    pub fn entries_for_template(&self, template: &str) -> Option<&Vec<TradeStatEntry>> {
        self.by_template.get(&normalize_template(template))
    }

    /// Look up a trade stat entry by its full trade ID.
    pub fn entry_by_trade_id(&self, trade_id: &str) -> Option<&TradeStatEntry> {
        self.by_trade_id.get(trade_id)
    }

    /// Get the trade stat number for a GGPK stat ID.
    ///
    /// Returns the numeric portion (e.g., `3299347043` for `"base_maximum_life"`).
    /// Caller adds the category prefix (`"explicit."`, `"implicit."`, etc.).
    pub fn trade_stat_number(&self, ggpk_stat_id: &str) -> Option<u64> {
        self.ggpk_to_trade.get(ggpk_stat_id).copied()
    }

    /// Get GGPK stat IDs for a trade stat number.
    pub fn ggpk_stat_ids(&self, trade_num: u64) -> Option<&Vec<String>> {
        self.trade_to_ggpk.get(&trade_num)
    }

    /// Build the full trade stat ID from a GGPK stat ID and mod category.
    ///
    /// Example: `("base_maximum_life", "explicit")` → `"explicit.stat_3299347043"`.
    pub fn full_trade_id(&self, ggpk_stat_id: &str, category: &str) -> Option<String> {
        let num = self.trade_stat_number(ggpk_stat_id)?;
        Some(format!("{category}.stat_{num}"))
    }

    /// Total number of trade stat entries.
    pub fn len(&self) -> usize {
        self.by_trade_id.len()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.by_trade_id.is_empty()
    }

    /// Number of GGPK stat IDs that have a trade mapping.
    pub fn mapped_stat_count(&self) -> usize {
        self.ggpk_to_trade.len()
    }
}

/// Normalize template text for matching.
///
/// Both our `ReverseIndex` and the trade API use `#` as value placeholders.
/// Normalize by lowercasing and trimming whitespace.
fn normalize_template(text: &str) -> String {
    text.to_lowercase().trim().to_string()
}

/// Extract the numeric stat ID from a trade stat ID.
///
/// `"explicit.stat_3299347043"` → `Some(3299347043)`
/// `"pseudo.pseudo_total_life"` → `None`
fn extract_stat_number(trade_id: &str) -> Option<u64> {
    let after_dot = trade_id.split('.').nth(1)?;
    let after_stat = after_dot.strip_prefix("stat_")?;
    after_stat.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_stat_number_numeric() {
        assert_eq!(
            extract_stat_number("explicit.stat_3299347043"),
            Some(3_299_347_043)
        );
    }

    #[test]
    fn extract_stat_number_named() {
        assert_eq!(extract_stat_number("pseudo.pseudo_total_life"), None);
    }

    #[test]
    fn normalize_template_case_insensitive() {
        assert_eq!(
            normalize_template("+# to Maximum Life"),
            normalize_template("+# to maximum Life")
        );
    }
}
