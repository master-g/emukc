## Context

main-decoder (`main-decoder/`) is a Bun+TypeScript project that decodes KanColle's obfuscated `main.js` via AST transformations. It already extracts battle-specific resource rules via `battle-knowledge.ts`, producing 241 rules (86 ship, 67 slotitem, 88 texture-provider) synced to `crates/emukc_bootstrap/assets/`.

The Rust-side `cache make-list` command (`crates/emukc_bootstrap/src/make_list/`) generates candidate resource paths using hardcoded ID ranges and validates them with HTTP HEAD requests. This is fragile — hardcoded "holes" lists must be manually maintained, and greedy mode fires hundreds/thousands of HEAD checks for non-existent resources.

The new extractor extends the existing AST matching patterns from battle-only modules to all modules, producing a comprehensive resource manifest.

## Goals / Non-Goals

**Goals:**
- Extract resource loading patterns (ship, slotitem, texture-provider, explicit paths) from ALL decoded modules
- Output a structured JSON manifest that Rust can consume to generate precise cache lists
- Eliminate HTTP HEAD checks for covered resource types
- Maintain zero changes to existing `battle-knowledge.ts`

**Non-Goals:**
- BGM, voice, SE, furniture, map resource discovery (Phase 2 — requires discovering new JS API patterns)
- Rust-side `make_cache_list` integration (Phase 2)
- Runtime resource prefetching in the game server
- Replacing or modifying `battle-knowledge.ts`

## Decisions

### 1. Standalone module vs extending battle-knowledge.ts

**Decision**: New file `resource-manifest.ts`, no import dependency on `battle-knowledge.ts`.

**Rationale**: battle-knowledge is tightly scoped to battle validation diagnostics. Resource manifest has different goals (full coverage, different output format). Independence prevents scope creep in battle knowledge.

**Alternative considered**: Import shared AST matchers from battle-knowledge. Rejected to avoid coupling — the two extractors serve different consumers and may diverge.

### 2. Output: parameter sources vs concrete paths

**Decision**: Extract parameter source expressions (e.g., `shipMstIdSource: "vo.ships[i].api_id"`) rather than resolving to concrete IDs.

**Rationale**: main-decoder doesn't have access to codex data (the game manifest). Rust-side does. Resolving expressions to IDs is Rust's responsibility. This keeps the TS extractor stateless and independent of game data versions.

**Alternative considered**: Pass codex data to TS and resolve IDs. Rejected — wrong layer of abstraction, couples decode pipeline to game data.

### 3. Pipeline integration: parallel with battle-knowledge

**Decision**: `extractResourceManifest()` runs independently after `extractModuleGraph()`, same position as `extractBattleKnowledge()`.

**Rationale**: Both extractors consume the same module graph input. No dependencies between them. Parallel execution is natural.

### 4. Deduplication strategy

**Decision**: Deduplicate entries by (kind, key fields) before output. For ship: (targetType, source). For slotitem: (targetType, source). For texture-provider: (provider). For explicit-path: per-path.

**Rationale**: Same resource loading pattern may appear in multiple modules. Dedup keeps manifest compact. Module provenance preserved via moduleId arrays.

## Risks / Trade-offs

**[AST pattern fragility]** → main.js updates may change function names or calling conventions. Mitigation: patterns are simple string-based callee matching, relatively stable. Fail-open: unknown patterns just produce no output.

**[Coverage unknown]** → Without scanning non-battle modules first, we don't know how many additional resource patterns exist. Mitigation: Phase 1 focuses on proven patterns. The explicit-path regex catches hardcoded URLs regardless of pattern matching.

**[No Rust consumer yet]** → Phase 1 output has no consumer. Mitigation: output format is simple and documented. Phase 2 integration is straightforward — read JSON, resolve IDs via codex, generate paths via SuffixUtils.
