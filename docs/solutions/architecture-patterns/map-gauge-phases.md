---
title: "Multi-gauge regular maps are one variant per phase, cut at KCNav's boundaries"
date: 2026-10-07
category: architecture-patterns
module: emukc_bootstrap
problem_type: architecture_pattern
component: service_object
severity: medium
applies_when:
  - "A regular map gains a gauge, a phase or an unlock condition (5-6, 7-2, 7-3, 7-5 today)"
  - "A boss win does not move a gauge, or moves the wrong one"
  - "A save from before a map had phases shows the wrong part of the map"
  - "Routing rules fail to apply to a map with phases, or make route-oracle skips nodes"
---

# Multi-gauge regular maps are one variant per phase, cut at KCNav's boundaries

## What

5-6, 7-2, 7-3 and 7-5 are played in phases. Each phase has its own boss and its own part of
the map; emptying a gauge opens the next. In the catalog a phase is a variant
(`MapVariantDefinition`), and the phases of a map are chained by `clear_to_variant_key`.

| Map | Phases | What moves it on |
| --- | --- | --- |
| 5-6 | `phase1`…`phase4` | G ×3 (placeholder, see below) → reach R → N ×2 → Z ×3 |
| 7-2 | `phase1`, `phase2` | G ×3 → M ×4 |
| 7-3 | `pre_p_unlock`, `post_p_unlock` | E ×3 → P ×4 |
| 7-5 | `phase1`…`phase3` | K ×2 → Q ×3 and one S rank at M, in either order → T ×3 |

All four refill on the first of the month and start over from the first phase.

## Where the data comes from

- **Where a phase ends** is in KCNav's map list (`maps/all/meta`, saved as
  `.data/temp/kcnav/meta.json`): `breakpoints` holds the first edge of each later phase, and
  an edge id is our `cell_no`. Phase *k* is the cells numbered below `breakpoints[k]`.
- **What moves a map on** has no machine-readable source. It is written by hand in
  `assets/map_gauge_rules.json`, with the pages it was read from in the file's note.
- `kcnav normalize` joins the two into `assets/map_gauge_phases.json`, which is what the
  build reads. It refuses a map the two disagree about: a multi-phase map without rules, or
  a phase count that differs.

`map_gauge_rules.json` is the file to edit. `map_gauge_phases.json` is generated.

## How a map is cut

`map_gauge_phases::apply_gauge_phases` runs in `assemble_final_map_catalog` after the cell
kinds are applied (a phase's boss is the boss cell that phase adds, which is only known then)
and before the label overlay and the routing rules (which pin things onto cells and must see
the phases as played).

- A map with one whole-map variant `""` is cut: each phase keeps the cells below its
  boundary, and `next_cells` is trimmed to them. `""` is removed.
- A map that already has every phase as a variant (7-3) is only checked against the
  boundaries and wired. Its variant names stay, so saves keep working.
- `boss_cell_no` becomes the boss the phase adds. Earlier bosses stay on the map as dead
  ends but no longer count: `boss_cell_nos()` is what gauge progress, the end of a sortie and
  the quest event's `boss_cell` all read.
- The map becomes `Monthly`, and `gauge_count` is the number of phases with a gauge.

Enemy fleets and ship drops are keyed by node label and go to every variant that has the
label, so each phase gets its own part with no extra work. After a rebuild, `kcnav
normalize` writes those two assets keyed by the phase variants.

## Three ways to move on

`MapVariantDefinition` carries them; `game/sortie_result.rs` plays them.

- `required_defeat_count`: boss wins that empty the gauge (`apply_sortie_map_result`).
- `advance_on_reach`: cells whose arrival opens the next phase (`advance_stage_on_reach`,
  called from `next_sortie`). Such a phase has no gauge: boss wins there do nothing, and
  `mapinfo` shows the gauge of the phase it leads to.
- `advance_needs_s_rank_at`: cells one of which must have been won with an S rank. The win
  is remembered in the record's `event_state` (`STAGE_UNLOCKED`); an emptied gauge waits
  until it is there, and the S rank opens the next phase at once if the gauge is already
  empty (`record_stage_unlock`). Entering a phase clears the flag.

`MapDefinition::chained_gauge(stage)` gives the gauge number and length `mapinfo` reports.
The number counts gauges, not phases, so 5-6's four phases report 1, 2, 2, 3.

## Routing rules per phase

The compass simulator writes every phase's rules for the whole map. `apply_route_rules`
takes the map's last phase as `whole`: the rules must fit that strictly, and an earlier
phase drops whatever it has no cells for yet. A visited-node condition is resolved against
the whole map, since a cell the phase lacks is simply never visited.

- 5-6: source phases 1, 2, 3 go to `phase1` + `phase2`, `phase3`, `phase4`. `phase2` is the
  route-opening phase, which the source does not model; it plays by phase 1's rules.
- 7-2 and 7-5 have one set of rules, which goes to every phase (`fan_out_variant_keys`).

`make route-oracle` compares 5-6 `phase1`, `phase3`, `phase4`, both 7-3 variants, and the
last phase of 7-2 and 7-5. It skips source nodes our phase has no open edge from.

## Saves from before a map had phases

`resolve_record_stage_id` treats a `stage_id` that names no variant as absent. A cleared
record then resolves to the last phase, so the map stays cleared; any other resolves to the
first. Nothing is rewritten until the record next changes.

## Known gaps

- **5-6 phase 1 is a placeholder.** The real gauge is a transport gauge of 280 TP, emptied
  by A or S ranks at G in proportion to what the fleet carries. Transport points are not
  modelled; three wins at G stand in, marked provisional in `map_gauge_rules.json`.
- **A gauge counts boss wins of rank B or better**, as every gauge map here does. The real
  game counts sunk boss flagships.
- What `mapinfo` reports during 5-6's route-opening phase (gauge 2, 0 of 2) is a guess; no
  capture of that state exists.
