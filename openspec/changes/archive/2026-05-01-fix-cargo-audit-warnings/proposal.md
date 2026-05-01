## Why

`cargo audit` reports 4 warnings across 2 advisories:
- **RUSTSEC-2025-0119**: `number_prefix 0.4.0` unmaintained (via `indicatif 0.17 → emukc_bootstrap`). Fixed upstream in `indicatif 0.18` which replaced `number_prefix` with `unit_prefix`.
- **RUSTSEC-2026-0097**: `rand` unsound — affects 0.8.5, 0.9.2, 0.10.0. Patched versions exist for 0.9.x (>=0.9.3) and 0.10.x (>=0.10.1). The 0.8.x line has no patch.

Bumping indicatif resolves number_prefix entirely. Updating rand in Cargo.lock resolves 2 of 3 rand warnings. The remaining rand 0.8.5 (via `tera` + `phf_generator`) has no available patch — suppress only that specific version.

## What Changes

- Bump workspace `indicatif` dependency from 0.17 to 0.18 (resolves RUSTSEC-2025-0119)
- Run `cargo update -p rand@0.9.2` to pull 0.9.3+ (resolves rand 0.9.x warning)
- Run `cargo update -p rand@0.10.0` to pull 0.10.1+ (resolves rand 0.10.x warning)
- Add RUSTSEC-2026-0097 to `.cargo/audit.toml` ignore list, scoped to the unpatchable rand 0.8.x via tera/phf_generator
- Fix any API breaking changes from indicatif 0.18 (e.g., progress bar template syntax, removed methods)
- Update `audit-config` spec with new suppression rationale

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `audit-config`: add suppression rule for RUSTSEC-2026-0097 scoped to rand 0.8.x (unpatchable, blocked by tera/phf_generator)

## Impact

- `Cargo.toml`: indicatif version bump
- `Cargo.lock`: indicatif 0.18 + unit_prefix (new), rand 0.9.3 + 0.10.1, number_prefix removed
- `crates/emukc_bootstrap/`: potential API changes from indicatif 0.18
- `.cargo/audit.toml`: new ignore entry for rand 0.8.x
- `openspec/specs/audit-config/spec.md`: updated requirements

## Non-goals

- Removing tera or phf_generator (rand 0.8.5 blockers) — these are upstream dependencies
- Forking or vendoring number_prefix — indicatif 0.18 already solved this
- Addressing RUSTSEC-2023-0071 (rsa) — already suppressed per existing spec
