# Plan

## Current Status

- `tsunkit` map graph and node enemy composition parsing has been moved into `emukc_bootstrap`.
- Runtime no longer falls back to `temp/tsunkit_nav` or `temp/kc_data`; `emukcd` now consumes codex artifacts only.
- `tsunkit` is currently treated as a bootstrap-time source, not a runtime source.
- Single-fleet sortie flow now covers `api_req_map/start`, `api_req_map/next`, day battle, battle result, and standard night battle.
- Practice flow now covers day battle, result settlement, and night battle on the shared battle core.
- Sortie enemy selection now uses weighted node compositions instead of always picking the first catalog entry.
- Fallback enemy fleets are now an explicit degraded path only when codex enemy data is missing.
- Placeholder single-fleet routes exist for `api_req_sortie/airbattle`, `api_req_sortie/goback_port`, and `api_req_battle_midnight/sp_midnight` on top of the existing sortie state machine.
- Sortie battle result now emits quest events, so normal single-fleet sortie quests can advance from real battle settlement.
- Sortie quest matcher now understands map/boss/result conditions, including `All(map)` cycle reset for multi-round quests.
- Practice battle result now emits exercise quest events, including exercise quests that carry fleet composition requirements.

## Constraints

- Keep source-specific parsing in `emukc_bootstrap`; do not reintroduce `tsunkit` raw formats into `emukc_model`.
- Keep runtime isolated from non-official external sources.
- Preserve `kc_data` as a fallback map source when `tsunkit_nav` cache is unavailable.

## Next Session

1. Replace the `airbattle` and `sp_midnight` placeholder aliases with true single-fleet battle variants and battle-specific response shaping.
2. Define how `goback_port` should interact with partially resolved node state beyond the current runtime cleanup behavior.
3. Add fixtures for branching maps and multi-variant event maps so enemy fleet selection and route progression are tested against real catalog structure.
4. Start combined-fleet input modeling so `api_req_combined_battle/*` can reuse the existing battle core instead of forking a parallel implementation.
5. Revisit sortie quest edge cases around event-map `phase`, `Clear` semantics, and mixed-map repeat quests after the single-fleet battle variants are real.

## Follow-up

- Re-evaluate whether `kc_data` should remain as a long-term fallback after `tsunkit_nav` bootstrap becomes stable.
- Consider adding fixture coverage for a branching map and a multi-node event map after the downloader lands.
