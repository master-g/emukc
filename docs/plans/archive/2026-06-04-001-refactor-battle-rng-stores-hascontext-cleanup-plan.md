---
title: "refactor: Deepen battle RNG, sortie/practice stores, and HasContext tuple seam (B+C+D)"
type: refactor
status: completed
date: 2026-06-04
origin: /tmp/claude-501/architecture-review-zh-20260604-122013.html (architecture review, candidates B/C/D)
---

# refactor: Deepen battle RNG, sortie/practice stores, and HasContext tuple seam (B+C+D)

## Summary

Three deletion-oriented cleanups surfaced by the architecture review, all with **zero interface change for callers**:

- **B** — `CryptoRng` re-implements `roll_scratch_damage` and `choose_index` with bodies identical to the `BattleRng` trait defaults. Pull the formula back behind the trait so production and test RNG cannot diverge.
- **C** — `TestSortieStore` / `TestPracticeStore` are pass-through twins that forward every method to an inner `SortieStore` / `PracticeStore`. Delete them; tests inject `SortieStore::new()` / `PracticeStore::new()` directly, since those already produce isolated instances.
- **D** — Of the four `HasContext` tuple impls, only `(DbConn, Codex)` is used. The other three (`(Arc<DbConn>, Arc<Codex>)`, `(Arc<Codex>, Arc<DbConn>)`, `(Codex, DbConn)`) are dead code. Delete them.

Each unit is independently landable. The plan is **Standard** depth: small surface, but B carries a real RNG-determinism subtlety that must be handled deliberately.

---

## Problem Frame

The architecture review (candidates B/C/D) identified three **shallow** modules — interfaces nearly as complex as their implementations — where deletion concentrates complexity rather than moving it (the deletion test). None of the three changes any public behavior or caller-facing signature; all three reduce duplication and remove dead surface.

The friction each addresses:

- **B (locality leak):** the scratch-damage formula `hp*0.06 + r*0.08` lives in two places (`emukc_battle::random::BattleRng` default + `emukc_gameplay::game::battle::rng::CryptoRng` override). If the formula is ever tuned on one side only, production and tests silently disagree — a latent correctness footgun.
- **C (pass-through adapter):** `TestSortieStore`/`TestPracticeStore` exist only to avoid the process-global static, but `SortieStore::new()` already yields a fresh isolated instance. One real adapter dressed as two.
- **D (interface as wide as implementation):** four tuple permutations exist so tests could write `(db, codex)` in any order, but only one ordering is ever constructed. Three impls are pure dead weight quadrupling the seam surface.

---

## Requirements

- **R1** — No KCSAPI handler signature, gameplay trait method signature, or response shape changes. (B, C, D)
- **R2** — Battle simulation output for a given seed must remain **bit-identical** before and after B, OR any change must be deliberate, justified, and re-baselined in tests. (B)
- **R3** — All existing tests pass after each unit; test isolation guarantees (parallel-safe, independent in-memory state) are preserved. (C)
- **R4** — `cargo clippy --workspace` is clean (no new dead-code or unused-import warnings introduced). (B, C, D)
- **R5** — Each unit lands as an independent atomic commit with no cross-unit coupling. (B, C, D)

---

## Key Technical Decisions

### KTD-1 — B preserves RNG sequence by keeping the backend, not the override (R2, blocking)

This is the load-bearing decision of the plan. The two `CryptoRng` overrides are **not** behaviorally identical to the trait defaults at the RNG-draw level:

- `roll_scratch_damage` override: calls `emukc_crypto::rng::i64(0..current_hp)`. Trait default: calls `self.roll_range(0, current_hp)` → `self.roll_range_impl` → `emukc_crypto::rng::i64(min..max)`. **Same backend, same draw.** Deleting this override is sequence-preserving. ✅
- `choose_index` override: calls `emukc_crypto::rng::usize(0..len)`. Trait default: calls `self.roll_range(0, len as i64)` → `emukc_crypto::rng::i64(0..len)`. **Different fastrand method** (`usize` vs `i64`). `fastrand::usize(0..n)` and `fastrand::i64(0..n)` are not guaranteed to consume the generator identically, so deleting this override **may shift the production RNG sequence**. ⚠️

