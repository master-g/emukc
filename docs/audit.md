# Recent Commit Audit

> Audit date: 2026-04-17
> Reviewed commits: `e940330`, `f99d8fa`, `54e1de6`, `e3ac8cb`, `8875ccb`

## Scope

This audit covers the last five commits on `feat/vibe`, with most of the time spent on battle damage logic, route evaluation, sortie state handling, and map unlock tests.

## Findings

### High

1. `f99d8fa` changes route matching semantics by grouping rules only by predicate kind.

   `route_predicate_key()` in `crates/emukc_gameplay/src/game/map_route.rs` now returns a fixed `&str` such as `"ship_type_count"` or `"los"` instead of a key derived from the full predicate. `evaluate_route_destination()` keeps only the minimum priority per key, then executes every rule from keys that share that minimum priority. That collapses different predicates of the same kind into one bucket.

   Real map data already has multiple `LoS` rules and multiple `ShipTypeCount` rules on the same cell. One example is `.data/generated/wikiwiki_map_catalog.debug.json`, where cell 2 has `LoS >= 28 && LoS <= 29`, `LoS >= 30`, and `LoS <= 27` as separate routing conditions. With the new keying, lower-priority same-kind rules can become executable when they should not be.

   This is a gameplay bug, not just a performance refactor. It can send fleets to the wrong node and distort weighted routing.

2. `e940330` gives the night recon bonus to any seaplane recon, not only to actual night recon equipment.

   `night_recon_bonus()` in `crates/emukc_gameplay/src/game/battle/core.rs` checks only `KcSlotItemType3::SeaBasedRecon`. That matches ordinary seaplane recon planes and night recon planes alike.

   In `.data/codex/start2.json`, normal `零式水上偵察機` uses `api_type: [5, 7, 10, 10, 2]`, while `九八式水上偵察機(夜偵)` uses `api_type: [5, 7, 10, 50, 2]`. The distinction sits in type4, not type3. The current code ignores that difference, so plain water recon planes now get the night recon contact bonus.

### Medium

3. `e940330` adds air-state-dependent night recon values, but the runtime path never passes an air state into night battle.

   `simulate_night_battle_v1()` now accepts `air_state: Option<&AirState>`, and `night_recon_bonus()` has separate `+5`, `+7`, and `+9` branches. The sortie and practice callers still pass `None`, so real battles never reach the superiority or supremacy branches.

   In practice, the new code behaves as a flat `+5` bonus in normal runtime. That does not match the commit message or the formula it claims to implement.

4. `e3ac8cb` moves `SortieStore` updates before `tx.commit()`, which creates a different consistency failure mode.

   The new ordering avoids the old crash window where the database commits and memory stays stale. It introduces the opposite problem: if `tx.commit()` fails, the request returns an error after the in-memory sortie state has already been removed or advanced.

   The code comment only covers the crash-after-commit case. It does not address commit failure, and the current ordering leaves database state and runtime state out of sync in that path.

5. `f99d8fa` weakens the public unlock regression test enough that it no longer tests unlock behavior.

   `tests/gameplay_tests/map/unlock.rs` still has a test named `clearing_1_1_unlocks_1_2`, but the body now checks only the initial map list for a new account. The new crate-internal test covers the cascade inside gameplay code, but the public-facing check that `get_map_infos()` exposes newly unlocked maps after a clear is gone.

   This is a test coverage regression. It makes future breakage in the API-facing path easier to miss.

### Low

6. I did not find a concrete bug in `54e1de6` while reviewing the overkill-display change.

   The `apply_damage()` tuple split and the raw-versus-effective damage wiring look coherent in the paths I checked. The new targeted test also passes.

## Targeted Verification

I ran a few focused tests while reviewing the changes:

- `cargo test -p emukc_gameplay clearing_map_1_1_unlocks_dependents_via_cascade -- --nocapture`
- `cargo test -p emukc_gameplay overkill_shows_raw_damage -- --nocapture`

Both passed.

## Notes

- This document records review findings. It does not include fixes.
- The highest-risk issues are the route predicate grouping change and the night recon type check.
