---
title: "Client views: gameplay returns a domain structure, the handler projects it"
date: 2026-09-19
category: architecture-patterns
module: emukc_gameplay
problem_type: architecture_pattern
component: service_object
severity: high
applies_when:
  - "Adding a KCSAPI endpoint that reads several domains at once"
  - "Moving read ordering or settlement out of a handler"
  - "Deciding whether gameplay should return wire types or domain types"
tags: [view, projection, handler, transaction, port, require_info, questlist]
related_components: [emukc_db, emukc_model]
---

# Client views: gameplay returns a domain structure, the handler projects it

## Context

`api_port`, `api_get_member/require_info` and `api_get_member/questlist` each
read most of a profile. The handlers did that themselves: they opened no
transaction but called a dozen `Ctx` methods in a specific order, and that order
carried gameplay invariants — the port handler had to clear stale sortie state
before reading, and settle material self-replenish before reading materials. A
gameplay invariant expressed as statement order inside the binary is invisible
to gameplay tests and impossible to reuse.

## Guidance

### One operation per view

Each view is one `Ctx` method returning one domain structure:

```rust
pub async fn port_view(&self, profile_id: i64) -> Result<PortView, GameplayError>
pub async fn require_info_view(&self, profile_id: i64) -> Result<RequireInfoView, GameplayError>
pub async fn quest_list_view(&self, profile_id: i64, tab_id: i64) -> Result<QuestListView, GameplayError>
```

They live in `game/view/` and their types (`PortView`, `RequireInfoView`,
`QuestListView`, `QuestListItem`) reach the prelude through `game::types`.

### The structures are domain types

Fields are named in domain vocabulary — `basic`, `materials`, `fleets`,
`ndocks`, `ships`, `port_bgm_id`, `combined_type` — never with an `api_` prefix,
and the view never constructs a `Resp`. The handler is reduced to one call plus
a `project(view) -> Resp` function, which is also where the handler's own
constants stay (`api_log`, `api_c_flags`, `api_event_object` and the
`TODO(#0)` placeholders): they are presentation, not state.

This follows `api_req_map/projection.rs`. Sortie and practice do the opposite —
gameplay builds the wire struct directly — and that inconsistency is known and
deliberately left alone; new views SHALL follow the projection form.

### The order is part of the operation

`port_view` fixes it: clear stale sortie state → `find_profile` +
`update_materials_impl` → read basic, materials, fleets, ndocks and game
settings in one transaction → commit → `self.get_ships`. Ships stay outside the
transaction because `Ctx::get_ships` owns the `api_onslot_max` /
`api_sp_effect_items` filling, which is private to the `ship` module and has no
`_impl`; that is the same position the handler read them from before.
`require_info_view` has the same shape with `get_furnitures` in that role.

### Derived state moves with the read

`quest_list_view` owns the tab filtering, the counts and the `state` /
`progress_flag` / `quest_type` derivation that used to sit in the handler. Tab
ids are the client's tab bar (1 daily, 2 weekly, 3 monthly, 4 oneshot, 5 other,
9 activated), while `label_type` is `api_label_type` (1 oneshot, 2 daily, 3
weekly, 6 monthly, 7 quarterly, 101..=112 yearly), so `tab_shows` maps one to
the other. The move first kept the old `label_type == tab_id` compare, pinned by
a test, and fixed it in its own commit.

## Rationale

- The invariant "settle before you read" becomes testable:
  `view/port.rs::port_view_clears_pending_sortie_state` drives a real sortie into
  a pending battle and asserts the store is clean after `port_view`.
- One transaction per view instead of a dozen independent connections gives the
  client a consistent snapshot.
- A domain return type means the next consumer (a CLI dump, a second API
  version) does not have to go through the wire shape.

## Anti-patterns

- Sequencing `Ctx` calls in a handler because "it is just a read". If the order
  matters, it is gameplay.
- Returning `KcApi*` types from a view operation. Project in `src/bin/`.
- Fixing incidental oddities (the empty tab 9) while moving code. Move first,
  pin the behavior, change it in its own commit.

## Applies when

- A new endpoint reads more than one domain.
- An existing handler grows a second statement that is not projection.

## See also

- `docs/plans/2026-09-18-002-refactor-deepen-shallow-modules-plan.md` — KD4,
  KTD5, AE7; unit U7.
- `gameplay-context.md` — why the operation is an inherent method on `Ctx`.
