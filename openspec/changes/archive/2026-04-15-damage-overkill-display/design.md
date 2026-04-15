## Context

`BattleRuntimeShip::apply_damage` (`core.rs:180-218`) currently returns a single `i64` — the clamped damage value (`raw_damage.min(self.current_hp)`). All API response fields (`api_damage`, `api_fydam`/`api_eydam`, `api_fdam`/`api_edam`) use this clamped value, so overkill is invisible to the client.

There are ~14 call sites across shelling, torpedo, airstrike, OASW, and night battle phases. All follow the same pattern: compute `raw`, call `apply_damage(random, raw, idx)`, record `dealt` in response fields.

## Goals / Non-Goals

**Goals:**
- API response damage fields show pre-clamp (raw) values
- HP tracking still uses effective (clamped) values
- Sinking protection behavior unchanged

**Non-Goals:**
- Changing damage formulas
- Changing MVP calculation (already uses `dealt` which tracks actual HP removed — keep using effective)
- Changing client-facing response structure (same JSON fields, different values)

## Decisions

### 1. Return type: `(i64, i64)` tuple instead of struct

**Decision**: `apply_damage` returns `(raw_damage, effective_damage)`.

**Rationale**: Minimal API change. Callers already have `raw` computed before the call — they can ignore the first element or use it directly. No new types needed.

**Alternative considered**: A `DamageResult { raw, effective }` struct — rejected because it adds a type for a two-field return that's immediately destructured at every call site.

### 2. Sinking protection returns raw input unchanged

**Decision**: When sinking protection triggers, the returned `raw_damage` is the original input (the lethal damage amount), not the proportional damage.

**Rationale**: The client sees the damage that was "dealt" before protection kicked in. The proportional reduction is internal — the client sees HP drop by less than the displayed damage, which matches real KanColle's behavior where protected ships still show the incoming damage number.

### 3. `damage_dealt` (MVP) uses effective damage

**Decision**: `ship.damage_dealt += effective` (not raw). Only actual HP removed counts toward MVP.

**Rationale**: Matches real KanColle — MVP is based on actual damage contribution, not overkill.

### 4. Sunk ships return raw = 0, effective = 0

**Decision**: Early return `(0, 0)` when ship is already sunk.

**Rationale**: No change from current behavior — sunk ships don't take damage.

## Risks / Trade-offs

- **[Test breakage]** → Many tests assert on damage values. Need to update expected values from clamped to raw. Low risk, mechanical change.
- **[Sinking protection display]** → When protection triggers, API shows lethal raw damage but HP only drops by proportional amount. Client may show unexpected HP delta. This matches real KanColle — acceptable.
