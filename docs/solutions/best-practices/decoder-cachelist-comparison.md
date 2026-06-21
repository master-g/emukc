---
title: "Decoder cache-list comparison example contract"
date: 2026-06-22
category: best-practices
module: emukc_bootstrap
problem_type: tooling_decision
component: tooling
severity: medium
applies_when:
  - "Comparing decoder-first candidate cache lists against baseline bootstrap output"
  - "Planning migration from legacy fallback to decoder-authoritative generation"
tags: [decoder, cachelist, comparison, migration, bootstrap]
related_components: [emukc_cache]
---

# Decoder cache-list comparison example contract

## Context

The repo provides a runnable comparison example that evaluates a
decoder-produced cache-list candidate against the current bootstrap baseline.
This contract documents its input handling, reporting shape, and the
migration-readiness diagnostics it must surface. It originated as an
openspec capability spec and is preserved here as captured knowledge.

## Guidance

**Inputs and candidate loading.**

- Accept an explicit decoder manifest path and load it as the candidate
  source; never modify `crates/emukc_bootstrap/assets/resource_manifest.json`.
- Fail with a clear validation error before producing output if the candidate
  manifest path is missing or invalid.
- Generate both a candidate cache list (from the explicit decoder manifest)
  and a baseline cache list (from the current bootstrap strategy) in the same
  run. Default baseline strategy is `Manifest`; a supported override may be
  selected.
- Treat an explicit `cache_rules.json` (`--rules`) path under a decoder output
  `resources/` directory as the root of a decoder bundle: derive sibling
  coverage assets from that same output tree and evaluate the candidate using
  the full bundle, not `cache_rules.json` alone.

**Decoder bundle consumption.**

- When the candidate manifest path points into a decoder output resources
  directory, derive sibling decoder coverage assets from that same tree and
  build the candidate from the full available bundle without requiring a
  bootstrap-asset sync first.
- If an optional sibling coverage asset is missing, report which assets were
  unavailable and proceed with whatever loaded successfully.
- The decoder pipeline must make the resource manifest available as a normal
  output artifact (written to the decoder output area in addition to any
  optional bootstrap sync), so the comparison can consume decoder output
  directly.

**Report shape — global metrics.**

- Compare using unique resource `path` values, not `_id` or line order.
- Include: candidate count, baseline count, intersection count,
  only-baseline count, only-candidate count, and at least one
  percentage-based coverage metric.
- Group deltas by resource prefix or category.

**Report shape — domain-level breakdown.**

- Include domain-level baseline/candidate/overlap metrics for major cache-list
  domains: ship, slot, sound, map, furniture, BGM, useitem, voice.
- Surface sparse ship/slot categories that are significantly over- or
  under-generated in a grouped delta section.

**Report shape — authority breakdown (decoder-first candidates).**

- When the candidate is built from a decoder rule bundle, include counts for
  rule-authored candidate paths and fallback-authored candidate paths, plus
  grouped fallback residual prefixes or family labels.
- If the candidate is entirely decoder-authoritative for measured domains,
  show zero fallback-authored paths and an empty residual section.

**Template-backed ownership.**

- Report template-backed decoder ownership separately from generic
  rule-authored and fallback-authored totals: grouped template-backed
  rule-authored counts by family or domain, plus grouped fallback residuals
  for partial/unresolved template families.
- When the pipeline reports a reason for a template-backed fallback residual,
  preserve that reason in the machine-readable report and surface it at the
  grouped blocker level.
- Treat unresolved template-backed families and template-domain fallback
  residuals as migration blockers until decoder-authoritative ownership is
  proven. Keep `baseline_only_count` and `candidate_only_count` visible even as
  residuals shrink; migration readiness stays false while any measured
  template-backed residual blocker remains.

**Sound-domain reporting.**

- When a decoder-first run produces fallback-authored sound paths, surface the
  relevant `kcs/sound/*` residual families in a grouped section or
  migration-blocker summary; the sound fallback share must stay visible without
  manual path inspection.
- Preserve enough sound-domain detail to verify that fallback reductions came
  from the sound migration work rather than unrelated changes.
- Distinguish explicit audio asset coverage (`se`/`bgm`/titlecall) from
  algorithmic `kcs/sound/*` rule coverage so migration analysis stays
  actionable.

**Migration readiness.**

- Emit a migration summary: if the candidate has baseline-only paths,
  unresolved rule keys, or fallback-dependent decoder families, list those
  blocker categories explicitly and do not present the candidate as
  migration-ready.
- If the candidate has full baseline recall and no unresolved or
  fallback-dependent families, surface that no remaining migration blockers
  exist for the measured domains while keeping supporting metrics available.

**Default-vs-Rules no-op warning.**

- When `--baseline default --rules` is passed, emit a warning that `Default`
  delegates to `Rules`, then continue the comparison (which will show 100%
  overlap).

## Why This Matters

The comparison example is the bridge between decoder extraction work and the
decision to switch the bootstrap default to decoder-first generation. Without
these reporting guarantees, migration readiness becomes a judgment call based
on raw path lists, which hides high-impact sparse-category gaps and
template-backed residuals.

## When to Apply

- After a decoder run, to validate that newly extracted rules improved
  candidate coverage.
- Before proposing a bootstrap default-switch: the migration-blocker summary
  must be clean for the measured domains.

## Examples

- A run comparing a decoder bundle candidate against the `Manifest` baseline
  reports `rule_authored=12340`, `fallback_authored=210`, with residuals
  grouped under `kcs/sound/kc9998` and `gauge.map` — the summary lists those
  as template-backed migration blockers and marks the candidate not ready.
- `--baseline default --rules` prints a no-op warning and reports 100%
  overlap.

## Related

- `docs/solutions/best-practices/decoder-first-cachelist-pipeline.md`
- `docs/solutions/best-practices/decoder-coverage-assets.md`
- `docs/solutions/best-practices/cache-manifest-integration.md`
