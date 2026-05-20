---
title: refactor: Anchor wikiwiki overlay in kcdata topology
type: refactor
status: backlog
date: 2026-05-09
---

# refactor: Anchor wikiwiki overlay in kcdata topology

## Summary

Redesign the wikiwiki→kcdata merge pipeline. Currently wikiwiki parsing assigns independent BFS-based cell numbers, then a fragile label-matching bridge remaps them to kcdata route IDs. The redesign captures pre-conversion label-keyed data from wikiwiki (routing rules, enemy fleets, ship drops) and resolves labels directly against kcdata's authoritative Label→CellNo index — eliminating BFS numbering, cell_no remapping, and the identity fallback from the kcdata merge path.

---

## Problem Frame

Wikiwiki HTML parsing produces its own cell numbering (BFS traversal order). Kcdata uses route IDs as cell numbers. These two numbering schemes have no inherent relationship. The current merge relies on matching `node_label` strings between the two systems via `semantic_cell_no_map_from_labels` (`merge.rs:187-199`). This bridge fails when:

1. Kcdata uses numeric node keys (`to: 5` → label "5") that don't match wikiwiki letter labels ("E")
2. Multiple kcdata routes target the same named node → duplicate labels excluded from mapping
3. `remap_cell_no` falls back to identity (`merge.rs:326-328`) → wikiwiki cell_no passes through unchanged, may accidentally match wrong kcdata cell

The result: routing rules silently dropped, enemy fleets missing, gameplay errors when no rules match topology.

---

## Requirements

- R1. Wikiwiki routing rules, enemy fleets, and ship drops are attached to kcdata cells via direct label→cell_no lookup against kcdata's authoritative index
- R2. No BFS-based cell numbering in the kcdata merge path — wikiwiki labels resolve directly to kcdata cell numbers
- R3. Unmatched labels are logged and data dropped (no identity fallback, no silent mis-targeting)
- R4. Fallback path (no kcdata available) continues to work — wikiwiki produces full MapCatalog with BFS numbering
- R5. Existing gameplay-layer fallback plan (`2026-05-09-001`) remains valid as defense-in-depth

---

## Scope Boundaries

- Wikiwiki parser's HTML parsing, route condition parsing, enemy/drop extraction — unchanged
- `build_nodes()` and BFS numbering — kept for fallback path only, removed from kcdata merge path
- Gameplay layer (`map_route.rs`) — not modified (existing fallback plan covers it)
- kcdata YAML parsing — unchanged

### Deferred to Follow-Up Work

- Numeric kcdata node key handling (Option 2/3 from design discussion) — accept the gap per Option 1
- Removing `build_nodes()` entirely (requires deprecating the no-kcdata fallback path)

---

## Context & Research

### Relevant Code and Patterns

- `crates/emukc_bootstrap/src/parser/wikiwiki_map/types.rs:117-124` — `RouteRuleDraft` already holds `from_label`/`to_label` (label-keyed, pre-conversion)
- `crates/emukc_bootstrap/src/parser/wikiwiki_map/types.rs:110-114` — `EnemyNodeRows` already keyed by label in `BTreeMap<String, EnemyNodeRows>`
- `crates/emukc_bootstrap/src/parser/wikiwiki_map/types.rs:127-129` — `ShipDropDraft` already has `node_label`
- `crates/emukc_bootstrap/src/parser/wikiwiki_map/mod.rs:347-398` — where label→cell_no conversion happens (the interception point)
- `crates/emukc_model/src/codex/map/merge.rs:187-199` — fragile label bridge to be replaced
- `crates/emukc_model/src/codex/map/merge.rs:259-265` — `build_cell_no_map` to be replaced
- `crates/emukc_bootstrap/src/map_pipeline/assemble.rs:57-111` — current merge orchestration to be replaced

### Institutional Learnings

