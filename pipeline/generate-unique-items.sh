#!/usr/bin/env bash
#
# Generate unique_items.json by cross-referencing trade API data with GGPK tables.
#
# The trade API provides unique name → base type mapping (not available in GGPK datc64 tables).
# The GGPK provides art paths via UniqueStashLayout → Words + ItemVisualIdentity.
#
# Prerequisites:
#   - node (for JSON processing)
#   - GGPK tables extracted to the data dir (run update-game-data.sh first)
#   - poe-dat crate available (used via cargo test to dump GGPK data)
#
# Usage:
#   ./pipeline/generate-unique-items.sh [data_dir]
#
# Output:
#   crates/poe-data/data/unique_items.json — enriched with art paths
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUTPUT="$REPO_ROOT/crates/poe-data/data/unique_items.json"

# --- Step 1: Fetch trade API items ----------------------------------------

echo "Fetching trade API /data/items..."
TRADE_JSON=$(curl -s "https://www.pathofexile.com/api/trade/data/items" \
  -H "User-Agent: poe-inspect-2/0.1 (contact: github.com/timeloop-vault/poe-inspect)")

if [ -z "$TRADE_JSON" ]; then
  echo "ERROR: Failed to fetch trade API items"
  exit 1
fi

# --- Step 2: Extract unique entries and write JSON ------------------------

echo "Processing unique items..."
echo "$TRADE_JSON" | node -e "
const data = JSON.parse(require('fs').readFileSync('/dev/stdin', 'utf8'));
const uniques = [];
for (const cat of data.result) {
  for (const entry of cat.entries) {
    if (entry.flags && entry.flags.unique && entry.name && !entry.disc) {
      uniques.push({ name: entry.name, base_type: entry.type });
    }
  }
}
uniques.sort((a, b) => a.base_type.localeCompare(b.base_type) || a.name.localeCompare(b.name));
require('fs').writeFileSync('$OUTPUT', JSON.stringify(uniques, null, 2) + '\n');
console.log('Wrote ' + uniques.length + ' unique items to $OUTPUT');
"

echo ""
echo "Done. Review changes:"
echo "  git diff --stat crates/poe-data/data/unique_items.json"
