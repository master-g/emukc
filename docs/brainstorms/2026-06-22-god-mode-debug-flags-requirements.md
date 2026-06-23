# God Mode & One Hit Kill Debug Flags

## Summary

Two boolean debug flags in `GameConfig` that short-circuit damage application in battle simulation:

- **God mode**: friendly ships take zero damage
- **One hit kill**: enemy ships are destroyed in a single hit

Intended for development/debugging only — lets the developer quickly trivialize battles to test post-battle flows (quest progression, drops, results screen) without spending time on fleet composition or RNG.

## Requirements

### R1: God mode

- When enabled, `BattleRuntimeShip::apply_damage` returns `(raw_damage, 0)` for all friendly ships — no HP subtracted, no sinking protection logic triggered.
- Does not affect enemy ships.

### R2: One hit kill

- When enabled, enemy ships hit by any attack have their HP set to 0 (sunk).
- Does not affect friendly ships.

### R3: Configuration

- Both flags default to `false`.
- Configured via `emukc.config.toml` under the existing game config section.
- Loaded at startup through the normal codex/config pipeline.

## Success Criteria

- With god_mode=true, a solo flagship can clear any map without losing HP.
- With one_hit_kill=true, every enemy ship sinks on the first hit.
- With both enabled, battles are trivially winnable for testing post-battle flows.
- With both disabled (default), battle behavior is unchanged.

## Scope Boundaries

### In scope

- Two new fields on `GameConfig`
- Damage short-circuit logic in `BattleRuntimeShip::apply_damage`
- Config plumbing from `emukc.config.toml` to battle runtime

### Out of scope

- UI toggle or CLI flag (config file only for now)
- Multiplayer balancing (single-player emulator)
- Per-ship or per-fleet overrides (global only)
