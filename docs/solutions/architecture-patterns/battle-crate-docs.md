---
title: "Battle crate docs: zero clippy, documented enums, formation dedup"
date: 2026-06-22
category: architecture-patterns
module: emukc_battle
problem_type: architecture_pattern
component: service_object
severity: medium
applies_when:
  - "Maintaining the emukc_battle crate against clippy and doc standards"
  - "Adding public enums or functions to the battle crate"
  - "Refactoring formation modifier functions"
tags: [battle, clippy, missing-docs, rustdoc, formation-modifier, dead-code]
related_components: [emukc_gameplay]
---

# Battle crate docs: zero clippy, documented enums, formation dedup

## Context

`crates/emukc_battle/` is the core simulation engine. It must stay clippy-
clean under the workspace's `-W warnings` gate, document its public API, and
avoid duplicated formation-modifier logic. This spec collects the
documentation and hygiene invariants that keep the crate reviewable.

## Guidance

The following invariants hold for `emukc_battle`:

### Clippy and docs

- **Zero clippy warnings.** `cargo clippy --workspace` SHALL produce zero
  warnings originating from `crates/emukc_battle/`.
- **Types module allows missing docs.** The `types` module SHALL use
  `#[allow(missing_docs)]` to suppress missing-doc warnings on data structures
  whose field names are self-explanatory (mirroring KanColle API naming).
- **Public enums and functions documented.** All public enums (`BattleType`,
  `EngagementType`, `AirState`, `BattleOutcome`) and public functions
  (`simulate_day`, `simulate_night`, `calculate_mvp`, `calculate_win_rank`,
  `apply_cap`) SHALL have `///` doc comments of at least one line describing
  their purpose.
- **Dead code annotated.** Unused constants and functions reserved for future
  features SHALL carry `#[allow(dead_code)]` and a `// TODO:` comment naming
  the feature that will use them.
- **Doc backticks.** Doc comments SHALL wrap type names and code identifiers
  in backticks per rustdoc convention.

### Documentation of non-obvious behavior

- **RNG cross-phase continuity.** `simulate_day` SHALL have a doc comment
  explaining that the `rng` parameter is consumed sequentially across all
  battle phases (kouku, OASW, opening torpedo, shelling 1, shelling 2,
  closing torpedo), so the same seed always produces a deterministic full
  battle, and changing phase order or adding/removing phases changes all
  subsequent random outcomes.
- **Air Stage2 simplification.** The kouku Stage2 anti-air fire calculation in
  `simulation/kouku.rs` SHALL carry a `// NOTE:` comment explaining it uses a
  linear approximation (`total_aa / 400 × plane_count`) instead of the real
  per-ship AA with slot-level shootdowns, and that this is a known deviation
  from KanColle's actual formula.

### Formation modifier deduplication

- **Single `formation_modifier`.** The `shelling_formation_modifier` and
  `torpedo_formation_modifier` functions in `damage.rs` SHALL be replaced by a
  single `formation_modifier` function; the two named functions SHALL NOT
  exist.
- **`asw_formation_modifier` stays separate.** `asw_formation_modifier` SHALL
  remain its own function with its own values (Diamond=1.2, Echelon=1.1, Line
  Abreast=1.3) because its values differ from the shelling/torpedo modifiers.

## Why This Matters

The workspace gate (`-W warnings`) fails the build on any warning. A single
un-documented public enum or duplicated modifier function either blocks the
gate or silently rots. Centralizing these hygiene rules here means a new
contributor adding a public function knows to document it without re-deriving
the convention.

## When to Apply

- When adding any public enum or function to `emukc_battle`.
- When introducing a dead-code reservation (`#[allow(dead_code)]`).
- When touching `damage.rs` formation logic.

## Examples

```rust
/// Formation modifier for shelling and torpedo phases (shared values).
fn formation_modifier(formation: Formation) -> f64 { /* ... */ }

/// ASW formation modifier — kept separate because its values differ.
fn asw_formation_modifier(formation: Formation) -> f64 { /* Diamond=1.2 ... */ }
```

## Related

- `crates/emukc_battle/src/damage.rs` — formation modifiers.
- `crates/emukc_battle/src/simulation/mod.rs` — `simulate_day` RNG-continuity doc.
- `crates/emukc_battle/src/simulation/kouku.rs` — Stage2 simplification note.
- `docs/solutions/architecture-patterns/rng-facade.md` — the RNG consumed sequentially here.