**Decision:** Delete the `roll_scratch_damage` override unconditionally (proven sequence-preserving). For `choose_index`, default posture is **behavior-preserving**: keep production semantics by having the trait default and the override agree on backend. Concretely, evaluate two sub-options at implementation time and pick the sequence-preserving one:

- **(a)** Delete only `roll_scratch_damage`; leave `choose_index` override in place. Smallest safe change, removes the genuine duplication (the formula), keeps the one override that has a real backend difference. **Recommended default.**
- **(b)** Delete both overrides AND change the trait-default `choose_index` to not exist as an override anywhere — accept the sequence shift, re-baseline any seed-dependent battle tests. Only if the team explicitly wants `choose_index` unified and accepts re-baselining.

The synthesis call-out already flagged this; user confirmed behavior-preserving default. Proceed with (a) unless implementation reveals `usize`/`i64` are provably identical for the `[0, len)` ranges in use (in which case (b) is free and preferred).

### KTD-2 — C keeps the `SortieRepository` / `PracticeRepository` seam (R1, R3)

The repository **traits** earn their keep (deletion test: removing them re-spreads global-vs-injected coupling across session functions) and stay. Only the `TestSortieStore` / `TestPracticeStore` **structs** — pure pass-throughs — are deleted. `SortieStore` and `PracticeStore` already implement their repository traits and already isolate per instance via `::new()`.

### KTD-3 — D deletes dead impls only; no named struct introduced (R1)

The review's "after" sketch floated a named `TestContext { db, codex }`. Research shows that is unnecessary: every construction site already uses the `(DbConn, Codex)` tuple, and a binary-crate `TestContext` for router tests already exists separately (`src/bin/net/router/kcsapi/mod.rs`, an unrelated struct). Introducing a new gameplay-level struct would force migrating ~9 value sites for no gain. **Decision:** delete the three unused tuple impls, keep `(DbConn, Codex)`, migrate nothing.

---

## Scope Boundaries

**In scope:**
- B — collapse `CryptoRng` formula duplication (behavior-preserving)
- C — delete `TestSortieStore` / `TestPracticeStore` + their `lib.rs` exports + internal tests migration
- D — delete three unused `HasContext` tuple impls

**Out of scope (non-goals):**
- Candidate A (merge sortie/practice battle-round bridge) — structural, large surface
- Candidate E (`with_tx` transaction combinator) — touches ~80 methods
- Any change to `BattleRng` primitive signatures (`random_f64_range`, `roll_range_impl`)
- Any change to the `rng-facade` spec (B operates within it — see `openspec/specs/rng-facade/`)

### Deferred to Follow-Up Work
- Candidate A and E remain as documented review candidates for a later plan.
- If implementation proves `fastrand::usize` and `fastrand::i64` identical over `[0,len)`, a follow-up could unify `choose_index` fully (KTD-1 option b).

---

## Implementation Units

### U1. B — Collapse the scratch-damage formula behind BattleRng

**Goal:** Remove the duplicated damage formula from `CryptoRng` so it lives only in the `BattleRng` trait default, without changing battle output for any seed.

**Requirements:** R1, R2, R4, R5

**Dependencies:** none

**Files:**
- `crates/emukc_gameplay/src/game/battle/rng.rs` — remove `roll_scratch_damage` override; resolve `choose_index` per KTD-1
- `crates/emukc_battle/src/random.rs` — trait defaults are the surviving home; doc-comment touch only if needed
- `crates/emukc_battle/src/simulation/mod.rs` — existing seed-based tests act as the regression baseline (no new file needed, but add a focused test if a gap exists)

**Approach:**
- Delete `CryptoRng::roll_scratch_damage` (proven sequence-preserving per KTD-1: same `emukc_crypto::rng::i64` backend as the trait default path).
- For `choose_index`: apply KTD-1 option (a) by default — leave the override in place because its `rng::usize` backend differs from the default's `rng::i64`. If implementation confirms the two fastrand methods consume the generator identically for `[0, len)`, apply option (b) instead and re-baseline.
- After deletion, `CryptoRng` should implement only `random_f64_range` and `roll_range_impl` (plus `choose_index` if option (a)).

