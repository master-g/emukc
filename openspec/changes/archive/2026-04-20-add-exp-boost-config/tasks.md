## 1. Model — GameConfig fields

- [x] 1.1 Add `ExpConfig` struct in `crates/emukc_model/src/codex/game_config.rs` with `ct_exp_boost: f64` (default 1.0) and `practice_exp_boost: f64` (default 1.0), using serde defaults
- [x] 1.2 Add `exp: ExpConfig` field to `GameConfig` struct with serde default

## 2. Gameplay — Sortie exp calculation

- [x] 2.1 Update `calculate_sortie_ship_exp` in `crates/emukc_gameplay/src/game/sortie_result.rs` to accept `ct_exp_boost: f64` parameter instead of using hardcoded 300; change `ct_mult` from `i64` to `f64`, apply via `(base_exp as f64 * ct_mult).floor() as i64`
- [x] 2.2 Update caller(s) of `calculate_sortie_ship_exp` to pass `game_config.exp.ct_exp_boost` from Codex

## 3. Gameplay — Practice exp calculation

- [x] 3.1 Update `calculate_practice_ship_exp` in `crates/emukc_gameplay/src/game/battle/practice.rs` to accept `ct_exp_boost: f64` and `practice_exp_boost: f64` parameters; replace hardcoded 300 with configurable `ct_exp_boost`; apply `practice_exp_boost` multiplicatively
- [x] 3.2 Update caller(s) of `calculate_practice_ship_exp` to pass both boost values from Codex GameConfig

## 4. Verification

- [x] 4.1 `cargo build` passes
- [x] 4.2 `cargo test -p emukc_gameplay` passes
- [x] 4.3 `cargo clippy --workspace` passes
