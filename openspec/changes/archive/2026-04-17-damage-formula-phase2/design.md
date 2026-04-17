## Context

`crates/emukc_gameplay/src/game/battle/core.rs` contains four damage calculators (`calculate_shelling_damage`, `calculate_torpedo_damage`, `calculate_night_damage`, `calculate_asw_damage`). Each takes `(random, attacker, defender, formation, engagement)` and computes basic power from ship stats only. Equipment star levels (`api_level` on `KcApiSlotItem`) and type-specific formulas are absent.

Current function signatures accept `&Codex` only in `calculate_asw_damage`. The other three do not access equipment data.

## Goals / Non-Goals

**Goals:**
- Add `&Codex` parameter to all four damage calculators for equipment lookup
- Implement 改修強化 (star bonus) for day shelling, torpedo, and night battle
- Implement CV special formula for CV/CVL/CVB shelling
- Implement CL 軽砲補正 for CL/CLT shelling
- Implement 夜偵 contact bonus for night battle
- Implement ASW depth charge projector armor reduction
- All changes in `core.rs` only — no new files

**Non-Goals:**
- Submarine-specific armor correction (× 0.7 / × 0.55) — deferred to later phase
- Combined fleet / support expedition formulas
- Night battle CI damage multiplier changes
- OASW special ship conditions (Isuzu K2, Tatsuta K2) — separate track

## Decisions

### 1. Pass `&Codex` to all damage calculators

Current: only `calculate_asw_damage` takes `&Codex`. Change: all four functions take `&Codex` as first parameter after `random`. Callers already have `codex` available in `simulate_shelling_side`, `simulate_raigeki`, `simulate_opening_torpedo`, `simulate_night_hougeki`.

Rationale: avoids global state, matches existing ASW pattern, minimal signature churn since callers are all internal.

### 2. Equipment star lookup via `api_level`

`KcApiSlotItem.api_level` already stores the star level (0-10). No model changes needed. Lookup via `codex.find::<ApiMstSlotitem>(&si.api_slotitem_id)` for type info.

### 3. CV special formula: ship type check

Use existing `ship_type(codex, ship)` helper that returns `Option<KcShipType>`. Match on `CV | CVL | CVB`. Count bomber slots by iterating `slot_items` and checking `KcSlotItemType3`.

### 4. CL 軽砲補正: count by type3

Small caliber = `SmallCaliberMainGun`, medium caliber = `MediumCaliberMainGun`. Count equipped items of each type, apply `√single + 2√twin`.

### 5. 夜偵: pass air state into night battle

`simulate_night_battle_v1` currently has no air state input. Add `air_supremacy: Option<bool>` parameter (Some(true) = supremacy, Some(false) = superiority, None = no advantage). Callers (sortie/practice handlers) can derive from day battle's `api_disp_seiku`.

### 6. ASW armor reduction: subtract from defense

Compute projector armor reduction in `calculate_asw_damage`, subtract from `calculate_defense_power` result before `resolve_damage`.

## Risks / Trade-offs

- **[Star bonus affects existing test outputs]** → Existing tests use default equipment (★0), so bonus is 0. Only new tests with star>0 need verification.
- **[夜偵 air state not available in night-only battles]** → For sp_midnight (night-only), air state is unknown. Use `None` → no bonus. Acceptable: sp_midnight has no air phase.
- **[Depth charge projector item IDs]** → Initially use simplified type-based detection (all DepthCharge treated as projector). Refine with specific item IDs later if needed.
