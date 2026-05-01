## ADDED Requirements

### Requirement: Audit configuration suppresses false-positive rsa advisory
The project SHALL include a `.cargo/audit.toml` file that ignores RUSTSEC-2023-0071, as `sqlx-mysql` (which depends on `rsa`) is a Cargo.lock residual not present in the compile graph.

#### Scenario: cargo audit reports no vulnerabilities after configuration
- **WHEN** `cargo audit` is run after applying the configuration
- **THEN** RUSTSEC-2023-0071 SHALL NOT appear in the output

#### Scenario: ignore rule is documented with reason
- **WHEN** a developer reads `.cargo/audit.toml`
- **THEN** the ignore entry for RUSTSEC-2023-0071 SHALL include a note explaining it is a false positive due to unused `sqlx-mysql`

### Requirement: rustls-webpki upgraded to fix three advisories
The workspace Cargo.lock SHALL resolve `rustls-webpki` to version ≥0.103.13, eliminating RUSTSEC-2026-0104, RUSTSEC-2026-0098, and RUSTSEC-2026-0099.

#### Scenario: cargo audit shows no rustls-webpki advisories
- **WHEN** `cargo audit` is run after the upgrade
- **THEN** no advisories referencing `rustls-webpki` SHALL appear

#### Scenario: project builds successfully after upgrade
- **WHEN** `cargo build` is run after the dependency update
- **THEN** the build SHALL complete without errors

### Requirement: unicode-segmentation upgraded past yanked version
The workspace Cargo.lock SHALL resolve `unicode-segmentation` to version ≥1.13.2, eliminating the yanked version warning.

#### Scenario: cargo audit shows no yanked warnings for unicode-segmentation
- **WHEN** `cargo audit` is run after the upgrade
- **THEN** no yanked warning for `unicode-segmentation` SHALL appear

### Requirement: rand advisory accepted as low-risk
RUSTSEC-2026-0097 (rand unsound) SHALL remain visible in `cargo audit` output as an informational warning. No ignore rule SHALL be added for this advisory.

#### Scenario: rand advisory appears in audit output
- **WHEN** `cargo audit` is run
- **THEN** RUSTSEC-2026-0097 SHALL appear in the warnings section
- **AND** it SHALL NOT appear in the ignored/suppressed section

### Requirement: indicatif bumped to 0.18 to eliminate number_prefix
The workspace `Cargo.toml` SHALL specify `indicatif = "0.18"` or higher, resolving `number_prefix` (RUSTSEC-2025-0119) by replacing it with `unit_prefix` via the upstream indicatif 0.18 release.

#### Scenario: cargo audit reports no number_prefix advisory
- **WHEN** `cargo audit` is run after the bump
- **THEN** RUSTSEC-2025-0119 SHALL NOT appear in the output

#### Scenario: project builds and progress bars function correctly
- **WHEN** `cargo build` completes and the bootstrap process runs
- **THEN** progress bars SHALL display correctly with no panics or template errors

### Requirement: rand 0.9.x and 0.10.x updated to patched versions
The workspace Cargo.lock SHALL resolve `rand` 0.9.x to >=0.9.3 and `rand` 0.10.x to >=0.10.1, eliminating RUSTSEC-2026-0097 for those versions.

#### Scenario: cargo audit reports no rand 0.9.x or 0.10.x warnings
- **WHEN** `cargo audit` is run after the update
- **THEN** no rand advisory for versions 0.9.x or 0.10.x SHALL appear

### Requirement: Audit configuration suppresses unpatchable rand 0.8.x advisory
The project SHALL include RUSTSEC-2026-0097 in the `.cargo/audit.toml` ignore list, with a comment noting that rand 0.8.5 is pulled by `tera` and `phf_generator`, has no available patch (0.8.x line is unpatched), and the unsound condition is unreachable because EmuKC uses `tracing` rather than `log::set_logger`.

#### Scenario: cargo audit does not report rand 0.8.x warning
- **WHEN** `cargo audit` is run
- **THEN** RUSTSEC-2026-0097 for rand 0.8.x SHALL NOT appear

#### Scenario: ignore rule documents the blocker
- **WHEN** a developer reads `.cargo/audit.toml`
- **THEN** the ignore entry SHALL explain the advisory is suppressed because tera/phf_generator pin to 0.8.x and no patch exists
