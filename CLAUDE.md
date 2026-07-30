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

## Agent skills

### Issue tracker

Issues and PRDs are tracked in GitHub Issues. See `docs/agents/issue-tracker.md`.

### Triage labels

Triage uses the five default canonical labels. See `docs/agents/triage-labels.md`.

### Domain docs

Domain documentation uses the single-context layout. See `docs/agents/domain.md`.

## Project Overview

EmuKC is a server-side emulator for the web browser game Kantai Collection (KanColle), written in Rust. It implements the game's API server, allowing a browser client to connect and play against locally-stored game data.

`docs/solutions/` contains documented solutions to past bugs and design decisions, organized by category with YAML frontmatter (`module`, `tags`, `problem_type`). Relevant when implementing or debugging in documented areas.

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

- **Rust edition 2024**, stable toolchain, minimum rust-version 1.96.0
- **Soft tabs** (4 spaces) for indentation (see `.rustfmt.toml` and `.editorconfig`)
- `unsafe_code` is **denied** workspace-wide
- `missing_docs` is warned
- Imports use `emukc_internal::prelude::*` in the binary crate for convenience
- Configuration: `emukc.config.toml` (see `emukc.config.example.toml`)
- Database: SQLite via SeaORM, in-memory DB (`new_mem_db()`) for tests
- Pre-commit hooks are expected (see README)
- Always use soft tabs, according to `.editorconfig` and `.rustfmt.toml`.

## Balance Defaults Policy

Any change to a `Default` impl in `crates/emukc_model/src/codex/` that affects gameplay numerics (XP multipliers, drop rates, repair times, material caps) MUST:

1. Be in its own commit, separate from infrastructure or refactor work.
2. Use commit prefix `feat(balance):` for new behavior or `chore(balance):` for value tuning.
3. List the previous value(s) in the commit body.
4. Reference a `docs/plans/` plan (ce-plan) describing the change.
5. Add or update a regression test asserting the new value, so future accidental flips fail CI.

Pure boolean QoL defaults (e.g., picture-book unlocks) are exempt from rule 5 but still subject to rules 1-4.

## Testing Conventions

Integration tests live in `tests/gameplay_tests/` and test gameplay logic directly (no HTTP). Each test uses an independent in-memory database. The `Codex` is loaded from `.data/codex` on disk (requires prior bootstrap).

Battle diagnostics also have two dedicated test layers:

- `main-decoder/test/` for TypeScript-side battle knowledge extraction
- `crates/emukc_bootstrap/src/battle_rules.rs` for Rust-side validator / incident analysis

## Do-Not-Modify Files

These files are checked in but are generated outputs, frozen baselines, or governing contracts. Hand-edits are silently lost on the next sync or break a baseline. Regenerate or amend them through their designated workflow instead.

- `crates/emukc_bootstrap/assets/*.json` — battle knowledge decoded from `main.js` by the `main-decoder` subproject and synced in via `cd main-decoder && bun run decode -- --sync-battle-assets` (see *Client-Derived Battle Validation*). Includes `battle_protocol_fields.json`, `battle_resource_rules.json`, `battle_module_index.json`, `battle_slot_resource_triggers.json`, etc.
- `main-decoder/out/battle/*.json` — the upstream decoder output that feeds the sync above. Regenerate with `bun run decode`, never edit.
- `tests/gameplay_tests/battle_golden.rs` — the frozen deterministic full-sortie transcript. If a legitimate logic change alters the outcome, re-freeze it deliberately and explain the diff in the PR; never hand-patch individual assertions.
- `Cargo.lock` — pinned dependency versions for a binary crate. Bump a specific dependency with `cargo update -p <crate>`, not by hand.

`docs/solutions/**` is not forbidden but is institutional knowledge: update it deliberately when the code it documents changes, and never delete or rewrite it as a side effect of unrelated work.

## PR Review Rules

There is no CI server — `.github/` holds agent prompts/skills, not workflows. Quality gates are enforced locally and at review time.

**Hard gates (must pass before requesting review):**

- `cargo fmt --all --check` — pre-commit hook.
- `cargo clippy --workspace -- -W warnings` — pre-commit hook; `-W warnings` means any warning fails.
- `cargo test` (plus the crate-specific and `--test gameplay_tests` subsets) green. Skipped or `#[ignore]`d tests must be surfaced in the PR — silent skips fail review.

**Change hygiene:**

- Conventional Commits (`feat:`, `fix:`, `test:`, `docs:`, `chore:`, `refactor:`). No AI attribution in the message.
- Balance/numeric changes follow the *Balance Defaults Policy* above: own commit, `feat(balance):`/`chore(balance):`, previous values in the body, a `docs/plans/` plan (ce-plan), regression test.
- Surgical changes: every changed line traces to the request. No opportunistic reformatting, no refactor of adjacent code, no removal of pre-existing dead code (flag it instead).

