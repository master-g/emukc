## 1. Type Definitions

- [x] 1.1 Define `ResourceManifestEntry` union type (ship, slotitem, texture-provider, explicit-path) in `resource-manifest.ts`
- [x] 1.2 Define `ResourceManifest` top-level type (version, generatedAt, entries)
- [x] 1.3 Export types from `main-decoder/src/types.ts` or `resource-manifest.ts`

## 2. Core Extractor

- [x] 2.1 Implement ship resource matcher: AST traversal for `resources.getShip` and `ShipLoader.add` calls, extract id/damaged/type args
- [x] 2.2 Implement slotitem resource matcher: AST traversal for `resources.getSlotitem` and `SlotLoader.add` calls, extract id/type args
- [x] 2.3 Implement texture-provider matcher: AST traversal for `getTexture` calls, extract provider name and numeric texture IDs
- [x] 2.4 Implement explicit-path extraction: regex scan for `resources/` patterns in all module sources
- [x] 2.5 Implement module traversal loop: iterate all modules in ModuleGraph, run each matcher, collect entries with moduleId/moduleName provenance
- [x] 2.6 Implement deduplication: merge entries by (kind, key fields), aggregate moduleIds

## 3. Output & CLI

- [x] 3.1 Implement `extractResourceManifest()` main function: accepts ModuleGraph, returns ResourceManifest
- [x] 3.2 Add `--sync-resource-manifest` flag to CLI argument parser in `cli.ts`
- [x] 3.3 Wire flag into `pipeline.ts`: call extractor and write JSON to `crates/emukc_bootstrap/assets/resource_manifest.json`
- [x] 3.4 Export new function from `main-decoder/src/index.ts`

## 4. Tests

- [x] 4.1 Unit test: ship resource matcher with sample AST (getShip + ShipLoader.add)
- [x] 4.2 Unit test: slotitem resource matcher with sample AST (getSlotitem + SlotLoader.add)
- [x] 4.3 Unit test: texture-provider matcher with sample AST (getTexture with IDs)
- [x] 4.4 Unit test: explicit-path extraction from module source strings
- [x] 4.5 Unit test: deduplication logic (same pattern from multiple modules)
- [x] 4.6 Integration test: full pipeline with `--sync-resource-manifest` flag

## 5. Verification

- [x] 5.1 Run `bun run check` — no type errors
- [x] 5.2 Run `bun test` — all tests pass (27/27)
- [x] 5.3 Run `bun run decode -- --sync-resource-manifest` — produces valid `resource_manifest.json`
- [x] 5.4 Verify manifest contains entries from both battle and non-battle modules
