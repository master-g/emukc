## Context

`battle-architecture-crate-extract` left behind stale imports and an unused method after the Phase 5-6 migration. These produce clippy warnings but no behavioral issues.

## Goals / Non-Goals

**Goals:** Remove all 12 auto-fixable clippy warnings in `emukc_gameplay`.

**Non-Goals:** Touch `emukc_battle` warnings (deferred to `fix-battle-attack-system`). Add missing docs (future change). Fix the 2 non-auto-fixable warnings (the `profile_id` field will get `#[allow(dead_code)]`).

## Decisions

### D1: Delete `insert_active_sortie` outright

No callers remain. The `SortieRepository::insert_active` trait method (which returns `Option<ActiveSortieState>`) replaced it. Direct deletion.

### D2: `#[allow(dead_code)]` on `profile_id` field

`SortieNightBattleSession::profile_id` is set during construction but never directly read — its value is embedded in the returned packet. The field exists for context/debugging. Silence with targeted allow rather than deletion.

### D3: Run `cargo clippy --fix` then manually verify

Clippy can auto-fix 12 of 14 warnings. Run the auto-fix, then manually add `#[allow(dead_code)]` for the remaining 2.
