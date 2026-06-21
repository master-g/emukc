---
title: "refactor: Battle RNG and Practice Store Hardening — Finish the Half-Done Extraction"
type: refactor
date: 2026-06-22
origin: openspec/changes/harden-battle-refactor-followup/ (translated into ce-plan format per the openspec sunset migration)
---

# refactor: Battle RNG and Practice Store Hardening

## Summary

The `d4de3ff` battle-architecture extraction introduced `SortieRepository` and `BattleRng` traits, but a follow-up audit found the refactor stopped halfway. This plan closes the six audit findings the extraction left open: the practice battle path still bypasses the new dependency-injection seam; RNG injection is partial; `CryptoRng` is a misnomer; `choose_index` is unsafe on empty input; practice night battle swallows a decode failure; and `roll_scratch_damage` is duplicated dead code.

This is a **refactor hardening** — it does not change any battle simulation algorithm, any KCSAPI route, any DB schema, or any Codex data. It adds the missing injection seams so tests can assert deterministically and so the production RNG stops lying about its guarantees.

## Problem Frame

The extraction commit landed three independent improvements:

1. A new `emukc_battle` crate housing pure simulation code.
2. A `BattleRng` trait at the simulation entry points so callers supply randomness.
3. A `SortieRepository` trait so tests can inject isolated sortie session storage.

The follow-up audit shows three regressions/gaps in that refactor, plus three smaller defects:

- **Practice path was forgotten.** `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs` still relies on `super::PENDING_PRACTICE_BATTLES.lock().unwrap()`. This static `Mutex<HashMap<i64, PracticeBattleSession>>` survives across tests, leaks state, and bypasses the `HasContext` plumbing the sortie path now uses.
- **RNG injection only goes one layer deep.** `simulate_day(codex, ctx, rng)` accepts an `&mut impl BattleRng`, but `run_day_battle(store, codex, input)` constructs `let mut rng = CryptoRng;` itself. Tests can swap `SortieRepository` but cannot swap RNG; deterministic assertions on damage values require monkey-patching the orchestrate functions.
- **`CryptoRng` lies.** It claims platform entropy but delegates to `emukc_crypto::rng`, which wraps `fastrand` (xoshiro256++). For non-cryptographic RNG that is fine, but the name implies a security guarantee that is not delivered.

Three smaller defects:

- `BattleRng::choose_index` uses `debug_assert!(len > 0)` then divides by `len`. Release builds silently elide the assert and may panic on `% 0`, and the doc claims it returns 0 for empty input — the behavior is genuinely undefined.
- Practice night battle: `EngagementType::from_api_id(session.formation[2]).unwrap_or(EngagementType::SameCourse)` swallows storage corruption.
- `CryptoRng::roll_scratch_damage` duplicates the `BattleRng` trait default body.

## Requirements

- R1. Practice battle session state is reachable via `HasContext::practice_store() -> &dyn PracticeRepository`, mirroring the sortie path. The process-global `PENDING_PRACTICE_BATTLES` static is removed from public scope.
- R2. Both `simulate_*` and `run_*_battle` take a caller-supplied `&mut dyn BattleRng`. Orchestration never constructs RNG internally.
- R3. `CryptoRng` is renamed to `ProductionRng`; its doc comment states it is **non-cryptographic** and backed by `fastrand`.
- R4. `BattleRng::choose_index` has a total signature: returns `Option<usize>`. Empty input returns `None` without panicking in either debug or release, and without consuming randomness.
- R5. `EngagementType` decode failures in practice night battle log at error level and return `None` rather than silently mapping to `SameCourse`.
- R6. `roll_scratch_damage` is removed from concrete impls; only the trait default exists.

## Non-goals

- Replacing the `fastrand` backend with a CSPRNG. Battle determinism does not need cryptographic strength.
- Adding seeded battle replay to the running server. Seeding remains a test-only feature surfaced through `SeededRng`.
- Refactoring `SortieRepository` itself; the existing trait shape is preserved.
- Changing the `emukc_battle` simulation algorithms.
- No DB schema changes, no KCSAPI route changes, no Codex changes.

## Key Technical Decisions

### KTD1. Introduce `PracticeRepository`, do not extend `SortieRepository`

**Decision:** Introduce a separate `PracticeRepository` trait in `crates/emukc_gameplay/src/game/battle/repository.rs`. Practice and sortie battles have different lifecycles (practice has no map state, no result staging via the same shape) and different KCSAPI route ownership (`api_req_practice/` vs `api_req_sortie/`). Coupling them through `SortieRepository` would muddy the contract.

