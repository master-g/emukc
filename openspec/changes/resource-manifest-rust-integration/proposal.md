## Why

The Rust-side `cache make-list` command uses hardcoded ID lists (`EVENT_SHIP_HOLES`, `ENEMY_SHIP_HOLES`, `SPECIAL_SHIPS`, `SP_REMODEL_SHIPS`, etc.) and HTTP HEAD checks to discover game resources. These lists require manual maintenance and become stale between game updates. Greedy mode fires hundreds of HTTP HEAD requests for non-existent resources. Phase 1 (`resource-manifest-extractor`) already produces `resource_manifest.json` with 416 entries covering ship, slotitem, texture-provider, and explicit-path resource patterns from all 790 decoded modules. Phase 2 consumes this manifest in Rust to generate precise cache lists without guesswork.

## What Changes

- New Rust module to deserialize `resource_manifest.json` into typed structs
- New manifest-driven resource list generator that resolves parameter source expressions to concrete IDs via the Codex
- New `Manifest` strategy in `CacheListMakeStrategy` that uses manifest data instead of hardcoded ranges
- Elimination of `HOLES_COLLECTOR`, `EVENT_SHIP_HOLES`, `ENEMY_SHIP_HOLES`, `SPECIAL_SHIPS`, and similar static lists for resource types covered by the manifest
- Greedy mode falls back to manifest-based discovery instead of brute-force HTTP HEAD for covered types

## Capabilities

### New Capabilities
- `cache-manifest-integration`: Rust-side consumption of resource_manifest.json. Deserializes manifest entries, resolves parameter expressions to concrete IDs via Codex, generates resource paths using SuffixUtils. Provides a new Manifest strategy for cache list generation.

### Modified Capabilities
- `resource-manifest`: Adding requirement that the manifest output format SHALL be stable across main.js versions (versioned schema, backward-compatible field additions only), ensuring Rust consumer stability.

## Impact

- **crates/emukc_bootstrap/src/make_list/**: New manifest module, modified strategy dispatch, removal of hardcoded hole lists for covered types
- **crates/emukc_bootstrap/assets/resource_manifest.json**: Consumed at build time (already exists from Phase 1)
- **crates/emukc_model/**: New types for manifest deserialization (or in emukc_bootstrap if bootstrap-specific)
- **No breaking changes** — existing Default/Greedy strategies continue working; Manifest strategy is additive

## Non-goals

- BGM, voice, SE, furniture, map resource discovery (future phases requiring new AST patterns in main-decoder)
- Removing Greedy strategy entirely (still useful for resource types not covered by manifest)
- Runtime resource prefetching in the game server
- Modifying the TypeScript extractor output format
