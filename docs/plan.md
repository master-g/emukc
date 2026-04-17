# EmuKC Plan

> Merged from `docs/plan.md`, `docs/audit.md`, and `docs/battle/plan.md`.
> Last updated: 2026-04-17

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
- **Sortie durability gap fix** (2026-04-15): Store update placed after `tx.commit()`. Crash after commit leaves store stale, but store rebuilds from DB on restart. Commit failure no longer corrupts in-memory state.
- **Route predicate key fix** (2026-04-16): `route_predicate_key` returns `String` with full predicate content (fields + values), not a kind-only `&str`. Previous discriminant-based `match` collapsed different thresholds of the same predicate type into one bucket, causing wrong routing.
- **Unlock cascade test** (2026-04-16): Crate-internal test exercises `apply_sortie_map_result` → `check_and_unlock_dependencies_impl` cascade. External test drives full sortie-clear path (start_sortie → battle → result → verify `get_map_infos` shows 1-2).
- **Enemy battle-data source** (2026-04-17, verified): `enemy_ship_extra` codex field holds 841 abyssal ship entries from kcwiki bootstrap. `build_sortie_enemy_ship()` uses 3-tier fallback: enemy_extra → ship_extra → manifest. Enemy DD torpedo stats (api_raisou) populated correctly from bootstrap data. Gap #1 (Enemy DD torpedo) and Gap #2 (Enemy master-data) resolved.

## Constraints

- Keep source-specific parsing in `emukc_bootstrap`; do not reintroduce raw external HTML formats into `emukc_model`.
- Keep runtime isolated from non-official external sources.
- Treat `wikiwiki` as primary offline semantic source for maps.
- Treat `kc_data` only as structural complement, with `kc_data-only` reserved for explicit degraded mode.

---

## Current Gaps

| # | Gap | Impact | Priority |
|---|-----|--------|----------|
| 1 | ~~Enemy DDs skip torpedo~~ | **RESOLVED** (2026-04-17): 841 enemy ships in `enemy_ship_extra` | — |
| 2 | ~~Enemy master-data source~~ | **RESOLVED** (2026-04-17): kcwiki bootstrap + 3-tier fallback | — |
| 3 | **Day battle formula accuracy** | Partially done (改修強化, CV special, CL 軽砲補正, 夜偵, ASW 減甲 landed). Remaining: defense randomization, damage state modifier, artillery spotting, critical hit, ammo modifier, contact modifier, carrier night attack | Medium |
| 4 | **Missing special OASW** | Isuzu K2 / Tatsuta K2 等無条件 OASW 未実装 | Medium |
| 5 | **Target taxonomy** | Attacker-side land/surface legality not fully wired | Medium |
| 6 | **Display/response rules** | Still partly hardcoded | Lower |
| 7 | **Combined fleet / LBAS / support** | 14+ endpoints, major feature gap | Large effort |
| 8 | **Arrival-context routing** | Only sortie-wide `VisitedNode`, not per-arrival context | Lower |
| 9 | **Battle verification infrastructure** | No behavioral validation, no resource existence checks, no battle invariants beyond sinking protection. See Track 5 | Medium-High |

---

## Next Tracks

### Track 1: Enemy Battle-Data Source (COMPLETED 2026-04-17)

`enemy_ship_extra` codex field with 841 abyssal entries. `build_sortie_enemy_ship()` uses 3-tier fallback: enemy_extra → ship_extra → manifest. Regression tests cover all fallback paths.

Only after enemy stats are stable: `airbattle` / `sp_midnight` specialization and broader sortie fidelity.

### Track 2: Damage Formula Corrections (PARTIALLY COMPLETED 2026-04-17)

Landed in `e940330`: 改修強化, CV special (`1.5× + 55`), CL 軽砲補正 (`√単装 + 2√連装`), 夜偵 (+5/+7/+9), ASW 減甲 (`√(DC_asw − 2) × 0.25`).

Remaining sub-items (see `docs/battle/damage-formula-reference.md` for full gap table):

1. Defense randomization: replace fixed `A×k` with `floor(0.7×A + 0.6×rand(0, floor(A)−1))` — affects ALL attack types
2. Damage state modifier: chuuha×0.7 (shelling/ASW), ×0.8 (torpedo); taiha×0.4 (shelling/ASW), ×0 (torpedo)
3. Artillery spotting (弾着観測射撃): DA/CI post-cap modifiers, requires AS+ and recon
4. Critical hit system: 1.5× post-cap, trigger rates
5. Ammo modifier: `floor(remaining/50)/100` post-cap
6. Contact modifier: ×1.12–1.20 pre-cap for airstrike
7. Carrier night air attack: entirely separate formula `K_a × (FP + TP + DB×K_b)`
8. Per-type airstrike cap: separate TB/DB calculations instead of blended

