## Why

`cargo audit` reports 4 vulnerabilities and 4 warnings across the dependency tree. The most critical is RUSTSEC-2026-0104 (reachable panic in `rustls-webpki` CRL parsing, published 2026-04-22). Other findings include two `rustls-webpki` certificate validation bugs, a `rand` unsound advisory, a yanked `unicode-segmentation` version, and a false-positive `rsa` advisory from a `Cargo.lock` residual (`sqlx-mysql` not in the compile graph).

## What Changes

- Upgrade `rustls-webpki` from `0.103.10` to `≥0.103.13` (fixes RUSTSEC-2026-0104, -0098, -0099)
- Upgrade `unicode-segmentation` from `1.13.1` to `≥1.13.2` (fixes yanked version)
- Run a targeted `cargo update` to resolve both without pulling unrelated major version bumps
- Add `.cargo/audit.toml` to ignore the `rsa` false positive (sqlx-mysql not compiled, advisory has no fix available)
- Note the `rand` unsound advisory (RUSTSEC-2026-0097) as low-risk — requires custom logger + `rand::rng()` interplay, project uses tracing

## Capabilities

### New Capabilities
- `audit-config`: cargo-audit configuration to suppress known false positives and document accepted low-risk advisories

### Modified Capabilities

None.

## Impact

- `Cargo.lock` — version bumps for `rustls-webpki` and `unicode-segmentation`
- `.cargo/audit.toml` — new file, audit ignore rules
- No source code changes; no API changes; no behavioral changes
- Build may pull in transitive updates from `cargo update` (97 packages have newer versions available, but only targeted updates are planned)

## Non-goals

- Full `cargo update` of all 89 outdated packages (separate concern)
- Upgrading `rand` across the dependency tree (upstream tera/sqlx will resolve naturally)
- Removing `sqlx-mysql` from `Cargo.lock` (harmless residual, not worth a separate effort)
- Changing any Rust source code
