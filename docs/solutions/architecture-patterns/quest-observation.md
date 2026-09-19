---
title: "Quest observation: domain modules report outcomes, one function advances quests"
date: 2026-09-19
category: architecture-patterns
module: emukc_gameplay
problem_type: architecture_pattern
component: domain_rule
severity: high
applies_when:
  - "Adding a gameplay operation that could advance a quest"
  - "Adding a new quest condition or QuestActionEvent variant"
  - "Deciding where in a transaction quest progress should be written"
tags: [quest, observe, gameplay-outcome, transaction, exhaustive-match]
related_components: [emukc_model]
---

# Quest observation: domain modules report outcomes, one function advances quests

## Context

Nine domain modules each called `update_quest_progress_for_action` themselves,
several of them from inside an `_impl` that another domain reused within the
same transaction. Answering "what advances a quest?" meant grepping the whole
crate, and a reused `_impl` could advance the same quest twice in one write —
for example a ship drop path calling `add_ship_impl`, which advanced the
construction quests on its own.

## Guidance

### One entry point

`game/quest/observe.rs` defines `GameplayOutcome` — one variant per
`QuestActionEvent`, same fields, no `api_` vocabulary — and

```rust
pub(crate) async fn observe<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
    outcomes: &[GameplayOutcome],
) -> Result<(), GameplayError>
```

It is the only caller of `update_quest_progress_for_action` outside the quest
module's own files, so that one file answers what advances a quest. The
`match` from `GameplayOutcome` to `QuestActionEvent` is exhaustive: a new
variant that is not mapped does not compile.

### Where the call goes

The `Ctx` method that owns the transaction SHALL call `observe(&tx, ..)` exactly
once, after its domain writes are done and before `tx.commit()`:

```rust
let tx = db.begin().await?;
let (resp, outcomes) = supply_fleet_impl(&tx, codex, profile_id, ..).await?;
observe(&tx, codex, profile_id, &outcomes).await?;
tx.commit().await?;
```

`observe` takes a slice, not a single outcome, because one write can produce
several facts: `create_slotitem` builds up to three items, `charge_supply`
resupplies each ship, `destroy_ship` produces a `ShipScrapped` plus one
`SlotItemScrapped` per equipment, and a sortie result produces
`SortieBattleCompleted` plus one `EnemyShipSunk` per sunk enemy.

### `_impl` functions return outcomes, they do not observe

An `_impl` is reusable inside someone else's transaction, so it MUST NOT touch
quest progress. It returns its outcomes instead — `destroy_items_impl` returns
materials and outcomes, `supply_fleet_impl` / `powerup_impl` /
`ndock_start_repair_impl` / `speed_up_ship_repairation_impl` likewise, and
`settle_sortie_battle_impl` puts them in `SortieSettlement.outcomes`. When one
domain's `_impl` is nested inside another's, the outermost transaction owner
merges the outcomes and observes once.

This is what makes the position safe: `update.rs`'s progress evaluation reads
only the quest progress tables and the codex, and `progress_after_event` is pure
arithmetic, so it does not depend on any intermediate state of the other tables
written earlier in the transaction.

### Variants without a producer stay

`GameplayOutcome::SlotItemImproved` has no producer. Four quests (618, 619,
1166, 1167) require slot item improvement and the matcher already handles them;
`api_req_kousyou/remodel_slot*` is simply unimplemented (P1 in
`docs/api_coverage.md`). The variant and the header note stay so the mapping is
there when the handler lands. Deleting an unproduced variant deletes quest
support for a feature that is only missing, not cancelled.

## Rationale

- Locality: one file, one exhaustive match, one grep.
- Double-advance is structurally impossible once `_impl`s stop calling quest —
  the only observer is the transaction owner, and it runs once.
- The compiler, not a review checklist, enforces that new outcomes are mapped.

## Anti-patterns

- Calling `update_quest_progress_for_action` from a domain module. Return a
  `GameplayOutcome`.
- Calling `observe` from inside an `_impl`, or twice in one `Ctx` method.
- Observing after `tx.commit()`. Quest progress then survives a rollback of the
  write that caused it.
- Deleting a producer-less variant to silence a warning.

## Applies when

- Any new `Ctx` method that writes something a quest could count.
- Any new `QuestActionEvent`.

## See also

- `docs/plans/2026-09-18-002-refactor-deepen-shallow-modules-plan.md` — KD3,
  KTD6, AE8; unit U8.
- `quest.md` — the quest tree, conditions and rewards.
- `sortie-settlement.md` — the settlement that hands its outcomes back.