- Previous plan `2026-05-05-003-refactor-map-topology-routing-separation-plan.md` established the topology/routing separation that this refactor builds on
- Plan `2026-05-09-001-fix-route-topology-mismatch-fallback-plan.md` addresses the gameplay symptom (blocked gameplay) — this plan addresses the root cause (data quality loss in merge)

---

## Key Technical Decisions

- **KD1: Capture pre-conversion data, don't re-parse.** `RouteRuleDraft`, `EnemyNodeRows`, and `ShipDropDraft` already exist as label-keyed intermediate types. Capture them before `build_nodes()` converts to cell_nos. No new parsing needed.
- **KD2: kcdata Label→CellNo index is the sole mapping authority.** Built from kcdata's `node_label` values. No bidirectional mapping, no fallback. Unmatched wikiwiki labels are logged and dropped.
- **KD3: Fallback path unchanged.** When kcdata is absent, `build_nodes()` + `into_map_catalog()` still produces a full MapCatalog. This path is not modified.
- **KD4: Parallel overlay storage.** `WikiwikiMapDefinition` gains a parallel `overlays` map alongside `variants`. Overlay data is label-keyed; variant data is cell_no-keyed. Both are produced from the same parsed HTML.

---

## Open Questions

### Resolved During Planning

- How to handle numeric kcdata node keys? → Accept the gap. Log warnings. Topology is always correct from kcdata. (KD2)
- Should `build_nodes()` be removed? → No, kept for fallback path. Removed from kcdata merge path only. (KD3)

### Deferred to Implementation

- Exact field layout of `WikiwikiLabelOverlay` — struct fields may adjust during implementation
- Whether `label_to_cell_no()` should be a method on `MapVariantDefinition` or a standalone function

---

## High-Level Technical Design

> *This illustrates the intended approach and is directional guidance for review, not implementation specification. The implementing agent should treat it as context, not code to reproduce.*

### Current Flow

```
HTML → RouteRuleDraft (labels)
     → EnemyNodeRows (labels)
     → ShipDropDraft (labels)
            ↓ build_nodes() assigns BFS cell_nos
     → node_to_cell map
            ↓ label→cell_no conversion
     → WikiwikiMapVariantDefinition (cell_no-keyed)
            ↓ into_map_catalog()
     → MapCatalog (cell_no-keyed)
            ↓ merge_routing_overlay_from_wikiwiki()
            ↓ build_cell_no_map() / semantic_cell_no_map_from_labels()
     → kcdata MapCatalog with wikiwiki overlay
```

### New Flow (kcdata merge path)

```
HTML → RouteRuleDraft (labels)      ──→ WikiwikiLabelOverlay (label-keyed)
     → EnemyNodeRows (labels)       ──→   routing_rules: Vec<RouteRuleDraft>
     → ShipDropDraft (labels)       ──→   enemy_nodes: BTreeMap<String, EnemyNodeRows>
                                         ship_drops: Vec<ShipDropDraft>
                                              ↓
kcdata MapVariantDefinition              ↓
  → label_to_cell_no() index            ↓
  → merge_label_overlay()  ←─────────────┘
    resolve label → kcdata cell_no
    unmatched labels: log + drop
              ↓
kcdata variant with wikiwiki overlay attached
```

The fallback path (no kcdata) still uses `build_nodes()` → `into_map_catalog()` unchanged.

---

## Implementation Units

### U1. Define `WikiwikiLabelOverlay` types

**Goal:** Create the label-keyed overlay type that captures pre-conversion wikiwiki data.

**Requirements:** R1, R2

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_bootstrap/src/parser/wikiwiki_map/types.rs`
- Modify: `crates/emukc_bootstrap/src/parser/wikiwiki_map/mod.rs` (re-export)

**Approach:**

Add to `types.rs`:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WikiwikiMapOverlayCatalog {
    pub maps: BTreeMap<i64, WikiwikiMapOverlayDefinition>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WikiwikiMapOverlayDefinition {
    pub map_id: i64,
    pub variants: BTreeMap<String, WikiwikiLabelOverlay>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WikiwikiLabelOverlay {
    pub variant_key: String,
    pub routing_rules: Vec<RouteRuleDraft>,        // label-based, not cell_no
    pub enemy_nodes: BTreeMap<String, EnemyNodeRows>, // already label-keyed
    pub ship_drops: Vec<ShipDropDraft>,             // has node_label field
    pub required_defeat_count: Option<i64>,
    pub parse_warnings: Vec<String>,
}
```

