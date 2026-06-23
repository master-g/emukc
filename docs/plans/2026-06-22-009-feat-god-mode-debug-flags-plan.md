---
title: "feat: God mode and one hit kill debug flags"
status: active
type: feat
created: 2026-06-22
sequence: 009
---

# feat: God mode and one hit kill debug flags

## Summary

Two boolean debug flags in `GameConfig` that short-circuit damage application in battle simulation. God mode makes friendly ships immune to damage; One hit kill destroys enemies in a single hit. Intended for development/debugging — trivialize battles to test post-battle flows.

## Requirements

- **R1**: God mode — `apply_damage` returns `(raw, 0)` for friendly ships when enabled
- **R2**: One hit kill — enemy HP set to 0 on any hit when enabled
- **R3**: Both default `false`, loaded via codex game config (`game_config.json`), backward compatible with existing files without the new fields
- Origin: `docs/brainstorms/2026-06-22-god-mode-debug-flags-requirements.md`

---

## Implementation Units

### U1. Add debug fields to GameConfig

**Goal:** Add `god_mode: bool` and `one_hit_kill: bool` to `GameConfig` with serde defaults.

**Files:**

- `crates/emukc_model/src/codex/game_config.rs`

**Approach:** Add two fields with `#[serde(default)]` to `GameConfig`. Both default to `false`.

**Test scenarios:**

1. `GameConfig::default()` has both fields as `false`
2. Deserializing JSON without the new fields succeeds with defaults

**Verification:** `cargo build -p emukc_model` compiles; existing `game_config.json` deserializes.

---

### U2. Add fields to BattleRuntimeShip and propagate in BattleContext

**Goal:** Add the two flags to `BattleRuntimeShip` so `apply_damage` can check them.

**Dependencies:** U1

**Files:**

- `crates/emukc_battle/src/types/runtime.rs` — add fields to `BattleRuntimeShip` and `BattleContext`; derive `Default` for `BattleContext`
- `crates/emukc_battle/src/state.rs` — propagate flags from `BattleContext` into ships in `from_context`
- ~22 test sites with `BattleContext { … }` literals (`simulation/mod.rs`, `simulation/shelling.rs`, `simulation/asw.rs`, `simulation/torpedo.rs`) — mechanical: append `..Default::default()`

**Approach:** Add `god_mode: bool` and `one_hit_kill: bool` to both structs. Derive `Default` for `BattleContext` so existing struct literals can use `..Default::default()` without listing the new fields. Default `false` in `BattleRuntimeShip::new()`. `from_context` copies from context to each ship. The ~28 existing `BattleContext { … }` literals across battle and gameplay test modules need `..Default::default()` appended — mechanical fallout from adding fields to a struct that currently has no `Default`.

**Test scenarios:**

1. `BattleRuntimeShip::new` defaults both fields to `false`
2. `from_context` copies flags from `BattleContext` to ships

**Verification:** `cargo build -p emukc_battle` compiles; existing battle tests pass with `..Default::default()` additions only (no assertion or behavior changes).

---

### U3. Short-circuit damage in apply_damage

**Goal:** Implement the actual debug behavior.

**Dependencies:** U2

**Files:**

- `crates/emukc_battle/src/types/runtime.rs` — `apply_damage` method
- `crates/emukc_battle/src/types/mod.rs` — tests

**Approach:** After the `is_sunk` guard at the top of `apply_damage`:

- `if self.is_friendly && self.god_mode`: return `(raw_damage, 0)`
- `if !self.is_friendly && self.one_hit_kill`: set `current_hp = 0`, return `(raw_damage, raw_damage)`

**Test scenarios:**

1. God mode: friendly ship with `god_mode=true`, `apply_damage(999)` → HP unchanged, effective=0
2. One hit kill: enemy ship with `one_hit_kill=true`, `apply_damage(1)` → HP=0
3. Both disabled (default): existing behavior unchanged
4. God mode does not affect enemy ships
5. One hit kill does not affect friendly ships

**Verification:** `cargo test -p emukc_battle` green.

---

### U4. Wire flags from codex into battle call sites

**Goal:** Pass `codex.game_cfg.god_mode` and `codex.game_cfg.one_hit_kill` into `BattleContext` at all construction sites.

**Dependencies:** U1, U2, U3

**Files:**

- `crates/emukc_gameplay/src/game/sortie/mod.rs` — sortie BattleContext construction
- `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs` — practice BattleContext
- `crates/emukc_gameplay/src/game/battle/sortie/orchestrate.rs` — sp_midnight path (manually constructed ships)
- `crates/emukc_gameplay/src/game/sortie_tests.rs` — test BattleContext constructions (3 sites, use `..Default::default()`)

**Approach:** Read flags from `codex.game_cfg` and set on `BattleContext`. For sp_midnight path, set fields on manually-created ships before passing to `NightBattleInput`.

**Test scenarios:**

1. With `god_mode=true` in config, sortie battle leaves friendly HP unchanged
2. With `one_hit_kill=true` in config, enemy ships sink on first hit
3. With both `false` (default), battle behavior unchanged

**Verification:** `cargo test -p emukc_gameplay` green. Manual test: set flag in config, run battle sim, verify behavior.

---

## Scope Boundaries

### In scope

- Two GameConfig fields + apply_damage short-circuit + call-site wiring

### Out of scope

- UI toggle or CLI flag (config file only)
- Per-ship or per-fleet overrides (global only)

### Deferred to follow-up work

- CLI flag like `--god-mode` for quick toggling without editing config
