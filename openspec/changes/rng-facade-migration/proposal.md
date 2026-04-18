## Why

All game RNG uses `rand` v0.10.0 with ChaCha12 (cryptographic PRNG). Game mechanics don't need crypto-grade randomness — ChaCha12 is 3-5x slower than suitable alternatives and brings a heavy dependency tree. Swapping the RNG backend currently requires touching 16+ files across 3 crates because every call site imports `rand` directly.

## What Changes

- Add a `rng` facade module in `emukc_crypto` that wraps the RNG backend behind a stable API
- Switch backend from `rand` (ChaCha12) to `fastrand` (Wyrand): faster, zero-dep, quality sufficient for game mechanics
- Migrate all 16 RNG call sites from direct `rand::` imports to `emukc_crypto::rng` facade
- Remove `rand` dependency from workspace

## Capabilities

### New Capabilities
- `rng-facade`: Centralized RNG abstraction in `emukc_crypto::rng` — provides `GameRng` (seeded) and thread-local free functions. Future backend swaps (fastrand → biski64, SmallRng, etc.) modify one file only.

### Modified Capabilities
<!-- No spec-level behavior changes — RNG outputs remain statistically equivalent. This is a pure internal refactor. -->

## Impact

- **Dependencies**: Remove `rand` v0.10.0 (workspace + 2 crates), add `fastrand` v2.x to `emukc_crypto`
- **Crates affected**: `emukc_crypto` (new module), `emukc_gameplay` (10 files), `emukc_cache` (1 file), binary crate (3 files)
- **No API changes**: All KCSAPI responses remain identical
- **Breaking**: None external. Internal trait signatures unchanged.

## Non-goals

- No cryptographic RNG changes (token generation, password hashing untouched)
- No changes to game mechanics or battle formulas
- No new crate creation — facade lives in existing `emukc_crypto`
