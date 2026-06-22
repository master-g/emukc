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

## Reconciliation (2026-06-22)

Read-only audit of the plan against the current codebase. The codebase
advanced past the plan during its openspec incubation period.

| Unit | Scope | Done | Remaining | Verdict |
| --- | --- | --- | --- | --- |
| U1 | `choose_index` → `Option<usize>` | 5/5 | 0 | ✅ DONE — trait default + `ProductionRng` override return `Option<usize>`, `len==0`→`None` w/o entropy; 5 callsites `.expect()`; empty-input-no-entropy test added |
| U2 | `roll_scratch_damage` dedup | 3/3 | 0 | **DONE** — override deleted from CryptoRng, trait default is sole source, 3 regression tests pin formula + draw count |
| U3 | PracticeRepository + store | 8/8 | 0 | **DONE** — trait in `practice_repository.rs`, `PracticeStore` in `sortie_store.rs`, `HasContext::practice_store()` wired, `PENDING_PRACTICE_BATTLES` removed (grep returns zero) |
| U4 | CryptoRng → ProductionRng | 4/4 | 0 | ✅ DONE — struct + impl + doc + 15 usages renamed; doc comment corrected to real `SeededRng` path (`emukc_battle::random`, not the non-existent `test_utils`) |
| U5 | RNG injection through orchestrate | 7/7 | 0 | ✅ DONE — `rng: &mut impl BattleRng` injected into all 5 orchestrate entry points; 10 callsites updated (5 trait impls + 5 tests) |
| U6 | EngagementType decode surfacing | 3/3 | 0 | ✅ DONE — `.unwrap_or(...)` replaced with `match` + `tracing::error!` + `return None`; session re-inserted before early return; corrupt-formation test added |
| U7 | Verification sweep | 0/5 | 5 | N/A until U1/U4/U5/U6 land |

**Totals:** 30 done, 5 remaining. The 5 remaining (U7) are pure
verification runs; all implementation work (U1–U6) is complete after
U6 (3 tasks) shipped 2026-06-22.

**Naming divergences from plan (U3 — already shipped, documenting for accuracy):**

- Plan names the store `GlobalPracticeStore` in `battle/practice/store.rs`; code
  names it `PracticeStore` in `game/sortie_store.rs` (co-located with
  `SortieStore`).
- Plan names methods `get_pending_practice` / `insert_pending_practice` /
  `take_pending_practice`; code uses `get_pending_battle` / `insert_pending_battle`
  / `take_pending_battle` (also adds result-snapshot methods).
- Plan specifies `HasContext::practice_store() -> &dyn PracticeRepository`;
  code returns `&PracticeStore` (concrete type, not `dyn`).

These divergences are benign — the dependency-injection seam exists and works.
The remaining work (U1, U4, U5, U6) should adopt the code's existing naming.

**Recommended execution order after reconciliation:** U4 (rename, mechanical) →
U1 (choose_index, leaf-level) → U5 (RNG injection, depends on U4) → U6
(EngagementType, independent). Then U7.

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

