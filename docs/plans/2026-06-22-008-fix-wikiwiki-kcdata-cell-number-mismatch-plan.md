---
title: "fix: Bridge wikiwiki routing rules to kcdata topology via auto-derived label overlay"
status: active
type: fix
created: 2026-06-22
sequence: 008
---

# fix: Bridge wikiwiki routing rules to kcdata topology via auto-derived label overlay

## Summary

~678 `RuleTargetNotInNextCells` warnings fire at codex load time across 33 of 37 manifest maps (the other 4 are simple maps without convergent routes). Root cause: wikiwiki (agent skill) assigns cell numbers via BFS traversal while kcdata uses route IDs as cell numbers. The two numbering systems are incompatible. The existing label-based remapping (`build_cell_no_map` → `unique_labeled_cells`) discards duplicate labels — common on maps with convergent routes — causing routing rules to fall through to identity remapping and point at wrong cells.

The fix auto-derives a label-keyed overlay from the wikiwiki catalog's own node labels at assembly time, then routes through the already-correct `merge_label_overlay()` path which uses `multi_label_index()` (handles duplicates) and validates against real topology edges.

## Problem Frame

### Two incompatible cell-number spaces

| Property | kcdata | wikiwiki (agent skill) |
|----------|--------|----------------------|
| cell_no source | route ID (`routes` dict key) | BFS traversal order |
| Trust level | human-verified, very high | agent-parsed, may have gaps |
| Produces `next_cells` | ✓ topology authority | ✗ not produced |
| Produces `routing_rules` | ✗ | ✓ with conditions/predicates |

### The remapping failure

`build_cell_no_map()` calls `unique_labeled_cells()` which drops any label appearing on multiple cells. On map 52 (5-2): nodes B, F, K, L, O each appear on 2+ cells (convergent routes). All these labels are dropped. The remap map shrinks to identity, and wikiwiki routing rules point at wrong cells.

Concrete example (map 52):

- kcdata cell 7 = node **G**, next_cells = [10(J), 19(L)]
- wikiwiki cell 7 = node **L** — completely different node
- Routing rule "7→6" means wikiwiki "L→K" but gets applied to kcdata cell 7 (G), which has no edge to cell 6

### The correct path already exists but is never reached

`merge_label_overlay()` in `crates/emukc_bootstrap/src/map_pipeline/label_overlay.rs` handles everything correctly:

- Uses `multi_label_index()` to find ALL cells matching a label
- Validates that each routing rule's `from→to` corresponds to a real edge in `next_cells`
- Only creates rules for edges that actually exist in kcdata topology

But assembly never calls it because `wikiwiki_overlay` is always `None` in both the CLI normalize path and the runtime repo-asset path.

## Scope Boundaries

### In scope

- Auto-derive label-keyed overlay from wikiwiki catalog data at assembly time
- Route through `merge_label_overlay()` instead of broken legacy remapping
- Rebuild wikiwiki asset file with corrected routing rules
- Validate warning reduction

### Out of scope (non-goals)

- Agent skill changes — cell-number output format is self-consistent within wikiwiki space
- kcdata data changes — trusted, human-verified topology
- `next_cells` population for non-start kcdata nodes — separate topology completeness issue
- Event/seasonal map filtering — already addressed in prior codex-load filtering work

### Deferred to follow-up work

- Removing the legacy `merge_routing_overlay_from_wikiwiki_legacy` path — kept as safety fallback in this plan; can be removed in a follow-up once the overlay path is proven stable across all maps
- Investigating residual warnings after fix — may indicate genuine kcdata data gaps

---

## Key Technical Decisions

### KTD-1: Auto-derive overlay at assembly time, not at catalog build time

The overlay is derived from the wikiwiki catalog's `cells[].node_label` and `routing_rules` at the moment of assembly. This covers both the CLI `normalize` path and the runtime `load_repo_source_set` path with a single code change. No new asset files or serialization formats needed.

### KTD-2: Explicit overlay takes precedence over auto-derived

If `wikiwiki_overlay` is already `Some` (from `into_map_catalog_with_overlay()` producing native overlays), it takes precedence. Auto-derivation only fills the `None` gap. This ensures forward compatibility if the agent skill ever produces native label-keyed overlays.

### KTD-3: Legacy path retained as fallback

`merge_routing_overlay_from_wikiwiki_legacy` is kept but becomes effectively dead code. After auto-derivation, `wikiwiki_overlay` is always `Some` when `wikiwiki_catalog` is `Some` — even if the derived catalog is empty. The legacy arm is only entered when no wikiwiki catalog exists at all, at which point its inner guard also fails. Removing it is deferred to follow-up.

