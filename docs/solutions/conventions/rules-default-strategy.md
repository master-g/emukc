---
title: "Rules default strategy: Default == Rules, Greedy == Rules + holes report"
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
tags: [cache-list, make-strategy, decoder-rules, greedy, holes-report, manifest]
related_components: [emukc_cache]
---

# Rules default strategy: Default == Rules, Greedy == Rules + holes report

## Context

After migrating cache-list generation to the decoder-driven rules pipeline,
the `CacheListMakeStrategy` variants were redefined. `Default` is no longer a
legacy hardcoded path — it delegates to the decoder rules bundle, matching
`Rules` output. `Greedy` wraps `Rules` and adds a holes report. This
convention records that redefinition so strategy selection is unambiguous.

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

### Greedy strategy

- **`Greedy` wraps `Rules` plus a holes report.** `CacheListMakeStrategy::Greedy`
  SHALL delegate path generation to the `Rules` code path, then produce a
  `holes_report.txt` file if holes exist.
- **No holes, no report.** When no holes are detected, `Greedy` SHALL NOT
  generate a `holes_report.txt` file.

### Legacy removal

- **Legacy hardcoded paths removed.** The hardcoded path-generation branches
  that run without decoder assets SHALL be removed from `source/mod.rs`,
  `source/kcs/mod.rs`, and `source/kcs2/resources/mod.rs`. Any
  `CacheListMakeStrategy` variant except `Minimal` and `Manifest` SHALL load
  and use the decoder rules bundle for path generation.

### Comparison example baseline

- **`decoder_cachelist_compare` defaults to `Manifest`.** The example SHALL
  default `--baseline` to `manifest` (not `default`), so a plain run compares
  against the manifest-derived baseline.

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
Greedy   → Rules bundle output + holes_report.txt (if holes)
Minimal  → minimal hardcoded set (exempt from rules)
Manifest → manifest-derived set (exempt from rules)
```

## Related

- `crates/emukc_bootstrap/src/source/` — the strategy implementations.
- `examples/` — `decoder_cachelist_compare`.
- `docs/solutions/best-practices/cache-manifest-integration.md` — how the rules bundle consumes decoder assets.
