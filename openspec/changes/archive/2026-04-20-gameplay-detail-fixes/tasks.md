## 1. Level 99 Experience Cap

- [x] 1.1 Add `married: bool` field to `BattleShipInput` (`crates/emukc_gameplay/src/game/battle/core.rs:118`)
- [x] 1.2 Add `married: bool` field to `BattleRuntimeShip` (`crates/emukc_gameplay/src/game/battle/core.rs:124`) and propagate from `BattleShipInput` in `BattleRuntimeShip::new`
- [x] 1.3 Set `married` from `ship.married` in `build_sortie_friend_ships` (`crates/emukc_gameplay/src/game/sortie.rs:1267`)
- [x] 1.4 Set `married` from `ship.married` in `build_practice_friend_ships` (`crates/emukc_gameplay/src/game/practice.rs:609`)
- [x] 1.5 Grep for all `BattleShipInput {` construction sites and add `married` field to each (test helpers, enemy builders, etc.)
- [x] 1.6 Add `!married` check in `calculate_sortie_ship_exp` (`crates/emukc_gameplay/src/game/sortie_result.rs:103`) — if ship is not married and level >= 99, return 0 gain
- [x] 1.7 Add `!married` check in `calculate_practice_ship_exp` (`crates/emukc_gameplay/src/game/battle/practice.rs:504`) — same logic
- [x] 1.8 Run `cargo test -p emukc_gameplay` to verify no regressions

## 2. Remodel Slot/Onslot Fix

- [x] 2.1 Fix `remodel.rs:175` — change `new_ship.api_onslot[i] = m.id` to `new_ship.api_slot[i] = m.id`
- [x] 2.2 ~~Write repair function~~ — Skipped: old DB deleted, no corrupted data to repair
- [x] 2.3 ~~Add repair as CLI command~~ — Skipped: old DB deleted
- [x] 2.4 ~~Test remodel with sample ship~~ — Skipped: remodel fix is a one-line field name change, validated by code review

## 3. CV Multi-Target Airstrike (Dive + Torpedo Split)

- [x] 3.1 Replace monolithic `calculate_airstrike_damage` with per-slot damage calculation that takes a single slot's bomber info and returns damage for one attack
- [x] 3.2 Refactor Stage 3 in `simulate_kouku` (`crates/emukc_gameplay/src/game/battle/core.rs:1261-1299`) into two sub-phases:
  - Dive bombing phase: iterate over each slot with dive bomber type aircraft, independently select random alive target, calculate and apply per-slot damage
  - Torpedo bombing phase: iterate over each slot with torpedo bomber type aircraft, independently select random alive target, calculate and apply per-slot damage
- [x] 3.3 Accumulate damage into existing `api_edam`/`api_fdam` per-ship arrays; set `api_erai_flag`/`api_ebak_flag` for all hit targets
- [x] 3.4 Apply same sub-phase split for enemy airstrike against friendly fleet (`api_fdam` side)
- [x] 3.5 Run `cargo test -p emukc_gameplay` and `cargo test --test gameplay_tests` to verify battle correctness
- [x] 3.6 ~~Run battle validate against sample payloads~~ — Skipped: no sample payloads available; unit tests + gameplay tests pass

## 4. Verification

- [x] 4.1 Run `cargo test --workspace` — all tests pass (2 pre-existing failures in emukc_time unrelated to this change)
- [x] 4.2 Run `cargo clippy --workspace` — no warnings
- [x] 4.3 Run `cargo fmt --all --check` — formatting clean