`RouteRuleDraft`, `EnemyNodeRows`, and `ShipDropDraft` need `Serialize`/`Deserialize` derives added (currently `Debug, Clone` only). Make them public if needed for the overlay type.

**Patterns to follow:**
- Existing `WikiwikiMapCatalog` / `WikiwikiMapDefinition` structure in same file

**Test scenarios:**
- Serialization round-trip: construct overlay, serialize to JSON, deserialize, assert equality
- Empty overlay serializes correctly

**Verification:**
- `cargo test -p emukc_bootstrap` passes
- New types compile and derive correctly

---

### U2. Extract overlay in `parse_map_page()`

**Goal:** Capture pre-conversion label-keyed data alongside the existing cell_no-keyed variant.

**Requirements:** R1, R4

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_bootstrap/src/parser/wikiwiki_map/mod.rs`
- Modify: `crates/emukc_bootstrap/src/parser/wikiwiki_map/types.rs`

**Approach:**

In `parse_map_page()` (currently `mod.rs:306`), after parsing `route_rules`, `enemy_nodes`, and `drop_drafts` (line ~346), and BEFORE `build_nodes()` call (line 347):

1. Package the raw label-keyed data into a `WikiwikiLabelOverlay`
2. Store it in `WikiwikiMapDefinition.overlays` (new field: `BTreeMap<String, WikiwikiLabelOverlay>`)

The existing `build_nodes()` → cell_no conversion continues to produce `WikiwikiMapVariantDefinition` for the fallback path. No existing code is removed — overlay is additive.

Also update `parse_wikiwiki_map()` and `parse_wikiwiki_map_debug()` to expose the overlay catalog alongside the existing catalog. Options:
- Return a tuple `(WikiwikiMapCatalog, WikiwikiMapOverlayCatalog)` from `parse_map_page()`
- Or add `overlays` field to `WikiwikiMapDefinition`

Choose whichever causes less API churn during implementation.

**Patterns to follow:**
- Existing `parse_map_page()` structure — extract data from same source, just earlier in the pipeline

**Test scenarios:**
- Parse a wikiwiki HTML page → overlay contains same routing rule count as existing variant
- Overlay routing rules have label-based references (from_label, to_label), not cell_nos
- Overlay enemy_nodes keyed by label, not cell_no
- Existing `into_map_catalog()` still works (fallback path not broken)

**Verification:**
- `cargo test -p emukc_bootstrap` passes
- Existing tests unaffected

---

### U3. Add `label_to_cell_no()` on `MapVariantDefinition`

**Goal:** Provide the authoritative Label→CellNo index from kcdata's topology.

**Requirements:** R1

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_model/src/codex/map/types.rs`
- Modify: `crates/emukc_model/src/codex/map/merge.rs`
- Test: `crates/emukc_model/src/codex/map/merge.rs` (existing test module)

**Approach:**

Add a method to `MapVariantDefinition` that produces `BTreeMap<String, i64>` from `node_label → cell_no`. This replaces the role of `unique_labeled_cells()` + `build_cell_no_map()` for the kcdata merge path.

The implementation is essentially `unique_labeled_cells()` (currently `merge.rs:267-287`) but exposed as a method. Consider moving or reusing the existing function.

**Patterns to follow:**
- `unique_labeled_cells()` in `merge.rs:267-287` — same deduplication logic

**Test scenarios:**
- Variant with labeled cells returns correct mapping
- Variant with duplicate labels (same label on different cell_nos) excludes the label
- Variant with no labeled cells returns empty map
- "Start" label correctly mapped to cell_no 0

