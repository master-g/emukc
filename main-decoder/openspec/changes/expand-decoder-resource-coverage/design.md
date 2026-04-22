## Context

main-decoder is a Bun/TypeScript pipeline that deobfuscates KanColle's main.js (6.2.8.0, 2,152 modules) and extracts resource knowledge. Currently only battle-related modules (196) produce assets consumed by Rust's `emukc_bootstrap` crate. The existing `resource-manifest.ts` already scans all modules for `getShip`/`getSlotitem`/`getTexture`/explicit-path patterns and syncs `resource_manifest.json`, but Rust's `make_list` still relies on many hand-maintained lists for default cache generation.

Rust's `make_list` module generates cache resource lists using 4 strategies (Minimal, Default, Greedy, Manifest). The Default strategy relies on hardcoded arrays like `SPECIAL_SHIPS` (40 IDs), `SP_REMODEL_SHIPS` (50 IDs), `BTXT_FLAT_IDS` (300+ IDs), `SE` (333 IDs), `ENEMY_PLANE_MAX_ID` (25), plus hardcoded category arrays for ships (14+ categories) and slots (6+ categories).

The Manifest strategy already has the infrastructure to read JSON assets and generate paths via `generate.rs`/`resolve.rs`, but only handles ship/slot/explicit-path entries from `resource_manifest.json`. This change keeps texture-provider knowledge in that existing manifest flow and focuses the new assets on replacing hardcoded Default-strategy lists only where decoded `main.js` coverage is demonstrably sufficient.

## Goals / Non-Goals

**Goals:**
- Extract ship/slot target type catalogs and Rust-facing generation groups from main.js so Rust can stop hardcoding category arrays
- Extract ship/slot ID subsets only when membership is directly observable in main.js source
- Extract audio resource patterns (SE, BGM, voice) from main.js
- Extract UI resource patterns (map, furniture, use items) from main.js
- Rust `make_list` consumes extracted JSON assets only where decoded coverage is strong enough to replace hardcoded logic
- All new extractors have test coverage

**Non-Goals:**
- Replacing the Greedy strategy (remote existence checking remains valuable for discovering new/unknown resources)
- Modifying the battle knowledge extraction system (already works well)
- Changing the Codex data model or bootstrap download process
- Replacing `resource_manifest.json` or moving texture-provider extraction out of that existing manifest flow
- Extracting versioned img (`kcs2/img/`) or plain resources (`kcs2/*.js/css/html`) — these are structural, not data-driven
- Proving exhaustive ship/slot ID coverage for categories whose membership is only visible through runtime API data, CDN existence checks, or Rust-side curated baselines
- Replacing Rust's existing exhaustive ship/slot ID baselines (`SPECIAL_SHIPS`, `SP_REMODEL_*`, `REWARDS`, `BTXT_FLAT_IDS`, etc.) with partial `main.js` observations in this change

## Decisions

### D1: New extractors follow the `resource-manifest.ts` pattern

Each new extractor is a standalone TS module that takes a `ModuleGraph` and returns a typed result. They share the same AST traversal helpers (`parseFactorySource`, `expressionToSource`, etc.) from existing code.

**Rationale**: Consistent with existing architecture. Each extractor focuses on one resource domain. Easy to test independently.

**Alternative considered**: One mega-extractor for all resources. Rejected — too complex, hard to test, mixes concerns.

### D2: Assets synced as separate JSON files, not one monolith

Each capability produces its own JSON file in `crates/emukc_bootstrap/assets/`:
- `resource_categories.json` — ship/slot target type catalogs plus named generation groups used by Rust's Default strategy
- `resource_id_sets.json` — best-effort ship/slot ID subsets directly observed in `main.js`, plus coverage metadata for unresolved categories
- `audio_resources.json` — SE ids, categorized BGM ids, and voice metadata
- `ui_resources.json` — nested UI data for map files, furniture, use items, areas, and world select files

**Rationale**: Separation of concerns. Rust can load only what it needs. Easier to diff/review individual assets. Matches existing pattern (4 separate battle JSON files).

**Alternative considered**: Single `all_resources.json`. Rejected — large file, unnecessary coupling, harder to evolve schemas independently.

### D3: Rust loads consumed assets at compile time via `include_str!`

