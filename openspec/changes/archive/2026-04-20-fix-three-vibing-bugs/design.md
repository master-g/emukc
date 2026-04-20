## Context

Three independent bugs introduced during the vibing sprint (commits 7e0d397–e866d02). Each is a localized logic error with no cross-cutting architectural impact.

## Goals / Non-Goals

**Goals:**
- Restore correct manifest path generation for `damagedSource == "true"` ship entries.
- Restore correct sortie cell initialization (all cells unpassed at start).
- Fix stale target list in airstrike phase so each slot rechecks alive defenders.

**Non-Goals:**
- No refactoring beyond the minimal fix for each bug.
- No changes to non-standard category handling, sortie mechanics beyond cell init, or airstrike damage formulas.

## Decisions

### 1. Manifest generator (`crates/emukc_bootstrap/src/make_list/manifest/generate.rs`)

**Current** (line 174): `gen_base = !matches!(damaged, Some(true))` — when `damagedSource == "true"`, both `gen_base` and `gen_variants` are false, producing zero paths for standard categories.

**Fix**: When `damaged == Some(true)`, generate the base path but skip variants (same pattern as `Some(false)` — just the undamaged base art). Change to:
```rust
let gen_base = true; // always generate base for standard categories
let gen_variants = damaged.is_none() && !variants.is_empty();
```

**Alternative considered**: Generate damaged variant when `damaged == Some(true)`. Rejected — the `"true"` entries in the manifest indicate the source has a damaged asset, but the variant path generation already handles `_dmg` suffixes separately for full/full_dmg. Standard categories don't have explicit damaged variants in the same way.

### 2. Sortie cell init (`crates/emukc_gameplay/src/game/sortie.rs:1019`)

**Current**: `passed: cell.cell_no != 0` — marks every non-zero cell as passed.

**Fix**: Revert to `passed: false` as in 7e0d397's original fix. Also fix the test assertion at `tests/sortie_battle.rs:412` that was updated to bless the wrong behavior.

### 3. Airstrike target refresh (`crates/emukc_gameplay/src/game/battle/core.rs`)

**Current**: `alive_targets` collected once at line 1229, reused for all slots in both dive-bombing (line 1257) and torpedo-bombing (line 1290) phases.

**Fix**: Move `alive_targets` collection into each slot iteration — recompute before selecting a target for each individual slot. This matches the intended "random alive target per slot" behavior stated in the code comment.

**Alternative considered**: Remove dead entries from the list after each hit. Rejected — recomputing is simpler and handles sunk ships correctly without mutation complexity.

## Risks / Trade-offs

- **Manifest regeneration required**: After fixing the generator, `resource_manifest.json` must be regenerated. Low risk — deterministic generation.
- **Sortie test regression**: The test at `sortie_battle.rs:412` was changed to match the buggy behavior. Must revert the assertion. Risk: other tests may depend on the wrong state. Mitigation: run full test suite.
- **Airstrike performance**: Recomputing alive targets per-slot adds negligible overhead (6 ships max per side).
