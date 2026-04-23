## Context

The repository already has two pieces of the workflow the user wants:

1. `main-decoder` can extract `resource_manifest.json` and related decoder assets.
2. Rust `emukc_bootstrap` already knows how to build cache lists through `make_list`, including the `Manifest` strategy.

What is missing is an iteration tool that can take a decoder-produced manifest from an explicit path, build a candidate cache list without replacing checked-in bootstrap assets, and compare that candidate against the current bootstrap baseline. Today, the comparison loop is manual and tends to involve syncing files into `crates/emukc_bootstrap/assets/`, running ad hoc commands, and diffing outputs by hand.

This change does not touch gameplay traits such as `SortieOps`, `QuestOps`, or `MaterialOps`, and it does not change any KCSAPI handler or route group. The affected surface is examples, decoder output plumbing, and Rust-side bootstrap tooling.

## Goals / Non-Goals

**Goals:**
- Add a runnable example under `/Users/mg/github/emukc/examples/` that builds a candidate cache list from a caller-specified decoder manifest path.
- Reuse existing Rust `make_list` logic instead of reimplementing cache-list generation in the example.
- Generate a baseline cache list using the current bootstrap strategy in the same run so the example can emit a direct comparison report.
- Produce a structured report with overlap counts and grouped path deltas to guide decoder iteration.
- Make the decoder manifest available as a normal output artifact that the example can consume directly.

**Non-Goals:**
- Replacing `emukcd cache make-list` or changing its semantics.
- Automatically deciding which side of a diff is “correct”.
- Replacing all current bootstrap strategies with decoder-driven generation.
- Changing gameplay logic, database scope, or KCSAPI behavior.

## Decisions

### 1. Add an explicit manifest override path to Rust cache-list generation

**Decision:** Introduce reusable Rust helper(s) in `crates/emukc_bootstrap/src/make_list/` that can build cache-list items in memory while optionally using a caller-provided `ResourceManifest` instead of the repo-tracked asset.

**Rationale:** The example needs to compare an arbitrary decoder output without rewriting `crates/emukc_bootstrap/assets/resource_manifest.json`. An explicit override keeps the workflow non-destructive and makes the comparison logic reusable from tests or future tooling.

**Alternative considered:** Copy the decoder manifest into the repo asset path before running the existing CLI. Rejected because it is destructive, race-prone, and defeats the goal of fast iteration.

### 2. Keep the comparison tool as an example, not a new CLI command

**Decision:** Implement the workflow as a new example, likely under `/Users/mg/github/emukc/examples/decoder_cachelist_compare.rs`, rather than extending `src/bin/cli/cache/`.

**Rationale:** The user explicitly asked for an example, and the workflow is experimental/iterative rather than core operator UX. This keeps the production CLI stable while still making the tool easy to run.

**Alternative considered:** Add a new `emukcd cache compare` subcommand. Rejected because it would expand the public CLI surface for an iteration workflow that is still evolving.

### 3. Compare unique resource paths and report grouped deltas

**Decision:** The example will normalize both lists to unique `path` sets, compare them by set operations, and emit both summary metrics and grouped deltas (for example by `kcs2/resources/<domain>/<category>` prefix).

**Rationale:** `_id` values and insertion order are implementation details. The path set is the stable semantic unit for coverage/regression analysis. Grouped deltas make the output actionable without requiring manual inspection of tens of thousands of lines.

**Alternative considered:** Raw line-by-line diff of `.nedb` files. Rejected because it is noisy and dominated by ordering/ID differences rather than resource semantics.

### 4. Baseline strategy is configurable, with `Default` as the default

**Decision:** The example will accept a baseline strategy argument with `Default` as the default and optional support for `Manifest` / `Greedy` where practical.

**Rationale:** The current bootstrap baseline is usually `Default`, but advanced comparison runs may want to compare against another strategy without rewriting the example.

**Alternative considered:** Hardcode `Default` only. Rejected because it would limit the tool’s usefulness once decoder coverage matures.

### 5. Decoder manifest should exist both in `out/` and in synced assets when requested

**Decision:** Update `main-decoder/src/pipeline.ts` so the resource manifest can be consumed as a normal output artifact (for example under `main-decoder/out/resources/resource_manifest.json`) in addition to any optional sync into bootstrap assets.

**Rationale:** The comparison example should consume the decoder’s own output artifact directly. Requiring a sync into bootstrap assets would reintroduce the same mutation-based workflow we are trying to avoid.

**Alternative considered:** Make the example read only `crates/emukc_bootstrap/assets/resource_manifest.json`. Rejected because it couples comparison to synced repo state instead of decoder output.

## Risks / Trade-offs

- `[Override path drifts from production behavior]` -> A candidate manifest loaded from an arbitrary path might differ from the repo-tracked manifest in shape or version. Mitigation: validate and surface clear errors in the example before comparison starts.
- `[Comparison output becomes too large]` -> Full path-level diffs can be overwhelming. Mitigation: report grouped deltas plus bounded sample entries in the human-readable summary, while still writing the full structured report to disk.
- `[Tooling duplication]` -> The example could duplicate logic already present in `make_list`. Mitigation: push shared logic into reusable bootstrap helpers and keep the example as a thin orchestrator.
- `[Greedy baseline is expensive]` -> Comparing against `Greedy` may be slow. Mitigation: keep `Default` as the default baseline and require explicit opt-in for expensive modes.

## Migration Plan

1. Add reusable Rust helpers for in-memory cache-list generation and manifest overrides.
2. Add the comparison example and its report format.
3. Update `main-decoder` to write the resource manifest as an output artifact suitable for direct consumption.
4. Document the example workflow and verify it against a real decoder output.

Rollback is straightforward: remove the example and helper layer while leaving the existing CLI and bootstrap asset flow unchanged.

## Open Questions

- Should the comparison example write both a machine-readable JSON report and a human-readable text summary, or is JSON + stdout enough?
- Should the example support comparing against `Greedy` immediately, or defer that mode until after the default workflow is stable?
