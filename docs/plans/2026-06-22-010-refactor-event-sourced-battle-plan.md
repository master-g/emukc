---
title: "refactor: Owned-pass battle simulation with event transforms"
status: completed
type: refactor
created: 2026-06-22
sequence: 010
origin: docs/brainstorms/2026-06-22-event-sourced-battle-requirements.md
---

# refactor: Owned-pass battle simulation with event transforms

## Summary

> **Status reconciliation (2026-06-23):** The owned-pass rewrite (original U2:
> FleetState, U5: phase event emitters, U6: packet builder) remains deferred.
> Debug behavior was delivered via the **bridge** approach — a post-simulation
> `debug_overlay` module that derives events from HP diffs, applies transforms,
> and overrides packets. See
> `docs/solutions/architecture-patterns/debug-overlay-bridge.md`.

Refactor `crates/emukc_battle/` from `&mut BattleState` to owned-pass architecture. Phase functions become `fn(state: FleetState, rng) -> (FleetState, Vec<BattleEvent>)` — externally pure (no `&mut` in signatures), internally tracking HP for correct intra-phase targeting. Events carry resolved damage (including RNG-consumed proportional damage). A pure reducer derives final state from events. Debug features (god_mode, one_hit_kill) are event-stream transforms applied between simulation and reduction.

## Problem Frame

Current `apply_damage` mixes 5 concerns with `if` branches. The broader simulation has 384 `&mut` references across ~11k lines with 202 tests. Debug features must be embedded as branches because state mutation and damage logic are inseparable under the current `&mut` pattern.

### Architecture revision from doc review

The original plan proposed pure event-sourcing (simulation emits events, reducer derives all state). Doc review identified two architectural blockers:

1. **Intra-phase HP dependencies**: targeting, scratch damage, and double attacks within a single phase require real-time HP knowledge. A per-phase entry snapshot is insufficient.
2. **Interleaved RNG**: sinking protection consumes RNG interleaved with targeting/damage RNG. A post-hoc reducer cannot consume RNG without reordering the stream.

**Resolution**: owned-pass pattern. Phase functions take owned `FleetState`, return `(FleetState, Vec<BattleEvent>)`. Internal mutability for computation is acceptable — the purity contract is "same inputs → same outputs," not "zero mutation internally." Events carry resolved damage (proportional damage amounts pre-computed with RNG consumed during simulation). The reducer is truly pure — no RNG.

---

## High-Level Technical Design

*Directional guidance for review, not implementation specification.*

```
// Phase function: owned state in, owned state + events out
fn simulate_shelling(codex, state: FleetState, rng) -> (FleetState, Vec<BattleEvent>)

// Orchestrator: threads state through phases
fn simulate_day(codex, ctx, rng) -> (FleetState, Vec<BattleEvent>) {
    let mut state = FleetState::from(ctx);
    let mut events = Vec::new();
    for phase in flow.phases {
        let (new_state, phase_events) = execute_phase(codex, state, rng);
        state = new_state;
        events.extend(phase_events);
    }
    (state, events)
}

// Debug transforms: pure, between simulation and reduction
let (state, events) = simulate_day(codex, ctx, rng);
let events = god_mode_transform(events);      // zeroes friendly damage
let events = one_hit_kill_transform(events);  // makes enemy hits lethal + sinks unhit

// Reducer: pure, no RNG
let result = reduce(&events, &initial_state);

// Packet builder: pure
let packet = to_packet(&result, &events);
```

**Key distinction from the rejected pure event-sourcing**: state flows through phases as owned values, not derived from events per phase. Events are the *output* of each phase alongside updated state — the reducer exists for debug transforms, not for primary state computation.

---

## Implementation Units

### U0. Fix non-compiling codebase and capture golden transcripts

**Goal:** Fix the `force_sink` compilation error from plan 009, then capture golden transcripts as the determinism safety net before any refactor.

**Dependencies:** None

**Files:**

- `crates/emukc_battle/src/state.rs` — fix or remove `force_sink` call
- `crates/emukc_battle/tests/golden_transcript.rs` (new)

**Approach:**

1. Fix `state.rs:202` — either define `force_sink()` on `BattleRuntimeShip` or remove the uncommitted finalize_day block (revert to pre-force-sink state)
2. Run `simulate_day` with `SeededRng::new(1)` through `SeededRng::new(20)` on fixed battle setups (day + night)
3. Serialize each `BattlePacket` to JSON, store as golden files
4. Verify all 202 existing tests pass after the fix

**Execution note:** Capture golden transcripts BEFORE any other unit. This is the safety net for the entire refactor.

**Test scenarios:**

