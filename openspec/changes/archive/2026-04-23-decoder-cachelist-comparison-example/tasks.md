## 1. Rust Reusable Helpers

- [x] 1.1 Refactor `crates/emukc_bootstrap/src/make_list/` so cache-list generation can build an in-memory path set or item list without requiring the file-writing wrapper
- [x] 1.2 Add a reusable manifest-loading override path so callers can generate a candidate list from an explicit decoder manifest file instead of only the repo-tracked asset
- [x] 1.3 Add comparison/report helper types that compute overlap, only-baseline, only-candidate, and grouped path-prefix deltas from two generated lists

## 2. Comparison Example

- [x] 2.1 Add a new example under `examples/` that accepts a decoder manifest path, baseline strategy selection, and output/report options
- [x] 2.2 Make the example generate both the baseline bootstrap cache list and the decoder-driven candidate list in one run using the shared Rust helpers
- [x] 2.3 Make the example emit a human-readable summary plus a structured report artifact suitable for further analysis

## 3. Decoder Output Plumbing

- [x] 3.1 Update `main-decoder/src/pipeline.ts` and related types so the resource manifest is written as a normal output artifact in `out/resources/`
- [x] 3.2 Keep existing optional sync-to-bootstrap behavior while ensuring the comparison example can consume decoder output directly
- [x] 3.3 Update decoder docs/tests for the manifest output location expected by the comparison workflow

## 4. Verification

- [x] 4.1 Add Rust tests covering manifest override generation and comparison report calculations
- [x] 4.2 Add decoder-side tests covering resource manifest output artifact generation
- [x] 4.3 Run `cd main-decoder && bun test`, `cargo test -p emukc_bootstrap`, and at least one real example comparison run against decoder output
