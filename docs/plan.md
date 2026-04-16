# EmuKC Plan

> Merged from `docs/plan.md`, `docs/audit.md`, and `docs/battle/plan.md`.
> Last updated: 2026-04-16

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
- **入渠 material response fix** (2026-04-15): `api_req_nyukyo/start` and speedup now return correct `api_material` with all 8 values.
- **Battle damage overkill display** (2026-04-15): `apply_damage` returns `(raw, effective)` tuple; API response shows raw damage including overkill; HP tracking uses effective (clamped).
- **Sortie durability gap fix** (2026-04-15): Store update moved before `tx.commit()` so crash cannot leave DB persisted but memory stale.
- **Route predicate key optimization** (2026-04-16): `route_predicate_key` replaced JSON serialization with discriminant-based `match` returning `&str`. Zero allocations, no serde dependency.
- **Unlock cascade test** (2026-04-16): `clearing_map_1_1_unlocks_dependents_via_cascade` crate-internal test exercises `apply_sortie_map_result` → `check_and_unlock_dependencies_impl` cascade. External test simplified to verify public `get_map_infos` behavior only.

## Constraints

- Keep source-specific parsing in `emukc_bootstrap`; do not reintroduce raw external HTML formats into `emukc_model`.
- Keep runtime isolated from non-official external sources.
- Treat `wikiwiki` as primary offline semantic source for maps.
- Treat `kc_data` only as structural complement, with `kc_data-only` reserved for explicit degraded mode.

---

## Current Gaps

| # | Gap | Impact | Priority |
|---|-----|--------|----------|
| 1 | **Enemy DDs skip torpedo** | 雷撃戦 phase: many enemy DDs have `api_raisou=[0,0]` due to manifest fallback | High |
| 2 | **Enemy master-data source** | Many abyssal IDs have HP=1 via manifest fallback; limits battle fidelity | High |
| 3 | **Day battle formula accuracy** | Missing: 改修強化, CV special formula, CL 軽砲補正, armor × 0.7/0.55 simplification | Medium-High |
| 4 | **Missing special OASW** | Isuzu K2 / Tatsuta K2 等無条件 OASW 未実装 | Medium |
| 5 | **Target taxonomy** | Attacker-side land/surface legality not fully wired | Medium |
| 6 | **Display/response rules** | Still partly hardcoded | Lower |
| 7 | **Combined fleet / LBAS / support** | 14+ endpoints, major feature gap | Large effort |
| 8 | **Arrival-context routing** | Only sortie-wide `VisitedNode`, not per-arrival context | Lower |

---

## Next Tracks

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

- **Test-only `From<BattleShipInput>` hides protection behavior**: `#[cfg(test)]` impl defaults to `(input, false, false)` → enemy + non-sortie, disabling protection. Tests must explicitly call `BattleRuntimeShip::new(..., true, true)` to test protection.
- **`route_predicate_key` JSON serialization** (FIXED 2026-04-16): Replaced with discriminant-based `match` returning `&str`.

### Battle

- **`apply_sortie_map_result` returns 0 (non-first-clear) on variant switch** (`sortie_result.rs:398`): If map needs multiple Boss defeats to clear, `api_first_clear` always returns 0. Verified as by-design for stage-variant progression.
- **Missing special OASW**: Isuzu K2, Tatsuta K2 等改二無条件 OASW.
- **Installation / PT**: Merged into surface-like bucket. Attacker-side legality not differentiated.

### Map

- **`node_label` merge identity**: Current merge primary key is `cell_no`, not `node_label`. Verified as correct — `node_label` is display-only, `cell_no` is the canonical key.
- **Arrival-context routing (`ArrivedFrom`)**: Only sortie-wide `VisitedNode`, not per-arrival context.

### Testing

- **Missing end-to-end integration test**: Simulate Boss victory through full public API flow, verify `api_next_map_ids` contains unlocked maps.

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
