---
title: "Bootstrap initialization workflow documentation (BOOTSTRAP.md)"
date: 2026-06-22
category: best-practices
module: emukc_bootstrap
problem_type: best_practice
component: documentation
severity: medium
applies_when:
  - "Writing or updating the BOOTSTRAP.md onboarding guide"
  - "A new user needs to initialize the emulator from a fresh clone"
tags: [bootstrap, documentation, onboarding, config, troubleshooting]
related_components: [emukc, emukc_cache]
---

# Bootstrap initialization workflow documentation (BOOTSTRAP.md)

## Context

The emulator requires a multi-step initialization before it can serve a game
client: toolchain setup, configuration, manifest download (bootstrap), cache
list generation (make-list), resource download (populate), and server start.
`BOOTSTRAP.md` is the single document a new user follows to get from a fresh
clone to a running server. Without it covering every step in order — including
the one-command shortcut and troubleshooting — users get stuck on missing
config fields, download failures, or incomplete caches.

## Guidance

`BOOTSTRAP.md` SHALL document the complete initialization flow in order:

1. **Environment preparation** — Rust toolchain install, project clone.
2. **Configuration file creation** — copy `emukc.config.example.toml` to
   `emukc.config.toml`.
3. **Bootstrap command** — `cargo run -- bootstrap` downloads manifests and
   builds the Codex snapshot.
4. **Cache make-list command** — `cargo run -- cache make-list` generates the
   cache list.
5. **Cache populate command** — `cargo run -- cache populate` downloads the
   resource files.
6. **Server start** — `cargo run -- serve`.

`BOOTSTRAP.md` SHALL document a **one-command mode**: running `cargo run`
without a subcommand automatically completes bootstrap, cache list generation,
and resource download in sequence. The document MUST explain this so users know
the manual steps above are optional when the one-command path is acceptable.

`BOOTSTRAP.md` SHALL explain every required `emukc.config.toml` field:

- `workspace_root` — user data storage location.
- `cache_root` — game cache directory.
- `bind` — server listen address.
- `proxy` — proxy settings (used for downloading resources).
- `game_cdn` / `gadgets_cdn` — CDN address lists.

`BOOTSTRAP.md` SHALL include a command-parameter quick reference for bootstrap
and cache subcommands:

- bootstrap: `--overwrite`, `--force-update`, `--proxy`.
- cache make-list: `--output`, `--overwrite`, `--greedy`, `--manifest`,
  `--concurrent`.
- cache populate: `--src`, `--concurrent`.

`BOOTSTRAP.md` SHALL include troubleshooting guidance for:

- Configuration file not found.
- Bootstrap download failure (network / proxy problems).
- Codex load failure.
- Incomplete cache resource download.

## Why This Matters

A new user who clones the repo and reads `BOOTSTRAP.md` MUST be able to
complete the full zero-to-running-server sequence without external help. Gaps
in the document — a missing step, an unexplained config field, or no
troubleshooting for the common network failure — translate directly into
blocked onboarding and support burden.

## When to Apply

- When writing or updating `BOOTSTRAP.md`.
- When a new CLI flag or config field is added to bootstrap or cache commands —
  the quick-reference and field tables MUST be kept in sync.
- When a new failure mode is observed in bootstrap/populate — add it to the
  troubleshooting section.

## Examples

A user wanting to override existing data or adjust concurrency finds the
parameter quick reference sufficient to construct the right command without
reading source code. A user whose bootstrap fails on a flaky network follows
the troubleshooting guidance to verify proxy settings and retry.

## Related

- `cli-progress.md` — progress display for the commands BOOTSTRAP.md documents.
- `populate-error-classification.md` — the version-rollback classification
  the troubleshooting section references.
