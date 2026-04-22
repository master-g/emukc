# Integration Tests

## Overview

Integration test framework for gameplay API logic to prevent regressions. Tests directly invoke gameplay layer without HTTP server.

Runnable samples live under `examples/` and should be invoked with `cargo run --example <name>`.

## Running Tests

```bash
# Run all gameplay tests
cargo test --test gameplay_tests

# Run specific module
cargo test --test gameplay_tests quest

# Run specific test
cargo test --test gameplay_tests test_composition_exact_match_requirement
```

## Data Freshness

Gameplay tests usually load `.data/codex`, which is a snapshot written by bootstrap. If you
change `crates/emukc_bootstrap/assets/wikiwiki_map_catalog.json`, those tests will keep seeing the
old map catalog until the runtime codex is refreshed.

- **Repo asset**: `crates/emukc_bootstrap/assets/wikiwiki_map_catalog.json`
- **Runtime snapshot**: `.data/codex/map_catalog.json`

Typical workflow for map-data work:

1. Update parser / asset
2. Regenerate the repo asset if needed
3. Refresh the runtime codex before running tests that load `.data/codex`

When a test needs the current repo-tracked wikiwiki asset immediately, prefer a helper that
rebuilds `codex.maps` from the repo asset instead of relying on the stale `.data/codex` snapshot.

## Test Structure

```
examples/
├── bootstrap_download.rs          # Bootstrap download example
├── dump_tree.rs                   # Filesystem tree example
├── kache_test.rs                  # Cache fetch example
└── model_loader.rs                # Manifest/model loading example

tests/
├── gameplay_tests.rs               # Test entry point
├── gameplay_tests/
│   └── quest/
│       ├── mod.rs
│       └── composition.rs          # Quest composition tests
├── fixtures/
│   └── battle/
│       └── incident_slot_102.json  # Test fixture data
└── README.md
```

## Current Coverage

### Quest System

- **composition.rs** - Validates composition quest exact match logic
  - Fixes: 2-ship requirement should not pass with 1 ship
  - Validates `min == max` enforcement

## Adding New Tests

### Create New Test Module

1. Create file: `tests/gameplay_tests/<module>/<test_name>.rs`
2. Add to `tests/gameplay_tests/<module>/mod.rs`: `mod <test_name>;`
3. Add to `tests/gameplay_tests.rs` if new module: `#[path = "gameplay_tests/<module>/mod.rs"] mod <module>;`

### Add a New Example

1. Create a file under `examples/`
2. Add or update the `[[example]]` entry in `Cargo.toml` if a custom path is needed
3. Run it with `cargo run --example <name>`

### Example Test

```rust
#[test]
fn test_feature() {
    use emukc_internal::prelude::*;
    let codex = Codex::load(std::path::Path::new(".data/codex"), true).unwrap();
    // Test logic...
    assert_eq!(actual, expected);
}
```

For map-routing work, this only sees the latest data after the codex snapshot is refreshed.

## Test Principles

1. **Isolation** - Each test uses independent temp database
2. **Fast** - Tests complete in < 1 second
3. **Clear** - Test names describe what they validate
4. **Minimal** - Test core logic only

## Fixed Bugs

- ✅ Idle quest progress updated incorrectly
- ✅ 1 ship satisfies 2-ship requirement
- ✅ Composition quests auto-complete on activation