**Process gates (non-trivial changes):**

- New gameplay behavior, spec changes, or cross-crate contracts go through ce-plan first: draft a `docs/plans/` plan → implement against its Implementation Units → capture lessons with `/ce-compound` into `docs/solutions/` → verify with `/code-review` (or `/review`). Behavioral contracts that used to live in `openspec/specs/` are now captured knowledge under `docs/solutions/architecture-patterns/`.
- A diff that regenerates synced assets or re-freezes a golden transcript must explain why in the PR description.

**Review entry points:** `/code-review` and `/review` (standards + spec axes, run in parallel) for self-review before opening a PR; `/commit` runs fmt + clippy and drafts a conventional commit.

## 技术栈

- Rust workspace，edition 2024，`rust-version = "1.96.0"`；`rust-toolchain.toml` 使用 stable，并安装 `rustfmt`、`clippy`。
- HTTP/异步层：Axum 0.8、Tokio 1.x、Tower/Tower HTTP；日志使用 `tracing`。
- 持久化：SeaORM 1.1 + SQLite；资源缓存使用 redb；配置使用 TOML。
- `main-decoder/` 是 Bun + TypeScript 子项目，负责从 `main.js` 解码并生成 Rust 侧消费的战斗/资源资产。

## 命令

- 常用入口：`make build [PROFILE=debug|release]`、`make test`、`make clippy`、`make fmt`、`make serve`。
- 最终质量门：`cargo fmt --all --check`、`cargo clippy --workspace -- -W warnings`、`cargo test`。
- 数据准备：`make bootstrap`；需要已配置 `emukc.config.toml`，生成的本地数据位于忽略目录 `.data/`、`z/`。
- 解码并同步生成资产：`make decode-main`；先在 `main-decoder/` 安装 Bun 依赖并完成 bootstrap。
- 缓存：`make cache-make-list`、`make cache-populate CONCURRENT=16`。
- 可复现战斗：`make battle-sim SCENARIO=fresh_1_1 SEED=1`；可加 `FIND=night MAX_SEEDS=1000`。

## 代码风格

- 使用空格缩进：Rust/通用文件 4 空格，YAML/JSON 2 空格；以 `.editorconfig` 和 `.rustfmt.toml` 为准。
- 工作区禁止 `unsafe_code`，`missing_docs` 为 warning；不要绕过 Clippy 告警。
- 严格保持 crate 单向分层；跨领域写操作使用 gameplay trait，内部事务复用函数采用 `_impl` + `C: ConnectionTrait`。
- 二进制 crate 可使用 `emukc_internal::prelude::*`；新增代码优先沿用邻近模块既有模式。
- 提交使用 Conventional Commits，英文消息，不加入 AI attribution。

## 禁止文件

- 不手改 `crates/emukc_bootstrap/assets/*.json` 与 `main-decoder/out/battle/*.json`；通过 `make decode-main`/decoder 同步流程再生。
- 不逐项手改 `tests/gameplay_tests/battle_golden.rs`；行为变化时必须有意重新冻结并解释差异。
- 不手改 `Cargo.lock`；依赖升级使用 `cargo update -p <crate>`。
- 不提交本地/敏感状态：`.env`、`emukc.config.toml`、`.data/`、`z/`、`target/` 及其他 `.gitignore` 所有路径。
- `docs/solutions/**` 是制度化知识；只在对应行为或决策变化时有意更新，不随无关任务删除或重写。

## 审查规则

- 仓库没有 CI 服务；请求 review 前必须本地通过 fmt、严格 Clippy 与完整测试，并明确报告 skipped/ignored 测试。
- 改动必须 surgical：每行都能追溯到请求，不顺手重构、格式化邻近代码或清理既有死代码。
- gameplay 行为、规范变化或跨 crate 合约先写 `docs/plans/` 计划，按 Implementation Units 实施，再沉淀 `docs/solutions/` 并执行代码审查。
- gameplay 数值默认值遵守 Balance Defaults Policy：独立提交、`feat(balance):`/`chore(balance):`、正文列旧值、关联计划、添加回归测试。
- 生成资产或 frozen golden 的差异必须在 PR/交付说明中解释来源与必要性。
- 使用 GitHub Issues 与 `docs/agents/` 约定管理需求、triage 和领域上下文。

## 项目记忆 (回写约定)
跨会话的持久信息记录在 [PROJECT_MEMORY.md](./PROJECT_MEMORY.md)。
**完成每个重要任务后务必回写**: 把确认的决策写入「已验证的事实」、踩的坑写入「失败尝试」、用进展更新「上次会话」、把计划写入「下次运行」。
保持 PROJECT_MEMORY.md 在 300~400 行；超长时使用 `bootstrap-claude` skill 的 `scripts/memory.py status/compact`（保留事实与计划，淘汰最旧日志）。
