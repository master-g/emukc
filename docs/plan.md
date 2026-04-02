# Plan

## Current Status

- `wikiwiki` map extraction and routing parsing now live in `emukc_bootstrap`.
- Runtime map loading still does `repo wikiwiki catalog + kc_data structural complement`; it is not codex-only yet.
- Single-fleet sortie flow now covers `api_req_map/start`, `api_req_map/next`, day battle, battle result, and standard night battle.
- Practice flow now covers day battle, result settlement, and night battle on the shared battle core.
- Sortie enemy selection now uses weighted node compositions instead of always picking the first catalog entry.
- Fallback enemy fleets are now an explicit degraded path only when codex enemy data is missing.
- Placeholder single-fleet routes exist for `api_req_sortie/airbattle`, `api_req_sortie/goback_port`, and `api_req_battle_midnight/sp_midnight` on top of the existing sortie state machine.
- Sortie battle result now emits quest events, so normal single-fleet sortie quests can advance from real battle settlement.
- Sortie quest matcher now understands map/boss/result conditions, including `All(map)` cycle reset for multi-round quests.
- Practice battle result now emits exercise quest events, including exercise quests that carry fleet composition requirements.
- `kc_data` route-only numeric placeholder nodes no longer generate fake runtime cells, and `1-1 map/start` now correctly stays at four cells.
- Remaining map work is no longer in the wikiwiki route parser: repo asset route rules are now at `0` `Unknown`, `0` `SourceUnknown`, and `0` `parse_warnings`.
- Remaining battle blocker is data-source, not formula: many early abyssal IDs still have no usable HP/armor/firepower source, so sortie enemy fallback builds `HP=1` enemies.

## Constraints

- Keep source-specific parsing in `emukc_bootstrap`; do not reintroduce raw external HTML formats into `emukc_model`.
- Keep runtime isolated from non-official external sources.
- Treat `wikiwiki` as the primary offline semantic source for maps.
- Treat `kc_data` only as a structural complement source, with `kc_data-only` reserved for explicit degraded mode when the repo-tracked wikiwiki asset is unavailable.

## Next Session

1. Keep regression fixtures for the real-page text forms now covered: footnote anchors, residual helper headers, fullwidth-indent probability annotations, and previously source-unknown route lines.
2. Introduce an enemy master/stat data source into codex/bootstrap and switch `build_sortie_enemy_ship()` to use it before manifest fallback.
3. Only after enemy stats are stable, continue with `airbattle` / `sp_midnight` specialization and broader sortie battle fidelity work.

## Follow-up

- Add explicit fixture coverage for a map where wikiwiki semantics and `kc_data` structure are both required, so the complement boundary stays regression-tested.
- Do not introduce an AST runtime unless a concrete wikiwiki rule family can be parsed reliably but cannot be compiled into flat `RouteRule`.
- Consider whether the repo-tracked wikiwiki asset should eventually move out of `emukc_bootstrap/assets` into a more model-centric generated-data location.
- Revisit whether runtime should keep merging `kc_data` on startup, or whether that merge should move entirely into offline codex generation once the complement path is stable enough.
