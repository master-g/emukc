# Plan

## Current Status

- `tsunkit` map graph and node enemy composition parsing has been moved into `emukc_bootstrap`.
- Runtime no longer falls back to `temp/tsunkit_nav` or `temp/kc_data`; `emukcd` now consumes codex artifacts only.
- `tsunkit` is currently treated as a bootstrap-time source, not a runtime source.

## Constraints

- Keep source-specific parsing in `emukc_bootstrap`; do not reintroduce `tsunkit` raw formats into `emukc_model`.
- Keep runtime isolated from non-official external sources.
- Preserve `kc_data` as a fallback map source when `tsunkit_nav` cache is unavailable.

## Next Session

1. Add a bootstrap-stage `tsunkit_nav` fetch/cache workflow that writes local artifacts under a dedicated temp directory.
2. Fetch and cache three artifact groups: `maps`, `nodesummary`, and per-node `enemycomps`.
3. Design the fetch strategy around slow `enemycomps` endpoints:
   - conservative concurrency
   - retry/timeout policy
   - resumable or incremental execution
   - normal-world-first scope before event maps
4. Wire the new fetch workflow into the existing bootstrap command so it can produce `map_catalog.json` without manual cache preparation.
5. Verify end-to-end that a fresh bootstrap produces a codex with map graph plus enemy fleets, and that server startup works with only codex artifacts present.

## Follow-up

- Re-evaluate whether `kc_data` should remain as a long-term fallback after `tsunkit_nav` bootstrap becomes stable.
- Consider adding fixture coverage for a branching map and a multi-node event map after the downloader lands.
