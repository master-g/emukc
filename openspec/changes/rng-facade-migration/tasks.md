## 1. Facade Module Setup

- [ ] 1.1 Add `fastrand = "2"` to workspace `Cargo.toml` `[workspace.dependencies]`
- [ ] 1.2 Add `fastrand` dependency to `crates/emukc_crypto/Cargo.toml`
- [ ] 1.3 Create `crates/emukc_crypto/src/rng.rs` with full facade API:
  - `GameRng` struct: `seeded(u64)`, `i64(Range)`, `i64_inclusive(RangeInclusive)`, `usize(Range)`, `u32(Range)`, `f64()`, `f64_range(min, max)`, `shuffle(&mut [T])`, `choose(&[T])`, `choose_iter(I)`
  - Free functions: mirror of `GameRng` methods using thread-local state
  - Inclusive ranges use `saturating_add(1)` conversion internally
  - `f64_range` uses `min + f64() * (max - min)`
  - `choose_iter` uses index-based selection for `ExactSizeIterator`, reservoir fallback otherwise
- [ ] 1.4 Expose `rng` module in `crates/emukc_crypto/src/lib.rs`

## 2. Migrate Gameplay Crate

- [ ] 2.1 Migrate `crates/emukc_gameplay/src/game/battle/core.rs` — replace `Option<StdRng>` with `Option<GameRng>`, update all `BattleRandom` methods (4 call sites, uses `random_range` with i64 and u32→f64 conversion)
- [ ] 2.2 Migrate `crates/emukc_gameplay/src/game/map_route.rs` — replace `rand::{RngExt, rng}` with facade free functions (2 call sites, exclusive ranges)
- [ ] 2.3 Migrate `crates/emukc_gameplay/src/game/sortie.rs` — replace `rand::{RngExt, rng}` with facade free functions (1 call site, exclusive range)
- [ ] 2.4 Migrate `crates/emukc_gameplay/src/game/expedition.rs` — replace 5 `rng()` call sites with facade; includes inclusive ranges (`1..=5`, `1..=10`, `0..=max_count`, `1..=max_count`) and float range (`0.0..100.0`)
- [ ] 2.5 Migrate `crates/emukc_gameplay/src/game/use_item.rs` — replace `rand::{RngExt, rng, seq::IteratorRandom}` with facade; includes inclusive range (`0..=3`, `20..=31`) and `IteratorRandom::choose` call (`.iter().skip(1).choose()`)
- [ ] 2.6 Migrate `crates/emukc_gameplay/src/game/compose/powerup.rs` — replace `rand::{RngExt, rng}` with facade free functions
- [ ] 2.7 Migrate `crates/emukc_gameplay/src/game/compose/marriage.rs` — replace `rand::{RngExt, rng}` with facade; includes inclusive range (`3..=6`)
- [ ] 2.8 Migrate `crates/emukc_gameplay/src/game/practice.rs` — replace `rand::{RngExt, rng, seq::IndexedRandom}` with facade; includes inclusive ranges (`1..=10`, `1..=3`) and `IndexedRandom::choose` call
- [ ] 2.9 Migrate `crates/emukc_gameplay/src/game/sortie_result.rs` — replace `rand::{RngExt, rng}` with facade free functions
- [ ] 2.10 Remove `rand` from `crates/emukc_gameplay/Cargo.toml`

## 3. Migrate Cache Crate

- [ ] 3.1 Migrate `crates/emukc_cache/src/kache.rs` — replace `rand::{rng, seq::SliceRandom}` with `emukc_crypto::rng::shuffle` (1 call site)
- [ ] 3.2 Remove `rand` from `crates/emukc_cache/Cargo.toml`

## 4. Migrate Binary Crate

- [ ] 4.1 Migrate `src/bin/net/router/kcsapi/api_req_kousyou/createitem.rs` — replace `rand::{RngExt, rng, seq::IndexedRandom}` with facade; includes `choose(&mut r)` call and `random_range(0..100)`
- [ ] 4.2 Migrate `src/bin/net/router/kcsapi/api_req_kousyou/createship.rs` — replace `rand::{rng, seq::IndexedRandom}` with facade; includes `choose(&mut r)` call
- [ ] 4.3 Migrate `src/bin/cli/dev/nuke.rs` — replace `rand::{rng, seq::IndexedRandom}` with facade; includes `choose(&mut rng)` call

## 5. Cleanup and Verification

- [ ] 5.1 Remove `rand` from workspace `Cargo.toml` `[workspace.dependencies]` and binary crate `[dependencies]`
- [ ] 5.2 Verify `uuid` crate still compiles (transitive `rand` dep unaffected)
- [ ] 5.3 Run `cargo build` — verify clean compilation
- [ ] 5.4 Run `cargo test --workspace` — all tests pass
- [ ] 5.5 Run `cargo clippy --workspace` — no warnings