**Verification:**
- `cargo test -p emukc_model` passes

---

### U4. Implement `merge_label_overlay()` function

**Goal:** New merge function that resolves wikiwiki label-keyed overlay to kcdata cell_nos via the authoritative index.

**Requirements:** R1, R2, R3

**Dependencies:** U1, U3

**Files:**
- Create: `crates/emukc_bootstrap/src/map_pipeline/label_overlay.rs` (new file for overlay merge logic)
- Test: `crates/emukc_bootstrap/src/map_pipeline/label_overlay.rs` (same file, test module)

**Approach:**

New function `merge_label_overlay()`:

1. Takes `kcdata_variant: &mut MapVariantDefinition`, `overlay: &WikiwikiLabelOverlay`, `label_index: &BTreeMap<String, i64>`
2. For each `RouteRuleDraft` in overlay:
   - Look up `from_label` and `to_label` in label_index
   - If both found: convert to `RouteRule` with resolved cell_nos, validate `to_cell_no` exists in variant cells
   - If either missing: log warning with both labels, increment dropped counter
3. For each `(label, EnemyNodeRows)` in overlay:
   - Look up label in label_index
   - If found: create `EnemyFleetDefinition` with resolved cell_no
   - If missing: log warning, skip
4. For each `ShipDropDraft` in overlay:
   - Look up `node_label` in label_index
   - If found: attach drops at resolved cell_no
   - If missing: log warning, skip
5. Returns count of dropped items

No identity fallback. No `remap_cell_no`. Unmatched labels are explicitly dropped and logged.

**Technical design:**

```
fn merge_label_overlay(
    kcdata_variant: &mut MapVariantDefinition,
    overlay: &WikiwikiLabelOverlay,
    label_index: &BTreeMap<String, i64>,
) -> usize
```

The function is standalone (not on a trait) because it bridges two crate-specific types.

**Test scenarios:**
- Happy path: all labels match → rules, fleets, drops attached at correct cell_nos
- Partial match: some labels match, some don't → matched data attached, unmatched logged
- No match: zero labels match → all data dropped, logged, returns non-zero count
- Fan-out: wikiwiki overlay with variant_key="" merged into multiple kcdata named variants → each validated independently
- Predicate rewriting: `VisitedNodeLabel` predicates in overlay rules → converted to `VisitedNode` with resolved cell_nos
- Enemy fleet at matched label → `EnemyFleetDefinition.cell_no` matches kcdata cell_no

**Verification:**
- `cargo test -p emukc_bootstrap` passes
- New test module covers all scenarios above

---

### U5. Wire into assembly pipeline

**Goal:** Replace the old fragile merge path with the new kcdata-anchored overlay merge.

**Requirements:** R1, R3, R4

