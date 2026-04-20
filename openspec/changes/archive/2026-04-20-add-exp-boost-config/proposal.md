## Why

Sortie and practice exp calculations hardcode CT flagship multiplier as 300 (300x) and have no configurable practice exp boost. Making both configurable via GameConfig enables tuning without code changes, and aligns default values with actual KanColle mechanics (~1.15x for high-level CT flagship, not 300x).

## What Changes

- Add `ct_exp_boost: f64` field to `GameConfig` — controls exp multiplier applied when a training cruiser (CT) is flagship during sortie. Default: `1.0` (no extra boost; actual KanColle CT bonus is 5-20% depending on CT level, but the current flat multiplier approach averages ~1.0).
- Add `practice_exp_boost: f64` field to `GameConfig` — controls additional exp multiplier for practice (演習). Default: `1.0`.
- Replace hardcoded `ct_mult = 300` in `sortie_result.rs` and `battle/practice.rs` with configurable value from codex.
- Two multipliers stack: sortie exp = base × ct_exp_boost × practice_exp_boost (when applicable).

## Capabilities

### New Capabilities
- `exp-boost-config`: Configurable CT flagship and practice exp multipliers in GameConfig, applied in sortie and practice exp calculations.

### Modified Capabilities

## Impact

- `crates/emukc_model/src/codex/game_config.rs` — new fields on `GameConfig`
- `crates/emukc_gameplay/src/game/sortie_result.rs` — replace hardcoded CT multiplier
- `crates/emukc_gameplay/src/game/battle/practice.rs` — replace hardcoded CT multiplier, add practice boost
- Codex serialization format — new fields with defaults (backward compatible via serde defaults)
