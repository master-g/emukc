## ADDED Requirements

### Requirement: emukc_gameplay SHALL compile with 0 clippy warnings from actionable categories

The `emukc_gameplay` crate SHALL have zero warnings for: unused imports, dead code, redundant closures, and unused variables (excluding `#[allow]`-annotated items).

#### Scenario: Clippy reports 0 actionable warnings
- **WHEN** `cargo clippy -p emukc_gameplay` is run
- **THEN** no warnings about unused imports, dead methods, or redundant closures appear
