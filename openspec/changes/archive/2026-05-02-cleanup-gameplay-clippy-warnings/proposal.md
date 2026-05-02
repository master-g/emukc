## Why

`cargo clippy` reports 14 warnings in `emukc_gameplay` (7 unused imports, 1 dead method, 1 redundant closure, 4 missing docs, 1 unused field). Of these, 12 are auto-fixable. Fix them now to keep the crate warning-clean and avoid masking new issues.

The 156 warnings in `emukc_battle` are intentionally deferred — the dead functions/constants are night-attack and airstrike code awaiting `fix-battle-attack-system`.

## What Changes

- Remove 7 unused imports across `practice/mod.rs`, `practice/orchestrate.rs`, `sortie/orchestrate.rs`, `sortie/mod.rs`, `repository.rs`
- Delete dead `insert_active_sortie` method from `SortieStore` (replaced by trait `insert_active` in Phase 5)
- Fix redundant closure in `sortie.rs`
- Add `#[allow(dead_code)]` to `SortieNightBattleSession::profile_id` (field set but never read — used for debug/tracing context)

## Capabilities

- `<none>` — pure hygiene, no behavior change

## Impact

- `crates/emukc_gameplay/src/game/battle/practice/mod.rs` — remove `Codex`, `level` imports
- `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs` — remove `PracticeBattleResponse` import
- `crates/emukc_gameplay/src/game/battle/sortie/orchestrate.rs` — remove `BattleContext`, `BattleSimulation`, `NightBattlePacket` imports
- `crates/emukc_gameplay/src/game/battle/sortie/mod.rs` — remove `response::enemy_slot_ids` import
- `crates/emukc_gameplay/src/game/battle/repository.rs` — remove `SortieNightBattleSession` import
- `crates/emukc_gameplay/src/game/sortie_store.rs` — delete `insert_active_sortie`
- `crates/emukc_gameplay/src/game/sortie.rs` — fix redundant closure
