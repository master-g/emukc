---
title: "Equipment stat bonuses come from KC3Kai's table and are checked against the client"
date: 2026-10-07
category: architecture-patterns
module: emukc_model
problem_type: architecture_pattern
component: service_object
severity: medium
applies_when:
  - "A ship's displayed stats differ from the real game for a particular ship and equipment"
  - "Upgrading the pinned KC3Kai commit or the game client"
  - "make gear-bonus-oracle fails"
  - "Calling Codex::ships_before_and_after or anything else that walks a remodel chain"
---

# Equipment stat bonuses come from KC3Kai's table and are checked against the client

## What

Some equipment gives particular ships extra stats (装備ボーナス). The real server adds them
to `api_karyoku`, `api_sakuteki` and so on; the client only displays a breakdown. Here
`Codex::cal_ship_status` adds them after the equipment's own stats, for seven stats:
firepower, torpedo, anti-air, armour, evasion, anti-submarine and line of sight.

```
make gear-bonus-update
  cargo run -- gear-bonus sync          # pinned commit -> .data/temp/kc3kai/<sha>/
  cd main-decoder && bun run gear-bonus # GearBonus.js + Meta.js -> assets/gear_bonus.json
  cargo run -- battle drift-check       # report what changed
cargo run -- bootstrap --codex-only     # the codex carries its own copy of the table
make gear-bonus-oracle                  # compare with the client's own bonus code
```

## Two sources, two roles

- **Data: KC3Kai** (`KC3Kai/KC3Kai`, MIT). `GearBonus.js` is one object literal: equipment id
  to conditions and grants. `main-decoder/src/gear-bonus.ts` reads the literal with Babel
  without executing anything, resolves "same as that class" aliases, turns "one or a list"
  into lists and fails on any key it does not know. The nation table comes from `Meta.js` of
  the same commit, because `byNation` rules are written against KC3Kai's own split of ship
  classes into nations.
- **Reader: a port.** `crates/emukc_model/src/codex/gear_bonus.rs` ports
  `KC3Gear.equipmentTotalStatsOnShipBonus`. The table only means something together with
  its reader — entry order, "first rule naming these ids wins" counters and the way a
  missing id restarts a sum are all reader behaviour — so the port follows the reader even
  where it looks odd.
- **Check: the client.** `SlotItemEffectUtil` in `main.js` is about 290 hand-written
  functions. It cannot be converted, but it runs as it is in Bun once
  `model.slot.getMst` is answered from the codex. `main-decoder/src/gear-bonus-oracle.ts`
  runs it and `gear-bonus probe` over about 1.2 million ship and loadout pairs.

## Where the port deliberately leaves the reader

KC3Kai's reader looks at improvement stars only for entries declaring `starsDist`; for the
rest a `minStars` rule counts every copy. The client never does that, and honouring stars
everywhere made four entries agree with it and none disagree, so the port always honours
them. Change this only with an oracle run showing the effect.

## Known differences

82 of 363 entries disagree with client 6.3.5.0 somewhere (1.8% of probes), listed in
`main-decoder/gear-bonus-known-diffs.json` with their probe counts. They are KC3Kai lagging
behind new ships and a few plain data errors on its side; the port itself was compared with
KC3Kai's original function on a sample of 92,000 probes and differs only where it
deliberately honours stars. The oracle fails on a new entry or a
changed count; `bun run gear-bonus-oracle -- --accept` re-records after a review.

Only loadouts the ship can actually carry are probed. Both sides say arbitrary things about
the rest, since the rules are written assuming the game's equip restrictions.

## Remodel chains can be circles

Rules such as `remodel: 2` need a ship's position in its remodel chain. Some ships convert
back and forth (Fletcher Mk.II, 宗谷, Glorious改), and 宗谷's three forms have no first form
at all. `Codex::ships_before_and_after` used to walk backwards until nothing came before and
never returned for those seven ships. It now collects every earlier form first and starts
from the one nothing remodels into, or the lowest id when the chain is a circle.

## Module ids

Webpack ids change with every client build, and `main-decoder/out/modules` keeps files of
earlier builds next to current ones. The oracle finds modules by the export they declare
(`Client.moduleExporting`), not by id or file name.
