## Why

The Rust-side `cache make-list` command uses hardcoded ID ranges and HTTP HEAD checks to discover game resources. This is incomplete (misses paths) and wasteful (thousands of HEAD requests for non-existent resources). main-decoder already has AST infrastructure that extracts battle-specific resource rules from KanColle's main.js. Extending this to scan all modules produces a precise resource manifest, eliminating guesswork and HEAD-check waste.

## What Changes

- New TypeScript extractor `main-decoder/src/resource-manifest.ts` that scans all decoded modules for resource loading patterns
- Extends existing AST matcher patterns (getShip, getSlotitem, getTexture, ShipLoader/SlotLoader.add, explicit kcs2/ paths) from battle-only to all modules
- New CLI flag `--sync-resource-manifest` for the decode pipeline
- New output file `crates/emukc_bootstrap/assets/resource_manifest.json`
- New tests for the extractor in `main-decoder/test/`

## Capabilities

### New Capabilities
- `resource-manifest`: Full-module resource discovery extractor for main-decoder. Extracts ship, slotitem, texture-provider, and explicit-path resource loading patterns from all decoded KanColle main.js modules. Outputs a structured JSON manifest for Rust-side consumption.

### Modified Capabilities
<!-- No existing spec-level behavior changes. The new extractor is additive and does not modify existing battle-knowledge extraction. -->

## Impact

- **main-decoder/src/**: New file `resource-manifest.ts`, minor additions to `pipeline.ts` and `cli.ts` for the new flag
- **main-decoder/test/**: New test file for the extractor
- **crates/emukc_bootstrap/assets/**: New `resource_manifest.json` output
- **No Rust code changes** in this phase — Rust-side consumption is Phase 2
- **No breaking changes** — fully additive, existing `--sync-battle-assets` flow unchanged

## Non-goals

- BGM, voice, furniture, map resource discovery (Phase 2, requires new AST patterns)
- Rust-side `make_cache_list` integration (Phase 2)
- Modifying or replacing the existing `battle-knowledge.ts` extractor
- Runtime resource loading in the game server