**Alternative considered:** extend `SortieRepository` with `get_pending_practice` / `take_pending_practice`. Rejected because the existing trait shape is already documented in the archived spec `sortie-repository`, and conflating practice into "sortie" breaks the domain analogy.

### KTD2. RNG parameter as last positional, not an `RngFactory` through `HasContext`

**Decision:** `run_day_battle`, `run_night_battle`, `run_sp_midnight_battle` (sortie) and `run_day_battle`, `run_night_battle` (practice) take `rng: &mut dyn BattleRng` as their final positional parameter. Trait methods on `SortieOps` / `PracticeOps` that call them construct `ProductionRng` once, then forward; gameplay tests construct `SeededRng` and pass it through.

**Alternative considered:** thread an `RngFactory` through `HasContext`. Rejected as over-engineered — RNG state is per-battle, not per-context, and `HasContext` is already heavy.

### KTD3. `choose_index` returns `Option<usize>`

**Decision:** change signature to `fn choose_index(&mut self, len: usize) -> Option<usize>`. Callers using a non-empty slice unwrap explicitly via `.expect("non-empty by construction")` or surface the `None` as a typed error. The trait doc documents the new contract.

**Alternative considered:** keep `usize` return and `panic!` on empty. Rejected — the existing comment promises silent zero, which is the worst of both worlds.

### KTD4. Rename `CryptoRng` → `ProductionRng`

**Decision:** rename the type and update the docstring to read "Non-cryptographic RNG backed by `emukc_crypto::rng` (fastrand). For deterministic test runs, use `SeededRng` from `emukc_battle::test_utils`." Cargo workspace-wide grep + `cargo check` covers the rename. Symbol is internal so we accept the breaking change without a deprecation alias.

**Alternative considered:** keep `CryptoRng`, only update docs. Rejected — the name actively misleads readers reviewing security-sensitive code paths.

### KTD5. Practice `EngagementType` decode surfaces as typed error + log

**Decision:** `EngagementType::from_api_id(stored)` becomes a typed `Result<EngagementType, BattleError>` at the orchestration boundary. The night battle returns `None` (already an `Option` return) when the stored formation is corrupt; we additionally log a `tracing::error!` so the corruption is visible.

## High-Level Technical Design

The six fixes are independent but share a common theme: lift construction out of orchestration and into the caller, so the dependency-injection seam the extraction started is completed end-to-end.

```
          ┌─────────────────────────────────────────────────┐
          │  HasContext                                      │
          │  ├─ sortie_store()  → &dyn SortieRepository      │ (existing)
          │  └─ practice_store() → &dyn PracticeRepository   │ (NEW — KTD1)
          └─────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┴──────────────────────┐
        ▼                                            ▼
  SortieOps blanket impl                      PracticeOps blanket impl
  constructs ProductionRng                    constructs ProductionRng
  forwards to run_*_battle(rng)              forwards to run_*_battle(rng)
        │                                            │
        ▼                                            ▼
  run_day_battle(store, codex, input, rng)   run_day_battle(store, ..., rng)
  run_night_battle(store, codex, input, rng) run_night_battle(store, ..., rng)
  run_sp_midnight_battle(store, ..., rng)
        │                                            │
        ▼                                            ▼
  simulate_day(codex, ctx, rng)               simulate_day(codex, ctx, rng)
  (no RNG construction inside)               (no RNG construction inside)
```

**Migration order:** the tasks are sequenced so each step compiles independently (see Implementation Units U1–U7). The `choose_index` signature change (U1) and `roll_scratch_damage` cleanup (U2) land first because they are leaf-level. The `PracticeRepository` extraction (U3) and the `CryptoRng` rename (U4) are mechanical. RNG injection (U5) depends on U4 (the renamed symbol). Practice night decode (U6) is independent. Rollback: each step is an isolated commit; revert in reverse order if needed.

## Behavioral notes

This change **does** carry spec deltas (3 capability modifications), unlike the migration plan's assumption of "no spec deltas." The behavioral contracts they define are:

- **`practice-battle-storage`** (NEW): `PracticeRepository` trait contract — synchronous, no `#[async_trait]`, atomic operations, `GlobalPracticeStore` for production, `TestPracticeStore` for tests, `HasContext::practice_store()` required with no default, `PENDING_PRACTICE_BATTLES` removed from public scope, practice night surfaces engagement decode failures.
- **`rng-facade`** (MODIFIED): `BattleRng::choose_index` returns `Option<usize>`; `ProductionRng` is non-cryptographic and replaces `CryptoRng`; `roll_scratch_damage` exists only as a trait default; no internal RNG construction in simulation.
- **`sortie`** (MODIFIED): orchestration entry points accept `rng: &mut dyn BattleRng`; `SortieOps` blanket impl constructs exactly one `ProductionRng` per battle entry point; tests inject `SeededRng` end-to-end.

