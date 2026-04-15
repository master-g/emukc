# EmuKC Plan

> Merged from `docs/plan.md`, `docs/audit.md`, and `docs/battle/plan.md`.
> Last updated: 2026-04-15

## Current Status

### Completed (feat/vibe branch)

- **Map unlock progression** (2026-04-13): `api_get_member/mapinfo` only shows unlocked maps; `api_req_sortie/battleresult` returns `api_next_map_ids` on map clear; sortie gated by unlock status; EO maps included in prerequisite chain.
- **Audit fixes phase 1** (2026-04-14): Route selection overflow bias fixed (`keys().last()`); night battle engagement modifier removed; sinking protection uses `entry_hp` + integer arithmetic; EO map prerequisites added; `std::sync::Mutex` → `parking_lot::Mutex`; test coverage for all fixes.
- **Single-fleet sortie flow**: `api_req_map/start`, `api_req_map/next`, day battle, battle result, standard night battle.
- **Practice flow**: Day battle, result settlement, night battle — shared battle core.
- **Sortie enemy selection**: Weighted node compositions from map catalog, not first-catalog-entry.
- **Battle phases**: 航空戦 (kouku), OASW, 開幕雷撃, 昼戦砲撃×2, 閉幕雷撃, 夜戦 CI/連击, sp_midnight.
- **BattleType**: Normal, AirBattle, LdAirBattle, LdShooting.
- **Sortie quest events**: Map/boss/result conditions, `All(map)` cycle reset for multi-round quests.
- **Sinking protection (轟沈ストッパー)**: Non-taiha-entry friendly ships survive; flagship always protected; practice/enemy excluded; post-condition assertion `verify_protected_ships_alive`.
- **Map route predicates**: 18 types implemented (Always, FleetSizeWeightedRandom, VisitedNode, FleetSize, EquipmentCount, ShipTypeCount, FlagshipShipType/Id, ContainsShipType/Id, ContainsShipSet, OnlyShipSet, OnlyShipTypes, ShipSetCount, ShipSetSpeedCount, Speed, LoS, DrumCanisterCount, And/Or/Not). 0 Unknown, 0 SourceUnknown in repo assets.
- **Data fidelity**: `api_si_list` per-attack-context equipment display; torpedo payload direction corrected; enemy bootstrap equipment non-manifest items discarded with `onslot` zeroed.

## Constraints

- Keep source-specific parsing in `emukc_bootstrap`; do not reintroduce raw external HTML formats into `emukc_model`.
- Keep runtime isolated from non-official external sources.
- Treat `wikiwiki` as primary offline semantic source for maps.
- Treat `kc_data` only as structural complement, with `kc_data-only` reserved for explicit degraded mode.

---

## Current Gaps

| # | Gap | Impact | Priority |
|---|-----|--------|----------|
| 1 | **入渠 material response bug** | Repair/speedup causes dev materials and buckets to display as 0 or 1 until port refresh | High |
| 2 | **Damage capping prevents overkill** | Damage clamped to target HP; real game allows overkill display | High |
| 3 | **Enemy DDs skip torpedo** | 雷撃戦 phase: many enemy DDs have `api_raisou=[0,0]` due to manifest fallback | High |
| 4 | **Enemy master-data source** | Many abyssal IDs have HP=1 via manifest fallback; limits battle fidelity | High |
| 5 | **Day battle formula accuracy** | Missing: 改修強化, CV special formula, CL 軽砲補正, armor × 0.7/0.55 simplification | Medium-High |
| 6 | **Missing CI types** | 主AP CI, 主雷達 CI, 瑞雲立体, 海空立体, 戦爆連合 CI not implemented | Medium |
| 7 | **Missing special OASW** | Isuzu K2 / Tatsuta K2 等無条件 OASW 未実装 | Medium |
| 8 | **Target taxonomy** | Attacker-side land/surface legality not fully wired | Medium |
| 9 | **Display/response rules** | Still partly hardcoded | Lower |
| 10 | **Combined fleet / LBAS / support** | 14+ endpoints, major feature gap | Large effort |
| 11 | **`sortie_battle_result` durability gap** | SortieStore update after `tx.commit()` — crash between them leaves inconsistent state | Low |

---

## Next Tracks

### Track 0.5: 入渠 Material Response Bug

**Problem**: After repairing ships (入渠), client shows dev materials (開発資材) and buckets (高速修復材) as 0 or 1. Must return to port to see correct values.

**Steps**:
1. Compare nyukyo API response format with real game captures
2. Verify `api_material` Vec contains all 8 values in correct order (fuel, ammo, steel, bauxite, torch, bucket, devmat, screw)
3. Ensure normal repair (non-highspeed) also returns material state
4. Check if `api_req_nyukyo/start` needs additional fields (e.g., `api_ship_id`, `api_ndock_id`)

**Key files**: `src/bin/net/router/kcsapi/api_req_nyukyo/`, `crates/emukc_gameplay/src/game/ndock.rs`

### Track 0.6: Battle Damage Overkill Capping

