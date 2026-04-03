#!/usr/bin/env bash
#
# Extract core datc64 tables from a PoE install and copy them into the repo.
#
# Prerequisites:
#   - cmake on PATH (for poe-bundle's Oodle C++ library)
#
# Usage:
#   ./pipeline/update-game-data.sh <poe_install_dir>
#
# Examples:
#   ./pipeline/update-game-data.sh "D:/games/PathofExile"
#   ./pipeline/update-game-data.sh "/home/user/.steam/steamapps/common/Path of Exile"
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DATA_DIR="$REPO_ROOT/crates/poe-data/data"
EXTRACT_CRATE="$SCRIPT_DIR/extract-game-data"

# --- Args -------------------------------------------------------------------

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <poe_install_dir>"
  echo ""
  echo "  poe_install_dir  Path to your PoE installation (contains Content.ggpk)"
  exit 1
fi

POE_PATH="$1"

if [[ ! -d "$POE_PATH" ]]; then
  echo "Error: PoE install directory not found: $POE_PATH"
  exit 1
fi

# --- Build extract-game-data if needed --------------------------------------

echo "Building extract-game-data..."
cargo build --release --manifest-path "$EXTRACT_CRATE/Cargo.toml"

EXTRACT_BIN="$EXTRACT_CRATE/target/release/extract-game-data"
if [[ ! -f "$EXTRACT_BIN" ]]; then
  EXTRACT_BIN="${EXTRACT_BIN}.exe"
fi

# --- Extract and copy -------------------------------------------------------

echo ""
echo "Extracting core tables from: $POE_PATH"
echo "Output dir: $DATA_DIR"
echo ""

ART_DIR="$REPO_ROOT/app/src/assets/uniques"
UNIQUE_ITEMS="$DATA_DIR/unique_items.json"

"$EXTRACT_BIN" -p "$POE_PATH" -o "$DATA_DIR" --art-dir "$ART_DIR" --unique-items "$UNIQUE_ITEMS"

echo ""
echo "Next steps:"
echo "  1. Run tests:  cargo test -p poe-dat -p poe-data -p poe-item"
echo "  2. Review:     git diff --stat crates/poe-data/data/ app/src/assets/uniques/"
echo "  3. Commit:     git add crates/poe-data/data/ app/src/assets/uniques/ && git commit -m 'Update game data for <league>'"
