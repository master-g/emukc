# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

EmuKC is a server-side emulator for the web browser game Kantai Collection (KanColle), written in Rust. It implements the game's API server, allowing a browser client to connect and play against locally-stored game data.

## Build & Development Commands

```bash
# Build
cargo build
cargo build --release

# Run (requires emukc.config.toml and bootstrapped data)
cargo run -- serve

# Quick dev: create account + start server
cargo run -- new-session -u <username> -p <password>

# Bootstrap game data (downloads manifests/resources)
cargo run -- bootstrap

# Run all tests
cargo test

# Run integration gameplay tests
cargo test --test gameplay_tests

# Run a specific gameplay test
cargo test --test gameplay_tests test_composition_exact_match_requirement

# Run crate-level tests
cargo test -p emukc_cache
cargo test -p emukc_gameplay

# Run examples (used as manual test harnesses)
cargo run --example model_loader
cargo run --example quest_test

# Lint
cargo clippy --workspace

# Format
cargo fmt --all
```

## Architecture

### Layered Crate Structure

The workspace follows a strict layered architecture. Dependencies flow downward only:

```
emukc (binary)          - CLI + HTTP server (axum)
  └── emukc_internal    - Re-exports all crates as a unified facade
      ├── emukc_gameplay  - Game logic traits + implementations
      ├── emukc_db        - SeaORM entities + SQLite persistence
      ├── emukc_model     - Data models, API types, third-party data
      ├── emukc_bootstrap - Downloads and prepares game data files
      ├── emukc_cache     - Game resource caching (redb key-value store)
      ├── emukc_network   - HTTP client for fetching remote resources
      ├── emukc_crypto    - Hashing, token generation
      ├── emukc_time      - Time utilities (re-exports chrono)
      ├── emukc_log       - Logging setup (tracing)
      ├── emukc_macros    - Proc macros
      └── emukc_app       - Runtime setup (mimalloc, stack size)
```

### Key Architectural Patterns

**Gameplay trait system** (`emukc_gameplay`): Each game domain (ships, quests, materials, fleets, etc.) defines an async trait (e.g., `ShipOps`, `QuestOps`, `MaterialOps`). All traits have blanket implementations for any type implementing `HasContext`, which provides access to `DbConn` and `Codex`. The top-level `Gameplay` trait composes all domain traits.

**Codex** (`emukc_model::codex::Codex`): An in-memory read-only snapshot of all game manifest data (ship stats, equipment data, quest definitions, etc.), loaded from disk at startup. It is the single source of truth for game configuration.

**Database entities** (`emukc_db::entity`): SeaORM entities organized under `entity::user` (accounts, tokens) and `entity::profile` (all per-player game state: ships, items, quests, fleets, settings, etc.).

**API response format**: KanColle API responses use a `svdata=` JSON prefix. All KCSAPI handlers return `KcApiResponse` which wraps `api_result`, `api_result_msg`, and `api_data`. See `src/bin/net/resp/kcs.rs`.

### Binary Structure (`src/bin/`)

- `emukcd.rs` - Entry point
- `cli/` - CLI commands (serve, bootstrap, cache, dev tools)
- `net/` - HTTP server
  - `router/kcsapi/` - Game API handlers mirroring KanColle's URL structure (`api_get_member/`, `api_req_kousyou/`, `api_port/`, etc.)
  - `router/api/v1/` - Custom REST API (auth, debug)
  - `auth.rs` - Session/token middleware
  - `resp/` - Response types

### Adding a New Game API

1. **Database**: Add SeaORM entity in `crates/emukc_db/src/entity/profile/`
2. **Model**: Add API types in `crates/emukc_model/src/kc2/`
3. **Gameplay**: Add `XxxOps` trait in `crates/emukc_gameplay/src/game/`, with `_impl` functions for reuse, blanket impl on `HasContext`
4. **Handler**: Add axum handler in `src/bin/net/router/kcsapi/`, register route in the module's `router()` function

### Gameplay `_impl` Pattern

Internal gameplay functions are suffixed with `_impl` (e.g., `add_ship_impl`, `add_material_impl`) and take a generic `C: ConnectionTrait` parameter. This allows them to participate in database transactions started by the public trait methods and be called from other gameplay modules.

## Code Style

- **Rust edition 2024**, stable toolchain, minimum rust-version 1.94.0
- **Hard tabs** for indentation (see `.rustfmt.toml`)
- `unsafe_code` is **denied** workspace-wide
- `missing_docs` is warned
- Imports use `emukc_internal::prelude::*` in the binary crate for convenience
- Configuration: `emukc.config.toml` (see `emukc.config.example.toml`)
- Database: SQLite via SeaORM, in-memory DB (`new_mem_db()`) for tests
- Pre-commit hooks are expected (see README)

## Testing Conventions

Integration tests live in `tests/gameplay_tests/` and test gameplay logic directly (no HTTP). Each test uses an independent in-memory database. The `Codex` is loaded from `.data/codex` on disk (requires prior bootstrap).
