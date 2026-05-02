## Why

The d4de3ff battle-architecture extraction introduced `SortieRepository` and `BattleRng` traits, but a follow-up audit found the refactor stopped halfway:

1. The **practice battle** path still uses a process-global `lazy_static! Mutex<HashMap<…>>` (`PENDING_PRACTICE_BATTLES`), making practice tests share state and bypass the new dependency-injection seam.
2. **RNG injection** is partial: `simulate_day`/`simulate_night` accept `&mut impl BattleRng`, but the orchestration layer hardcodes `let mut rng = CryptoRng;` inside every entry point, so callers (and tests) cannot swap RNG implementations.
3. `CryptoRng` is a **misnomer** — it delegates to `fastrand` via `emukc_crypto::rng`, which is a non-cryptographic xoshiro PRNG. The `Crypto` prefix promises platform entropy that isn't delivered.
4. `BattleRng::choose_index` uses `debug_assert!(len > 0)`, so a zero-length slice panics in dev but **silently returns 0 in release**, leading to out-of-bounds reads at the call site.
5. Practice night battle silently coerces an invalid `EngagementType` via `.unwrap_or(EngagementType::SameCourse)` — losing the bug signal when stored formation data is corrupted.
6. `BattleRng::roll_scratch_damage` is duplicated verbatim between the trait default and `CryptoRng` impl — pure dead code that drifts on next refactor.

## What Changes

- **Extract a `PracticeRepository` trait** (or extend `SortieRepository`) covering pending practice battle sessions; remove `PENDING_PRACTICE_BATTLES` static. Wire it through `HasContext::practice_store()` like the sortie path.
- **Lift RNG construction out of orchestration**: `run_day_battle`, `run_night_battle`, `run_sp_midnight_battle` (sortie + practice) take `&mut dyn BattleRng` as a parameter; only the binary edge constructs a concrete RNG.
- **Rename `CryptoRng` → `ProductionRng`** (or `ThreadLocalRng`). Update doc to clarify it is non-cryptographic, backed by `emukc_crypto::rng` (fastrand). Drop misleading `Crypto` prefix.
- **Make `BattleRng::choose_index` total**: return `Option<usize>` instead of panicking/returning 0 on empty input; or split into `choose_index_unchecked` + `choose_index` returning `Option`.
- **Surface `EngagementType` decode failures** in practice night battle: log + return error (or use `try_from`) rather than silently mapping to `SameCourse`.
- **Remove duplicated `roll_scratch_damage`** in `CryptoRng`; rely on trait default.

## Capabilities

### New Capabilities

- `practice-battle-storage`: defines a `PracticeRepository` trait for pending practice battle sessions and the contract that production + tests both consume it through `HasContext`.

### Modified Capabilities

- `sortie`: orchestration entry points (`run_day_battle`, `run_night_battle`, `run_sp_midnight_battle`) accept caller-supplied RNG via `&mut dyn BattleRng`. The orchestration layer SHALL NOT construct its own RNG.
- `rng-facade`: the production `BattleRng` implementation SHALL be named `ProductionRng` (not `CryptoRng`), and its docstring SHALL state that it is non-cryptographic.

## Non-goals

- Replacing the `fastrand` backend with a CSPRNG. Battle determinism does not need cryptographic strength.
- Adding seeded battle replay to the running server. Seeding remains a test-only feature surfaced through `SeededRng`.
- Refactoring `SortieRepository` itself; the existing trait shape is preserved.

## Impact

- **Affected crates**: `emukc_gameplay` (battle/practice/orchestrate.rs, battle/sortie/orchestrate.rs, battle/rng.rs, battle/repository.rs, context.rs), `emukc_battle` (random.rs `choose_index` signature), `emukc` binary edge.
- **Public API**: `CryptoRng` rename is a breaking change for any external consumer; we treat the symbol as internal but flag it in CHANGELOG. Orchestration `run_*_battle` signatures gain a `rng` parameter.
- **Tests**: gameplay integration tests gain the ability to inject `SeededRng` end-to-end (currently impossible because RNG is constructed inside the orchestrate functions).
- **No DB schema changes, no KCSAPI route changes, no Codex changes.**
