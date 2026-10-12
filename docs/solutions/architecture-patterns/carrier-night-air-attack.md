---
title: "The carrier night air attack: who flies, with what power, and what the client is told"
date: 2026-10-11
category: architecture-patterns
module: emukc_battle
problem_type: architecture_pattern
component: service_object
severity: medium
tags: [battle, night, carrier, night-air-attack, proficiency, n-mother-list]
applies_when:
  - "A carrier attacks at night when she should not, or does not when she should"
  - "A carrier's night damage looks like a surface ship's"
  - "The client draws a carrier's night attack as gunfire, or a gunship's as aircraft"
  - "A night aircraft, a built-in 夜戦特性 or a night-shelling carrier is added"
---

# The carrier night air attack

## What

A carrier attacks at night in one of two ways, and otherwise not at all
(`targeting.rs` `can_attack_night_ship`):

- **By air** (`simulation/night.rs` `can_night_air_attack`). She needs a night air group —
  夜間作戦航空要員 or a built-in 夜戦特性 (`EXEMPT_NIGHT_CV_IDS`), and a 夜間戦闘機, 夜間攻撃機
  or 夜間爆戦 with aircraft left in its slot — or to be Ark Royal with Swordfish. And a deck
  that works: more than half her hull, more than a quarter for an armoured carrier.
- **By gunfire** (`NIGHT_SHELLING_CV_IDS`): Graf Zeppelin, Saratoga, 大鷹改二 and the like
  shell as a surface ship does until heavily damaged, with the ordinary night formula.

光電管彗星, 岩井爆戦 and Swordfish fly with a night air group and count in its power and its
cut-ins, but do not make one. So 要員 with ordinary bombers is not enough: such a carrier
stays out of the night, where she used to attack.

## Where the rules come from

`KC3Kai/kancolle-replay` at `69097abc`: `kcships.js` `hasNBAirGear`, `canNBAirAttack`,
`CV.canNB`, `CV.NBPower`; `kcsim.js` 869-903 for the proficiency and the packet flag;
`kcSHIPDATA.js` for `hasBuiltInNightCrew` and `nightattack`. The client's own
`_getNormalAttackType` (night) keeps the list of carriers it draws as gunfire, which is the
source's `nightattack` plus Wasp and Wasp改. The source predates those two; that they shell at
night is from the wiki's Wasp page (「火力初期値が1以上のため夜間砲撃を行う。改も同様」). All eleven
are in `NIGHT_SHELLING_CV_IDS`. The wiki's reason is not a rule to derive the list from: nine other
carriers in the master data start with firepower above zero. Six of them (Langley, Independence,
Gambier Bay Mk.II and their remodels) the client does not draw as gunfire, so the list stays a
list.

The other three are Béarn's forms, and they are an open question: right after the eleven the
client draws Béarn (1055) and Béarn改 (1060) as gunfire, and Béarn amélioration (1061) as
gunfire or, with a torpedo, a torpedo attack. That it has an animation for them suggests they
shell at night; nothing read so far says so outright (the wiki's Béarn page does not mention
night battle). They are not in `NIGHT_SHELLING_CV_IDS`, so without a night air group they do not
attack at night. Issue #55 takes this up.

## The power

`night_air_power` replaces `火力 + 雷装 + 5 + 改修` for an attack by air; the night scout,
the cut-in's multiplier, the cap and the ammunition follow as for any night attack.

- The hull's own firepower and torpedo: master data plus 近代化改修 (`api_kyouka`). Not
  `api_karyoku`, which also holds the equipment and its fit bonuses.
- For each night aircraft: its firepower, `sqrt(改修)`, its torpedo if it is a 艦攻 and its
  bombing if it is not, 3 for each aircraft in the slot, and
  `0.45 x (火力 + 雷装 + 対潜 + 爆装) x sqrt(aircraft)`.
- 光電管彗星, 岩井爆戦 and Swordfish: no count term, and 0.3 for 0.45.
- Ark Royal without a night air group: her Swordfish' firepower, torpedo and `sqrt(改修)`.

## Proficiency

An attack by air takes the ship's `plane_proficiency`, the same three numbers the day shelling
of a carrier takes: accuracy and critical rate in `roll_attack`, critical damage in
`calculate_night_damage`. One draw an attack, as everywhere.

The day attack on a submarine takes it too (`asw_proficiency`), for the ships whose aircraft
make it — 航巡, 航戦, 水母, 揚陸艦, 軽空母 — and not in the opening (`Aim::in_the_opening`).

## Whom she attacks

A carrier flying her night air attack goes for the surface ships and takes a submarine only once
none is left (`nightPhaseTarget`; `AttackCapability::BothPreferSurface`). A carrier that shells
takes the submarine first, as any ship does.

Which carriers may attack a submarine at night at all is `canASWNight`: a light carrier whose own
anti-submarine is above zero, aircraft or not (`bare_asw`: grown with her level plus 近代化改修,
without equipment or what it grants); a fleet or armoured carrier never. The attack still lands
for scratch damage and is not flown.

Two guns between main and secondary make a double attack (`NBtypes`), so Graf Zeppelin with two
secondary guns makes one. That is her gunfire only: a carrier flying with a night air group has
the carrier cut-ins and no other attack type (`CV.NBtypes`), so with the same two guns and a
night air group she flies a plain night air attack, flag 1.

## What the client is told

`api_n_mother_list` is 1 for an attack by air. The client keeps it as `yasen_kubo` and, when
set, draws the attack as aircraft (`PhaseAttackKansaiki`) whatever the ship is. The cut-in
carries it as well. It is 0 for a double attack and for an attack on a submarine, as in the
source.

`night_turn` decides once whether the attack is flown and hands that on: the power goes to
`calculate_night_damage` as `air_power`, the roll gets `Aim::as_night_air_attack`. `accuracy.rs`
and `damage.rs` do not ask `simulation::night` themselves.

## How it is checked

- `night.rs` tests: who flies (crew, aircraft, empty slot, Ark Royal, the two hull limits),
  who shells, one power worked by hand, the proficiency on the damage and on the draw, the flag.
- The `night_air_attack` preset (six 赤城改二戊 with night fighters, a night attacker on the
  flagship) is in the sim→validate gate with `PhaseExpectation::NightAirAttack`: the night gate
  asserts that this preset flies and that no other does.
- `make headless-check SCENARIO=night_air_attack` fights 2-1 with it in the real client. Five
  of the six carry nothing that strikes by day, so the battle reaches the night.

## Known gaps

- しまね丸改, a 補給艦 the source lets fly at night: every carrier test here goes by ship type.
- Enemy carriers: an enemy is built from the manifest without equipment slots, so none flies,
  and the abyssal `nightattack` ships are not listed. They do not attack at night, as before.
- The fit bonuses the source adds to a carrier's torpedo (`eqBonusCV`), no torpedo against a
  land target, and the special-aircraft multipliers.
- A night attack on a submarine still always lands for scratch damage, so proficiency reaches
  the day attack on a submarine only.
- A main gun with a torpedo counts as a double attack here; the source makes it a cut-in
  (`NBtypes` 1440). A failed cut-in falls back on a double attack for a ship with a main gun;
  the source has no such fallback. Both wait for the night cut-in table to be redone.