---

## Implementation Units

### U1. Auto-derive label overlay from MapVariantDefinition

**Goal:** Create a function that converts a wikiwiki `MapVariantDefinition` (with cell-number-keyed routing rules and node labels) into a `WikiwikiLabelOverlay` (with label-keyed routing rule drafts).

**Dependencies:** None

**Files:**

- `crates/emukc_bootstrap/src/map_pipeline/label_overlay.rs` (new function + tests)
- `crates/emukc_bootstrap/src/parser/wikiwiki_map/types.rs` (existing types: `WikiwikiLabelOverlay`, `RouteRuleDraft`)

**Approach:**

Build a cell_no → label lookup from the variant's cells (many-to-one: multiple cells can share a label). Convert three data categories:

**Routing rules:** For each routing rule:

1. Look up `from_label = label_of(from_cell_no)` and `to_label = label_of(to_cell_no)`
2. If either endpoint has no label, skip the rule (can't bridge)
3. Convert to `RouteRuleDraft { from_label, to_label, predicate, probability_pct, raw_text, random_placeholder: false }`

**Enemy fleets:** For each `(cell_no, fleet)` in `enemy_fleets`:

1. Look up `label = label_of(cell_no)`
2. If no label, skip
3. Insert into `enemy_nodes` map keyed by label as `EnemyNodeRows { is_boss: false, compositions: fleet.compositions }`

**Ship drops:** For each `(cell_no, drops)` in `ship_drops`:

1. Look up `label = label_of(cell_no)`
2. If no label, skip
3. Push `ShipDropDraft { node_label: label, drop: <first drop> }` for each drop entry

The resulting overlay is consumed by the existing `merge_label_overlay()`, which handles duplicate labels correctly via `multi_label_index()` for all three categories.

Cell 0 (Start) is always labeled "Start" in wikiwiki catalogs (`into_map_catalog_with_overlay` line ~130: `ENTRY_NODE_LABEL`), so Start-origin rules convert cleanly.

**Patterns to follow:** `merge_label_overlay()` in the same file — it already demonstrates the label-index pattern and `RouteRuleDraft` construction. `merge_legacy_enemy_fleets_and_ship_drops()` in `assemble.rs` shows the equivalent cell-number-keyed conversion for enemy fleets and ship drops.

**Test scenarios:**

1. **Happy path:** Variant with unique labels A, B, C and rules A→B, B→C → overlay with two drafts, `from_label="A" to_label="B"` and `from_label="B" to_label="C"`
2. **Duplicate labels:** Variant with cells 5 and 11 both labeled "E", rule 5→6 → overlay draft `from_label="E" to_label=<label of cell 6>` — the multi-label fan-out is handled downstream by `merge_label_overlay`
3. **Cell without label:** Cell with `node_label = None`, routing rule referencing it → rule skipped, not included in overlay
4. **Empty routing rules:** Variant with no routing rules → empty overlay (empty `routing_rules` vec, empty `enemy_nodes`, empty `ship_drops`)
5. **Start cell origin:** Rule from cell 0 (labeled "Start") → correctly converted with `from_label="Start"`
6. **Predicate preservation:** Rule with `ShipTypeCount` predicate → predicate passed through unchanged in the draft
7. **Probability/raw_text preservation:** Rule with `probability_pct` and `raw_text` → both carried into the draft
8. **Enemy fleet conversion:** Variant with enemy fleet at cell 5 (labeled "E") → overlay `enemy_nodes` contains key "E" with the fleet's compositions
9. **Ship drop conversion:** Variant with ship drops at cell 10 (labeled "J") → overlay `ship_drops` contains entry with `node_label: "J"`
10. **Enemy fleet at unlabeled cell:** Fleet at cell with `node_label = None` → fleet skipped

**Verification:** New unit tests pass. `cargo test -p emukc_bootstrap label_overlay` green.

---

### U2. Wire auto-derived overlay into assembly pipeline

**Goal:** When `wikiwiki_overlay` is `None` but `wikiwiki_catalog` is available, auto-derive the overlay from the catalog before entering the assembly match block.

**Dependencies:** U1

**Files:**

- `crates/emukc_bootstrap/src/map_pipeline/assemble.rs` (modify `assemble_final_map_catalog`)
- `crates/emukc_bootstrap/src/map_pipeline/assemble.rs` (tests)

**Approach:**

In `assemble_final_map_catalog`, before the match on `(sources.kcdata_catalog, sources.wikiwiki_overlay)`:

1. If `sources.wikiwiki_overlay` is `None` and `sources.wikiwiki_catalog` is `Some`:
   - Iterate over the wikiwiki catalog's maps and variants
   - For each variant, call `auto_derive_label_overlay()` from U1
   - Build a `WikiwikiMapOverlayCatalog`
2. Use the derived overlay in the match — the existing `(Some(kcdata), Some(overlay))` arm handles the rest via `merge_label_overlay_catalog`

The `(Some(kcdata), None)` legacy arm becomes reachable only when the wikiwiki catalog is also `None` (no wikiwiki data at all), or when auto-derivation produces an empty overlay for all maps (edge case — effectively no-op since merge_label_overlay with empty overlay adds nothing, and legacy path also adds nothing from an absent catalog).

**Patterns to follow:** `merge_label_overlay_catalog()` in the same file — it already iterates maps/variants and fans out empty-key variants to named variants.

**Test scenarios:**

1. **Auto-derivation replaces legacy path:** Source set with kcdata + wikiwiki catalog (no explicit overlay) → assembly uses label overlay merge, not legacy remapping → routing rules correctly mapped
2. **Explicit overlay precedence:** Source set with explicit overlay provided → auto-derivation skipped, explicit overlay used as-is
3. **No wikiwiki catalog:** Source set with kcdata only → no auto-derivation, no overlay merge, kcdata used directly (routing_rules empty as before)
4. **Warning reduction integration:** Map 52 source data (kcdata topology + wikiwiki routing rules with mismatched cell numbers) → after assembly, `validate()` produces 0 `RuleTargetNotInNextCells` warnings for rules that have valid label-matched edges; rules with no matching edge are dropped (not warned)

**Verification:** Assembly tests pass. `cargo test -p emukc_bootstrap assemble` green. Full map pipeline test suite green.

---

### U3. Rebuild wikiwiki asset and validate end-to-end

**Goal:** Rebuild the tracked `wikiwiki_map_catalog.json` asset with the corrected overlay merge, then verify the warning count at codex load time.

**Dependencies:** U2

**Files:**

- `crates/emukc_bootstrap/assets/wikiwiki_map_catalog.json` (regenerated output)
- `.data/codex/map_catalog.json` (regenerated via bootstrap)

**Approach:**

1. Run `cargo run -- normalize --from-agent-json .data/temp/wikiwiki_map/agent-output-all-v2.json` to rebuild the wikiwiki asset
2. Run bootstrap to regenerate `.data/codex/map_catalog.json`
3. Start server and count `RuleTargetNotInNextCells` warnings
4. Spot-check map 52: verify routing rules now use kcdata cell numbers and `to_cell_no` values are present in `next_cells`

**Test scenarios:**

Test expectation: none — this is a data regeneration step, not a code change. Validation is done via server startup warning count.

**Verification:**

- Warning count for `RuleTargetNotInNextCells` drops from ~678 to near-zero (residual warnings would indicate genuine kcdata topology gaps, not remapping failures)
- All 37 manifest maps still present in the catalog
- Map 52 routing rules reference cells within `next_cells` of their source cell
- `cargo test` (full workspace) still green
- `cargo clippy --workspace --tests -- -D warnings` clean

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Auto-derived overlay drops rules that the legacy path incorrectly kept | Medium | Low (dropped rules were already pointing at wrong cells) | Compare rule count before/after per map; investigate any map that loses ALL rules |
| Maps with only wikiwiki data (no kcdata) lose routing rules | Low | Medium | Auto-derivation only applies when kcdata is present; the `(None, _)` path returns wikiwiki catalog directly |
| Residual warnings indicate kcdata `next_cells` gaps, not remapping | Medium | Low | These are genuine data quality issues; document in deferred work |
| Asset rebuild changes serialized JSON format subtly | Low | Low | `MapCatalog` serialization is stable; diff the asset before/after to verify only routing_rules changed |

---

## System-Wide Impact

- **Server startup:** Log noise reduced from ~678 WARN lines to near-zero. Easier to spot genuine issues.
- **Runtime routing:** Map routing engine uses `routing_rules` from codex catalog. Corrected rules improve routing accuracy for maps where the mismatch caused wrong conditional routing.
- **No API contract changes:** The codex catalog format is unchanged; only the content (routing_rules values) changes.
- **No database migration:** Catalog is loaded from disk at startup.
