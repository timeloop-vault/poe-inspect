#!/usr/bin/env bash
#
# Generate unique_items.json by cross-referencing trade API data with GGPK art.
#
# The trade API provides unique name → base type mapping (not in GGPK datc64).
# The GGPK provides art paths via extract-game-data --art-dir (→ _art_map.json).
#
# Prerequisites:
#   - node (for JSON processing)
#   - Run update-game-data.sh first (extracts tables + art)
#
# Usage:
#   ./pipeline/generate-unique-items.sh
#
# Output:
#   crates/poe-data/data/unique_items.json — with name, base_type, art fields
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUTPUT="$REPO_ROOT/crates/poe-data/data/unique_items.json"
ART_MAP="$REPO_ROOT/app/src/assets/uniques/_art_map.json"

# --- Step 1: Fetch trade API items ----------------------------------------

TRADE_CACHE="$REPO_ROOT/target/trade_items_cache.json"

echo "Fetching trade API /data/items..."
curl -s "https://www.pathofexile.com/api/trade/data/items" \
  -H "User-Agent: poe-inspect-2/0.1 (contact: github.com/timeloop-vault/poe-inspect)" \
  -o "$TRADE_CACHE"

if [ ! -s "$TRADE_CACHE" ]; then
  echo "ERROR: Failed to fetch trade API items"
  exit 1
fi

# --- Step 2: Extract unique entries, merge art, write JSON ----------------

echo "Processing unique items..."

# Read art map if available (produced by extract-game-data --art-dir)
ART_MAP_ARG=""
if [ -f "$ART_MAP" ]; then
  ART_MAP_ARG="$ART_MAP"
  echo "  Merging art from: $ART_MAP"
else
  echo "  No art map found — art fields will be empty"
fi

node -e "
const data = JSON.parse(require('fs').readFileSync('$TRADE_CACHE', 'utf8'));
const artMapPath = '$ART_MAP_ARG';
const artMap = artMapPath
  ? JSON.parse(require('fs').readFileSync(artMapPath, 'utf8'))
  : {};

const uniques = [];
let withArt = 0;
for (const cat of data.result) {
  for (const entry of cat.entries) {
    if (entry.flags && entry.flags.unique && entry.name && !entry.disc) {
      const art = artMap[entry.name] || '';
      if (art) withArt++;
      uniques.push({ name: entry.name, base_type: entry.type, art });
    }
  }
}
uniques.sort((a, b) => a.base_type.localeCompare(b.base_type) || a.name.localeCompare(b.name));
require('fs').writeFileSync('$OUTPUT', JSON.stringify(uniques, null, 2) + '\n');
console.log('Wrote ' + uniques.length + ' unique items (' + withArt + ' with art) to $OUTPUT');
"

echo ""
echo "Done. Review changes:"
echo "  git diff --stat crates/poe-data/data/unique_items.json"