1. 20 day battle packets captured as golden JSON
2. 20 night battle packets captured as golden JSON
3. All existing tests pass after fix

**Verification:** `cargo test -p emukc_battle` green. Golden files exist on disk.

---

### U1. Define BattleEvent types

**Goal:** Create the event vocabulary that phases emit alongside state.

**Dependencies:** U0

**Files:**

- `crates/emukc_battle/src/event.rs` (new)
- `crates/emukc_battle/src/lib.rs` (add `mod event`)

**Approach:**

Define fine-grained events (NOT packet wrappers — KTD-2 revised):

- `Damage { target: ShipRef, raw: i64, dealt: i64, phase: Phase }` — raw input + actual HP subtracted
- `ProportionalDamage { target: ShipRef, amount: i64 }` — sinking protection result (RNG pre-consumed)
- `Sunk { target: ShipRef }` — ship reached 0 HP
- `Targeted { attacker: ShipRef, target: ShipRef, phase: Phase }` — for damage_dealt tracking (MVP)
- `PhaseStart { phase }` / `PhaseEnd { phase }` — phase boundaries
- `AirCombat { kouku }` — wraps `BattleKouku` (already correct format for packet passthrough)
- `TorpedoSalvo { attacks }` — wraps torpedo data
- `ShellingExchange { hougeki }` — wraps `BattleHougeki` for packet assembly

Use `ShipRef(Side, usize)` — `Side::Friendly` / `Side::Enemy`.

**Note:** Events carry both resolved damage (`dealt`) and raw (`raw`) so the reducer and transforms can work independently. `ProportionalDamage` carries the pre-computed amount from RNG consumed during simulation.

**Test scenarios:**

1. Events serialize to JSON
2. Empty event log is valid
3. ShipRef distinguishes sides

**Verification:** `cargo build -p emukc_battle` compiles with event types.

---

### U2. Implement FleetState (owned-pass state type)

**Goal:** Create an immutable-by-contract state type that phases consume and produce.

**Dependencies:** U0

**Files:**

- `crates/emukc_battle/src/state.rs` — replace `BattleState` with `FleetState`
- `crates/emukc_battle/src/types/runtime.rs` — simplify `BattleRuntimeShip` (remove `apply_damage`, debug fields)

**Approach:**

`FleetState` owns `friendly: Vec<ShipState>` and `enemy: Vec<ShipState>`. `ShipState` has `current_hp`, `entry_hp`, `sunk: bool`, and ship data (immutable).

Phase functions:

```
fn simulate_shelling(codex, state: FleetState, rng: &mut impl BattleRng) -> (FleetState, Vec<BattleEvent>)
```

Internally, the phase clones ships into a local mutable Vec, processes attacks sequentially (preserving intra-phase HP visibility), and returns the updated state + emitted events.

`apply_damage` logic moves into a method on the local mutable `ShipState` inside phases. The sinking protection RNG is consumed here — events carry resolved values.

Remove `god_mode`/`one_hit_kill` from `BattleRuntimeShip` and `BattleContext` (KTD-3 from original plan, now validated by doc review — debug is event transforms only).

**Test scenarios:**

1. `FleetState::from(context)` captures correct initial HP
2. Phase function with identical inputs produces identical outputs (owned-pass determinism)
3. Intra-phase HP changes visible to subsequent attacks within same phase

**Verification:** `cargo build -p emukc_battle` compiles. No `&mut` in phase function signatures.

---

### U3. Implement debug event transforms

**Goal:** God mode and one hit kill as pure `Vec<BattleEvent>` transforms.

**Dependencies:** U1

**Files:**

- `crates/emukc_battle/src/transforms.rs` (new)
- `crates/emukc_battle/src/lib.rs` (add `mod transforms`)

**Approach:**

```
pub fn god_mode_transform(events: Vec<BattleEvent>) -> Vec<BattleEvent> {
    events.into_iter()
        .filter(|e| !matches!(e, BattleEvent::Damage { target: ShipRef(Friendly, _), .. }
                                | BattleEvent::ProportionalDamage { target: ShipRef(Friendly, _), .. }))
        .collect()
}

pub fn one_hit_kill_transform(events: Vec<BattleEvent>) -> Vec<BattleEvent> {
    let mut result = Vec::new();
    let mut sunk_enemies = HashSet::new();
    for e in events {
        match &e {
            BattleEvent::Damage { target: ShipRef(Enemy, idx), .. } => {
                sunk_enemies.insert(*idx);
                result.push(BattleEvent::Sunk { target: ShipRef(Enemy, *idx) });
            }
            _ => result.push(e),
        }
    }
    // Synthesize Sunk events for enemies that received zero Damage events
    // (never targeted) — AE3 requires ALL enemies dead
    result
}
```

