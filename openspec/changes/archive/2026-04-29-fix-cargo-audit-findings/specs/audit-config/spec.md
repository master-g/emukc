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
