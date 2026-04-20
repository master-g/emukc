## Context

EmuKC has four gameplay fidelity bugs affecting ship progression, remodeling, battle simulation, and slot data. All fixes are server-side behavioral corrections with no API contract changes.

Current state:
- Ships at level 99 still accumulate experience from sorties and practice — the level table has lv99 and lv100 at the same threshold (1,000,000 exp), so unmarried ships silently reach level 100+ through `exp_to_ship_level`
- Ship remodeling writes equipment database IDs into `api_onslot` (aircraft capacity) instead of `api_slot` (equipment slots) — `remodel.rs:175`: `new_ship.api_onslot[i] = m.id` should be `new_ship.api_slot[i] = m.id`. This causes remodeled ships to have empty equipment slots and garbage aircraft capacity values
- CV opening airstrike (`simulate_kouku` in `crates/emukc_gameplay/src/game/battle/core.rs:1219`) picks a single random target for all bombers combined; real KanColle splits Stage 3 into dive bombing and torpedo bombing sub-phases

## Goals / Non-Goals

**Goals:**
- Zero experience awarded to unmarried ships at level 99+ from sorties and practice
- Remodel correctly assigns equipment IDs to `api_slot` and preserves aircraft capacity in `api_onslot`
- CV airstrike Stage 3 split into dive bombing + torpedo bombing, each with per-slot independent targeting
- Fix existing database records corrupted by the remodel bug

**Non-Goals:**
- Overhauling the battle system or damage formulas
- Changing marriage system requirements or ceremony flow
- Modifying client-side behavior or API response structure

## Decisions

### 1. Level 99 cap: propagate `married` flag through `BattleShipInput`

**Decision**: Add `married: bool` to `BattleShipInput` struct. Populate it from `ship.married` (DB field) in `build_sortie_friend_ships` and `build_practice_friend_ships`. Check `!married` in both `calculate_sortie_ship_exp` and `calculate_practice_ship_exp` to return 0 gain.

**Rationale**: `KcApiShip` does not carry a `married` field — it is lost during the `From<Model> for KcApiShip` conversion (`crates/emukc_db/src/entity/profile/ship/mod.rs:316`). Using `api_lv >= 100` as a proxy is unreliable because the current bug already allows unmarried ships to reach level 100+ via the shared exp threshold. The DB `married` field is the only reliable source of truth.

**Why check in `calculate_*_ship_exp` (not just in result processing)**: The exp gain values are included in the battle result response sent to the client. If we only zero exp in `update_sortie_result_stats` (which runs after the response is built), the client would see non-zero exp that is never applied — inconsistent and confusing.

**Key files**:
- `crates/emukc_gameplay/src/game/battle/core.rs:118` — add `married: bool` to `BattleShipInput`
- `crates/emukc_gameplay/src/game/sortie.rs:1267` — set `married` from `ship.married`
- `crates/emukc_gameplay/src/game/practice.rs:609` — set `married` from `ship.married`
- `crates/emukc_gameplay/src/game/sortie_result.rs:103` — check `!married`, return 0 gain
- `crates/emukc_gameplay/src/game/battle/practice.rs:504` — same check

**Alternative considered**: Use `api_lv >= 100` as proxy — rejected because existing bug allows unmarried ships at level 100+.

### 2. Remodel slot/onslot assignment fix

**Decision**: Change `remodel.rs:175` from `new_ship.api_onslot[i] = m.id` to `new_ship.api_slot[i] = m.id`.

**Root cause analysis**: The `codex.new_ship()` function returns `api_slot: [-1; 5]` (all empty) and `api_onslot: mst.api_maxeq` (correct capacities). The remodel loop then creates new equipment items and writes their database IDs to `api_onslot[i]` instead of `api_slot[i]`. This single wrong field assignment causes:
1. **Empty equipment slots**: `api_slot` stays `[-1; 5]`, so `slot_1..5` in the DB are all -1 — equipment appears unequipped after remodel
2. **Garbage aircraft capacity**: `api_onslot` gets polluted with equipment DB IDs (e.g., 50, 100, 500+), which the client interprets as "this slot holds 50/100/500 planes"
3. **These are the same bug**: The user's issues #2 (equipment not equipped) and #4 (CA with 50-plane slots) share this root cause

**Why the previous audit missed it**: The audit correctly verified that `remodel_impl` unequips all and adds codex defaults, and that `new_ship` correctly creates equipment from slot data. The bug is in the *assignment step* — the one-line write to the wrong field.

**Existing data repair**: Ships previously remodeled have corrupted `onslot_*` and `slot_*` values. A repair query will identify ships where `onslot_*` values exceed reasonable bounds for their ship type, reset onslot to codex `api_maxeq` values, and attempt to re-link equipment via `slot_*`.

### 3. CV airstrike: split Stage 3 into dive bombing + torpedo bombing

**Decision**: Refactor Stage 3 in `simulate_kouku` into two sequential sub-phases:
1. **Dive bombing phase**: iterate over each slot with dive bomber type aircraft (and sea-based bombers, jet bomber/attacker variants that use dive bombing formula), independently select a random alive target per slot, calculate and apply damage
2. **Torpedo bombing phase**: same for torpedo bomber type slots

Each sub-phase iterates per-slot, not per-ship. A ship with 3 dive bomber slots performs 3 independent target selections.

**Rationale**: Real KanColle's Stage 3 has two distinct sub-phases. `calculate_airstrike_damage` currently aggregates all bombers into one power sum — this must be replaced with per-slot damage calculation. The existing function distinguishes bomber types via `KcSlotItemType3::CarrierBasedTorpedoBomber` (uses `api_raig`) vs others (use `api_baku`), which maps directly to the two sub-phases.

**API compatibility**: `BattleKoukuStage3` flags (`erai_flag`, `ebak_flag`, `fcl_flag`) are per-ship arrays. Multiple slot attacks on the same ship accumulate damage correctly. No client-visible structural change.

**Important**: `is_airstrike_attack_type` includes 5 types (CarrierBasedDiveBomber, CarrierBasedTorpedoBomber, SeaBasedBomber, JetFighterBomber, JetAttacker). The split is: torpedo bombers → torpedo phase (uses `api_raig` stat), everything else → dive phase (uses `api_baku` stat).

**Alternative considered**: Per-slot targeting without sub-phase split — rejected because real KanColle processes dive and torpedo bombing in separate waves with independent target selection.

## Risks / Trade-offs

- **Airstrike balance change**: Multi-target airstrikes significantly increase total fleet damage output per battle. Existing tests may need adjustment for new damage distributions. → Run full battle test suite after change. No existing tests cover airstrike behavior directly.
- **Level cap struct change**: Adding `married` to `BattleShipInput` touches multiple call sites (sortie, practice, tests). All `BattleShipInput { ... }` construction sites must set the field. → Grep for all construction sites before implementing.
- **Remodel data repair scope**: Need to identify all ships previously remodeled and fix their onslot/slot values. If the profile has been in use for a while, many ships may be affected. → Write a repair function that iterates all ships, checks for corrupted onslot values, and resets them from codex data.
- **Practice exp path difference**: Sortie exp uses `BattleShipInput`, practice exp uses `BattleRuntimeShip`. The `married` field must propagate through `BattleRuntimeShip::new` for the practice path. → Add `married` to `BattleRuntimeShip` and carry it from `BattleShipInput`.