These contracts are now captured (post-migration) in:

- `docs/solutions/architecture-patterns/rng-facade.md` — RNG facade contract.
- `docs/solutions/architecture-patterns/battle-crate-docs.md` — battle crate architecture.
- `docs/solutions/architecture-patterns/sortie.md` — sortie state machine and battle simulation contract.

## Implementation Units

### U1. `choose_index` signature change

- **Goal:** Make `BattleRng::choose_index` total — return `Option<usize>`, never panic on empty input.
- **Requirements:** R4.
- **Dependencies:** none (leaf-level).
- **Files:** `crates/emukc_battle/src/random.rs`.
- **Approach:** Change the trait method signature and update `SeededRng` + all callsites in one commit. CI fails fast if any callsite is missed.
- **Verification:** `cargo test -p emukc_battle` passes; the empty-input test asserts `None` without entropy consumption.

- [ ] 1.1 Edit `crates/emukc_battle/src/random.rs`: change `BattleRng::choose_index` signature from `fn choose_index(&mut self, len: usize) -> usize` to `fn choose_index(&mut self, len: usize) -> Option<usize>`. Remove `debug_assert!(len > 0)`. Make `len == 0` return `None`. Update doc comment to state empty input contract.
- [ ] 1.2 Update `SeededRng::choose_index` body in `crates/emukc_battle/src/random.rs` to return `Option<usize>` matching the new trait signature.
- [ ] 1.3 Update every callsite of `choose_index` in `crates/emukc_battle/`: callers that have already validated non-emptiness use `.expect("non-empty by construction")`; callers that genuinely have variable length propagate the `None` to the surrounding logic.
- [ ] 1.4 Add a `#[test]` in `crates/emukc_battle/src/random.rs` asserting `SeededRng::new(0).choose_index(0) == None` and asserting it does not consume entropy (subsequent `choose_index(1)` still returns the deterministic seeded value).
- [ ] 1.5 Run `cargo test -p emukc_battle` and confirm all tests pass.

### U2. `roll_scratch_damage` cleanup

- **Goal:** Remove the duplicated `roll_scratch_damage` body from `CryptoRng`; rely on the trait default only.
- **Requirements:** R6.
- **Dependencies:** none (leaf-level). Note: the body will be re-removed under the new name `ProductionRng` in U4.
- **Files:** `crates/emukc_gameplay/src/game/battle/rng.rs`, `crates/emukc_battle/src/random.rs`.
- **Verification:** scratch damage paths still pass `sortie_battle_response_passes_battle_rule_validation`.

- [ ] 2.1 Remove the `roll_scratch_damage` body from `CryptoRng` in `crates/emukc_gameplay/src/game/battle/rng.rs` (will be re-removed under its new name in step 4).
- [ ] 2.2 Confirm `BattleRng::roll_scratch_damage` trait default in `crates/emukc_battle/src/random.rs` is the single source of behavior.
- [ ] 2.3 Run `cargo test -p emukc_gameplay sortie_battle_response_passes_battle_rule_validation` and confirm scratch damage paths still pass.

### U3. `PracticeRepository` trait + `GlobalPracticeStore`

- **Goal:** Route practice battle session state through `HasContext::practice_store()`, mirroring the sortie path; remove the process-global static.
- **Requirements:** R1.
- **Dependencies:** none (can parallelize with U1, U2).
- **Files:** `crates/emukc_gameplay/src/game/battle/repository.rs` (new trait), `crates/emukc_gameplay/src/game/battle/practice/store.rs` (new file), `crates/emukc_gameplay/src/game/context.rs` (`HasContext`), `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs`, gameplay test fixtures.
- **Approach:** Keep `PENDING_PRACTICE_BATTLES` as a private detail of `GlobalPracticeStore` for one transitional commit, then remove the static in the same unit. Replace every `super::PENDING_PRACTICE_BATTLES.lock().unwrap()` access with store method calls.
- **Verification:** `cargo test -p emukc_gameplay` passes against the new store; `grep -r "PENDING_PRACTICE_BATTLES" crates/` returns zero matches.