**Dependencies:** U2, U4

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/assemble.rs`
- Modify: `crates/emukc_bootstrap/src/map_pipeline/sources.rs`
- Modify: `crates/emukc_bootstrap/src/map_pipeline/mod.rs`

**Approach:**

1. Thread `WikiwikiMapOverlayCatalog` through `ResolvedMapSources` (add new field)
2. In `assemble_final_map_catalog()`:
   - When kcdata present AND overlay present: call `merge_label_overlay()` (new path)
   - When kcdata absent AND wikiwiki catalog present: use `wikiwiki_catalog.unwrap_or_default()` (existing fallback, unchanged)
   - When only kcdata present: kcdata as-is (no overlay, unchanged)
3. Remove old `merge_routing_overlay_from_wikiwiki()` and `apply_overlay_checked()` from `assemble.rs`
4. Update `MapCatalogBuildReport` to reflect new overlay statistics

The `build_cell_no_map` / `semantic_cell_no_map_from_labels` calls in `assemble.rs` are no longer needed. The `merge_routing_overlay()` function from `merge.rs` may still be used by other code paths (public overlays, stat merge) — verify before removing.

**Test scenarios:**
- kcdata + wikiwiki overlay → rules attached via label index, no BFS numbering involved
- kcdata only → no overlay applied, kcdata topology untouched
- wikiwiki only (no kcdata) → fallback path produces full MapCatalog
- Mixed: some maps have kcdata, some don't → each map uses appropriate path

**Verification:**
- `cargo test -p emukc_bootstrap` passes
- `assemble_final_map_catalog` tests updated to use new path

---

### U6. Clean up and finalize tests

**Goal:** Remove dead code, ensure comprehensive test coverage.

**Requirements:** R1, R2, R3

**Dependencies:** U5

**Files:**
- Modify: `crates/emukc_model/src/codex/map/merge.rs` — remove unused functions if no other callers
- Modify: `crates/emukc_bootstrap/src/map_pipeline/assemble.rs` — remove old merge functions
- Test: all affected test files updated

**Approach:**

1. Audit callers of `build_cell_no_map`, `semantic_cell_no_map_from_labels`, `semantic_cell_no_map`, `remap_cell_no`, `apply_overlay_checked`, `merge_routing_overlay_from_wikiwiki`
2. If any function has zero remaining callers after U5, remove it
3. `merge_routing_overlay()` from `merge.rs` may still be used by `merge_variant_definition()` or public overlay merge — keep if used
4. Add integration test: load real kcdata files + real wikiwiki overlay → verify merge produces correct data
5. Verify `cargo test --workspace` passes

**Patterns to follow:**
- Existing test patterns in `merge.rs` and `assemble.rs`

**Test scenarios:**
- Full pipeline integration: kcdata YAML + wikiwiki HTML → assembled MapCatalog → validate routing rules, enemy fleets, ship drops are present
- Verify `fanout_rules_dropped` report field still works correctly
- Backward compatibility: existing `MapCatalog` JSON snapshots unchanged for maps where kcdata has no wikiwiki overlay

**Verification:**
- `cargo test --workspace` passes
- `cargo clippy --workspace` clean

---

## System-Wide Impact

- **Interaction graph:** Only the bootstrap assembly pipeline changes. Gameplay layer, API handlers, and kcdata parsing are unaffected.
- **Error propagation:** Unmatched labels produce structured warnings (tracing::warn) instead of silent identity-fallback mis-targeting.
- **State lifecycle risks:** None — this is a build-time pipeline, not runtime state.
- **API surface parity:** `parse_wikiwiki_map()` and `parse_wikiwiki_map_debug()` gain overlay output. Existing return types preserved for backward compatibility.
- **Unchanged invariants:** kcdata remains sole source of map topology. Wikiwiki overlay provides routing rules, enemy fleets, and ship drops only.

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Numeric kcdata node keys won't get wikiwiki overlay | Accept per Option 1. Log warnings. Topology is always correct. |
| Fallback path (no kcdata) regresses | Existing `build_nodes()` + `into_map_catalog()` path is not modified. Dedicated test. |
| `merge_routing_overlay()` still needed elsewhere | Audit callers before removing. Keep if used. |
| Overlay extraction doubles memory for parsed wikiwiki data | Overlay references the same data types (RouteRuleDraft etc). Overhead is minimal — pointers + labels, not full topology. |

---

## Sources & References

- Related plan: `docs/plans/2026-05-09-001-fix-route-topology-mismatch-fallback-plan.md`
- Related plan: `docs/plans/2026-05-05-003-refactor-map-topology-routing-separation-plan.md`
- Root cause analysis: `/ce-debug` session in this conversation
- Key code: `crates/emukc_model/src/codex/map/merge.rs` (label bridge)
- Key code: `crates/emukc_bootstrap/src/map_pipeline/assemble.rs` (merge orchestration)
- Key code: `crates/emukc_bootstrap/src/parser/wikiwiki_map/mod.rs` (wikiwiki parsing + conversion)