Filtering (not NoOp sentinel) — avoids polluting event vocabulary.

**Test scenarios:**

1. god_mode_transform filters friendly Damage events
2. god_mode_transform preserves enemy Damage events
3. one_hit_kill_transform converts enemy Damage to Sunk
4. one_hit_kill_transform preserves friendly Damage events
5. Composed transforms work
6. one_hit_kill adds Sunk for unhit enemies (AE3 coverage)

**Verification:** `cargo test -p emukc_battle transforms` green.

---

### U4. Implement reducer

**Goal:** Pure reducer that derives final state from events for debug-transformed event streams.

**Dependencies:** U1

**Files:**

- `crates/emukc_battle/src/reducer.rs` (new)
- `crates/emukc_battle/src/lib.rs` (add `mod reducer`)

**Approach:**

```
fn reduce(events: &[BattleEvent], initial: &FleetState) -> DerivedState
```

Processes events sequentially:

- `Damage { target, dealt, .. }` → subtract `dealt` from target HP (already clamped by emitter)
- `ProportionalDamage { target, amount }` → subtract amount
- `Sunk { target }` → HP = 0
- `Targeted { attacker, .. }` → accumulate damage_dealt for attacker (MVP)

No RNG. No sinking protection logic (that's resolved during simulation, carried in events).

**Test scenarios:**

1. Empty events → state matches initial HP
2. Single Damage → HP reduced
3. Damage on sunk → no-op
4. Multiple Damage cumulative
5. Sunk event → HP = 0

**Verification:** `cargo test -p emukc_battle reducer` green.

---

### U5. Rewrite simulation phases as owned-pass event emitters

**Goal:** Convert each phase from `&mut BattleState` to `fn(FleetState, rng) -> (FleetState, Vec<BattleEvent>)`.

**Dependencies:** U1, U2, U3, U4

**Files:**

- `crates/emukc_battle/src/simulation/mod.rs` — rewrite orchestrator
- `crates/emukc_battle/src/simulation/kouku.rs` — owned-pass
- `crates/emukc_battle/src/simulation/shelling.rs` — owned-pass
- `crates/emukc_battle/src/simulation/torpedo.rs` — owned-pass
- `crates/emukc_battle/src/simulation/asw.rs` — owned-pass
- `crates/emukc_battle/src/simulation/night.rs` — owned-pass
- `crates/emukc_battle/src/simulation/special_attack.rs` — owned-pass
- `crates/emukc_battle/src/simulation/day_cutin.rs` — owned-pass

**Approach:**

Each phase:

1. Receives owned `FleetState`
2. Internally clones ships to local mutable Vec for sequential attack processing
3. For each attack: selects target (reads current HP from local), calculates damage (consumes RNG), applies damage (including sinking protection — consumes RNG), emits events
4. Returns `(FleetState::from(local_ships), events)`

RNG consumption order is preserved — phases still draw in the same sequence. Events carry resolved values.

Night battle receives `FleetState` from day battle (owned-pass) instead of cloned `BattleRuntimeShip` slices. `NightBattleInput` changes to accept `FleetState` + events.

sp_midnight constructs `FleetState` directly from context, then calls night simulation.

**Execution note:** Verify against golden transcripts after EACH phase rewrite, not just at the end.

**Test scenarios:**

1. Each phase produces correct events for standard inputs
2. All enemies sunk mid-battle → subsequent phases no-op
3. Determinism: same seed → identical event log and state
4. Golden transcript match (U0 files)

**Verification:** All 40 golden transcripts pass. `cargo test -p emukc_battle` green.

---

### U6. Rewrite to_packet adapter and remove old types

**Goal:** Convert event log + fleet state into `BattlePacket`. Delete old mutable types.

**Dependencies:** U5

**Files:**

- `crates/emukc_battle/src/packet_builder.rs` (new)
- `crates/emukc_battle/src/state.rs` — delete old `BattleState`
- `crates/emukc_battle/src/types/runtime.rs` — delete `BattleRuntimeShip`, replace with `ShipState`

**Approach:**

`to_packet(state: &FleetState, events: &[BattleEvent]) -> BattlePacket`

Maps events to packet types. Final HP from `FleetState`. Packet structure unchanged.

Remove all `&mut BattleState`, `apply_damage`, `set_debug_flags`. `BattleContext` loses `god_mode`/`one_hit_kill` fields — transforms are applied externally.

**Test scenarios:**

1. Empty events → minimal packet
2. Full day battle → all phases populated
3. HP values match fleet state
4. Packet byte-identical to golden transcripts

**Verification:** Zero references to `apply_damage`, `BattleRuntimeShip`, or `&mut BattleState` in crate. All golden transcripts pass.

---

### U7. Update gameplay crate and verify end-to-end

**Goal:** Wire new API into gameplay call sites. Verify determinism and debug behavior.

**Dependencies:** U6

**Files:**

- `crates/emukc_gameplay/src/game/sortie/mod.rs`
- `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs`
- `crates/emukc_gameplay/src/game/battle/sortie/orchestrate.rs`
- `crates/emukc_gameplay/src/game/sortie_tests.rs`
- `crates/emukc_gameplay/tests/sortie_battle.rs`
- `crates/emukc_gameplay/tests/practice_battle.rs`

**Approach:**

Call sites:

```
let (state, events) = simulate_day(codex, ctx, rng);
let events = if codex.game_cfg.god_mode { god_mode_transform(events) } else { events };
let events = if codex.game_cfg.one_hit_kill { one_hit_kill_transform(events) } else { events };
let result = reduce(&events, &initial_state);
let packet = to_packet(&result, &events);
```

Remove `god_mode`/`one_hit_kill` from `BattleContext` — all `..Default::default()` additions from plan 009 are removed. All ~27 `BattleContext` literals are simplified.

**Test scenarios:**

1. Sortie battle → correct API response
2. Practice battle → correct API response
3. god_mode → friendly HP unchanged
4. one_hit_kill → all enemies dead
5. Both disabled → golden transcript match

**Verification:** `cargo test -p emukc_gameplay` green. All integration tests green.

---

## Scope Boundaries

### In scope

- Owned-pass refactor of `crates/emukc_battle/` (~11k lines, 202 tests)
- Event types, reducer, debug transforms
- Golden transcript determinism verification
- Gameplay crate integration

### Deferred to follow-up work

- Gameplay crate owned-pass migration
- Event stream visualization tooling
- Property-based RNG path coverage testing

### Out of scope

- Client-side battle prediction
- Multiplayer event replication

---

## Key Technical Decisions

### KTD-1: Owned-pass, not per-phase snapshot

Phase functions take owned `FleetState` and return `(FleetState, events)`. Internal mutability for sequential attack processing is acceptable. The purity contract is deterministic inputs → deterministic outputs, not zero internal mutation. This resolves the intra-phase HP dependency blocker from doc review.

### KTD-2: Fine-grained events, not packet wrappers

Events are `Damage`/`Sunk`/`ProportionalDamage`/`Targeted` — not wrappers around `BattleHougeki`. The packet builder ASSEMBLES `BattleHougeki` from events. This resolves the KTD-2 contradiction from the original plan (doc review Finding 1/4).

### KTD-3: ProportionalDamage carries RNG-resolved values

Sinking protection RNG is consumed during simulation (inside phase functions). The `ProportionalDamage` event carries the pre-computed amount. The reducer never touches RNG. This resolves the interleaved RNG blocker from doc review.

### KTD-4: god_mode/one_hit_kill removed from BattleContext

Debug flags are external event transforms, not context fields. `BattleContext` shrinks — plan 009's `..Default::default()` additions are reverted.

### KTD-5: Golden transcripts are sampled verification, not proof

20 seeds × 2 battle types catch the most common RNG paths. Special attacks and rare cutin types may not be exercised. Documented as a known limitation — property-based testing is deferred.

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| RNG draw order drift during rewrite | High | Critical | Golden transcripts (U0) verified after each phase rewrite |
| Intra-phase HP visibility breaks | Medium | High | Owned-pass pattern preserves sequential HP tracking inside phases |
| Post-hoc debug transforms differ from in-simulation behavior | Medium | Medium | god_mode zeroes damage but targeting/torpedo eligibility ran on real HP — acceptable for debug use, documented |
| 202 tests need rewriting | Certain | Medium | Tests assert on events + derived state — cleaner than old mutation-based assertions |
| Special attack / rare cutin RNG paths uncovered by golden transcripts | Medium | Medium | KTD-5 documents limitation; add more seeds for known complex paths |
| Night battle / sp_midnight propagation | Medium | Medium | Night receives FleetState from day (owned-pass); sp_midnight constructs directly |

---

## System-Wide Impact

- **Battle crate**: Full refactor. Phase signatures change from `&mut` to owned-pass. 202 tests updated. API surface changes.
- **Gameplay crate**: Call sites updated. `BattleContext` simplified (debug fields removed). ~27 struct literals simplified.
- **API responses**: Byte-identical for non-debug mode (golden transcripts). Debug mode produces semantically correct results (enemies dead, friendlies unharmed).
- **Debug experience**: New debug features = new transform functions. Zero simulation code changes.
