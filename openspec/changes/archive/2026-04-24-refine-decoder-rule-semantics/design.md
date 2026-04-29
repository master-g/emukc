## Context

`CacheListMakeStrategy::Rules` already exists in [`crates/emukc_bootstrap/src/make_list/source/mod.rs`](/Users/mg/github/emukc/crates/emukc_bootstrap/src/make_list/source/mod.rs) and is usable through [`examples/decoder_cachelist_compare.rs`](/Users/mg/github/emukc/examples/decoder_cachelist_compare.rs). The current comparison report shows a strong transitional result: the rules path reaches full baseline recall (`69208 / 69208`) but still emits `6382` candidate-only paths, with `96.62%` of that noise concentrated in seven ship and slot prefixes.

The remaining over-generation comes from two related issues:

- Decoder assets still preserve some raw target categories in `resource_manifest.json`, while `resource_categories.json` already normalizes parts of the same space into narrower semantic groups.
- Rust generation in [`crates/emukc_bootstrap/src/make_list/manifest/generate.rs`](/Users/mg/github/emukc/crates/emukc_bootstrap/src/make_list/manifest/generate.rs) still relies on broad fallback tables and universal selector expansion for variant families that are no longer actually universal in decoded `main.js`.

The decoder also still rebuilds path rules by reading Rust source in [`main-decoder/src/path-rules.ts`](/Users/mg/github/emukc/main-decoder/src/path-rules.ts), which is useful for parity today but conflicts with the long-term goal of making decoder output the primary semantic authority.

There are no gameplay trait, database, or KCSAPI route-group changes in this work. No new `SortieOps`, `MaterialOps`, or other trait methods are needed. No new Codex fields are required beyond the existing `start2` data already loaded into memory.

## Goals / Non-Goals

**Goals:**

- Make decoder-emitted rule data precise enough that the `Rules` path stops expanding `banner*` and `item_*2` families with universal fallback behavior.
- Align decoder outputs so ship/slot semantic meaning is expressed once and consumed consistently by Rust generation.
- Reduce dependency on Rust-authored fallback constants as decoder rule inputs, especially in `main-decoder/src/path-rules.ts`.
- Preserve the current fast comparison loop using `decoder_cachelist_compare` as the regression signal for precision improvements.

**Non-Goals:**

- Do not switch [`src/bin/cli/cache/make_list.rs`](/Users/mg/github/emukc/src/bin/cli/cache/make_list.rs) or [`src/bin/cli/auto/mod.rs`](/Users/mg/github/emukc/src/bin/cli/auto/mod.rs) to default to `CacheListMakeStrategy::Rules`.
- Do not redesign stable non-ship/slot domains such as BGM, map, furniture, sound, useitem, or voice.
- Do not remove legacy generation code under `crates/emukc_bootstrap/src/make_list/source/kcs*/`.
- Do not introduce new external dependencies or Codex data-loading requirements.

## Decisions

### D1: Introduce canonical ship/slot semantic metadata in decoder rule output

The decoder will continue to emit raw `resource_manifest.json` entries from [`main-decoder/src/resource-manifest.ts`](/Users/mg/github/emukc/main-decoder/src/resource-manifest.ts), but `cache_rules.json` will become the canonical place for ship/slot semantic refinements that are narrower than raw call extraction.

That semantic layer will cover:

- whether a target is base, damaged-only, or variant-expandable
- which ship groups may legally generate a target (friendly, abyssal, graph-driven, event-only, or a decoder-observed sparse subset)
- which slot targets are normalization-driven aliases rather than independent universal categories

Rationale:

- The decoder already observes more meaning than the raw manifest preserves.
- The remaining precision gaps are concentrated in semantic target families, not missing path templates.
- Making `cache_rules.json` canonical keeps Rust execution focused on applying decoder-authored meaning instead of re-deriving it from fallback tables.

Alternative considered: continue storing only raw manifest/category outputs and let Rust infer meaning. Rejected because it keeps rule intent split across two runtimes and is the direct cause of the current over-expansion.

### D2: Rust rule execution should prefer decoder semantic constraints before broad fallback

[`crates/emukc_bootstrap/src/make_list/manifest/generate.rs`](/Users/mg/github/emukc/crates/emukc_bootstrap/src/make_list/manifest/generate.rs) will treat decoder semantic rules as the first-class selector for ship and slot targets covered by `cache_rules.json`. Broad fallback behavior remains only for categories still marked unresolved or outside this change's semantic scope.

In practice this means:

