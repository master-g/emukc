## Context

`battle-architecture-crate-extract` (Phase 5-6) introduced the `SortieRepository` trait and split `battle/sortie` and `battle/practice` into sub-modules. The migration moved code quickly to unblock Iteration flow, leaving several code-quality issues:

- `SortieStore` has inherent methods named identically to `SortieRepository` trait methods (`get_pending_battle`, `insert_pending_battle`, `take_pending_battle`). Inside `impl SortieRepository for SortieStore`, `self.get_pending_battle(...)` resolves to the inherent method (Rust's inherent-priority rule), not the trait method. This works today because both have the same signature, but if an inherent method signature is modified independently, the trait impl silently calls the wrong target with no compiler error.
- Other inherent methods follow a naming convention (`get_active_sortie`, `remove_active_sortie`) that distinguishes them from trait methods (`get_active`, `remove_active`). The `get_pending_*` group is the outlier.
- `SortieRepository::insert_active` returns `Option<ActiveSortieState>` (previous value on overwrite), but all 3 call sites ignore it. This is a latent information loss — if a sortie is unexpectedly overwritten, no caller notices.
- `enemy_slot_ids(&BattleShipInput)` in `game/sortie.rs` is dead code in production; only `#[cfg(test)]` tests reference it. The same function already exists in `battle/sortie/response.rs`.
- `practice/mod.rs` imports `BattleType`, `EngagementType`, `CryptoRng` but doesn't use them (they belong in `orchestrate.rs`).

## Goals / Non-Goals

**Goals:**
- Rename `SortieStore` inherent methods with `_sortie` suffix to match existing convention (`get_active_sortie` → `get_active` trait, inherent stays `get_active_sortie`)
- Add missing `get_pending_result_sortie` inherent method for consistent delegation
- Add `#[must_use]` to `SortieRepository::insert_active`
- Delete dead `enemy_slot_ids` from `game/sortie.rs`; make the version in `battle/sortie/response.rs` `pub(crate)` for tests
- Remove unused imports from `practice/mod.rs`

**Non-Goals:**
- Replace `std::sync::Mutex` with `parking_lot::Mutex` in practice battle (pre-existing, no deadlock observed)
- DRY the four `HasContext` tuple impls (low-value churn, no bugs)
- Changing any battle behavior or API responses

## Decisions

### D1: Rename pattern: `_sortie` suffix for SortieStore inherent methods

**Decision**: Rename `get_pending_battle` → `get_pending_battle_sortie`, `insert_pending_battle` → `insert_pending_battle_sortie`, `take_pending_battle` → `take_pending_battle_sortie`. The `take_pending_result` and `insert_pending_result` inherent methods already have distinct names from the trait (`take_pending_result` is the same name but different return context — rename to `take_pending_result_sortie` for consistency).

**Rationale**: The existing inherent methods follow `get_active_sortie`, `insert_active_sortie`, `remove_active_sortie` naming. The pending methods were the only ones that collided with trait names. Suffixing `_sortie` is consistent with existing convention.

**Alternatives considered**:
- Use qualified syntax `SortieRepository::get_pending_battle(self, ...)` inside the trait impl — works but verbose and easy to forget
- Delete inherent methods and inline hashmap access in the trait impl — would break any remaining `pub(super)` callers

### D2: Delete enemy_slot_ids from game/sortie.rs

**Decision**: Delete `fn enemy_slot_ids(&BattleShipInput)` from `game/sortie.rs`. Make the identical function in `battle/sortie/response.rs` `pub(crate)`. Update test imports to reference `super::battle::sortie::enemy_slot_ids`.

**Rationale**: One source of truth. The function in `response.rs` is semantically in the right module (response construction). No production code in `game/sortie.rs` calls it — only 4 test assertions.

### D3: Add get_pending_result_sortie inherent method

**Decision**: Add `pub(super) fn get_pending_result_sortie(&self, profile_id: i64) -> Option<SortieBattleResultSnapshot>` to `SortieStore`. The `SortieRepository` trait impl delegates to it. This makes the trait impl consistent: every method delegates to a `_sortie` inherent method.

**Rationale**: The current trait impl inconsistently accesses `self.pending_results.lock()` directly for `get_pending_result` but delegates through inherent methods for everything else. Adding the inherent method makes maintenance clearer.

## Risks / Trade-offs

- **[Risk] Renames are grep-only changes**: No behavior change. All callers of the old inherent names are within `sortie_store.rs` itself (trait impl) or in `game/sortie.rs`. Mitigation: compile check catches stale references.
- **[Risk] `enemy_slot_ids` visibility change**: Making it `pub(crate)` could accidentally be used by new code outside the battle module. Acceptable — it's a stateless helper with no invariants.
