## Context

EmuKC uses `rand` v0.10.0 throughout the codebase. `StdRng` (ChaCha12) is used for both seeded battle RNG (`Option<StdRng>` in `BattleRandom`) and thread-local RNG (`rng()`). All 16 call sites import `rand` directly, making backend swaps expensive.

Current RNG consumers:
- `crates/emukc_gameplay/src/game/battle/core.rs` — seeded `StdRng` for deterministic battle replay
- `crates/emukc_gameplay/src/game/{map_route,sortie,expedition,use_item,practice,sortie_result}.rs` — thread-local `rng()`
- `crates/emukc_gameplay/src/game/compose/{powerup,marriage}.rs` — thread-local `rng()`
- `crates/emukc_cache/src/kache.rs` — shuffle CDN server indices
- `src/bin/net/router/kcsapi/api_req_kousyou/{createitem,createship}.rs` — crafting RNG
- `src/bin/cli/dev/nuke.rs` — dev tool RNG

API surface used: `random_range` (exclusive + inclusive ranges, integer + float), `SliceRandom::shuffle`, `IndexedRandom::choose`, `IteratorRandom::choose`, `SeedableRng::seed_from_u64`.

## Goals / Non-Goals

**Goals:**
- Introduce a single RNG facade module that all crates use instead of `rand` directly
- Switch backend to a faster non-cryptographic PRNG suitable for game mechanics
- Make future backend changes a one-file operation
- Maintain deterministic battle replay (seeded RNG must produce identical sequences)
- Abstract away backend API differences (inclusive ranges, float ranges, iterator choose)

**Non-Goals:**
- No changes to cryptographic functions in `emukc_crypto` (password hashing, tokens)
- No game mechanics changes — RNG output distribution stays equivalent
- No new workspace crate
- No removal of `rand` from `Cargo.lock` — `uuid` crate's `fast-rng` feature transitively depends on `rand`

## Decisions

### Decision 1: Facade location — `emukc_crypto::rng`

**Choice**: New `rng` module in existing `emukc_crypto` crate.

**Why**: `emukc_crypto` is already a leaf crate (no internal deps). Both `emukc_gameplay` and `emukc_cache` already depend on it. Zero new dependency edges.

**Alternatives considered**:
- New `emukc_rng` crate — cleaner separation but adds workspace member for ~100 LOC. Overhead not justified.
- `emukc_model::rng` — model crate is for data types, RNG doesn't belong there.

### Decision 2: Backend — `fastrand` v2.x (Wyrand)

**Choice**: `fastrand` with Wyrand algorithm.

**Why**: Zero dependencies, ~0.5-0.8ns/u64 (3-5x faster than ChaCha12), passes BigCrush + PractRand 1TB+, `Rng` is `Send + Sync`, supports seeding via `with_seed(u64)`.

**Alternatives considered**:
- `rand::SmallRng` (Xoshiro256++) — same speed, excellent quality, but still pulls full `rand` dependency tree. Doesn't solve the "many call sites" problem unless we also add a facade.
- `biski64` (~0.37ns/u64) — fastest option, passes BigCrush + PractRand, but very new (2025), less ecosystem maturity.
- `oorandom` — minimal but lacks shuffle/choose helpers.

### Decision 3: API design — struct + free functions

**Choice**:
- `GameRng` struct wrapping `fastrand::Rng` for seeded (deterministic) use
- Thread-local free functions for one-off calls
- Both `GameRng` and free functions expose identical API surface:
  - Integer ranges: `i64(Range<i64>)`, `usize(Range<usize>)`, `u32(Range<u32>)`
  - Inclusive integer ranges: `i64_inclusive(RangeInclusive<i64>)`, etc.
  - Float: `f64()` → `[0.0, 1.0)`, `f64_range(min, max)` → `[min, max)`
  - Collection: `shuffle(&mut [T])`, `choose(&[T])`, `choose_iter(I: Iterator)`

**Why**: Mirrors current dual usage pattern exactly — `BattleRandom` holds `Option<GameRng>`, other code uses free functions. Inclusive ranges, float ranges, and iterator choose are abstracted in the facade so call sites stay clean.

### Decision 4: Migration strategy — remove direct deps only

**Choice**: Remove `rand` from workspace direct dependencies after migration.

**Why**: No code needs `rand` after facade is in place. `uuid` crate's `fast-rng` feature pulls its own `rand` internally — this stays as a transitive dependency in `Cargo.lock`. Our workspace no longer declares `rand` as a direct dependency.

### Decision 5: fastrand API gaps — handled in facade

**Choice**: Facade bridges three API gaps between `rand` and `fastrand`:

1. **Inclusive ranges** — `rand` accepts `RangeInclusive` (`0..=3`), `fastrand` does not. Facade provides `_inclusive()` variants that internally convert `min..=max` → `min..(max+1)`.
2. **Float ranges** — `rand` has `random_range(0.0..100.0)`, `fastrand` only has `f64()` → `[0, 1)`. Facade provides `f64_range(min, max)` = `min + f64() * (max - min)`.
3. **Iterator choose** — `rand` has `IteratorRandom::choose`, `fastrand` has none. Facade implements `choose_iter` via index-based reservoir sampling or `nth()` on the iterator.

**Why**: These three gaps account for 14 call sites. Encapsulating them in the facade means call sites get clean API and future backend swaps don't revisit these conversions.

## Risks / Trade-offs

- **[Determinism change]** Seeded battles will produce different sequences with Wyrand vs ChaCha12 → Acceptable: no saved replays exist, test seeds will need updated expected values (if any assert on specific RNG outputs — current tests only assert structural results, not specific random values).
- **[Wyrand quality ceiling]** Minor PractRand failures at >1TB → Irrelevant: no code path generates anywhere near that volume.
- **[fastrand maturity]** Less audited than `rand` → Mitigated: `fastrand` is widely used (1B+ downloads), Wyrand is well-studied.
- **[Inclusive range overflow]** Converting `i64::MAX..=i64::MAX` → `i64::MAX..( i64::MAX+1)` overflows → Mitigated: no current call site uses `i64::MAX` as upper bound. All inclusive ranges use small constants. Facade uses checked arithmetic with `saturating_add(1)`.
- **[Float range precision]** `min + f64() * (max - min)` has subtle rounding at boundaries → Acceptable: game mechanics don't require cryptographic uniformity. The `battle/core.rs` `random_f64_range` already uses the same integer-multiply pattern (`u32..10001 / 10000.0`), so behavior is consistent.
- **[Iterator choose allocation]** `choose_iter` on non-`ExactSizeIterator` may require collecting or reservoir sampling → Mitigated: the single `IteratorRandom` call site (`use_item.rs:734`) uses `.iter().skip(1).choose()` on a `Vec`, which is `ExactSizeIterator`-compatible. No generic iterator support needed.