- `banner*` families must stop expanding from static variant tables when decoder rules identify a narrower canonical target or selector scope.
- `item_on2` and `item_up2` must stop behaving like universal slotitem categories when they are only runtime-normalized alternates of a narrower source family.
- unresolved selectors in [`crates/emukc_bootstrap/src/make_list/manifest/resolve.rs`](/Users/mg/github/emukc/crates/emukc_bootstrap/src/make_list/manifest/resolve.rs) remain tolerated, but should no longer be used to inflate categories already covered by semantic rules.

Rationale:

- This isolates fallback logic to genuinely unresolved cases.
- It preserves current robustness while letting decoder precision improvements actually change output.

Alternative considered: delete fallback logic immediately. Rejected because unresolved selectors still exist and fallback behavior remains necessary during the transition.

### D3: Stop treating Rust-authored path rules as decoder truth

[`main-decoder/src/path-rules.ts`](/Users/mg/github/emukc/main-decoder/src/path-rules.ts) currently parses Rust source files to mirror constants and hole lists into decoder output. This change will narrow that dependency by shifting ship/slot semantic meaning into decoder-owned rule extraction instead of backfilling it from Rust.

The target end state for this change is not “no path rules at all”, but:

- path templates and hard parity fallbacks may still exist in Rust
- decoder-generated ship/slot semantic scopes must no longer depend on parsing Rust constants

Rationale:

- A decoder-first pipeline cannot claim semantic authority while importing the same semantics from Rust.
- This is the smallest step that moves the system toward the desired `main.js -> decode -> rule -> cache list` model without forcing a full default-path switch.

Alternative considered: defer all single-source-of-truth work until after default switching. Rejected because it would harden the current circular dependency.

### D4: Keep this change precision-focused and measured by comparison output

The change will use [`examples/decoder_cachelist_compare.rs`](/Users/mg/github/emukc/examples/decoder_cachelist_compare.rs) and the generated `.data/decoder_rules_compare*.json` reports as the primary acceptance signal. The goal is to reduce candidate-only ship/slot noise substantially while maintaining `100%` baseline coverage.

Rationale:

- The remaining work is highly concentrated and measurable.
- Precision improvements are easy to regress silently without an explicit comparison loop.

Alternative considered: accept semantic changes based on targeted unit tests alone. Rejected because unit tests do not show whole-list overlap behavior.

## Risks / Trade-offs

- `[Risk] Decoder semantic rules become too narrow and introduce false negatives.` → Mitigation: keep baseline comparison mandatory and preserve fallback behavior for unresolved targets.
- `[Risk] Raw manifest entries and semantic rules diverge in confusing ways.` → Mitigation: document `cache_rules.json` as the canonical semantic layer for ship/slot generation and keep raw manifest extraction intentionally broad.
- `[Risk] Partially removing Rust-derived path rule inputs leaves mixed authority in the short term.` → Mitigation: scope this change only to ship/slot semantic meaning, not every path-rule field at once.
- `[Risk] Precision work stalls because the remaining domains look “good enough.”` → Mitigation: keep the change explicitly focused on the seven dominant candidate-only prefixes until they are reduced.

## Migration Plan

1. Extend decoder rule extraction in `main-decoder/src/cache-rules.ts` and adjacent decoder modules so ship/slot semantics needed by `banner*` and `item_*2` families are emitted explicitly.
2. Adjust decoder-side manifest/category generation in `main-decoder/src/resource-manifest.ts` and `main-decoder/src/resource-categories.ts` so canonical semantic meaning is no longer split inconsistently across outputs.
3. Update Rust rule execution in `crates/emukc_bootstrap/src/make_list/manifest/generate.rs` and related loaders to prefer decoder semantic constraints over broad fallback expansion for covered categories.
4. Re-run `decoder_cachelist_compare` and keep the change gated on maintaining `100%` baseline coverage while reducing ship/slot candidate-only noise.

Rollback strategy:

- Keep emitting the previous broad rule shape and restore the prior Rust fallback precedence.
- Comparison output returns to the current `6382` candidate-only baseline, but no bootstrap path is broken because default strategy switching is out of scope.

## Open Questions

- Should damaged-only semantic targets be represented as normalized canonical targets, or as raw targets plus a semantic flag that Rust interprets?
- Should slot normalization semantics live entirely in `cache_rules.json`, or should some alias families also be reflected in `resource_manifest.json` for debugging clarity?
- How far should this change go in reducing `path-rules.ts` Rust parsing: only ship/slot semantic fields, or also adjacent hole/category metadata when the extraction becomes straightforward?