- [ ] 3.1 Edit `crates/emukc_gameplay/src/game/battle/repository.rs`: add `PracticeRepository` trait with `get_pending_practice`, `insert_pending_practice`, `take_pending_practice`. No `#[async_trait]`.
- [ ] 3.2 Add `GlobalPracticeStore` struct in `crates/emukc_gameplay/src/game/battle/practice/store.rs` (new file). Internally `Mutex<HashMap<i64, PracticeBattleSession>>`. Implement `PracticeRepository`.
- [ ] 3.3 Edit `crates/emukc_gameplay/src/game/context.rs` (or wherever `HasContext` lives): add required method `fn practice_store(&self) -> &dyn PracticeRepository`.
- [ ] 3.4 Provide a process-global `OnceLock<GlobalPracticeStore>` accessible via the production `HasContext` impl, mirroring how the sortie store is wired.
- [ ] 3.5 Update gameplay test fixture to provide a `TestPracticeStore` implementation.
- [ ] 3.6 Edit `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs`: replace every `super::PENDING_PRACTICE_BATTLES.lock().unwrap()` access with calls to `store.get_pending_practice(...)`, `store.insert_pending_practice(...)`, `store.take_pending_practice(...)`.
- [ ] 3.7 Delete the `PENDING_PRACTICE_BATTLES` static declaration. Confirm `grep -r "PENDING_PRACTICE_BATTLES" crates/` returns zero matches.
- [ ] 3.8 Run `cargo test -p emukc_gameplay` to verify practice tests pass against the new store.

### U4. Rename `CryptoRng` → `ProductionRng`

- **Goal:** Drop the misleading `Crypto` prefix; the RNG is non-cryptographic (fastrand/xoshiro).
- **Requirements:** R3.
- **Dependencies:** U2 recommended (so the body removed in U2 stays removed under the new name).
- **Files:** `crates/emukc_gameplay/src/game/battle/rng.rs`, workspace-wide search-and-replace across `crates/emukc_gameplay/`, `crates/emukc_battle/`, `src/bin/`.
- **Verification:** `cargo check --workspace` and `cargo clippy --workspace` clean; `grep -r "CryptoRng" crates/ src/` returns zero matches.

- [ ] 4.1 In `crates/emukc_gameplay/src/game/battle/rng.rs`, rename `pub struct CryptoRng;` to `pub struct ProductionRng;`. Update its `BattleRng` impl.
- [ ] 4.2 Update doc comment to read: `/// Non-cryptographic RNG backed by emukc_crypto::rng (fastrand). For deterministic test runs, use SeededRng from emukc_battle::test_utils.`
- [ ] 4.3 Search-and-replace `CryptoRng` → `ProductionRng` across `crates/emukc_gameplay/`, `crates/emukc_battle/`, and `src/bin/`. Confirm `grep -r "CryptoRng" crates/ src/` returns zero matches.
- [ ] 4.4 Run `cargo check --workspace` and `cargo clippy --workspace` clean.

### U5. RNG injection through orchestration

- **Goal:** Lift RNG construction out of the orchestration entry points so callers (and tests) can supply their own RNG.
- **Requirements:** R2.
- **Dependencies:** U4 (uses the renamed `ProductionRng`).
- **Files:** `crates/emukc_gameplay/src/game/battle/sortie/orchestrate.rs`, `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs`, `SortieOps` / `PracticeOps` blanket impls, KCSAPI handlers (only if they call orchestration directly), gameplay tests.
- **Approach:** Add `rng: &mut dyn BattleRng` as the last positional parameter to every `run_*_battle` fn. Remove internal `let mut rng = ProductionRng;`. Update blanket impls to construct one `ProductionRng` per entry point and forward. Update tests to use `SeededRng` where deterministic assertions are wanted.
- **Verification:** `cargo test --workspace` passes; deterministic tests can now assert on specific damage/target-selection outcomes.

- [ ] 5.1 Edit `crates/emukc_gameplay/src/game/battle/sortie/orchestrate.rs`: add `rng: &mut dyn BattleRng` parameter (last positional) to `run_day_battle`, `run_night_battle`, `run_sp_midnight_battle`. Remove `let mut rng = ProductionRng;` from each fn body.
- [ ] 5.2 Edit `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs`: add `rng: &mut dyn BattleRng` parameter to `run_day_battle` and `run_night_battle` (practice variants). Remove internal RNG construction.
- [ ] 5.3 Update `SortieOps` trait blanket impls in the same crate: construct one `ProductionRng` per battle entry point, pass it through.
- [ ] 5.4 Update `PracticeOps` trait blanket impls similarly.
- [ ] 5.5 Update KCSAPI handlers in `src/bin/net/router/kcsapi/api_req_sortie/` and `src/bin/net/router/kcsapi/api_req_practice/` only if they call orchestration directly (they should go through the trait blanket impls, no change expected).
- [ ] 5.6 Update gameplay tests: where deterministic battle outcomes are required, construct `SeededRng::new(seed)` and pass it through the new orchestration parameter.
- [ ] 5.7 Run `cargo test --workspace`.

