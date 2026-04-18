## Context

EmuKC's battle simulation correctly applies sinking protection (轟沈ストッパー) internally — replacing lethal damage with proportional damage for protected ships — but reports the raw pre-protection damage to the client in all phase output arrays. The client uses these damage values to animate combat sequentially, so it sees ships reach 0 HP and triggers sinking animations, even though the server correctly kept them alive.

The map catalog data (derived from wikiwiki sources) has systematic errors: `boss_cell_no` is wrong for 30/32 maps, and many cells have incorrect `color_no`/`event_id`/`event_kind`. Real KC API capture data exists in `docs/real_data/map_start_data/` for 33 maps and serves as the ground truth.

## Goals / Non-Goals

**Goals:**
- Battle phase damage arrays report effective damage (post-sinking-protection) so the client correctly animates HP changes
- Map catalog data matches real KC API data for all maps with available captures
- Sortie `api_cell_data` correctly initializes all cells as un-passed at sortie start

**Non-Goals:**
- Changing how sinking protection itself works (formula, conditions)
- Adding per-phase HP snapshots to battle response (real KC uses cumulative damage subtraction)
- Fixing map routing rules (these appear correct; only cell metadata is wrong)
- Supporting maps without real KC capture data (event maps, etc.)

## Decisions

### D1: Report `dealt` instead of `raw_dealt` in all phase outputs

**Decision**: Change all 11 locations in `crates/emukc_gameplay/src/game/battle/core.rs` from `raw_dealt` to `dealt` in client-facing damage arrays (hougeki, torpedo, kouku, OASW, night battle).

**Rationale**: The `apply_damage()` method already returns both values. `dealt` is the actual HP subtracted. The internal state tracking (`ship.damage_dealt += dealt`) already uses `dealt`. Only the output arrays were using `raw_dealt` by mistake.

**Alternative considered**: Clamp damage to `target.current_hp` at report time — rejected because it would double-clamp (sinking protection already does this) and wouldn't reflect the proportional damage that protection converts to.

### D2: Data migration from real KC API captures

**Decision**: Write a one-time Python script to merge real KC `api_cell_data` (color_no, bosscell_no) and inferred `event_id`/`event_kind` into `map_catalog.json`.

**Mapping**: color_no → (event_id, event_kind):
- 0 → (0, 0) start
- 2 → (2, 0) resource
- 3 → (3, 0) maelstrom
- 4 → (4, 1) battle
- 5 → (5, 1) boss
- 9+ → (color_no, 1) special (air recon, escort, etc.)

**Rationale**: The wikiwiki catalog source produced corrupted cell metadata. Real API captures are authoritative. For cells with color_no=4 that might be air battles (event_id=7) vs normal battles (event_id=4), we default to event_id=4 since the real `api_cell_data` doesn't distinguish them. Event_kind=1 (battle) is always correct for color_no >= 4.

**Alternative considered**: Re-bootstrap from wikiwiki — rejected because the parsing bug would likely reproduce the same errors.

### D3: Fix `passed` to `false` at sortie start

**Decision**: Change `build_sortie_cell_data` in `crates/emukc_gameplay/src/game/sortie.rs:994` from `passed: cell.cell_no > 0` to `passed: false`.

**Rationale**: Real KC `api_cell_data` shows `api_passed: 0` for unvisited cells at start. The client uses this flag to display node labels. Marking all cells as passed hides labels and may affect edge rendering.

## Risks / Trade-offs

- **[Data gap]** Some color_no=4 cells might be air battles (event_id=7) rather than normal battles (event_id=4). → Mitigation: Default to event_id=4 (normal battle). Incorrect event_id doesn't affect routing or battle triggering (both use event_kind=1). Only affects enemy fleet selection in edge cases.
- **[Missing maps]** Real data unavailable for some event maps and map 6-3. → Mitigation: Skip those maps in migration; they remain with wikiwiki data.
- **[No rollback for data]** Once `map_catalog.json` is updated, reverting requires git. → Mitigation: Commit data fix separately from code fix for easy revert.
