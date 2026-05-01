## Context

`cargo audit` reports 2 advisories (4 warnings total). Three of four are fixable by bumping deps. One (rand 0.8.5) has no upstream patch — only suppressible.

Current state:
- `indicatif = "0.17"` in workspace `Cargo.toml` → pulls `number_prefix 0.4.0`
- `rand 0.8.5` via `tera` + `phf_generator` → no patch available
- `rand 0.9.2` via `quinn-proto` → patchable to 0.9.3
- `rand 0.10.0` via `uuid` → patchable to 0.10.1

Affected code: `crates/emukc_bootstrap/src/progress.rs` and `crates/emukc_bootstrap/src/populate.rs` use indicatif APIs directly.

## Goals / Non-Goals

**Goals:**
- Resolve RUSTSEC-2025-0119 (number_prefix) by bumping indicatif to 0.18
- Resolve rand 0.9.2 and 0.10.0 warnings via `cargo update`
- Suppress rand 0.8.5 warning with documented rationale
- Keep `cargo audit` output clean (0 vulns, 0 actionable warnings)

**Non-Goals:**
- Removing or replacing tera/phf_generator (rand 0.8.5 blockers)
- Changing progress bar behavior or appearance

## Decisions

**1. Bump indicatif 0.17 → 0.18** — indicatif 0.18 replaced `number_prefix` with `unit_prefix` (PR #709). This eliminates RUSTSEC-2025-0119 entirely. Alternative (suppress) rejected because upstream fix exists and is a simple version bump.

**2. `cargo update -p rand` for patchable versions** — rand 0.9.2 → 0.9.3+ and 0.10.0 → 0.10.1+ are within-semver-compatible patches. `cargo update` in Cargo.lock resolves them. No `Cargo.toml` changes needed — rand is not a direct workspace dependency.

**3. Suppress rand 0.8.5 only** — The 0.8.x line has no patch (advisory says patched: `>=0.9.3` and `>=0.10.1`). `tera` and `phf_generator` pin to 0.8.x. Suppressing with a comment documenting the blocker is the only option.

## Risks / Trade-offs

- [indicatif 0.18 API breaking changes may break progress bar code] → Mitigation: `cargo build` + visual check. Progress bar templates in `emukc_bootstrap/src/progress.rs` may need syntax updates.
- [Suppressed rand 0.8.5 advisory may become exploitable] → Mitigation: EmuKC uses `tracing` not `log::set_logger`, so the unsound condition is not reachable. Comment in `audit.toml` documents this.
- [tera/phf_generator may never bump rand] → Low risk. The advisory severity is INFO, not a real vulnerability in our usage.
