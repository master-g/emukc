## Why

The decoder-driven `Rules` cache list strategy achieves 100% baseline coverage (69,229/69,229 paths, zero misses) against the current `Default` hardcoded strategy, while producing 250 additional valid paths. The `Rules` strategy is already fully implemented and battle-tested via comparison tooling. It is time to switch the default so all users benefit from decoder-driven path generation without opting in.

## What Changes

- **BREAKING**: `CacheListMakeStrategy::Default` will no longer use the legacy hardcoded path generation logic. Instead it will behave identically to `CacheListMakeStrategy::Rules`, loading `cache_rules.json` and the decoder rules bundle.
- The `Manifest` strategy remains available as an explicit opt-in for the v1 decoder integration path.
- Remove the legacy hardcoded `Default` code path from `source/mod.rs` and `source/kcs2/resources/mod.rs` (the `make` function that handles `Default`/`Greedy` without decoder assets).
- Retain `Greedy` as a standalone strategy that composes `Rules` + hole reporting, rather than composing `Default`.
- Update the CLI `serve` and `cache` commands to reflect the new default.
- Update the comparison example to use `Manifest` as the baseline instead of `Default`.

## Capabilities

### New Capabilities

- `rules-default-strategy`: Switches `CacheListMakeStrategy::Default` to use the decoder-driven rules bundle, removes legacy fallback code paths, and retains `Greedy` as a `Rules` + holes wrapper.

### Modified Capabilities

- `decoder-cachelist-comparison`: Baseline strategy in the comparison example changes from `Default` to `Manifest`, since `Default` will no longer represent the legacy path.

## Non-goals

- Removing the `Manifest` strategy — it remains useful for debugging.
- Eliminating all fallback-authored paths — analysis confirms the 5,825 residual paths (8.4%) are bounded by fundamental limitations:
  - **BGM battle IDs** (257 hardcoded): main.js selects battle BGM at runtime via `api_mst_mapbgm` lookup + hardcoded historical IDs. The decoder cannot statically extract these — they are not literal constants in the code. The `bgm.category` template already covers mapbgm-derived battle IDs via manifest, but the full 257-ID enumeration is inherently an empirical list.
  - **Gauge variants** (`_2`, `_3`, etc.) and image sidecars: gauge JSON files contain image filenames that cannot be known without downloading and parsing each JSON. Event multi-stage gauge variants require runtime probing. The `gauge.map` template already covers base gauge JSON paths from `api_mst_mapinfo`.
  - These fallback paths are correct residuals, not gaps to fix.
- Changing the `cache_rules.json` schema or main-decoder output format.
- Touching any KCSAPI handlers or gameplay traits.

## Impact

- `crates/emukc_bootstrap/src/make_list/` — primary impact zone
- `crates/emukc_bootstrap/src/make_list/source/mod.rs` — remove legacy `Default` branch
- `crates/emukc_bootstrap/src/make_list/source/kcs/` — simplify strategy dispatch
- `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/mod.rs` — remove legacy `make` path
- `examples/decoder_cachelist_compare.rs` — update baseline strategy
- CLI commands in `src/bin/cli/` that reference `Default` strategy
- Existing tests that assert `Default` behavior will need updating to match `Rules` output
