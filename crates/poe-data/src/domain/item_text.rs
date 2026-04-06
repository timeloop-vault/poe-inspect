// ── Usage instruction prefixes ──────────────────────────────────────────
//
// WHY HARDCODED: The PoE client appends usage instruction text to item
// tooltips. These strings exist in the GGPK `currencyitems` table (1,925
// rows, per-item), but we don't extract full usage text yet. The prefixes
// are stable across leagues — GGG doesn't change how "Right click" works.
//
// TODO: Extract from `currencyitems.datc64` per-item usage descriptions
// to make this fully data-driven. See `_reference/ggpk-data-3.28/`.
//
// Confirmed via 3.28 Mirage.

/// Known prefixes for GGG usage instruction text.
///
/// Used by poe-item's resolver to identify and drop usage instruction
/// sections during item text classification.
pub const USAGE_INSTRUCTION_PREFIXES: &[&str] = &[
    "Right click",
    "Place into",
    "Travel to",
    "Can be used",
    "This is a Support Gem",
    "Shift click to unstack",
    "Use Intelligence",
    "Give this",
];

/// Whether a line starts with a known GGG usage instruction prefix.
#[must_use]
pub fn is_usage_instruction(line: &str) -> bool {
    USAGE_INSTRUCTION_PREFIXES
        .iter()
        .any(|prefix| line.starts_with(prefix))
}
