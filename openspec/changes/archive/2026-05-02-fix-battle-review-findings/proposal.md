## Why

Code review of `battle-architecture-crate-extract` surfaced several code-quality issues introduced during Phase 5-6 migration: inherent/trait method name collisions risk silent divergence, dead code remains in production paths, and inconsistent delegation patterns make maintenance fragile. Fix now before these settle into the codebase and harder to untangle.

## What Changes

- Rename `SortieStore` inherent methods (`get_pending_battle`, `insert_pending_battle`, `take_pending_battle`) to avoid name collision with `SortieRepository` trait methods of the same name
- Delete `enemy_slot_ids(&BattleShipInput)` from `game/sortie.rs` (dead code in production; only tests use it); update tests to import from `battle::sortie::response`
- Remove unused imports in `practice/mod.rs` (`BattleType`, `EngagementType`, `CryptoRng`)
- Normalize `SortieRepository` impl for `SortieStore` to consistently delegate through inherent methods (add missing `get_pending_result` inherent)
- Add `#[must_use]` to `SortieRepository::insert_active` since callers currently ignore the `Option<ActiveSortieState>` return, indicating a latent logic gap
- **Non-goals**: Replacing `std::sync::Mutex` with `parking_lot::Mutex` in practice battle code (pre-existing, no observable issue); DRYing the four `HasContext` tuple impls (low-value churn)

## Capabilities

### New Capabilities

- `<none>`

### Modified Capabilities

- `<none>`

## Impact

- `crates/emukc_gameplay/src/game/sortie_store.rs` — inherent method renames, new `get_pending_result` inherent, `#[must_use]` on trait
- `crates/emukc_gameplay/src/game/battle/repository.rs` — `#[must_use]` annotation
- `crates/emukc_gameplay/src/game/sortie.rs` — remove dead `enemy_slot_ids`, update test imports
- `crates/emukc_gameplay/src/game/battle/sortie/response.rs` — make `enemy_slot_ids` `pub(crate)` for test import
- `crates/emukc_gameplay/src/game/battle/practice/mod.rs` — remove unused imports