**Problem**: `core.rs:190` clamps effective damage to `self.current_hp`. Real KanColle calculates and displays overkill (e.g., 100+ damage against 1 HP enemy).

**Steps**:
1. Separate "effective damage" (HP subtracted) from "raw damage" (pre-clamp value)
2. Record both in `BattlePacket` — API response shows raw damage
3. HP tracking uses effective; display uses raw
4. Cross-check with real game captures

**Key files**: `crates/emukc_gameplay/src/game/battle/core.rs`, `BattlePacket`, hougeki serialization

### Track 0.7: Enemy Destroyer Torpedo Attack

**Problem**: 雷撃戦 phase: enemy DDs not firing torpedoes. Many have `api_raisou=[0,0]` from manifest fallback (data-source issue).

**Steps**:
1. Check wikiwiki for abyssal DD torpedo stats
2. Verify `build_sortie_enemy_ship()` sets `api_raisou` correctly when data available
3. If data-source issue, Track 1 resolves this. If logic bug, fix phase selection.
4. Add test with known enemy DD (e.g., 駆逐イ級) having non-zero torpedo

**Key files**: `crates/emukc_gameplay/src/game/battle/core.rs`, enemy ship builder, wikiwiki/kc_data enemy extraction

### Track 1: Enemy Battle-Data Source

Introduce enemy master/stat data source into codex/bootstrap. Switch `build_sortie_enemy_ship()` to use it before manifest fallback. Add regression tests for normal-map enemy coverage.

Only after enemy stats are stable: `airbattle` / `sp_midnight` specialization and broader sortie fidelity.

### Track 2: Damage Formula Corrections

1. Day battle: 改修強化, CV special (`1.5× + 55`), CL 軽砲補正 (`√単装 + 2√連装`), armor correction
2. Torpedo: 改修強化 (魚雷★ × 1.2), armor correction
3. Night battle: 改修強化, 夜偵 constant (+5/+7/+9)
4. ASW: 爆雷投射機 / Hedgehog √(装備対潜-2) 減甲

### Track 3: Target Legality / Taxonomy

Complete `Installation`/`PT`/submarine attacker-side legality. Foundation for combined fleet / support / event battle.

### Track 4: Advanced Battle Topologies

Combined fleet, support expedition, LBAS. Only after Tracks 1–3.

---

## Audit Reference

Remaining findings from code audit (2026-04-14) not yet addressed:

### Architecture

- **`sortie_battle_result` durability gap**: `SortieStore` updated after `tx.commit()` (`sortie.rs:583`). Crash between commit and store update → DB persistent but memory stale. Low risk for turn-based game. (`sortie_battle_impl` at `sortie.rs:946` does store update before commit — no issue there.)
- **Test-only `From<BattleShipInput>` hides protection behavior**: `#[cfg(test)]` impl defaults to `(input, false, false)` → enemy + non-sortie, disabling protection. Tests must explicitly call `BattleRuntimeShip::new(..., true, true)` to test protection.
- **`route_predicate_key` uses JSON serialization as grouping key** (`map_route.rs:451`): Expensive and fragile. Prefer discriminant-based key or `BTreeSet` with `RoutePredicate` comparison.

### Battle

- **`apply_sortie_map_result` returns 0 (non-first-clear) on variant switch** (`sortie_result.rs:398`): If map needs multiple Boss defeats to clear, `api_first_clear` always returns 0. Verify this matches intended stage-variant design.
- **Victory rate formula simplified**: `calculate_win_rank` uses absolute enemy damage rate and half-sunk rule. Original uses relative damage rate and flagship-sunk override rules.
- **Missing CI types**: 主AP CI (1.3×), 主雷達 CI (1.2×), 瑞雲立体 (1.35×), 海空立体 (1.3×), 戦爆連合 CI (FBA/BBA/BA).
- **Missing special OASW**: Isuzu K2, Tatsuta K2 等改二無条件 OASW.
- **Installation / PT**: Merged into surface-like bucket. Attacker-side legality not differentiated.

### Map

- **4 remaining `Unknown` predicates**: Low priority, extend parser vocabulary.
- **`node_label` merge identity**: Current merge primary key is `cell_no`, not `node_label`.
- **Arrival-context routing (`ArrivedFrom`)**: Only sortie-wide `VisitedNode`, not per-arrival context.

### Testing Gaps

- **`clearing_1_1_unlocks_1_2` tests wrong thing**: Modifies DB directly instead of simulating Boss win → `apply_sortie_map_result` → `check_and_unlock_dependencies_impl` cascade.
- **Missing end-to-end integration test**: Simulate Boss victory, verify `api_next_map_ids` contains unlocked maps.

---

## Validation

```bash
cargo test -p emukc_gameplay
cargo test --workspace
cargo clippy --workspace
cargo run -- serve  # manual sortie flow verification
```

---

## Follow-up

- Add fixture coverage for a map requiring both wikiwiki semantics and `kc_data` structure.
- Do not introduce AST runtime unless a concrete rule family can be parsed reliably but cannot compile into flat `RouteRule`.
- Revisit whether runtime should keep merging `kc_data` on startup, or move merge entirely into offline codex generation.
