# Plan

## Current Status

- `wikiwiki` map extraction and routing parsing now live in `emukc_bootstrap`.
- Runtime map loading still does `repo wikiwiki catalog + kc_data structural complement`; it is not codex-only yet.
- Single-fleet sortie flow now covers `api_req_map/start`, `api_req_map/next`, day battle, battle result, and standard night battle.
- Practice flow now covers day battle, result settlement, and night battle on the shared battle core.
- Sortie enemy selection now uses weighted node compositions instead of always picking the first catalog entry.
- Fallback enemy fleets are now an explicit degraded path only when codex enemy data is missing.
- Placeholder single-fleet routes exist for `api_req_sortie/airbattle`, `api_req_sortie/goback_port`, and `api_req_battle_midnight/sp_midnight` on top of the existing sortie state machine.
- Sortie battle result now emits quest events, so normal single-fleet sortie quests can advance from real battle settlement.
- Sortie quest matcher now understands map/boss/result conditions, including `All(map)` cycle reset for multi-round quests.
- Practice battle result now emits exercise quest events, including exercise quests that carry fleet composition requirements.
- `kc_data` route-only numeric placeholder nodes no longer generate fake runtime cells, and `1-1 map/start` now correctly stays at four cells.
- Remaining map work is no longer in the wikiwiki route parser: repo asset route rules are now at `0` `Unknown`, `0` `SourceUnknown`, and `0` `parse_warnings`.
- Remaining battle blocker is data-source, not formula: many early abyssal IDs still have no usable HP/armor/firepower source, so sortie enemy fallback builds `HP=1` enemies.
- **Map unlock progression implemented** (2026-04-13): `api_get_member/mapinfo` now only shows unlocked maps; `api_req_sortie/battleresult` returns `api_next_map_ids` on map clear; sortie gated by unlock status.

## Constraints

- Keep source-specific parsing in `emukc_bootstrap`; do not reintroduce raw external HTML formats into `emukc_model`.
- Keep runtime isolated from non-official external sources.
- Treat `wikiwiki` as the primary offline semantic source for maps.
- Treat `kc_data` only as a structural complement source, with `kc_data-only` reserved for explicit degraded mode when the repo-tracked wikiwiki asset is unavailable.

---

## Current Gaps (as of 2026-04-13)

| Gap | Impact | Priority |
|-----|--------|----------|
| ~~**Map unlock progression**~~ | ~~New accounts can sortie any map immediately~~ | ~~Highest~~ **DONE** |
| **入渠 material response bug** | Repair/speedup causes dev materials and buckets to display as 0 or 1 until port refresh | High |
| **Damage capping prevents overkill** | Damage clamped to target HP, real game allows overkill | High |
| **Enemy destroyers skip torpedo attack** | 雷撃戦 phase: enemy DDs not firing torpedoes | High |
| **Enemy master-data source** | Many abyssal IDs have HP=1 via manifest fallback | High |
| **Damage formula accuracy** | Night battle engagement modifier bug, simplified formulas | Medium-High |
| **Target taxonomy** | Attacker-side land/surface legality not wired | Medium |
| **Display/response rules** | Still partly hardcoded | Lower |
| **Combined fleet / LBAS / support** | 14+ endpoints, major feature gap | Large effort |

---

## Next Session

### Track 0: Map Unlock Progression System ✅ DONE (2026-04-13)

Implemented in `openspec/changes/map-unlock-progression/`. All automated tasks complete (27/29, remaining 2 are manual verification).

**What was done**:
- `MapCatalog.prerequisites: HashMap<i64, i64>` — static prerequisite table for regular maps (same-area sequential + cross-area boss)
- `map_record.unlocked: bool` column — per-player unlock state, migrated with DEFAULT true for existing accounts
- `is_map_unlocked_by_default()` — new profiles: only 1-1 unlocked; areas 1-7 regular maps locked by default; EO/event/test maps unlocked
- `build_map_infos()` — filters out `unlocked = false` records
- `start_sortie()` — rejects sortie to locked maps with `GameplayError::Locked`
- `check_and_unlock_dependencies_impl()` — cascade unlock on map clear
- `api_next_map_ids` field on `SortieBattleResultResponse` — returns newly unlocked map IDs
- 9 unit tests + 3 integration tests, all passing

### Track 0.5: 入渠 Material Response Bug

**Problem**: After repairing ships (入渠), client shows dev materials (開発資材) and buckets (高速修復材) as 0 or 1. Must return to port (`api_port/port`) or claim rewards to see correct values.

**Root cause analysis**:
- `api_req_nyukyo/start` only returns `api_material` when `api_highspeed=1` (bucket repair). Normal repair returns empty response (`KcApiResponse::empty()`)
- `api_req_nyukyo/speedchange` returns `api_material` but the `Vec<i64>` only contains 8 raw values — verify the field mapping is correct
- The material deduction uses `deduct_material_impl()` which returns the updated material state, but the response may not be propagating all 8 material types correctly
- Client expects material state updates after docking actions; missing fields cause it to fall back to stale/zero values