### Track 3: Target Legality / Taxonomy

Complete `Installation`/`PT`/submarine attacker-side legality. Foundation for combined fleet / support / event battle.

### Track 4: Advanced Battle Topologies

Combined fleet, support expedition, LBAS. Only after Tracks 1–3.

### Track 5: Battle Verification

Two core concerns driving this track:

1. **Format correctness**: Battle response must conform to `apilist.txt` and `main.js` expectations — no semantic/logic errors that cause the client to crash.
2. **Behavioral correctness**: Battle actions must match the original game — impossible actions (wrong equipment in `api_si_list`, invalid CI types) cause `main.js` to request non-existent resources → 404 → client crash.

The crash chain: `wrong behavior → wrong api_si_list → getSlotitem(btxt_flat) → 404 → crash`. Entity existence ≠ resource existence (cf. `incident_slot_102.json`).

#### Three-Layer Verification Architecture

**V1: Structural validation** (existing):
- Protocol field completeness, flag-payload consistency, array length alignment, entity ID existence.
- Implemented: `validate_day_battle_response()` in `emukc_bootstrap/battle_rules.rs`.
- Driven by: `battle_protocol_fields.json`, `battle_resource_rules.json` from `main-decoder`.

**V2: Behavioral validation** (new):
- `api_si_list` only contains equipment the ship actually has equipped.
- Attack type matches equipment combination (no DD doing carrier CI).
- Dead ships don't attack.
- Phase ordering correct (kouku → OASW → opening torpedo → shelling → closing torpedo).
- HP delta per phase = sum of individual damages.
- Win rank consistent with fleet HP ratios.
- MVP consistent with damage dealt.
- Implementation: `validate_battle_behavior()` post-simulation function.

**V3: Resource existence validation** (new):
- Every slotitem in `api_si_list` has `btxt_flat` and `item_up` resources in cache.
- Ship damage-state resources exist (banner_dmg, full_dmg).
- CI textures exist for the selected CI type.
- Cross-reference: `battle_slot_resource_triggers.json` (17 triggers from `main-decoder`) + cache manifest.
- Implementation: `validate_battle_resources()` using codex + cache.

#### Open Questions for Exploration

1. **Can V2 be guaranteed by construction?** — If `api_si_list` generation is type-safe (only draws from ship's actual equipment), V2 may not need post-hoc validation. Need to audit current generation code.
2. **Resource coverage completeness** — main-decoder extracts 17 slot resource triggers. Is this exhaustive? Does it cover night battle, OASW, etc.?
3. **main.js version drift** — Each KC update may change cutin modules. Need process to detect when battle knowledge assets are stale.
4. **Test infrastructure** — Consider property-based testing (proptest) for battle invariants, and golden-file/snapshot testing for regression detection.

#### Existing Assets

- `docs/battle/damage-formula-reference.md`: Wiki formulas vs current code with gap analysis.
- `docs/battle/rules.md`: Implemented rules register with A/B/C confidence levels.
- `main-decoder/`: Extracts battle knowledge from `main.js` (protocol fields, resource rules, slot triggers).
- `crates/emukc_bootstrap/assets/battle_*.json`: Synced battle knowledge assets.
- `tests/fixtures/battle/`: Incident corpus (currently 1 entry — `incident_slot_102.json`).

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

# Battle validation
cargo run -- battle validate --input <battle.json>
cargo run -- battle analyze-incident --input <battle.json> --missing-url <url>
cd main-decoder && bun run decode -- --sync-battle-assets
```

---

## Follow-up

- Add fixture coverage for a map requiring both wikiwiki semantics and `kc_data` structure.
- Do not introduce AST runtime unless a concrete rule family can be parsed reliably but cannot compile into flat `RouteRule`.
- Revisit whether runtime should keep merging `kc_data` on startup, or move merge entirely into offline codex generation.
- Expand `tests/fixtures/battle/` incident corpus for V3 resource validation coverage.
- Audit `api_si_list` generation code to determine if V2 behavioral guarantees can be enforced by construction.