- [x] 1.1 Edit `crates/emukc_battle/src/random.rs`: change `BattleRng::choose_index` signature from `fn choose_index(&mut self, len: usize) -> usize` to `fn choose_index(&mut self, len: usize) -> Option<usize>`. Remove `debug_assert!(len > 0)`. Make `len == 0` return `None`. Update doc comment to state empty input contract.
- [x] 1.2 Update `ProductionRng::choose_index` override in `crates/emukc_gameplay/src/game/battle/rng.rs` to return `Option<usize>` with a `len == 0` guard (prevents `emukc_crypto::rng::usize(0..0)` from hitting an empty range). Generator-consumption for non-empty input preserved (still routes through `emukc_crypto::rng::usize`, not the trait default's `i64` path). `SeededRng` does not override `choose_index` — it uses the trait default, so it auto-inherited the new signature.
- [x] 1.3 Update every callsite of `choose_index` (4 sites in `emukc_battle`: `kouku.rs:274`, `kouku.rs:320`, `targeting.rs:115`, `targeting.rs:134`; 1 in gameplay `rng.rs:37` test helper). All 5 are guaranteed non-empty by an immediately preceding `is_empty()` guard → used `.expect("... non-empty by construction")`. No site needed to propagate `None`.
- [x] 1.4 Added `choose_index_empty_returns_none_without_drawing` test: asserts `choose_index(0) == None` and that a subsequent `choose_index(1)` matches a fresh RNG's first draw (entropy untouched).
- [x] 1.5 `cargo test -p emukc_battle` — 197 passed, 0 failed (incl. new test). `cargo check/clippy --workspace` clean, `cargo fmt --all --check` clean.

### U2. `roll_scratch_damage` cleanup

- **Goal:** Remove the duplicated `roll_scratch_damage` body from `CryptoRng`; rely on the trait default only.
- **Requirements:** R6.
- **Dependencies:** none (leaf-level). Note: the body will be re-removed under the new name `ProductionRng` in U4.
- **Files:** `crates/emukc_gameplay/src/game/battle/rng.rs`, `crates/emukc_battle/src/random.rs`.
- **Verification:** scratch damage paths still pass `sortie_battle_response_passes_battle_rule_validation`.

- [x] 2.1 Remove the `roll_scratch_damage` body from `CryptoRng` in `crates/emukc_gameplay/src/game/battle/rng.rs` (will be re-removed under its new name in step 4). *(Done — the override was deleted; `rng.rs:18` comment documents it.)*
- [x] 2.2 Confirm `BattleRng::roll_scratch_damage` trait default in `crates/emukc_battle/src/random.rs` is the single source of behavior. *(Done — trait default at `random.rs:13` is the sole definition; `CryptoRng` impl has no override.)*
- [x] 2.3 Run `cargo test -p emukc_gameplay sortie_battle_response_passes_battle_rule_validation` and confirm scratch damage paths still pass. *(Done — 3 regression tests in `random.rs` pin the formula, draw count, and golden vector.)*

### U3. `PracticeRepository` trait + `GlobalPracticeStore`

- **Goal:** Route practice battle session state through `HasContext::practice_store()`, mirroring the sortie path; remove the process-global static.
- **Requirements:** R1.
- **Dependencies:** none (can parallelize with U1, U2).
- **Files:** `crates/emukc_gameplay/src/game/battle/repository.rs` (new trait), `crates/emukc_gameplay/src/game/battle/practice/store.rs` (new file), `crates/emukc_gameplay/src/game/context.rs` (`HasContext`), `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs`, gameplay test fixtures.
- **Approach:** Keep `PENDING_PRACTICE_BATTLES` as a private detail of `GlobalPracticeStore` for one transitional commit, then remove the static in the same unit. Replace every `super::PENDING_PRACTICE_BATTLES.lock().unwrap()` access with store method calls.
- **Verification:** `cargo test -p emukc_gameplay` passes against the new store; `grep -r "PENDING_PRACTICE_BATTLES" crates/` returns zero matches.

- [x] 3.1 Edit `crates/emukc_gameplay/src/game/battle/repository.rs`: add `PracticeRepository` trait with `get_pending_practice`, `insert_pending_practice`, `take_pending_practice`. No `#[async_trait]`. *(Done — trait lives in separate file `practice_repository.rs` (not `repository.rs`); methods named `get_pending_battle` / `insert_pending_battle` / `take_pending_battle` (not `_practice`); also adds result-snapshot methods. Synchronous, no `#[async_trait]`.)*
- [x] 3.2 Add `GlobalPracticeStore` struct in `crates/emukc_gameplay/src/game/battle/practice/store.rs` (new file). Internally `Mutex<HashMap<i64, PracticeBattleSession>>`. Implement `PracticeRepository`. *(Done — named `PracticeStore`, lives in `game/sortie_store.rs` (co-located with `SortieStore`), not `battle/practice/store.rs`. `GLOBAL_PRACTICE_STORE` static wired.)*
- [x] 3.3 Edit `crates/emukc_gameplay/src/game/context.rs` (or wherever `HasContext` lives): add required method `fn practice_store(&self) -> &dyn PracticeRepository`. *(Done — `HasContext` in `gameplay.rs:27` defines `fn practice_store(&self) -> &PracticeStore` (concrete type, not `&dyn`).)*
- [x] 3.4 Provide a process-global `OnceLock<GlobalPracticeStore>` accessible via the production `HasContext` impl, mirroring how the sortie store is wired. *(Done — `GLOBAL_PRACTICE_STORE` referenced in `gameplay.rs:10`, returns `&PracticeStore` via blanket impl at `gameplay.rs:52`.)*
- [x] 3.5 Update gameplay test fixture to provide a `TestPracticeStore` implementation. *(Done — `PracticeStore` is the concrete test store; used in `battle/sortie/mod.rs:95` test and `battle/practice/mod.rs:160` test. No separate `TestPracticeStore` — same `PracticeStore` serves both prod and test.)*
- [x] 3.6 Edit `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs`: replace every `super::PENDING_PRACTICE_BATTLES.lock().unwrap()` access with calls to `store.get_pending_practice(...)`, `store.insert_pending_practice(...)`, `store.take_pending_practice(...)`. *(Done — orchestrate takes `practice_repo: &dyn PracticeRepository` param and calls `practice_repo.insert_pending_battle(...)`, `practice_repo.take_pending_battle(...)`.)*
- [x] 3.7 Delete the `PENDING_PRACTICE_BATTLES` static declaration. Confirm `grep -r "PENDING_PRACTICE_BATTLES" crates/` returns zero matches. *(Done — grep returns zero matches across `crates/` and `src/`.)*
- [x] 3.8 Run `cargo test -p emukc_gameplay` to verify practice tests pass against the new store. *(Done — practice tests compile and pass against `PracticeStore`.)*

### U4. Rename `CryptoRng` → `ProductionRng`

- **Goal:** Drop the misleading `Crypto` prefix; the RNG is non-cryptographic (fastrand/xoshiro).
- **Requirements:** R3.
- **Dependencies:** U2 recommended (so the body removed in U2 stays removed under the new name).
- **Files:** `crates/emukc_gameplay/src/game/battle/rng.rs`, workspace-wide search-and-replace across `crates/emukc_gameplay/`, `crates/emukc_battle/`, `src/bin/`.
- **Verification:** `cargo check --workspace` and `cargo clippy --workspace` clean; `grep -r "CryptoRng" crates/ src/` returns zero matches.

- [x] 4.1 In `crates/emukc_gameplay/src/game/battle/rng.rs`, rename `pub(crate) struct CryptoRng;` to `pub(crate) struct ProductionRng;`. Updated its `BattleRng` impl; also renamed test helpers `crypto_draws`→`production_draws` and `crypto_rng_is_deterministic_after_seed`→`production_rng_is_deterministic_after_seed` for consistency.
- [x] 4.2 Doc comment reads: `/// Non-cryptographic production RNG backed by emukc_crypto::rng (fastrand). For deterministic test runs, use SeededRng from emukc_battle::random.` — **deviation from plan:** the plan's `emukc_battle::test_utils` path does not exist; `SeededRng` is `#[cfg(test)]` at `emukc_battle::random`. Used the real path.
- [x] 4.3 Search-and-replace `CryptoRng` → `ProductionRng` across `crates/emukc_gameplay/`, `crates/emukc_battle/`. `grep -r "CryptoRng" crates/ src/` returns zero matches (15 sites renamed: 6 in rng.rs, 4 in sortie/orchestrate.rs, 1 in sortie/mod.rs test, 2 in practice/orchestrate.rs, 1 doc link in emukc_battle/random.rs).
- [x] 4.4 `cargo check --workspace` clean, `cargo clippy --workspace` clean, `cargo test production_rng_is_deterministic_after_seed` passes, `cargo fmt --all --check` clean.

### U5. RNG injection through orchestration

- **Goal:** Lift RNG construction out of the orchestration entry points so callers (and tests) can supply their own RNG.
- **Requirements:** R2.
- **Dependencies:** U4 (uses the renamed `ProductionRng`).
- **Files:** `crates/emukc_gameplay/src/game/battle/sortie/orchestrate.rs`, `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs`, `SortieOps` / `PracticeOps` blanket impls, KCSAPI handlers (only if they call orchestration directly), gameplay tests.
- **Approach:** Add `rng: &mut dyn BattleRng` as the last positional parameter to every `run_*_battle` fn. Remove internal `let mut rng = ProductionRng;`. Update blanket impls to construct one `ProductionRng` per entry point and forward. Update tests to use `SeededRng` where deterministic assertions are wanted.
- **Verification:** `cargo test --workspace` passes; deterministic tests can now assert on specific damage/target-selection outcomes.

- [x] 5.1 Edit `crates/emukc_gameplay/src/game/battle/sortie/orchestrate.rs`: added `rng: &mut impl BattleRng` as last positional param to `run_day_battle`, `run_night_battle`, `run_sp_midnight_battle`. Removed internal `let mut rng = ProductionRng;` from each. **Deviation from plan:** used `impl BattleRng` (generic) instead of `dyn BattleRng` — `emukc_battle::simulate_day`/`simulate_night` and ~30 internal functions use `&mut impl BattleRng`; changing them all to `dyn` is out of U5 scope. The same generic `T` flows caller → orchestrate → simulate_day → all internals.
- [x] 5.2 Edit `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs`: added `rng: &mut impl BattleRng` to `run_day_battle` and `run_night_battle`. Removed internal RNG construction.
- [x] 5.3 Updated `SortieOps` blanket impl in `sortie/mod.rs`: construct one `ProductionRng` per call (3 sites: `run_night_battle`, `run_sp_midnight_battle`, `run_day_battle`), pass `&mut rng`.
- [x] 5.4 Updated `PracticeOps` blanket impl in `practice.rs`: construct one `ProductionRng` per call (2 sites: `run_day_battle`, `run_night_battle`), pass `&mut rng`.
- [x] 5.5 Confirmed KCSAPI handlers do not call orchestration directly — they go through the trait blanket impls. No change needed.
- [x] 5.6 Updated gameplay tests: 5 test callsites (3 in `sortie_tests.rs`, 3 in `practice/mod.rs`) now construct `ProductionRng` and pass `&mut rng`. Existing tests do not require deterministic outcomes (no `SeededRng` needed yet).
- [x] 5.7 `cargo test --workspace` — 821 tests passed, 0 failed. `cargo check/clippy --workspace` clean, `cargo fmt --all --check` clean.

### U6. Practice night `EngagementType` decode

- **Goal:** Surface corrupt stored engagement data as a logged error + `None` return, rather than silently coercing to `SameCourse`.
- **Requirements:** R5.
- **Dependencies:** U3 (the practice orchestrate fn is already refactored to use the store by this point).
- **Files:** `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs`.
- **Verification:** `cargo test -p emukc_gameplay practice_night_battle` passes; the corrupt-formation test asserts `None`.

- [x] 6.1 Replaced the `EngagementType::from_api_id(session.formation[2]).unwrap_or(EngagementType::SameCourse)` in `practice/orchestrate.rs::run_night_battle` with a `match`: on `None`, `tracing::error!(profile_id, raw = session.formation[2], "practice night battle: corrupt engagement id")` then re-insert the session via `practice_repo.insert_pending_battle(profile_id, session)` and `return None;`. The session re-insert preserves the existing `!can_midnight` guard's contract so callers can still read the session after the early return.
- [x] 6.2 Added `run_night_battle_returns_none_when_engagement_id_is_corrupt` test: constructs a `PracticeBattleSession` with `formation[2] = 99` (outside valid 1–4 range) and `can_midnight = true`, asserts the result is `None` and that the session is still present in the store.
- [x] 6.3 `cargo test -p emukc_gameplay practice` — 17 passed, 0 failed (incl. new test). `cargo check/clippy --workspace` clean, `cargo fmt --all --check` clean.

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
