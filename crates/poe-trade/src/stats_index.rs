//! Trade stats index: maps between GGPK stat IDs and trade API stat IDs.
//!
//! The join key is template text (e.g., `"+# to maximum Life"`) which appears
//! in both our `ReverseIndex` (from `stat_descriptions.txt`) and the trade API's
//! `/data/stats` endpoint.

use std::collections::HashMap;

use poe_data::{GameData, ReverseIndex};

use crate::types::{TradeStatEntry, TradeStatsIndex, TradeStatsResponse};

/// Result of building the trade stats index, with match statistics.
#[derive(Debug)]
pub struct IndexBuildResult {
    pub index: TradeStatsIndex,
    /// Number of trade entries with `stat_` IDs that matched a reverse index template.
    pub matched: u32,
    /// Number of trade entries with `stat_` IDs that did NOT match.
    pub unmatched: u32,
    /// Total trade entries (including pseudo/named stats that aren't cross-referenced).
    pub total: u32,
    /// Templates from trade API that didn't match any reverse index entry.
    pub unmatched_templates: Vec<String>,
}

impl TradeStatsIndex {
    /// Build an index from a raw trade API response.
    ///
    /// Cross-references with `GameData.reverse_index` to build the bidirectional
    /// GGPK stat ID ↔ trade stat number mapping.
    pub fn from_response(response: &TradeStatsResponse, game_data: &GameData) -> IndexBuildResult {
        let mut by_template: HashMap<String, Vec<TradeStatEntry>> = HashMap::new();
        let mut by_trade_id: HashMap<String, TradeStatEntry> = HashMap::new();
        let mut ggpk_to_trade: HashMap<String, u64> = HashMap::new();
        let mut trade_to_ggpk: HashMap<u64, Vec<String>> = HashMap::new();

        // Build a case-insensitive lookup from reverse index template keys.
        //
        // Two issues to handle:
        // 1. Case: ReverseIndex uses casing from stat_descriptions.txt, trade API
        //    uses its own casing. Normalize both to lowercase.
        // 2. Signed placeholders: stat_descriptions.txt uses `{0:+d}` format specifier
        //    which means "display with sign". The `+` is NOT part of the template key
        //    (it becomes just `#`), but the trade API text INCLUDES the `+` sign
        //    (showing `+#`). So we must try matching with `+#` → `#` fallback.
        let ri_case_map: HashMap<String, String> = game_data
            .reverse_index
            .as_ref()
            .map(|ri| {
                ri.template_keys()
                    .into_iter()
                    .map(|k| (k.to_lowercase(), k))
                    .collect()
            })
            .unwrap_or_default();

        let mut total = 0u32;
        let mut matched = 0u32;
        let mut unmatched_templates: Vec<String> = Vec::new();

        for category in &response.result {
            for entry in &category.entries {
                total += 1;

                // Index by full trade ID
                by_trade_id.insert(entry.id.clone(), entry.clone());

                // Index by normalized template text (lowercase)
                let normalized = entry.text.to_lowercase().trim().to_string();
                by_template
                    .entry(normalized.clone())
                    .or_default()
                    .push(entry.clone());

                // Cross-reference: only for stat_ entries (not pseudo/named)
                if let Some(trade_num) = extract_stat_number(&entry.id) {
                    if let Some(ri) = &game_data.reverse_index {
                        // Try exact match first, then fallback with +# → # substitution.
                        // stat_descriptions.txt uses `{0:+d}` format specifier for signed
                        // values. The `+` is NOT literal text, so our template key has
                        // just `#`, while the trade API shows `+#`.
                        let stat_ids = resolve_template(&normalized, &ri_case_map, ri);

                        if let Some(stat_ids) = stat_ids {
                            matched += 1;
                            for stat_id in &stat_ids {
                                ggpk_to_trade.insert(stat_id.clone(), trade_num);
                            }
                            trade_to_ggpk.entry(trade_num).or_default().extend(stat_ids);
                        } else {
                            unmatched_templates.push(entry.text.clone());
                        }
                    }
                }
            }
        }

        let unmatched = unmatched_templates.len() as u32;

        tracing::info!(
            total,
            matched,
            unmatched,
            ggpk_mapped = ggpk_to_trade.len(),
            "Trade stats index built"
        );

        if !unmatched_templates.is_empty() && unmatched_templates.len() <= 30 {
            tracing::debug!(
                ?unmatched_templates,
                "Trade stats without reverse index match"
            );
        }

        let index = Self {
            by_template,
            by_trade_id,
            ggpk_to_trade,
            trade_to_ggpk,
        };

        IndexBuildResult {
            index,
            matched,
            unmatched,
            total,
            unmatched_templates,
        }
    }