**Steps**:
1. Compare nyukyo API response format with real game captures (check `api_req_nyukyo/start` and `speedchange` responses)
2. Verify `api_material` Vec contains all 8 material values in correct order (fuel, ammo, steel, bauxite, torch, bucket, devmat, screw)
3. Ensure normal repair (non-highspeed) also returns material state — the fuel/steel cost should be reflected
4. Check if `api_req_nyukyo/start` needs additional fields beyond `api_material` (e.g., `api_ship_id`, `api_ndock_id`)
5. Verify `deduct_material_impl` return value is complete and not missing any material categories

**Key files**: `src/bin/net/router/kcsapi/api_req_nyukyo/`, `crates/emukc_gameplay/src/game/ndock.rs`

### Track 0.6: Battle Damage Overkill Capping

**Problem**: Damage is clamped to target HP (`core.rs:190`: `let effective = raw_damage.min(self.current_hp)`). In real KanColle, overkill damage is calculated and displayed (e.g., 100+ damage against a 1 HP enemy).

**Root cause**: `BattleRuntimeShip::apply_damage()` clamps effective damage to current HP. The raw damage is calculated correctly but then capped before being recorded. The client uses raw damage values for display (HP bar animation, damage numbers).

**Steps**:
1. Separate "effective damage" (HP actually subtracted) from "raw damage" (full calculated value before HP clamping)
2. In the battle packet (`BattlePacket`), record both raw and effective damage so the API response shows raw damage to the client
3. HP tracking should use effective (clamped) damage; display should use raw damage
4. Verify `api_hougeki.api_damage` format — it should contain the raw (uncapped) damage values
5. Cross-check with real game battle captures to confirm the expected damage values

**Key files**: `crates/emukc_gameplay/src/game/battle/core.rs:190`, `BattlePacket` struct, hougeki serialization

### Track 0.7: Enemy Destroyer Torpedo Attack Missing

**Problem**: During the 雷撃戦 (torpedo phase) of day battle, enemy destroyers are not firing torpedoes.

**Root cause hypothesis**: The torpedo phase filters participants by `api_raisou[0] > 0` (torpedo stat > 0). Many enemy destroyers built from manifest fallback have `api_raisou = [0, 0]` because the manifest doesn't provide torpedo stats for abyssal ships. This is a data-source issue (same as Track 1 enemy master data) combined with possible phase-selection logic.

**Investigation steps**:
1. Check wikiwiki for abyssal ship torpedo stats — confirm that real enemy DDs have non-zero 雷装 values
2. Verify `build_sortie_enemy_ship()` sets `api_raisou` correctly for enemy destroyers with available data
3. Check the torpedo phase entry conditions in `core.rs` — is `api_raisou[0] > 0` the only gate, or are there ship-type filters?
4. If the issue is data-source, Track 1 (enemy master data) will resolve this. If there's a logic bug, fix the phase selection.
5. Add a test case with a known enemy DD (e.g., 駆逐イ級) that has non-zero torpedo stat, verify it fires in torpedo phase

**Key files**: `crates/emukc_gameplay/src/game/battle/core.rs` (torpedo phase), enemy ship builder in sortie module, wikiwiki/kc_data enemy data extraction

### Track 1: Enemy Battle-Data Source

Introduce an enemy master/stat data source into codex/bootstrap and switch `build_sortie_enemy_ship()` to use it before manifest fallback. Add regression tests for current normal-map enemy coverage.

Only after enemy stats are stable, continue with `airbattle` / `sp_midnight` specialization and broader sortie battle fidelity work.

### Track 2: Damage Formula Corrections

1. Fix night battle damage not affected by engagement modifier (audit bug)
2. More accurate day battle formulas (CV special, light cruiser correction, improvement stars)
3. Cross-check with `main-decoder` client battle rules

### Track 3: Target Legality / Taxonomy

Complete `Installation`/`PT`/submarine attacker-side legality. Foundation for combined fleet / support / event battle.

### Track 4: Advanced Battle Topologies

Combined fleet, support expedition, LBAS. Only after Tracks 1-3.

---

## Follow-up

- Add explicit fixture coverage for a map where wikiwiki semantics and `kc_data` structure are both required, so the complement boundary stays regression-tested.
- Do not introduce an AST runtime unless a concrete wikiwiki rule family can be parsed reliably but cannot be compiled into flat `RouteRule`.
- Consider whether the repo-tracked wikiwiki asset should eventually move out of `emukc_bootstrap/assets` into a more model-centric generated-data location.
- Revisit whether runtime should keep merging `kc_data` on startup, or whether that merge should move entirely into offline codex generation once the complement path is stable enough.

## Battle Subsystem Detail

See `docs/battle/plan.md` for the battle subsystem baseline, current gaps, and per-track details.

## Map Subsystem Detail

See `docs/map/audit.md` for map audit results (2026-04-10).

## Validation

```bash
cargo test -p emukc_gameplay
cargo test --workspace
cargo clippy --workspace
cargo run -- serve  # manual sortie flow verification
```