**Execution note:** Characterization-first. Before deleting, capture a seed-fixed battle simulation snapshot (existing `simulation/mod.rs` tests with `SeededRng` are the baseline) and confirm it is unchanged after the edit. The risk is sequence drift, so the test must run against the *production* draw path, not only `SeededRng`.

**Patterns to follow:** `crates/emukc_battle/src/random.rs` trait-default-over-primitives pattern; `SeededRng` test impl shows the minimal two-primitive shape `CryptoRng` should converge to.

**Test scenarios:**
- Happy path: a fixed-seed `simulate_day` run produces the same `BattlePacket` (hougeki damages, hourai_flag, nowhps) before and after the edit. Covers R2.
- Edge case: scratch-damage branch specifically exercised — a low-firepower vs high-armor matchup that guarantees scratch damage (`karyoku=1`, `soukou=200`), assert resulting HP loss is unchanged from baseline.
- Edge case: `current_hp <= 1` scratch path returns `max(1)` (boundary in the formula) — unchanged.
- Regression guard: assert `CryptoRng` no longer defines `roll_scratch_damage` (compile-level — the override is gone; the default is exercised).
- `Test expectation:` if KTD-1 option (b) is taken, the above seed-based assertions are re-baselined to the new sequence and a comment records the intentional shift.

**Verification:** `cargo test -p emukc_battle` and `cargo test -p emukc_gameplay` pass; seed-fixed battle snapshots match baseline (option a) or are re-baselined with a recorded rationale (option b); `cargo clippy --workspace` clean.

---

### U2. C — Delete TestSortieStore / TestPracticeStore pass-throughs

**Goal:** Remove the two pass-through test stores and their public exports; redirect their callers to `SortieStore::new()` / `PracticeStore::new()`.

**Requirements:** R1, R3, R4, R5

**Dependencies:** none (independent of U1)

**Files:**
- `crates/emukc_gameplay/src/game/sortie_store.rs` — delete `TestSortieStore` (struct + `impl`/`Default`/`SortieRepository`, ~lines 186–254) and `TestPracticeStore` (struct + impls, ~lines 321–373); migrate the in-file `#[cfg(test)] mod tests` (lines 421–454) to use `PracticeStore::new()`
- `crates/emukc_gameplay/src/lib.rs` — remove the two `pub use` exports (`game::sortie_store::TestPracticeStore`, `game::sortie_store::TestSortieStore`)
- `crates/emukc_gameplay/src/game/battle/sortie/mod.rs` — line 94 test: `TestSortieStore::new()` → `SortieStore::new()`
- `crates/emukc_gameplay/src/game/battle/practice/mod.rs` — lines 267/271 test: `TestPracticeStore` import + `::new()` → `PracticeStore::new()`

**Approach:**
- `SortieStore` and `PracticeStore` already implement `SortieRepository` / `PracticeRepository` and isolate per instance, so callers swap the type name only — call sites pass `&store` exactly as before.
- The deleted structs' only added value was `clear()` delegation, which `SortieStore`/`PracticeStore` expose directly.
- Remove now-unused imports the deletion orphans (e.g., `use crate::game::sortie_store::TestPracticeStore;`).

**Patterns to follow:** the surviving `SortieStore` / `PracticeStore` impls in the same file are the canonical adapter; tests in `battle/sortie/mod.rs` already mix `CryptoRng` + a store instance.

**Test scenarios:**
- Happy path: migrated `practice_store_insert_get_take_cycle` runs against `PracticeStore::new()` and still asserts insert→get→take→empty cycle.
- Edge case: `practice_store_empty_take_returns_none` against `PracticeStore::new()` — unchanged behavior.
- Integration: `practice_store_instances_are_isolated` — two `PracticeStore::new()` instances remain independent (this is the property `TestPracticeStore` claimed to add; prove `PracticeStore` already has it).
- Regression guard: `battle/sortie/mod.rs::sortie_session_is_stored_until_result_is_taken` passes with `SortieStore::new()` substituted.

