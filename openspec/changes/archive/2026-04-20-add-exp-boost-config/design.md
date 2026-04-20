## Context

Exp calculation in `emukc_gameplay` hardcodes CT flagship multiplier as integer 300 (used as `base_exp * ct_mult`). Two locations:
- `crates/emukc_gameplay/src/game/sortie_result.rs:112-116` — sortie ship exp
- `crates/emukc_gameplay/src/game/battle/practice.rs:513-517` — practice ship exp

Actual KanColle CT bonus is 5-20% based on CT level. The hardcoded 300x is a dev override. No configurable practice exp boost exists.

GameConfig lives in `crates/emukc_model/src/codex/game_config.rs` as a serde-serializable struct within the Codex system. Codex is read-only after load — config changes require restart.

## Goals / Non-Goals

**Goals:**
- Make CT flagship exp multiplier configurable via GameConfig (`ct_exp_boost`)
- Make practice exp multiplier configurable via GameConfig (`practice_exp_boost`)
- Replace hardcoded values in sortie_result.rs and practice.rs
- Two multipliers stack multiplicatively

**Non-Goals:**
- Level-dependent CT bonus (actual KanColle scales by CT level; we use a flat multiplier)
- Runtime config changes (Codex is read-only after load)
- Changes to admiral exp or expedition exp

## Decisions

### 1. f64 multiplier fields on GameConfig

Add two `f64` fields with serde defaults. Value 1.0 = no change. `ct_exp_boost` replaces the `ct_mult` integer (300 → 1.0 default). `practice_exp_boost` is new (default 1.0).

**Alternative**: Keep ct_mult as percentage (300 = 300%). Rejected — f64 multiplier is consistent with existing `time_factor`/`cost_factor` pattern in `DockingConfig`.

### 2. Access GameConfig via Codex

Both `_impl` functions need `GameConfig` access. Codex is available via `HasContext`. Pass `game_config` reference into exp calculation functions rather than the raw `ct_flagship` bool.

### 3. Stacking order

Sortie: `final_exp = base_exp × ct_exp_boost × practice_exp_boost` (practice boost not applied in sortie)
Practice: `final_exp = base_exp × ct_exp_boost × practice_exp_boost`

### 4. Type change for ct_mult

Change `ct_mult` from `i64` (300) to `f64` (1.0). Adjust multiplication to produce `i64` result via `.floor()`. This is a behavior change — the default 1.0 produces same result as non-CT sortie (ct_mult=1).

## Risks / Trade-offs

- **Default value mismatch**: Default 1.0 means CT flagship gives no bonus unless config is changed. Intentional — actual KanColle CT bonus is small and level-dependent; flat multiplier can't replicate that. Users who want CT bonus can set e.g. 1.15 → Mitigation: document in config example.
- **Integer truncation**: f64 → i64 via `.floor()` may differ from current integer math. Negligible at these scales.
- **Backward compat**: New serde fields with `#[serde(default)]` — old config files load fine.
