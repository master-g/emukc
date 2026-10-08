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
  - "make gear-bonus-oracle fails, or gear-bonus-corrections.json needs a new entry"
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

All three are about improvement stars, and each was settled by an oracle run:

- KC3Kai's reader looks at stars only for entries declaring `starsDist`; for the rest a
  `minStars` rule counts every copy. The port always honours stars.
- `byStars` reads the star record of the other equipment's entry and finds nothing when
  that equipment has no entry. The port reads the stars of what is carried.
- `isMultiple` is declared in the table but never read. The port honours it.

## Corrections

Unpatched, 82 of 363 entries disagree with client 6.3.5.0 somewhere. KC3Kai lags behind new
ships, limits to a few ships star bonuses the client gives to all, and cannot express some
of what the client does. `main-decoder/gear-bonus-corrections.json` is hand-maintained and
merged by the converter, per entry key: `replace` stands in for the source's rules, `append`
follows them. With it the oracle reports no difference over 2.26 million probes, and
`gear-bonus-known-diffs.json` is empty.

### Where the two disagree, the client is right

Checked on 2026-10-08 against a third table, the bonus data of noro6's 制空権シミュレータ
(`noro6/kc-web`, `src/classes/item/ItemBonus.ts`, last changed 2026-09-19), evaluated on
every probe where KC3Kai and the client differ:

- with one kind of equipment carried, it gives the client's value on all 30,853 differing
  stats, none KC3Kai's;
- with several kinds carried, the client's on 21,528, KC3Kai's on 6,424 and neither's on
  30, those 6,454 all one pair: 450 (13号対空電探改(後期型)) carried with 517. KC3Kai and noro6 give the pair
  its bonus on any destroyer, the client only next to a D-type gun of 3 stars or more.
  Nothing found settles that one; the asset follows the client.

The value that looked most like a slip in the client, evasion +6 per copy of 571
(53cm連装魚雷改) from 9 stars against KC3Kai's +1, is also what wikiwiki.jp's table of
observed values has. It was reverted to +1 for a day on that suspicion and put back. A
large or odd value in the client is no reason to prefer KC3Kai; KC3Kai lags.

Two things in the converted form exist only for corrections:

- an entry key joining ids with `+` (`286+577`) counts the copies of all of them together,
  for bonuses capped across two kinds of equipment;
- `requires` on a synergy asks for other equipment by id, with a minimum of stars or copies.

A few corrections restate a client function (286/577, 470/529, 517, 569, 578). Most were
fitted: the client's value minus the server's for every ship carrying one to three copies at
every star level, turned into per-copy, once-only and by-count rules listed by ship id, and
the same again with other equipment carried. Fitted rules explain nothing and are only known
to match on the probed loadouts — when one looks wrong, read the client function.

`make update` runs the oracle after its drift report, so a client upgrade that changes a bonus is reported
there (it does not stop the update). After a client or KC3Kai upgrade: `make gear-bonus-update`, rebuild the codex,
`make gear-bonus-oracle`, and put what differs into the corrections file. While working on
one entry, `EMUKC_BIN=target/release/emukcd bun run gear-bonus-oracle -- --only <key>`
skips the rebuild the embedded asset would otherwise trigger;
`.data/temp/gear_bonus_diffs.jsonl` lists every differing probe.

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
