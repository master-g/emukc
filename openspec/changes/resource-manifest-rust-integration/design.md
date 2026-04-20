## Context

Phase 1 (`resource-manifest-extractor`) produces `crates/emukc_bootstrap/assets/resource_manifest.json` — 416 entries extracted from all 790 decoded main.js modules. Each entry describes a resource loading pattern (ship, slotitem, texture-provider, explicit-path) with parameter source expressions rather than concrete IDs.

The Rust-side `cache make-list` (`crates/emukc_bootstrap/src/make_list/`) currently generates resource paths via hardcoded ID ranges and validates existence with HTTP HEAD requests. Static hole lists (`EVENT_SHIP_HOLES`, `ENEMY_SHIP_HOLES`, `SPECIAL_SHIPS`, etc.) are brittle and game-version-specific. Greedy mode fires thousands of HEAD checks for non-existent resources.

## Goals / Non-Goals

**Goals:**
- Deserialize `resource_manifest.json` into typed Rust structs in `emukc_bootstrap`
- Resolve parameter source expressions to concrete ship/slotitem IDs via the Codex (`ApiManifest`)
- Generate resource paths using `SuffixUtils` from resolved entries
- Provide a `Manifest` strategy that produces precise lists without HTTP HEAD checks for covered types
- Reduce or eliminate hardcoded hole lists for types fully covered by the manifest

**Non-Goals:**
- BGM, voice, SE, furniture, map resource discovery (future phases)
- Removing Greedy strategy (still needed for uncovered types)
- Runtime resource prefetching
- Modifying the TypeScript extractor

## Decisions

### 1. Manifest types location: `emukc_bootstrap` not `emukc_model`

**Decision**: New `manifest` module in `crates/emukc_bootstrap/src/make_list/manifest/` with its own `types.rs`.

**Rationale**: The manifest types are only consumed by the make_list pipeline. No other crate needs them. Keeping them in emukc_bootstrap avoids touching emukc_model's public API.

**Alternative considered**: `emukc_model::resource_manifest`. Rejected — these types are bootstrap-internal, not shared game data.

### 2. Expression resolution: simple pattern matching, not AST evaluation

**Decision**: Map known `shipMstIdSource`/`slotMstIdSources` expressions to resolution strategies via a lookup table. Known patterns:
- `"self.shipModel.mstID"`, `"this._mst_id"`, `"_0x..."` → iterate all ship IDs in Codex
- `"vo.ship.api_id"`, `"vo.ships[i].api_id"` → iterate all ships in manifest
- `"false"` / `"true"` for damaged → literal boolean
- `"[expr1, expr2]"` for multiple sources → union of resolved IDs

Unknown expressions produce a warning and skip the entry (fail-open).

**Rationale**: Full expression evaluation requires a JS runtime. The manifest contains ~20 unique expression patterns — a lookup table covers them all. New patterns from game updates are rare and produce warnings, not crashes.

**Alternative considered**: Embedding a JS expression evaluator (e.g., rquickjs). Rejected — massive dependency for ~20 patterns, overkill.

### 3. Strategy integration: new `Manifest` variant, not replacing existing strategies

**Decision**: Add `CacheListMakeStrategy::Manifest` as a new variant. Default and Greedy strategies remain unchanged.

**Rationale**: Manifest strategy produces different output than Default (it derives from decoded patterns, not hardcoded knowledge). Users opt-in explicitly. Backward compatibility preserved.

**Alternative considered**: Merging manifest logic into Default. Rejected — would change existing behavior and make rollback harder.

### 4. Ship path generation: reuse existing `SuffixUtils` patterns

**Decision**: The manifest-driven generator uses the same path templates and `SuffixUtils::create()` calls as `source/kcs2/resources/ship.rs`. The manifest provides (kind, targetType, resolved IDs) → the generator maps to existing path patterns.

**Rationale**: `SuffixUtils` already handles the hash suffix computation. Reusing existing patterns ensures path compatibility with the cache system.

### 5. Explicit paths: add directly without resolution

**Decision**: Explicit-path entries from the manifest are added to the cache list as-is (they're already complete resource paths).

**Rationale**: No ID resolution needed — the paths are final. Just strip the `kcs2/` prefix if needed for the cache list format.

### 6. Texture provider entries: defer to future phase

**Decision**: Phase 2 reads texture-provider entries but does NOT generate cache paths for them. Texture resource resolution requires additional mapping logic (provider name → path template) not yet designed.

**Rationale**: Texture provider patterns are complex (multiple IDs per provider, unknown path templates). Ship and slotitem are well-understood. Ship/slotitem alone cover the bulk of the hardcoded lists.

## Risks / Trade-offs

**[Expression pattern drift]** → Game updates may introduce new `shipMstIdSource` expressions. Mitigation: fail-open with warning logging. Coverage check in tests.

**[Incomplete coverage]** → Manifest may not cover all resource types currently handled by Default/Greedy (e.g., sp_remodel, ship_type, reward_card). Mitigation: Manifest strategy handles what it can; uncovered types fall back to Default logic or are noted as gaps.

**[Manifest version mismatch]** → `resource_manifest.json` may be from a different game version than the Codex. Mitigation: manifest version field checked at load; warn if stale but don't block.

**[No automatic manifest regeneration]** → Manifest must be manually regenerated when main.js updates. Mitigation: existing `--sync-resource-manifest` flag; CI could automate this in future.
