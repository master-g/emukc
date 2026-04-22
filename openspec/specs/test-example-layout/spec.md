# test-example-layout Specification

## Purpose
TBD - created by archiving change separate-tests-and-examples. Update Purpose after archive.
## Requirements
### Requirement: Root-crate examples live under examples
The repository SHALL store runnable root-crate Cargo examples under `examples/`, and the root package metadata SHALL reference those files from `Cargo.toml` without changing the public example names.

#### Scenario: Existing examples are relocated without renaming
- **WHEN** the root crate defines examples such as `model_loader`, `bootstrap_download`, `dump_tree`, or `kache_test`
- **THEN** each example target MUST point to a source file under `examples/` and the example target name MUST remain unchanged

### Requirement: Tests directory remains test-only
The repository SHALL reserve `tests/` for integration tests, fixtures, and test-only support code, and SHALL NOT place standalone runnable examples there.

#### Scenario: Contributor adds or reviews test assets
- **WHEN** a contributor inspects or adds files under `tests/`
- **THEN** they MUST find only test entrypoints, test modules, fixtures, or test helpers, and no Cargo example source files

### Requirement: Repository guidance distinguishes tests from examples
Contributor-facing documentation SHALL describe the boundary between `tests/` and `examples/` and SHALL preserve the commands used to run tests versus examples.

#### Scenario: Contributor follows repository guidance
- **WHEN** a contributor reads the repository guidance for adding or running test-related code
- **THEN** the documentation MUST direct runnable samples to `examples/`, direct integration tests to `tests/`, and keep test commands and `cargo run --example ...` commands unambiguous

