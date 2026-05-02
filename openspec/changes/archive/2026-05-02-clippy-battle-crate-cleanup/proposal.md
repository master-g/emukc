## Why

`emukc_battle` crate (recently extracted in `d4de3ff`) produces 156 clippy warnings — 91% `missing_docs`, 6% dead code, 3% misc. All warnings are noise that hides real issues and degrades CI signal.

## What Changes

- Suppress `missing_docs` on `types.rs` (108 struct fields + 16 structs + 9 variants — self-documenting data structures). Add real doc comments to remaining pub items (enums, methods, associated functions, module).
- Annotate dead code (4 constants, 4 functions, 1 field, 1 import group) with `#[allow(dead_code)]` + TODO comments. These are reserved for unimplemented night battle / airstrike features.
- Fix `too_many_arguments` in `simulation/mod.rs:149` by introducing a parameter struct.
- Fix doc backtick in `lib.rs:1`.

## Capabilities

### New Capabilities

- `battle-crate-docs`: documentation coverage and clippy hygiene for `emukc_battle` public API
- `battle-sim-params`: parameter struct for battle simulation entry point

### Modified Capabilities

(none — no spec-level behavior changes)

## Impact

- `crates/emukc_battle/` — all changes confined here
- No API or behavioral changes
- No dependency changes

## Non-goals

- Rewriting battle types for better ergonomics
- Implementing the night battle / airstrike features that would consume the dead code
- Adding `#[deny(warnings)]` to CI (separate concern)
