## 1. Night battle sinking protection fix

- [x] 1.1 Edit `crates/emukc_battle/src/types.rs`: add `pub is_sortie: bool` field to `NightBattleInput` struct
- [x] 1.2 Edit `crates/emukc_battle/src/simulation/mod.rs`: in `simulate_night`, use `input.is_sortie` when constructing `BattleRuntimeShip` instances instead of hardcoding `false`
- [x] 1.3 Edit `crates/emukc_gameplay/src/game/battle/sortie/orchestrate.rs`: set `is_sortie: true` in `NightBattleInput` construction
- [x] 1.4 Edit `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs`: set `is_sortie: false` in `NightBattleInput` construction
- [x] 1.5 Update existing night battle tests in `crates/emukc_battle/src/simulation/night.rs` to supply `is_sortie` field
- [x] 1.6 Add regression test: sortie night battle with non-taiha ship survives lethal damage
- [x] 1.7 Add regression test: practice night battle with non-taiha ship is sunk by lethal damage

## 2. Remove dead BattleState.is_sortie field

- [x] 2.1 Edit `crates/emukc_battle/src/state.rs`: remove `is_sortie` field from `BattleState` struct and the `#[allow(dead_code)]` annotation
- [x] 2.2 Edit `crates/emukc_battle/src/state.rs`: in `from_context`, remove `let is_sortie = context.is_sortie;` line (the value is already passed to `BattleRuntimeShip::new` via the iterator)
- [x] 2.3 Edit `crates/emukc_battle/src/simulation/mod.rs`: remove `is_sortie` from the manual `BattleState` construction in `simulate_night`
- [x] 2.4 Run `cargo check --workspace` to verify no references remain

## 3. Formation modifier deduplication

- [x] 3.1 Edit `crates/emukc_battle/src/damage.rs`: rename `shelling_formation_modifier` to `formation_modifier`
- [x] 3.2 Edit `crates/emukc_battle/src/damage.rs`: delete `torpedo_formation_modifier` function
- [x] 3.3 Edit `crates/emukc_battle/src/damage.rs`: update `calculate_torpedo_damage` to call `formation_modifier`
- [x] 3.4 Run `cargo check -p emukc_battle` to verify

## 4. Documentation

- [x] 4.1 Edit `crates/emukc_battle/src/simulation/mod.rs`: add doc comment to `simulate_day` explaining RNG cross-phase continuity
- [x] 4.2 Edit `crates/emukc_battle/src/simulation/kouku.rs`: add `// NOTE:` comment at Stage2 explaining the linear AA approximation

## 5. Verification

- [x] 5.1 Run `cargo fmt --all`
- [x] 5.2 Run `cargo clippy --workspace` clean
- [x] 5.3 Run `cargo test -p emukc_battle` clean
- [x] 5.4 Run `cargo test -p emukc_gameplay` clean
