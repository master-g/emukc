## 1. choose_index signature change

- [ ] 1.1 Edit `crates/emukc_battle/src/random.rs`: change `BattleRng::choose_index` signature from `fn choose_index(&mut self, len: usize) -> usize` to `fn choose_index(&mut self, len: usize) -> Option<usize>`. Remove `debug_assert!(len > 0)`. Make `len == 0` return `None`. Update doc comment to state empty input contract.
- [ ] 1.2 Update `SeededRng::choose_index` body in `crates/emukc_battle/src/random.rs` to return `Option<usize>` matching the new trait signature.
- [ ] 1.3 Update every callsite of `choose_index` in `crates/emukc_battle/`: callers that have already validated non-emptiness use `.expect("non-empty by construction")`; callers that genuinely have variable length propagate the `None` to the surrounding logic.
- [ ] 1.4 Add a `#[test]` in `crates/emukc_battle/src/random.rs` asserting `SeededRng::new(0).choose_index(0) == None` and asserting it does not consume entropy (subsequent `choose_index(1)` still returns the deterministic seeded value).
- [ ] 1.5 Run `cargo test -p emukc_battle` and confirm all tests pass.

## 2. roll_scratch_damage cleanup

- [ ] 2.1 Remove the `roll_scratch_damage` body from `CryptoRng` in `crates/emukc_gameplay/src/game/battle/rng.rs` (will be re-removed under its new name in step 4).
- [ ] 2.2 Confirm `BattleRng::roll_scratch_damage` trait default in `crates/emukc_battle/src/random.rs` is the single source of behavior.
- [ ] 2.3 Run `cargo test -p emukc_gameplay sortie_battle_response_passes_battle_rule_validation` and confirm scratch damage paths still pass.

## 3. PracticeRepository trait + GlobalPracticeStore

- [ ] 3.1 Edit `crates/emukc_gameplay/src/game/battle/repository.rs`: add `PracticeRepository` trait with `get_pending_practice`, `insert_pending_practice`, `take_pending_practice`. No `#[async_trait]`.
- [ ] 3.2 Add `GlobalPracticeStore` struct in `crates/emukc_gameplay/src/game/battle/practice/store.rs` (new file). Internally `Mutex<HashMap<i64, PracticeBattleSession>>`. Implement `PracticeRepository`.
- [ ] 3.3 Edit `crates/emukc_gameplay/src/game/context.rs` (or wherever `HasContext` lives): add required method `fn practice_store(&self) -> &dyn PracticeRepository`.
- [ ] 3.4 Provide a process-global `OnceLock<GlobalPracticeStore>` accessible via the production `HasContext` impl, mirroring how the sortie store is wired.
- [ ] 3.5 Update gameplay test fixture to provide a `TestPracticeStore` implementation.
- [ ] 3.6 Edit `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs`: replace every `super::PENDING_PRACTICE_BATTLES.lock().unwrap()` access with calls to `store.get_pending_practice(...)`, `store.insert_pending_practice(...)`, `store.take_pending_practice(...)`.
- [ ] 3.7 Delete the `PENDING_PRACTICE_BATTLES` static declaration. Confirm `grep -r "PENDING_PRACTICE_BATTLES" crates/` returns zero matches.
- [ ] 3.8 Run `cargo test -p emukc_gameplay` to verify practice tests pass against the new store.

## 4. Rename CryptoRng → ProductionRng

- [ ] 4.1 In `crates/emukc_gameplay/src/game/battle/rng.rs`, rename `pub struct CryptoRng;` to `pub struct ProductionRng;`. Update its `BattleRng` impl.
- [ ] 4.2 Update doc comment to read: `/// Non-cryptographic RNG backed by emukc_crypto::rng (fastrand). For deterministic test runs, use SeededRng from emukc_battle::test_utils.`
- [ ] 4.3 Search-and-replace `CryptoRng` → `ProductionRng` across `crates/emukc_gameplay/`, `crates/emukc_battle/`, and `src/bin/`. Confirm `grep -r "CryptoRng" crates/ src/` returns zero matches.
- [ ] 4.4 Run `cargo check --workspace` and `cargo clippy --workspace` clean.

## 5. RNG injection through orchestration

- [ ] 5.1 Edit `crates/emukc_gameplay/src/game/battle/sortie/orchestrate.rs`: add `rng: &mut dyn BattleRng` parameter (last positional) to `run_day_battle`, `run_night_battle`, `run_sp_midnight_battle`. Remove `let mut rng = ProductionRng;` from each fn body.
- [ ] 5.2 Edit `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs`: add `rng: &mut dyn BattleRng` parameter to `run_day_battle` and `run_night_battle` (practice variants). Remove internal RNG construction.
- [ ] 5.3 Update `SortieOps` trait blanket impls in the same crate: construct one `ProductionRng` per battle entry point, pass it through.
- [ ] 5.4 Update `PracticeOps` trait blanket impls similarly.
- [ ] 5.5 Update KCSAPI handlers in `src/bin/net/router/kcsapi/api_req_sortie/` and `src/bin/net/router/kcsapi/api_req_practice/` only if they call orchestration directly (they should go through the trait blanket impls, no change expected).
- [ ] 5.6 Update gameplay tests: where deterministic battle outcomes are required, construct `SeededRng::new(seed)` and pass it through the new orchestration parameter.
- [ ] 5.7 Run `cargo test --workspace`.

## 6. Practice night EngagementType decode

- [ ] 6.1 Edit `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs::run_night_battle`. Replace `EngagementType::from_api_id(session.formation[2]).unwrap_or(EngagementType::SameCourse)` with a `match` that on `None` calls `tracing::error!(profile_id, raw = session.formation[2], "practice night battle: corrupt engagement id")` and `return None;`.
- [ ] 6.2 Add a `#[test]` constructing a `PracticeBattleSession` with `formation[2]` set to an invalid value, invoking `run_night_battle`, and asserting the result is `None`.
- [ ] 6.3 Run `cargo test -p emukc_gameplay practice_night_battle`.

## 7. Verification

- [ ] 7.1 Run `cargo build --workspace` cleanly.
- [ ] 7.2 Run `cargo test --workspace`.
- [ ] 7.3 Run `cargo clippy --workspace -- -D warnings`.
- [ ] 7.4 Run `cargo fmt --all -- --check`.
- [ ] 7.5 Run `openspec validate harden-battle-refactor-followup --strict` clean.
