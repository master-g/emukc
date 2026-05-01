## 1. Dependency bumps

- [x] 1.1 Bump `indicatif` from `"0.17"` to `"0.18"` in workspace `Cargo.toml`
- [x] 1.2 Run `cargo update -p rand` to pull patched versions (0.9.3+, 0.10.1+)
- [x] 1.3 Fix any indicatif 0.18 API breaking changes in `crates/emukc_bootstrap/src/progress.rs` and `crates/emukc_bootstrap/src/populate.rs`

## 2. Audit configuration

- [x] 2.1 Add RUSTSEC-2026-0097 to `.cargo/audit.toml` ignore list with comment documenting rand 0.8.x blocker (tera/phf_generator, no patch available, unreachable in EmuKC)
- [x] 2.2 Remove RUSTSEC-2025-0119 note from ignore list if it was added (should not be needed after indicatif bump)

## 3. Verification

- [x] 3.1 Run `cargo build` and confirm clean build
- [x] 3.2 Run `cargo audit` and confirm 0 vulnerabilities, 0 warnings
