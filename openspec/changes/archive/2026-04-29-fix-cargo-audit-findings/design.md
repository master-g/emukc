## Context

`cargo audit` reports 4 vulnerabilities and 4 warnings in the EmuKC workspace. Current dependency state:

| Crate | Version | Advisory | Severity |
|-------|---------|----------|----------|
| rustls-webpki | 0.103.10 | RUSTSEC-2026-0104, -0098, -0099 | medium-high |
| rsa | 0.9.10 | RUSTSEC-2023-0071 | medium (false positive) |
| rand | 0.8.5, 0.9.2, 0.10.0 | RUSTSEC-2026-0097 | low |
| unicode-segmentation | 1.13.1 | yanked | info |

Dependency chains:
- `rustls-webpki` comes from `reqwest 0.13 → rustls 0.23.37 → rustls-webpki 0.103.10`
- `rsa` comes from `sqlx-mysql` (Cargo.lock residual, not in compile graph)
- `rand 0.8.5` comes from `tera`, `rand 0.10.0` from `uuid`, `rand 0.9.2` from `quinn-proto`
- `unicode-segmentation` comes from `convert_case`, `tera`, `derive_more`

## Goals / Non-Goals

**Goals:**
- Eliminate the 3 `rustls-webpki` advisories by upgrading to ≥0.103.13
- Fix the yanked `unicode-segmentation` warning
- Suppress the false-positive `rsa` advisory via audit config
- Leave `rand` advisory as accepted low-risk (document reasoning)

**Non-Goals:**
- Full `cargo update` of all 89 outdated packages
- Upgrading `rand` across the dependency tree
- Removing `sqlx-mysql` from `Cargo.lock`
- Any source code changes

## Decisions

### 1. Targeted `cargo update` vs full update

**Decision:** Use targeted `cargo update -p <crate>` for the two fixable packages only.

**Rationale:** A full `cargo update` would bump 89 packages at once, making it harder to attribute any regressions. Targeted updates minimize blast radius. The dry-run confirms both updates are minor-patch bumps within their SemVer range.

**Alternative considered:** Full `cargo update` — rejected due to risk surface. Can be done separately if desired.

### 2. Audit config location: `.cargo/audit.toml`

**Decision:** Create `.cargo/audit.toml` at workspace root with ignore rules for `RUSTSEC-2023-0071` (rsa).

**Rationale:** `sqlx-mysql` is a Cargo.lock residual not in the compile graph. The `rsa` advisory has no fix available. Ignoring it in audit config documents the reasoning and keeps CI clean.

### 3. `rand` advisory handling: accept and document

**Decision:** Do not ignore RUSTSEC-2026-0097 in audit config. Let it appear as a warning.

**Rationale:** The unsound condition requires a custom logger + `rand::rng()` interplay. Project uses `tracing` via `emukc_log`, which doesn't trigger the condition. The advisory affects 3 different `rand` versions deep in the tree — fixing requires upstream changes in `tera`, `uuid`, `quinn-proto`. No action needed from EmuKC side.

## Risks / Trade-offs

- [Targeted update pulls transitive deps] → `cargo update -p` only bumps the named crate within its SemVer constraint; transitive deps only change if the new version requires them. Dry-run confirmed minimal impact.
- [`.cargo/audit.toml` suppresses real future issues under same ID] → Low risk. RUSTSEC-2023-0071 is specific to `rsa` timing side-channel, and `sqlx-mysql` is not compiled. Can remove the ignore if `sqlx-mysql` is ever added to the build.
- [rand warning persists in future audits] → Acceptable. Will naturally resolve as upstream crates update their `rand` dependencies.
