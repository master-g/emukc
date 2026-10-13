---
title: "Resource cells and maelstroms: a hand-kept table by cell, pinned onto the map catalog"
date: 2026-10-13
category: architecture-patterns
module: emukc_gameplay
problem_type: architecture_pattern
component: service_object
severity: medium
tags: [sortie, map, resource-cell, maelstrom, itemget, happening, landing-craft, drum, rng]
applies_when:
  - "Changing what a resource cell gives or a maelstrom takes"
  - "A map's cells changed and assembling the catalog fails on cell effects"
  - "A seeded sortie that passes a resource cell or a maelstrom moved"
  - "Changing when what a sortie gathered reaches headquarters"
---

# Resource cells and maelstroms: a hand-kept table by cell, pinned onto the map catalog

## What

What a resource cell (`event_id` 2) gives and a maelstrom (`event_id` 3) takes is written
down cell by cell in `crates/emukc_bootstrap/assets/map_cell_effects.json`, by map name and
node label. Assembling the map catalog pins it onto cell numbers
(`map_cell_effects.rs` `apply_cell_effects`, `MapCatalog::cell_effects`), and the sortie
reads it there (`sortie/cell_effect.rs` `resolve_cell_effect`), on the first cell of a
sortie as on any later one.

Nothing is read from the cell's colour or its `event_kind`: every resource cell is drawn
with colour 2 and every maelstrom with 3, and 1-6 G has `event_kind` 4.

## The asset

- Hand-maintained, like `map_limited_drops.json`: no machine-readable source has these.
  After a change: `cargo run -- bootstrap --codex-only`, then `make drift-accept`.
- Assembly fails both ways: a resource cell or maelstrom of a regular map without an entry,
  and an entry for a cell that is not one or is the other kind.
- `guess` on an entry says what it does not have from its source.
- `landing_craft` and `drums` are equipment ids with their names; a test holds the names
  against the master data.

## Where the figures come from

| What | Source |
| --- | --- |
| Kind and range of 52 resource cells; kind, rate and cap of 25 maelstroms | wikiwiki's seven area pages, read 2026-10-13 |
| The bonus of a drum and of a landing craft; the maelstrom formula, the radar table, the strong maelstroms | wikiwiki 資材 |
| 7-4 O | zekamashi 「７－４－O(港マス)」; its bases 40 and 20 are worked back from its two examples |
| What counts as a landing craft | the 資源マス column of wikiwiki 装備最大値/大発系装備早見表/テーブル |
| The wire values | the client's `DropItemModel` and `HappeningModel` |

Ours, not a source's:

- A figure within a range is drawn evenly in steps of 5. The sources say 5刻み and no more.
- A strong maelstrom comes one time in ten (`STRONG_MAELSTROM_PERCENT`); wikiwiki has
  「1割ほど」.
- 4-4 D, 7-3 K and 7-3 L take 30% of fuel without a cap; the source has question marks.
  Where it could not confirm a cap there is none.
- 5-1 H has the general bonus of 2 and 3, the source naming none; 7-3 O has bauxite's 1.5
  and 2 where the source prints 2 and 3 with a question mark. 2-5's two cells have no
  bonus: the general one is for maps other than the extra operations.
- 1-6's 「大発動艇搭載数」 counts every landing craft, as the other cells do.
- One cap a maelstrom: where the source measured both the plain and the strong one
  (1-3 H, 3-3 F, 6-2 D) they are the same figure.
- 3-3 C's cap is the area page's 40; 資材's worked example has 30 and defers to the area
  pages.

## Rules

- **A gain** is its base, plus `per_drum` and `per_craft` for each drum and landing craft
  in the fleet (the sum rounded down), `cap` at most. A gain whose bounds are one figure
  draws no random number.
- **Fuel, ammunition, steel and bauxite reach headquarters in port.** They are put by in
  the sortie store (`SortieStore::add_pending_gain`), apart from the active sortie, and the
  port view adds them (`port_view`). The store is written after the transaction that
  settled the cell is through (`put_by`), and emptied after the one that added to
  headquarters; a sortie that begins drops what an earlier one never brought home. Special
  materials and furniture boxes are the player's at once. A page reloaded at sea forfeits them, as upstream: the client asks for
  `require_info` when it starts (`RequireInfoAPI`, chained after `api_start2/getData`) and
  at no other time, so `require_info_view` throws away what was put by. A retreat and a
  sortie fought to its end both come home by `api_port/port` alone; `goback_port` is the
  escort withdrawal.
- **A maelstrom** takes of each ship `left × rate × (100 − spared) / 10000`, in whole
  numbers, her cap at most. `spared` is 0, 25, 40, 50, 55, 58, 60 by how many ships carry a
  radar; a ship counts once and her reinforcement slot does not count. The loss reported is
  that of the ship that lost most, which is what the client shows.
- **Random numbers:** one for a gain with a range, one for a maelstrom that can be a strong
  one, none anywhere else.

## The wire

- A material is `api_usemst` 4 with `api_id` and `api_icon_id` 1 to 7 (fuel, ammunition,
  steel, bauxite, 高速建造材, 高速修復材, 開発資材): the client looks the icon up in a table
  of its own, in `api_material`'s order. A furniture box is `api_usemst` 5 with its
  `api_mst_useitem` id (10, 11, 12). `docs/apilist.txt` has 11 and 12 as `api_usemst`
  values; the client's `DropItemModel` would show nothing for those.
- A maelstrom's `api_mst_id` is 1 or 2; the client reads the kind from it and from nothing
  else. It was 0 before, which the client's table turns into −1.

## Known gaps

- What was gathered is in memory with the rest of a sortie: a server restart loses it.
  Kept so (owner's decision, 2026-10-13): the sortie itself is lost with it, the client
  has to reload, and a reload voids what was gathered anyway.
- No cap on the bonus of drums and landing craft at an ordinary cell, and the even draw
  within a range, for want of a source.
- 6-3's reconnaissance cells, 5-6's landing cell and the escort-success cells
  (`api_itemget_eo_*`) have rules of their own that are not here (#100).
- Event maps, and a combined fleet's two ways of sparing its ships a maelstrom.
- Played in the client on 2026-10-13, by hand-given steps on the `transport_5_6` preset and
  without a page error: 4-1 B (the fuel icon and ×113, and 113 more fuel in port), 2-3 H
  (43 ammunition, in port likewise), 3-3 F (「うずしおが発生しました！」) and 3-3 H (the
  furniture box's icon and ×1, which is the `api_usemst` 5 path). Not played: a special
  material, a strong maelstrom, 7-4 O's two items, and what the client shows of a
  maelstrom's loss and of the radar. There is no scenario for it: which cells a fleet
  comes to is the compass's choice (3-3 F leads to H in about three runs of eleven).

## When a test breaks

- Assembly says a cell has no entry: the map gained a resource cell or a maelstrom; add it
  to the asset from the area page.
- A seeded sortie moved: a gain with a range and a strong maelstrom's cell each draw once.
  Move the seed, not the assertion.
- `tests/gameplay_tests/map/cell_effects.rs` parks the fleet on a cell before the one under
  test and steps until the compass takes it there. A cell the test fleet's composition
  cannot reach (4-3 E, 3-3 C, 5-4 K with six 951s) never comes up; pick another cell or
  another fleet.