**Verification:** `cargo test -p emukc_gameplay` passes; no references to `TestSortieStore`/`TestPracticeStore` remain (`grep` clean); `cargo clippy --workspace` clean (no unused-import warnings).

---

### U3. D — Delete the three unused HasContext tuple impls

**Goal:** Remove the three dead `HasContext` tuple impls, keeping only `(DbConn, Codex)`.

**Requirements:** R1, R4, R5

**Dependencies:** none (independent of U1, U2)

**Files:**
- `crates/emukc_gameplay/src/gameplay.rs` — delete `impl HasContext for (Arc<DbConn>, Arc<Codex>)` (lines 41–57), `impl HasContext for (Arc<Codex>, Arc<DbConn>)` (lines 59–75), `impl HasContext for (Codex, DbConn)` (lines 95–111); keep `impl HasContext for (DbConn, Codex)` (lines 77–93)
- Remove the now-unused `use std::sync::Arc;` if the Arc impls were its only consumer in this file

**Approach:**
- Research confirmed every value-construction site uses `(DbConn, Codex)` (≥9 sites across `sortie_tests.rs`, `user/account.rs`, and `tests/*.rs`); the other three orderings appear only as impl definitions with zero call sites.
- This is a pure dead-code deletion — no migration, no call-site edits.
- Verify `Arc` import is still needed elsewhere in `gameplay.rs` before removing it (the `HasContext` trait and surviving impl may not use it).

**Patterns to follow:** the surviving `(DbConn, Codex)` impl is the canonical form already used everywhere.

**Test scenarios:**
- `Test expectation: none` — pure dead-code deletion with no behavioral change. Coverage is the existing test suite continuing to compile and pass (it only ever used `(DbConn, Codex)`).
- Regression guard (compile-level): workspace builds; `grep` for `Arc<DbConn>, Arc<Codex>` / `Arc<Codex>, Arc<DbConn>` / `(Codex, DbConn)` outside `gameplay.rs` history returns nothing.

**Verification:** `cargo build --workspace` and `cargo test -p emukc_gameplay` pass unchanged; `cargo clippy --workspace` clean; the three impls are gone and the fourth remains.

---

## Risks & Dependencies

| Risk | Unit | Severity | Mitigation |
|---|---|---|---|
| Deleting `choose_index` override shifts production RNG sequence | U1 | Medium | KTD-1 default keeps the override (option a); only delete the proven-equivalent `roll_scratch_damage`. Seed-fixed snapshot test gates the change. |
| A seed-dependent battle test silently re-baselines without notice | U1 | Low | Characterization-first execution note; any baseline change must be deliberate and commented. |
| Orphaned imports after struct/impl deletion | U2, U3 | Low | `cargo clippy --workspace` in each unit's verification catches unused imports. |
| Public-export removal breaks an external consumer of the prelude | U2 | Low | `TestSortieStore`/`TestPracticeStore` are test-only helpers; confirmed used only within the crate's own tests. Removing from `lib.rs` prelude is safe. |

**Sequencing:** U1, U2, U3 are mutually independent and may land in any order or in parallel. Recommended order U1 → U2 → U3 (highest-subtlety first, per the synthesis "warm up" rationale), but not required.

---

## Sources & Research

- Architecture review (candidates B/C/D): `/tmp/claude-501/architecture-review-zh-20260604-122013.html`
- RNG facade decision record: `openspec/specs/rng-facade/spec.md` (B operates within this spec; no spec change)
- Backend equivalence basis for KTD-1: `crates/emukc_crypto/src/rng.rs` — `i64`/`usize`/`i64_inclusive` all delegate to `fastrand`; `usize` vs `i64` over `[0,len)` is the one non-obvious draw-equivalence question, resolved conservatively.
- Call-site census (research): `(db, codex)` is the sole tuple ordering constructed; `TestSortieStore` used 1×, `TestPracticeStore` used 5×, all in-crate tests.
