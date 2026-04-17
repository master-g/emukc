## 1. Fix resolve_damage zero-power guard

- [x] 1.1 Add `if capped_power <= 0.0 { return 0; }` guard at top of `resolve_damage()` in `crates/emukc_gameplay/src/game/battle/core.rs`

## 2. Tests

- [x] 2.1 Add test: taiha torpedo (HP ≤25%) deals 0 damage, not scratch damage
- [x] 2.2 Run `cargo test -p emukc_gameplay` — all pass