JSON assets that are actually wired into Rust in this change are loaded via `include_str!("../assets/xxx.json")` from small Rust helper modules and deserialized with serde. Same pattern as existing battle assets in `crates/emukc_bootstrap/src/battle_rules.rs`.

**Rationale**: Zero runtime I/O. Assets are part of the compiled binary. Consistent with existing battle asset loading. Advisory-only assets such as `resource_id_sets.json` do not need to be loaded until a later migration change promotes them into runtime use.

### D4: New `--sync-assets` CLI flag replaces `--sync-battle-assets`

Consolidate `--sync-battle-assets` and `--sync-resource-manifest` into a single `--sync-assets` flag that syncs all asset types. Old flags kept as aliases for backwards compatibility.

**Rationale**: Simpler UX. Users don't need to know which assets are "battle" vs "resource" vs "audio".

### D5: `resource_id_sets.json` is observational, not migratory

`resource_id_sets.json` captures only IDs that are literally enumerable in decoded `main.js` (numeric literals in control flow, inline arrays, object tables, preload manifests, etc.). It does not try to "complete" categories by consulting Rust baselines, `start2` data, or CDN checks. Rust does not consume this asset as a source-of-truth replacement in this change.

**Rationale**: This keeps the extractor honest. Some current Rust ID lists are not actually encoded exhaustively in `main.js`; pretending otherwise would produce brittle, misleading assets.

### D6: Phase ordering — resource categories first, then observational ID sets, then audio, then UI

Ship/slot categories have the clearest migration path because their target types are directly visible in `main.js` and already proven by the current implementation. Observational ID-set extraction reuses similar traversal infrastructure but stays best-effort. Audio and UI require more new AST pattern discovery, so they follow after the ship/slot foundation is stable.

### D7: `ui_resources.json` is nested by domain

`ui_resources.json` uses a domain-oriented structure:
- `map.defaultFiles` and `map.eventFiles`
- `furniture.*`
- `useItem.cardIds` and `useItem.underlineIds`
- `area.sallyIds` and `area.airunitIds`
- `worldSelect.files`

**Rationale**: This matches how Rust consumes the data. It avoids ambiguous flat field names and makes `useitem/card` vs `useitem/card_` explicit.

### D8: `map` data is stored as explicit file lists

The map portion of `ui_resources.json` stores explicit file lists for default and event maps instead of only area/map identifiers.

**Rationale**: Current map output contains many irregular `_image`, `_info`, and event-suffixed files. Explicit file lists are the safest way to guarantee non-regression against today's `map.rs` behavior.

### D9: Runtime loading is tolerant; tests enforce coverage

Rust loaders use serde defaults, log warnings for empty or partial fields, and skip only the missing data instead of panicking. Coverage and non-regression guarantees are enforced in tests and verification commands, not by crashing at runtime.

**Rationale**: This keeps the binary resilient when assets lag behind a main.js update, while still preventing silent regressions from landing through CI.

## Risks / Trade-offs

- **AST pattern coverage**: New extractors rely on recognizing resource-loading patterns in deobfuscated code. Some patterns may use `_0x` obfuscated identifiers that aren't fully resolved → Mitigation: extractors report coverage stats; fall back to greedy strategy for unknown resources
- **`main.js` incompleteness for concrete ID sets**: Some ship/slot resource subsets are only recoverable today via runtime state or CDN checks, not from literal source alone → Mitigation: `resource_id_sets.json` is explicitly best-effort and not wired into Rust replacement logic in this change
- **Asset staleness**: JSON assets are snapshots of one main.js version. When game updates, assets must be regenerated → Mitigation: existing `bun run decode -- --sync-assets` workflow; Rust logs warnings and skips stale subsets instead of panicking
- **Build time**: Including more JSON assets increases compile time slightly → Mitigation: assets are small (<500KB total); compile-time cost is negligible
- **Map asset size**: Explicit file lists are larger than compressed area/map tuples → Mitigation: the added JSON is still small relative to the repo, and explicit lists greatly reduce ambiguity
- **False positives**: AST extraction might find resource patterns that don't actually exist on CDN → Mitigation: Rust already has Greedy strategy for existence checking; manifest strategy produces superset that's acceptable for cache population
