# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Battle engine overhaul**: phase-aware day battle simulation with `BattleType` enum
  - `Normal`, `AirBattle`, `LdAirBattle`, `LdShooting` modes controlling which phases run
  - 3-stage air battle (fighter power → stage 1 shootdown → stage 2 AA → airstrike damage)
  - `AirState` enum with proper threshold calculation (supremacy/superiority/parity/denial/incapability)
  - Full ASW damage formula with formation modifiers, synergy bonuses, and depth charge type detection
  - OASW (opening anti-submarine) phase triggered by sonar + sufficient ASW stat
  - `midnight_flag` now battle-type aware (LdAirBattle/LdShooting disallow midnight follow-up)
- **Sortie battle mode split**: each battle mode dispatches through `sortie_battle_impl()` free function
  - `sortie_airbattle()`, `sortie_ld_airbattle()`, `sortie_ld_shooting()`
  - `sortie_sp_midnight_battle()` as independent night-start flow with formation parameter
  - Router handlers: `/ld_airbattle`, `/ld_shooting` endpoints; `/sp_midnight` accepts `api_formation`
- **Enemy data pipeline**: kcwiki parser with nullable stat fields and `BoolOrInt::Float` variant
  - Three-tier enemy stat fallback: enemy_ship_extra → ship_extra → manifest-only
- **Map system**: 7-3 post-clear overlay and `choose_clear_transition_subset_match()` for phase disambiguation
- **Battle fidelity fixes**: 梯形 ASW formation modifier, airstrike damage attribution via `best_bomber_index()`
- **Route-level tests**: handler tests covering airbattle, ld_airbattle, ld_shooting, sp_midnight flows
- **Map periphery (non-battle nodes)**: resource acquisition and maelstrom (渦潮) effects at non-battle cells
  - `KcApiMapItemGet` / `KcApiMapHappening` API model types
  - `SortieItemGet` / `SortieHappening` gameplay response types with projection layer
  - Resource gain is a placeholder amount from fleet size and cell number; maelstrom loss with radar reduction (type3=12/13/93 detection via DB)
- **Battle damage persistence**: ship HP now updated in DB after battle result, enabling multi-node sortie damage carry-over
- **Sortie resource consumption**: ships pay fuel and ammo (from manifest max) per battle node

### Changed

- **Breaking**: `ExpConfig::default().ct_exp_boost` reverted from `250.0` to `1.0` (stock KanColle behavior). Users who relied on the 250× CT flagship XP boost must set `ct_exp_boost = 250.0` explicitly under `[exp]` in `emukc.config.toml`.
