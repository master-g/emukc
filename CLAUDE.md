# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

EmuKC is a Kancolle (Kantai Collection) game server emulator written in Rust. It emulates the game's HTTP API endpoints, allowing clients to interact with a local server instead of the official DMM servers.

## Architecture

### Layered Architecture

The codebase follows a strict layered architecture for implementing game APIs:

1. **Database Layer** (`emukc_db`) - Raw database operations and schema
2. **Model Layer** (`emukc_model`) - Data structures and game entities
3. **Gameplay Layer** (`emukc_gameplay`) - Game logic, rules, and state management
4. **API Handler Layer** (`src/bin/net/router/kcsapi/`) - HTTP endpoints that call gameplay functions

**Critical Rule**: When implementing new APIs, always follow this flow: database → model → gameplay → API handler. Never skip layers or put game logic directly in API handlers.

### Workspace Crates

- `emukc_app` - Application template and utilities
- `emukc_bootstrap` - Downloads and parses third-party game data resources
- `emukc_cache` - Caching implementation for assets and responses
- `emukc_crypto` - Game-specific cryptographic primitives (not security-focused)
- `emukc_db` - Database layer using SeaORM with SQLite
- `emukc_dylib` - Dynamic library for faster incremental builds
- `emukc_gameplay` - Core game logic and state management
- `emukc_internal` - Convenience crate that re-exports all other crates
- `emukc_log` - Logging utilities
- `emukc_macros` - Procedural macros
- `emukc_model` - Game data models and structures
- `emukc_network` - Network utilities
- `emukc_time` - Time-related utilities

### API Structure

KanColle API endpoints are organized under `src/bin/net/router/kcsapi/` by category:
- `api_get_member/` - Player data retrieval
- `api_port/` - Port/home screen data
- `api_req_furniture/` - Furniture and music
- `api_req_hensei/` - Fleet composition
- `api_req_hokyu/` - Resupply
- `api_req_kaisou/` - Ship modification
- `api_req_kousyou/` - Construction and arsenal
- `api_req_member/` - Player actions
- `api_req_nyukyo/` - Repair dock
- `api_req_quest/` - Quest/mission system
- `api_start2/` - Initial game data

See `apilist.md` for implementation status and priorities.

## Development Commands

### Build and Run

```bash
# Build the project
cargo build

# Build with release optimizations
cargo build --release

# Run the server (development)
cargo run --bin emukcd serve

# Run with faster incremental builds
cargo run --features dynamic_linking --bin emukcd serve
```

### Bootstrap

Before first run, bootstrap downloads required game data:

```bash
cargo run --bin emukcd bootstrap
# or
emukc bootstrap
```

### Testing

```bash
# Run all tests
cargo test

# Run tests for a specific package
cargo test -p emukc_gameplay

# Run a specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run examples (integration tests)
cargo run --example model_loader
cargo run --example bootstrap_download
cargo run --example quest_test
```

### Code Quality

```bash
# Check code without building
cargo check

# Run clippy linter
cargo clippy

# Format code
cargo fmt

# Run pre-commit hooks
pre-commit run --all-files
```

### API Testing

Use `atac` for interactive API testing:

```bash
atac -d doc/.atac
```

## Implementation Guidelines

### Adding New KanColle APIs

When implementing a new KanColle API endpoint (see `apilist.md` for missing APIs):

1. **Database**: Add tables/queries in `crates/emukc_db/`
2. **Model**: Define data structures in `crates/emukc_model/`
3. **Gameplay**: Implement game logic in `crates/emukc_gameplay/`
4. **Handler**: Create HTTP handler in `src/bin/net/router/kcsapi/api_*/`
5. **Router**: Register route in the appropriate `mod.rs` router

Example structure for `api_req_sortie/battle`:
- Gameplay logic → `crates/emukc_gameplay/src/game/sortie.rs`
- HTTP handler → `src/bin/net/router/kcsapi/api_req_sortie/battle.rs`
- Router registration → `src/bin/net/router/kcsapi/mod.rs`

### Response Format

All KanColle API responses follow the format:
```
svdata={json_response}
```

The `mocking_middleware` in `kcsapi/mod.rs` handles this automatically.

### Authentication

API endpoints under `/kcsapi/` require authentication via `kcs_api_auth_middleware`. The `/api_world/` endpoints are public (for login/registration).

## Configuration

Server configuration is in `emukc.config.toml`:
- `bind` - Server address and port
- `tls_cert` / `tls_key` - HTTPS certificate paths (optional)

See `HTTPS.md` for HTTPS setup instructions.

## Linting Rules

The project enforces strict linting (see `Cargo.toml` workspace.lints):
- `unsafe_code = "deny"` - No unsafe code allowed
- `missing_docs = "warn"` - Document public APIs
- Various clippy warnings for code quality

## Data Sources

Game data is sourced from multiple community projects (see README.md). The bootstrap process downloads and processes this data into the local cache.
