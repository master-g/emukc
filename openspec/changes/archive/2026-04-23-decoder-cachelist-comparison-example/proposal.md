## Why

The current decoder-to-cache-list iteration loop is too manual: `main-decoder` produces rules, Rust `make_list` consumes repo-tracked assets, and comparing a candidate decoder output against the current bootstrap baseline still requires ad hoc commands and asset replacement. A dedicated example that generates both lists and a comparison report would make coverage and regression analysis much faster.

## What Changes

- Add a new runnable example that builds a decoder-driven candidate cache list from an explicit manifest path and compares it against the current bootstrap strategy output.
- Reuse the existing Rust `make_list` infrastructure by exposing a non-destructive manifest override path instead of forcing the example to mutate repo-tracked assets.
- Emit a structured comparison report with totals, overlap, only-baseline/only-candidate counts, and grouped path deltas so decoder iterations can be evaluated quickly.
- Extend the decoder pipeline so the resource manifest can be consumed as a normal output artifact for the comparison workflow, not only as a synced bootstrap asset.

## Capabilities

### New Capabilities
- `decoder-cachelist-comparison`: Generate a candidate cache list from `main-decoder` output, compare it against the current bootstrap cache-list strategy, and surface actionable coverage/diff metrics without mutating checked-in assets.

### Modified Capabilities

## Non-goals

- Replacing the existing `cache make-list` CLI workflow.
- Changing gameplay logic, including traits such as `SortieOps`, `QuestOps`, or `MaterialOps`.
- Changing any KCSAPI route groups such as `api_req_sortie/`, `api_req_map/`, or `api_start2/`.
- Automatically proving decoder coverage is “correct”; the goal is fast comparison and iteration, not final policy decisions.

## Impact

- **Root examples**: new example under `examples/` for decoder-vs-bootstrap cache-list comparison.
- **emukc_bootstrap**: reusable helper(s) to build cache lists and compare path sets with an explicit manifest override.
- **main-decoder**: resource manifest output path becomes easier for external tooling/examples to consume.
- **Developer workflow**: decoder iterations gain a standard report instead of one-off diff scripts and manual asset swapping.
