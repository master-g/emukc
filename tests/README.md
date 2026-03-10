# Integration Tests

## Overview

Integration test framework for gameplay API logic to prevent regressions. Tests directly invoke gameplay layer without HTTP server.

## Running Tests

```bash
# Run all gameplay tests
cargo test --test gameplay_tests

# Run specific module
cargo test --test gameplay_tests quest

# Run specific test
cargo test --test gameplay_tests test_composition_exact_match_requirement
```

## Test Structure

```
tests/
├── gameplay_tests.rs               # Test entry point
├── gameplay_tests/
│   └── quest/
│       ├── mod.rs
│       └── composition.rs          # Quest composition tests
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

## Test Principles

1. **Isolation** - Each test uses independent temp database
2. **Fast** - Tests complete in < 1 second
3. **Clear** - Test names describe what they validate
4. **Minimal** - Test core logic only

## Fixed Bugs

- ✅ Idle quest progress updated incorrectly
- ✅ 1 ship satisfies 2-ship requirement
- ✅ Composition quests auto-complete on activation
