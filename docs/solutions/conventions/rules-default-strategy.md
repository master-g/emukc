---
title: "Rules default strategy: Default == Rules"
date: 2026-06-22
category: conventions
module: emukc_bootstrap
problem_type: convention
component: tooling
severity: medium
applies_when:
  - "Selecting a CacheListMakeStrategy for cache-list generation"
  - "Modifying the decoder-driven rules pipeline"
  - "Running the decoder_cachelist_compare example"
tags: [cache-list, make-strategy, decoder-rules, manifest]
related_components: [emukc_cache]
---

# Rules default strategy: Default == Rules

## Context

After migrating cache-list generation to the decoder-driven rules pipeline,
the `CacheListMakeStrategy` variants were redefined. `Default` is no longer a
legacy hardcoded path — it delegates to the decoder rules bundle, matching
`Rules` output. This convention records that redefinition so strategy
selection is unambiguous.

`Greedy` was part of this convention until 2026-09-21; see *Greedy, removed*
below.

## Guidance

The following conventions hold for `CacheListMakeStrategy`:

### Default strategy

- **`Default` uses the decoder rules bundle.** `CacheListMakeStrategy::Default`
  SHALL load the decoder rules bundle (`cache_rules.json` and sibling assets)
  and produce the same cache list output as `CacheListMakeStrategy::Rules`.
- **Default fails clearly when the bundle is missing.** If
  `cache_rules.json` cannot be loaded, `Default` SHALL return an error
  indicating the missing asset; it SHALL NOT silently fall back to legacy
  hardcoded generation.

### Greedy, removed 2026-09-21

`CacheListMakeStrategy::Greedy` and the `--greedy` / `--concurrent` flags no
longer exist. The convention above ("delegate to `Rules`, then produce a
`holes_report.txt`") described behaviour the code could not perform:

- The probing branches Greedy selected were unreachable. `source/mod.rs`
  hardcoded `Rules` after the strategy match, and `kcs2/mod.rs` plus
  `kcs2/resources/mod.rs` each overwrote the caller's strategy with
  `Manifest`, so `--greedy` produced output byte-identical to the default.
- `holes_report.txt` could never be written. Its collector had a reader and a
  clear, and no writer anywhere in the workspace; a second, unrelated
  `HolesReport` type was `#[expect(dead_code)]` and never constructed.

Do not restore this code from git history. Its four probe functions had drifted
apart (one used `add_unversioned` where the others used `add(p, v)`, which would
have left those entries never refreshed), and the families it probed are now
covered by decoder rules. If probe-based completion is ever needed again, derive
the candidate set from the decoder rules' coverage gaps rather than enumerating.

### Legacy removal

- **Legacy hardcoded paths removed.** The hardcoded path-generation branches
  that run without decoder assets SHALL be removed from `source/mod.rs`,
  `source/kcs/mod.rs`, and `source/kcs2/resources/mod.rs`. Any
  `CacheListMakeStrategy` variant except `Minimal` and `Manifest` SHALL load
  and use the decoder rules bundle for path generation.

### Comparison example baseline

- **`decoder_cachelist_compare` defaults to `Manifest`.** The example SHALL
  default `--baseline` to `manifest` (not `default`), so a plain run compares
  against the manifest-derived baseline. Its `greedy` baseline option and
  `--concurrent` flag were removed together with the strategy.

## Why This Matters

Before the migration, `Default` silently produced a different (legacy) list
than `Rules`, making strategy choice a hidden correctness lever. Redefining
`Default == Rules` removes that footgun: the default is always the
decoder-driven output, and the only strategies that differ (`Minimal`,
`Manifest`) are explicitly non-rules.

## When to Apply

- When choosing a strategy for a cache-list generation run.
- When modifying the decoder rules pipeline or its fallback branches.
- When running or updating the `decoder_cachelist_compare` example.

## Examples

```
Default  → Rules bundle output (same as Rules)
Minimal  → minimal hardcoded set (exempt from rules)
Manifest → manifest-derived set (exempt from rules)
```

## Related

- `crates/emukc_bootstrap/src/source/` — the strategy implementations.
- `examples/` — `decoder_cachelist_compare`.
- `docs/solutions/best-practices/cache-manifest-integration.md` — how the rules bundle consumes decoder assets.
