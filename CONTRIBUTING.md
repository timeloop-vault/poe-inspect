# Contributing to PoE Inspect

Thanks for your interest in contributing! This project is a real-time item evaluation overlay for Path of Exile.

## Getting Started

### Prerequisites

- **Rust** (edition 2024, stable toolchain)
- **Node.js** (for the Tauri frontend)
- **cmake** (only needed for `pipeline/extract-game-data`, not for the main build)

### Setup

```sh
git clone https://github.com/timeloop-vault/poe-inspect.git
cd poe-inspect

# Activate the pre-commit hook
git config core.hooksPath .githooks

# Install frontend dependencies
cd app && npm install && cd ..

# Build the workspace
cargo build

# Build the Tauri app
cargo build --manifest-path app/src-tauri/Cargo.toml
```

### Pre-commit Checks

A pre-commit hook runs automatically. You can also run manually:

```sh
cargo fmt
cargo clippy --workspace --tests
cargo clippy --manifest-path app/src-tauri/Cargo.toml --tests
cd app && npx tsc --noEmit
cd app && npx biome check --write --unsafe .
```

All checks must pass before committing. Zero warnings policy for clippy and biome.

## Project Structure

See [CLAUDE.md](CLAUDE.md) for the full architecture, dependency graph, and conventions.

Key points:
- **Rust workspace** with crates in `crates/` — the data pipeline and evaluation engine
- **Tauri v2 app** in `app/` — the overlay UI
- **pipeline/** — standalone tools for extracting game data (not part of the workspace)
- **fixtures/** — shared test data

## Updating Game Data

When a new PoE league or patch drops, game data files need updating:

```sh
./pipeline/update-game-data.sh <path_to_poe_install>
```

This extracts datc64 tables from the GGPK and copies them to `crates/poe-data/data/`.

## Pull Requests

- Keep PRs focused — one feature or fix per PR
- Follow existing code conventions (see CLAUDE.md)
- Ensure all pre-commit checks pass
- Add tests for new functionality where practical

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
