## Context

**Remodel HP bug**: `remodel_impl` creates a new ship instance with updated master data but preserves the old ship's current HP. The new master data may have a higher max HP (e.g., from kai → kai ni). The ship ends up with `nowhp < maxhp` immediately after remodel.

**CT dock bug**: The repair time formula uses `ship_type_mod` to scale repair duration. CT (Command Tender, 練習巡洋艦) is listed with `1.0` modifier (same as CL). In real KanColle, there may be a special CT modifier or a fleet-level mechanic where having a CT in the docking fleet reduces repair time. This needs verification.

## Goals / Non-Goals

**Goals:**
- HP fully restored after remodel
- CT dock time calculated correctly

**Non-Goals:**
- Changing remodel mechanics beyond HP restoration
- Changing repair time formula structure
- Other docking features

## Decisions

### D1: HP restoration at end of remodel

**Decision**: After `cal_ship_status` computes the new max HP, explicitly set `new_ship.api_nowhp = new_ship.api_maxhp` at the end of `remodel_impl`.

**Rationale**: Simple, direct fix. Matches real KanColle behavior where remodel always fully heals.

### D2: CT dock time investigation

**Decision**: First verify the correct CT modifier from wikiwiki. If CT has a special modifier, update `ship_type_mod`. If the "fleet CT reduces dock time" mechanic exists, implement it as an optional multiplier applied when a CT is present in the profile's fleet.

**Rationale**: The user reported "教练船入渠时间计算错误" — need to determine if this is a modifier error or a missing fleet-level mechanic.

## Risks / Trade-offs

- **[Risk] CT mechanic scope creep**: The fleet-level CT mechanic could be complex → Limit to verifying and fixing the modifier first; fleet mechanic as follow-up if needed
- **[Risk] Remodel HP edge case**: If a ship was already at full HP before remodel, this is a no-op → Acceptable, idempotent
