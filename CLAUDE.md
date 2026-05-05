# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Approach

- Think before acting. Read existing files before writing code.
- Be concise in output but thorough in reasoning.
- Prefer editing over rewriting whole files.
- Do not re-read files you have already read unless the file may have changed.
- Test your code before declaring done.
- No sycophantic openers or closing fluff.
- Keep solutions simple and direct.
- User instructions always override this file.
- Never include AI attribution (Co-Authored-By, "Generated with Claude", etc.) in commit messages.

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

# Run battle validation tests
cargo test -p emukc_gameplay sortie_battle_response_passes_battle_rule_validation
cargo test -p emukc_gameplay sortie_battle_validation_reports_invalid_enemy_ids

# Validate a battle payload against client-derived rules
cargo run -- battle validate --input <battle.json>

# Diagnose a missing battle resource incident
cargo run -- battle analyze-incident --input <battle.json> --missing-url <url>

# Run crate-level tests
cargo test -p emukc_cache
cargo test -p emukc_gameplay
cargo test -p emukc_bootstrap battle_rules

# Run examples (used as manual test harnesses)
cargo run --example model_loader
cargo run --example bootstrap_download
cargo run --example dump_tree
cargo run --example kache_test

# main-decoder (Bun + TypeScript)
cd main-decoder && bun run check
cd main-decoder && bun test
cd main-decoder && bun run decode
cd main-decoder && bun run decode -- --sync-battle-assets

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
- `cli/` - CLI commands (serve, bootstrap, cache, battle diagnostics, dev tools)
- `net/` - HTTP server
  - `router/kcsapi/` - Game API handlers mirroring KanColle's URL structure (`api_get_member/`, `api_req_kousyou/`, `api_port/`, etc.)
  - `router/api/v1/` - Custom REST API (auth, debug)
  - `auth.rs` - Session/token middleware
  - `resp/` - Response types

### Client-Derived Battle Validation

The repo now includes a tracked `main-decoder/` subproject that decodes `main.js` and extracts battle knowledge assets. These assets are synced into `crates/emukc_bootstrap/assets/` and then consumed by Rust-side battle diagnostics.

Key battle assets:

- `crates/emukc_bootstrap/assets/battle_protocol_fields.json`
- `crates/emukc_bootstrap/assets/battle_resource_rules.json`
- `crates/emukc_bootstrap/assets/battle_module_index.json`
- `crates/emukc_bootstrap/assets/battle_slot_resource_triggers.json`

Important boundary:

- `validate_day_battle_response(...)` and `analyze_day_battle_incident(...)` are explicit diagnostic tools, not runtime auto-checks.
- If you need battle diagnosis, use the `battle` CLI commands. Do not assume sortie/practice handlers run these checks automatically.

Typical workflow for a bad battle payload:

1. Save the KC API response or `api_data` JSON to a file.
2. Run `cargo run -- battle validate --input <battle.json>`.
3. If a client tried to load a missing resource, run `cargo run -- battle analyze-incident --input <battle.json> --missing-url <url>`.
4. If battle knowledge changed, refresh with `cd main-decoder && bun run decode -- --sync-battle-assets`.

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
- Always use soft tabs, accroding to `.editorconfig` and `.rustfmt.toml`.

## Balance Defaults Policy

Any change to a `Default` impl in `crates/emukc_model/src/codex/` that affects gameplay numerics (XP multipliers, drop rates, repair times, material caps) MUST:

1. Be in its own commit, separate from infrastructure or refactor work.
2. Use commit prefix `feat(balance):` for new behavior or `chore(balance):` for value tuning.
3. List the previous value(s) in the commit body.
4. Update or reference an openspec proposal under `openspec/changes/`.
5. Add or update a regression test asserting the new value, so future accidental flips fail CI.

Pure boolean QoL defaults (e.g., picture-book unlocks) are exempt from rule 5 but still subject to rules 1-4.

## Testing Conventions

Integration tests live in `tests/gameplay_tests/` and test gameplay logic directly (no HTTP). Each test uses an independent in-memory database. The `Codex` is loaded from `.data/codex` on disk (requires prior bootstrap).

Battle diagnostics also have two dedicated test layers:

- `main-decoder/test/` for TypeScript-side battle knowledge extraction
- `crates/emukc_bootstrap/src/battle_rules.rs` for Rust-side validator / incident analysis