    /// Look up trade stat entries by template text.
    pub fn entries_for_template(&self, template: &str) -> Option<&Vec<TradeStatEntry>> {
        let normalized = template.to_lowercase().trim().to_string();
        self.by_template.get(&normalized)
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

    /// Save the trade stats response to disk for caching.
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the file can't be created or serialization fails.
    pub fn save_response(
        response: &TradeStatsResponse,
        path: &std::path::Path,
    ) -> std::io::Result<()> {
        let file = std::fs::File::create(path)?;
        let writer = std::io::BufWriter::new(file);
        serde_json::to_writer(writer, response).map_err(std::io::Error::other)
    }

    /// Load a cached trade stats response from disk.
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the file can't be read or deserialization fails.
    pub fn load_response(path: &std::path::Path) -> std::io::Result<TradeStatsResponse> {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        serde_json::from_reader(reader).map_err(std::io::Error::other)
    }
}

/// Try to resolve a trade API template against the reverse index.
///
/// Tries multiple normalizations in order:
/// 1. Exact (lowercased) match
/// 2. Strip `+` before `#` (handles `:+d` signed format specifiers)
/// 3. Strip ` (Local)` suffix (trade API appends this to local weapon/armour mods)
/// 4. Both `+#` → `#` AND strip ` (Local)`
fn resolve_template(
    normalized: &str,
    ri_case_map: &HashMap<String, String>,
    ri: &ReverseIndex,
) -> Option<Vec<String>> {
    // 1. Exact
    if let Some(ids) = ri_case_map
        .get(normalized)
        .and_then(|orig| ri.stat_ids_for_template(orig))
    {
        return Some(ids);
    }

    // 2. Strip +# → #
    let without_plus = normalized.replace("+#", "#");
    if without_plus != *normalized {
        if let Some(ids) = ri_case_map
            .get(&without_plus)
            .and_then(|orig| ri.stat_ids_for_template(orig))
        {
            return Some(ids);
        }
    }

    // 3. Strip trade API stat suffixes (e.g., "(Local)", "(Shields)")
    //    These suffixes are PoE domain knowledge defined in poe-data::domain.
    let without_suffix = poe_data::domain::TRADE_STAT_SUFFIXES
        .iter()
        .find_map(|suffix| normalized.strip_suffix(&suffix.to_lowercase()));
    if let Some(stripped) = without_suffix {
        let stripped = stripped.to_string();
        if let Some(ids) = ri_case_map
            .get(&stripped)
            .and_then(|orig| ri.stat_ids_for_template(orig))
        {
            return Some(ids);
        }

        // 4. Both: strip +# AND (local)
        let both = stripped.replace("+#", "#");
        if both != stripped {
            if let Some(ids) = ri_case_map
                .get(&both)
                .and_then(|orig| ri.stat_ids_for_template(orig))
            {
                return Some(ids);
            }
        }
    }

    None
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
    fn extract_stat_number_pipe_id() {
        // Imbued entries use pipe format: "imbued.pseudo_built_in_support|3092222470"
        assert_eq!(
            extract_stat_number("imbued.pseudo_built_in_support|3092222470"),
            None
        );
    }
}
