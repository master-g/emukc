## Context

The d4de3ff commit ("refactor(battle): extract emukc_battle crate, introduce BattleRng + SortieRepository") landed three independent improvements:

1. A new `emukc_battle` crate housing pure simulation code.
2. A `BattleRng` trait at the simulation entry points so callers supply randomness.
3. A `SortieRepository` trait so tests can inject isolated sortie session storage.

The follow-up audit shows three regressions/gaps in that refactor:

- **Practice path was forgotten.** `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs` still relies on `super::PENDING_PRACTICE_BATTLES.lock().unwrap()`. This static `Mutex<HashMap<i64, PracticeBattleSession>>` survives across tests, leaks state, and bypasses the `HasContext` plumbing the sortie path now uses.
- **RNG injection only goes one layer deep.** `simulate_day(codex, ctx, rng)` accepts an `&mut impl BattleRng`, but `run_day_battle(store, codex, input)` constructs `let mut rng = CryptoRng;` itself. Tests can swap `SortieRepository` but cannot swap RNG; deterministic assertions on damage values require monkey-patching the orchestrate functions.
- **`CryptoRng` lies.** It claims platform entropy but delegates to `emukc_crypto::rng`, which wraps `fastrand` (xoshiro256++). For non-cryptographic RNG that is fine, but the name implies a security guarantee that is not delivered.

In addition, three smaller defects were observed:

- `BattleRng::choose_index` uses `debug_assert!(len > 0)` then divides by `len`. Release builds silently elide the assert and may panic on `% 0`, and the doc claims it returns 0 for empty input — the behavior is genuinely undefined.
- Practice night battle: `EngagementType::from_api_id(session.formation[2]).unwrap_or(EngagementType::SameCourse)` swallows storage corruption.
- `CryptoRng::roll_scratch_damage` duplicates the `BattleRng` trait default body.

## Goals / Non-Goals

**Goals:**

- Practice battle session state is reachable via `HasContext::practice_store() -> &dyn PracticeRepository`, mirroring the sortie path.
- Both `simulate_*` and `run_*_battle` take a caller-supplied `&mut dyn BattleRng`. Orchestration never constructs RNG.
- `CryptoRng` is renamed to `ProductionRng`; doc comment states it is **non-cryptographic** and backed by `fastrand`.
- `BattleRng::choose_index` has a total signature: returns `Option<usize>`. Empty input returns `None`.
- `EngagementType` decode failures in practice night battle return a typed error rather than mapping to `SameCourse`.
- `roll_scratch_damage` is removed from concrete impls; only the trait default exists.

**Non-Goals:**

- Replacing `fastrand` with a CSPRNG.
- Adding seeded battle replay to the live server.
- Restructuring `SortieRepository` itself; we mirror its shape, not redesign it.
- Changing the `emukc_battle` simulation algorithms.

## Decisions

### D1. Extend `SortieRepository` vs. introduce `PracticeRepository`

**Decision**: Introduce a separate `PracticeRepository` trait in `crates/emukc_gameplay/src/game/battle/repository.rs`. Practice and sortie battles have different lifecycles (practice has no map state, no result staging via the same shape) and different KCSAPI route ownership (`api_req_practice/` vs `api_req_sortie/`). Coupling them through `SortieRepository` would muddy the contract.

**Alternative considered**: extend `SortieRepository` with `get_pending_practice` / `take_pending_practice`. Rejected because the existing trait shape is already documented in archived spec `sortie-repository`, and conflating practice into "sortie" breaks the domain analogy.

### D2. RNG plumbing through orchestration entry points

**Decision**: `run_day_battle`, `run_night_battle`, `run_sp_midnight_battle` (sortie) and `run_day_battle`, `run_night_battle` (practice) take `rng: &mut dyn BattleRng` as their final positional parameter. Trait methods on `SortieOps` / `PracticeOps` that call them construct `ProductionRng` once, then forward; gameplay tests construct `SeededRng` and pass it through.

**Alternative considered**: thread an `RngFactory` through `HasContext`. Rejected as over-engineered — RNG state is per-battle, not per-context, and `HasContext` is already heavy.

### D3. `choose_index` total signature

**Decision**: change signature to `fn choose_index(&mut self, len: usize) -> Option<usize>`. Callers using a non-empty slice unwrap explicitly via `.expect("non-empty by construction")` or surface the `None` as a typed error. The trait doc documents the new contract.

**Alternative considered**: keep `usize` return and `panic!` on empty. Rejected — the existing comment promises silent zero, which is the worst of both worlds.

### D4. `CryptoRng` → `ProductionRng` rename

**Decision**: rename the type and update the docstring to read "Non-cryptographic RNG backed by `emukc_crypto::rng` (fastrand). For deterministic test runs, use `SeededRng` from `emukc_battle::test_utils`." Cargo workspace-wide grep + `cargo check` covers the rename. Symbol is internal so we accept the breaking change without a deprecation alias.

**Alternative considered**: keep `CryptoRng`, only update docs. Rejected — the name actively misleads readers reviewing security-sensitive code paths.

### D5. Practice `EngagementType` decode

**Decision**: `EngagementType::from_api_id(stored)` becomes a typed `Result<EngagementType, BattleError>` at the orchestration boundary. The night battle returns `None` (already an `Option` return) when the stored formation is corrupt; we additionally log a `tracing::error!` so the corruption is visible.

## Risks / Trade-offs

- [Repository duplication] → introducing a parallel `PracticeRepository` trait costs ~30 lines of trait + impl, but the alternative (overloading `SortieRepository`) costs more in conceptual coupling and breaks the existing archived spec contract.
- [RNG signature churn] → every internal caller of `run_day_battle` / `run_night_battle` gains an `rng` parameter. Mitigation: a single PR that touches orchestrate fns + their tests + the two KCSAPI handlers; pre-merge `cargo check --workspace` confirms full coverage.
- [`choose_index` callsite churn] → roughly a dozen callsites. Mitigation: provide a `choose_index_unchecked` blanket helper for callers that have already validated non-emptiness, so the diff stays small.
- [Rename surfaces in archived spec] → archived spec `battle-rng-port` references `CryptoRng`. We do not edit archived specs; the rename is documented as an addendum in `rng-facade` modified delta.

## Migration Plan

1. Add `PracticeRepository` trait + `GlobalPracticeStore` impl alongside the existing `SortieRepository` infrastructure. Keep `PENDING_PRACTICE_BATTLES` as a private detail of `GlobalPracticeStore` for one transitional commit, then remove the static.
2. Change `BattleRng::choose_index` signature; update `SeededRng`, `ProductionRng` (post-rename), and all callsites in `emukc_battle` in the same commit. CI fails fast if any callsite is missed.
3. Add `rng` parameter to `run_*_battle` orchestrate fns. Update `SortieOps` / `PracticeOps` blanket impls to construct `ProductionRng` and forward. Update tests to use `SeededRng` where deterministic assertions are wanted.
4. Rename `CryptoRng` → `ProductionRng` in the same commit as step 3 (mechanical, scoped).
5. Replace practice night `unwrap_or(SameCourse)` with `tracing::error!` + `return None`; add unit test asserting the error path is taken.
6. Remove `CryptoRng::roll_scratch_damage` body, relying on trait default; verify by deleting and running `cargo test -p emukc_battle`.

Rollback: each step is an isolated commit; revert in reverse order if needed.
