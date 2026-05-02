## Why

Audit of `battle-architecture-crate-extract` found 3 dead methods on `SortieStore`: `modify_active_sortie`, `with_pending_result_mut`, `with_pending_battle_mut`. These closure-based mutation helpers were replaced by get-modify-insert pattern at all call sites during Phase 5. They remain as dead code, producing compiler warnings. Delete them.

Also: `sortie/mod.rs` has blanket `#![allow(dead_code)]` — removes the module-level suppression and narrow it to the single unused item.

## What Changes

- Delete `modify_active_sortie` from `SortieStore`
- Delete `with_pending_result_mut` from `SortieStore`
- Delete `with_pending_battle_mut` from `SortieStore`
- Replace `#![allow(dead_code)]` in `sortie/mod.rs` with targeted `#[allow(dead_code)]` on `EngagementType` import (the only unused item)

## Capabilities

### New Capabilities

- `<none>`

### Modified Capabilities

- `<none>`

## Impact

- `crates/emukc_gameplay/src/game/sortie_store.rs` — 3 method deletions (~20 lines)
- `crates/emukc_gameplay/src/game/battle/sortie/mod.rs` — narrow `allow(dead_code)` scope
