## Context

`emukc_battle` crate was extracted from `emukc_gameplay` in commit `d4de3ff`. The extraction preserved all code but the new crate has `missing_docs` warned workspace-wide. The crate has 156 clippy warnings — all in `emukc_battle`, zero in other crates.

Current state of affected files:
- `crates/emukc_battle/src/types.rs` — data structures and enums, no doc comments
- `crates/emukc_battle/src/lib.rs` — crate root, one doc backtick issue
- `crates/emukc_battle/src/simulation/mod.rs` — `simulate_night` has 8 params (limit: 7)
- `crates/emukc_battle/src/targeting.rs` — 4 unused constants + 3 unused functions
- `crates/emukc_battle/src/damage.rs` — 2 unused functions
- `crates/emukc_battle/src/state.rs` — 1 unused field (`is_sortie`)

## Goals / Non-Goals

**Goals:**
- Zero clippy warnings for `emukc_battle`
- Meaningful doc comments on non-trivial public items (enums, functions, methods)
- Preserve dead code for planned night battle / airstrike features
- Reduce `simulate_night` parameter count below threshold

**Non-Goals:**
- Implementing night battle features that would consume dead code
- Adding `#![deny(warnings)]` to the crate
- Refactoring battle types for ergonomics

## Decisions

### D1: `#[allow(missing_docs)]` on `types.rs` module, not crate-level

Apply `#[allow(missing_docs)]` at the `types` module level only. The 108 struct fields are self-documenting (e.g., `api_stage1`, `api_disp_seiku`). Adding trivial docs to each is noise.

**Alternative considered:** `#![allow(missing_docs)]` at crate root — too broad, would suppress docs on functions that genuinely need them.

### D2: Doc comments on public enums and functions

Add 1-liner `///` doc comments to: `BattleType`, `EngagementType`, `AirState`, `BattleOutcome` enums; `simulate_day`, `simulate_night`, `calculate_mvp`, `calculate_win_rank`, `apply_cap` functions. These describe semantics not obvious from names alone.

### D3: `#[allow(dead_code)]` with TODO comments for reserved code

Dead constants/functions in `targeting.rs` and `damage.rs` are night battle / airstrike helpers preserved for future implementation. Annotate with `#[allow(dead_code)]` and a `// TODO: used by night battle / airstrike` comment.

### D4: `NightBattleInput` parameter struct for `simulate_night`

Introduce a `NightBattleInput` struct bundling the 6 non-codex/non-rng parameters (`friendly`, `enemy`, `friendly_formation_id`, `enemy_formation_id`, `engagement`, `air_state`). Signature becomes `(codex, input, rng)` — 3 params.

**Alternative considered:** Builder pattern — overkill for a pure function entry point.

### D5: Fix doc backtick in `lib.rs`

One-line fix: wrap bare type name in backticks in the crate doc.

## Risks / Trade-offs

- **Risk:** `#[allow(missing_docs)]` on `types.rs` may hide genuinely confusing fields → Mitigation: the fields mirror KanColle API names; if one is confusing, a future PR can add a doc comment to that specific field.
- **Risk:** `NightBattleInput` struct adds a type callers must construct → Mitigation: `simulate_night` is only called from `emukc_gameplay`; single call site, minimal disruption.