### U6. Practice night `EngagementType` decode

- **Goal:** Surface corrupt stored engagement data as a logged error + `None` return, rather than silently coercing to `SameCourse`.
- **Requirements:** R5.
- **Dependencies:** U3 (the practice orchestrate fn is already refactored to use the store by this point).
- **Files:** `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs`.
- **Verification:** `cargo test -p emukc_gameplay practice_night_battle` passes; the corrupt-formation test asserts `None`.

- [ ] 6.1 Edit `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs::run_night_battle`. Replace `EngagementType::from_api_id(session.formation[2]).unwrap_or(EngagementType::SameCourse)` with a `match` that on `None` calls `tracing::error!(profile_id, raw = session.formation[2], "practice night battle: corrupt engagement id")` and `return None;`.
- [ ] 6.2 Add a `#[test]` constructing a `PracticeBattleSession` with `formation[2]` set to an invalid value, invoking `run_night_battle`, and asserting the result is `None`.
- [ ] 6.3 Run `cargo test -p emukc_gameplay practice_night_battle`.

### U7. Verification

- **Goal:** Confirm the full workspace is healthy after all refactoring.
- **Requirements:** all.
- **Dependencies:** U1–U6 complete.
- **Verification:** all gates green.

- [ ] 7.1 Run `cargo build --workspace` cleanly.
- [ ] 7.2 Run `cargo test --workspace`.
- [ ] 7.3 Run `cargo clippy --workspace -- -D warnings`.
- [ ] 7.4 Run `cargo fmt --all -- --check`.
- [ ] 7.5 ~~Run `openspec validate harden-battle-refactor-followup --strict` clean.~~ *(Superseded: openspec is being sunset. The behavioral contracts this validation checked are now captured in `docs/solutions/architecture-patterns/rng-facade.md` and `battle-crate-docs.md`; verification is via `cargo test` + `cargo clippy`.)*

## Risks & Dependencies

- **Repository duplication (KTD1).** Introducing a parallel `PracticeRepository` trait costs ~30 lines of trait + impl, but the alternative (overloading `SortieRepository`) costs more in conceptual coupling and breaks the existing archived spec contract.
- **RNG signature churn (U5).** Every internal caller of `run_day_battle` / `run_night_battle` gains an `rng` parameter. *Mitigation:* a single PR that touches orchestrate fns + their tests + the two KCSAPI handlers; pre-merge `cargo check --workspace` confirms full coverage.
- **`choose_index` callsite churn (U1).** Roughly a dozen callsites. *Mitigation:* provide a `choose_index_unchecked` blanket helper for callers that have already validated non-emptiness, so the diff stays small.
- **Rename surfaces in archived spec (U4).** Archived spec `battle-rng-port` references `CryptoRng`. We do not edit archived specs; the rename is documented as an addendum in `rng-facade` modified delta (now in `docs/solutions/architecture-patterns/rng-facade.md`).
- **No DB schema changes, no KCSAPI route changes, no Codex changes.** This is pure internal refactoring.

## Sources / Research

- Audit origin: the `d4de3ff` commit ("refactor(battle): extract emukc_battle crate, introduce BattleRng + SortieRepository") and its follow-up audit.
- `crates/emukc_gameplay/src/game/battle/repository.rs` — `SortieRepository` trait to mirror for `PracticeRepository`.
- `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs` — `PENDING_PRACTICE_BATTLES` static and the `unwrap_or(SameCourse)` decode site.
- `crates/emukc_gameplay/src/game/battle/sortie/orchestrate.rs` — `run_*_battle` entry points that hardcode `CryptoRng`.
- `crates/emukc_gameplay/src/game/battle/rng.rs` — `CryptoRng` struct + duplicated `roll_scratch_damage`.
- `crates/emukc_gameplay/src/game/context.rs` — `HasContext` trait to extend with `practice_store()`.
- `crates/emukc_battle/src/random.rs` — `BattleRng` trait, `choose_index` signature, `SeededRng`, `roll_scratch_damage` default.
- `docs/solutions/architecture-patterns/rng-facade.md` — RNG facade contract (migrated from openspec).
- `docs/solutions/architecture-patterns/battle-crate-docs.md` — battle crate architecture (migrated from openspec).
