---
title: "Headless client check: the real client, driven without a person"
date: 2026-10-08
category: best-practices
module: emukc
problem_type: testing
component: tests/headless
severity: medium
tags: [headless, playwright, main-js, sortie, client]
---

# Headless client check

## Problem

Whether the client plays a packet correctly used to need the user at a browser. Two cheaper
ways answer most of it, and neither needs a person.

## Read the client first

`main-decoder/out/main.decoded.js` and the cached resources under `z/cache/kcs2/resources/`
answer "what does the client do with this field" by reading. The three open points of 5-6's
transport gauge were closed that way before any browser ran: the landing spot is in
`map/005/06_info.json`, the transport counter in `gauge/00506.json`, and the conditions in
`CellTaskLanding` and `PhaseTransportResult`. Search the decoded source for the `api_` field
name, then for the getter that wraps it.

## Then run it

`make headless-check SCENARIO=<preset>` runs `tests/headless/run.py`:

1. A fresh workspace under `.data/temp/headless/<preset>/` gets a copy of the codex with
   `god_mode` and `one_hit_kill` on (off for the scenarios named in `FAIR`), and a config on
   port 27777 without TLS. The user's
   `.data/emukc.db` is never opened.
2. `new-session --scenario <preset> --no-open --no-start` creates the profile, applies the
   preset from `emukc_gameplay::scenario::PRESETS` and marks the tutorial done.
3. Headless Chrome (Playwright for Python, `channel="chrome"`) loads the game. `main.js` is
   replaced by the decoded bundle with `globalThis.__clientRequire` exposed, the same patch
   `client-runtime.ts` uses, so page code can reach the client's modules.
4. The scenario's steps click through the game. The run fails on a page error, a response
   with status 400 or more, a step that times out, or a problem the scenario's check finds
   in the saved KCSAPI responses (`api/NNN_<path>.json`).

The report also sorts the resources the client asked for: `off_site` (other hosts),
`not_in_cache_list` (asked of this server, absent from `z/cache/cache_resources.nedb`),
`fetched_from_origin` and `missing_on_origin` (from the server's log; the last fails the
run). A file can be missing from the list and still be served, because playing fills the
cache from the origin, so only the comparison with the list shows a gap. The first run
found `ship/full_animation` missing from the list and eight libraries loaded from other
sites; both are fixed (`decoder-first-cachelist-pipeline.md`, `kcs2.rs`).

A scenario is a preset plus a line of steps in `SCENARIOS`. To find the way through a new
screen, pass steps on the command line and read the screenshots:
`python tests/headless/run.py <preset> "… c:600,400 w:3 s:look"`.

## What was learned driving it

- The canvas is 1200x720 at page offset (40, 0); positions in the steps are canvas ones.
- Some buttons ignore a press unless the pointer moved onto them first, so every click
  moves, waits 200 ms, then presses for 80 ms.
- Fixed waits are flaky. `u:<spots>:<api path>` clicks the spots in turn until the client
  makes the call; one neutral spot, 進撃 and 単縦陣 together carry a fleet from one battle
  result to the next without knowing which screen is up.
- A new profile has no world, so the first screen is the server list; and the client only
  offers a map whose whole prerequisite chain is cleared, so a preset clears the chain
  rather than unlocking the one map.
- The page runs on Chrome's virtual time (`Emulation.setVirtualTimePolicy`, policy
  `pauseIfNetworkFetchesPending`): every wait in the steps is a budget of the page's own
  milliseconds, spent as fast as its timers can run and held while a request is out. The
  client's logic is `createjs.Ticker` on `setTimeout` and its drawing is PIXI on
  `requestAnimationFrame`, so the ticks all happen and only a few frames are drawn. 5-6 with
  four battles went from six and a half minutes to under one, 6-5 from ten to under one.
  Multiplying the clock instead (16x) broke 6-5: loading does not speed up, and the fixed
  waits shrank below it.
- The client has no switch for this. Its battle skip (`skip`, `skip_battle`, `SkipButton`,
  `_skipBattle`) and its `_debug` arguments are left in the release build as empty shells.
- What only time or many repetitions would produce is written into the workspace database
  mid-run: `tire:<n>` sets every squadron's condition, which is how `air_corps_6_4` gets to
  see the orange fatigue icon after one sortie.
- A preset need not be about one battle. `air_corps_6_4` opens 中部海域 as far as 6-4 and
  leaves bombers and 設営隊 in the inventory; its steps deploy a squadron, order a sortie, buy
  a second air corps, sortie 6-4 with the air corps pointed at D, fight there and resupply at
  home. The client only sends `api_req_air_corps/set_action` when the panel closes (a click
  outside it), not when the order is changed. Its sim target is 1-1, because every entry of
  `PRESETS` is also battled by the sim gate.
- A spot tapped while waiting must be harmless on every screen it may land on: a tap meant to
  close the air corps panel, repeated after it closed, chose the map underneath.
- `quest_equipment` leaves three 工廠 quests ready to claim and claims them from the quest
  list: 614 (a conversion in place), 637 (equipment taken, none given, answered as 装備消費)
  and 641 (loose equipment taken). The first click on the list after 大淀 has left does
  nothing, so the steps spend one on the header; clicks made while she is still on screen are
  lost too. The 工廠 filter makes no call, while a tab on the left asks for the list again
  (遂行中任務 answers the three quests). A reward is several screens, so 閉じる is pressed until
  the client reads the quest list again.
- `air_raid_6_5` sorties 6-5 to its boss with the gauge down two bars (`sunk:65:2` writes the
  count) and the air corps ordered to defend (`order:2`, also written: the raid needs only the
  server to know). The raid comes with one `api_req_map/next` or another, so the check looks
  through all of them and the screenshots after each step may or may not show it. 6-5's boss
  is a combined fleet: its result is `api_req_combined_battle/battleresult`.
- That run found the cache list short of `slot/airunit_banner`, `airunit_fairy` and
  `airunit_name`: the list kept one equipment per plane picture (57), but the client loads
  them by the equipment's own id. The list now names every equipment the client's
  deployment list offers (263); the origin had all 618 added files.

- `gunnery_cutin` is fought without the cheats: one battle of 2-1, the night battle if the day
  leaves it open, and home. Its outcome differs from run to run, so the check is a relation
  rather than a value: the fleet's hit points after every attack in the day packet are what
  the night packet starts from, and what is left after that is what `api_port/port` answers.
  夜戦突入 sits where 撤退 does on the next choice (770, 365), so one spot serves both.
  `leveled_for_mid_boss` is the same sortie with six destroyers, who seldom finish by day: it
  is the one that reaches the night battle. `night_air_attack` is the same sortie again with
  six night carriers, three short of ammunition and three that cannot strike by day: every
  attack of theirs in the night packet has to say it was flown (`api_n_mother_list`), and a
  空母カットイン (sp 6 whichever it is) has to show the aircraft of her place: 63 on the
  flagship (night fighter, 光電管彗星), 62 on the second (night fighter, night attacker), 64 on
  the third (night fighter, two Swordfish) and on the rest (one night fighter). It fails when
  none of the first three cuts in or the day ends the battle (2 in 30 seeds). Seven runs with
  this fleet on 2026-10-15, four of them passing: 63, 62 and 64 each played, with
  their name plates (`night4` of one run: F6F-3N, 彗星一二型(三一号光電管爆弾搭載機)), without a
  page error.
- `night_zuiun` is the same sortie led by 最上改二特 (two 20.3cm(3号)連装砲, two 夜間瑞雲) and
  三隈改二特 (the guns, one 瑞雲, a 22号対水上電探), both short of ammunition. The check wants
  any 200 of theirs to hit once or twice (once when the first sinks the last enemy), not flown,
  showing three pieces. Its first run found a page error: `CutinZuiunNightCanvas` reads the
  name plate and picture of three pieces without a check, and the row showed one or two; main
  guns now fill it (`night-attack-types.md`). Five runs of five after the fix played the three
  shapes ([490, 490, 50], [490, 50, 50], [490, 28, 50]) without a page error; `night7` of the
  fourth has 三隈's three plates.
- `night_asw` is the same sortie with six submarines, 伊401改 below the level of the opening
  torpedo and five まるゆ, who sink next to nothing: the enemy lives into the night and attacks
  them there. Every such attack has to be one plain hit, not flown, a miss has to do nothing,
  and nobody may be sunk. It fails when the enemy is sunk by day; three runs of three reached
  the night (2026-10-12), each with a miss and a scratch among the attacks. The submarines
  attack too (#64): each with more than a quarter of her hull once and plainly, played as a
  torpedo, and the flagship, who carries a late-model torpedo and a submarine radar, with the
  torpedo cut-in when it comes up, showing the torpedo and then the radar. Six runs of six
  passed, the cut-in played in five; in the sixth she came into the night at 1 HP and made
  no attack.
- `night_cut_in` is the same sortie with six destroyers who each have something to roll for
  at night (a main gun and a torpedo, two torpedoes, three main guns, the two destroyer
  cut-ins, a double attack) and a star shell on the flagship. The check wants every attack to
  be of a type her equipment makes, with the hits that type has, the mixed cut-in to show the
  gun and then the torpedo, and at least one cut-in; it fails when none comes up or the day
  ends the battle. The run of 2026-10-12 played types 2, 5 and 7 and the star shell.
- `kongou_night_assault` is the same sortie led by 金剛改二丙 with a 32号対水上電探 and
  榛名改二乙, both short of ammunition so that the enemy lives into the night. The check wants
  a row of type 104 in the flagship's turn with her hit and her partner's (hers alone if it
  sinks the last enemy) and no second row of hers; it fails when the assault does not come up
  (78 in 100) or the day ends the battle. Seven runs on 2026-10-12: played four times, once
  with two hits and three times with one, without a page error. One of them found the
  cut-in's voice (`kcs/sound/kc…/991.mp3`) missing from the cache list.
- `night_contact` is the same sortie led by 由良改二 at level 90 with a
  零式水上偵察機11型甲改二 and a 二式水戦改: the fighter takes the air from an enemy that has
  no fighter, and at her level the scout comes into contact every time, so the check is a
  value: `api_disp_seiku` 1 or 2 and `api_touch_plane` `[540, -1]`. That the client shows it
  is in the screenshots only: our scout on the enemy's radar (`rader_e.touch_plane`) with
  触接中! under it. The scout's picture appears about a second after the label and stays; the
  label blinks (0.9 s in every 1.1), so one screenshot may show either without the other
  (`night3`, `night4`, `night6` of the first run: label alone, both, picture alone). The
  request for the picture (`slot/item_up/0540_…`) proves nothing: the day's `PhaseSakuteki`
  loads the same one for the scout it sends out. Three runs of three on 2026-10-13, without
  a page error. In a sortie the enemy's scout (`api_touch_plane[1]`) cannot be played: no
  abyssal equipment in `api_mst_slotitem` has the night scout's icon (`api_type[3]` 50). An
  exercise can: the opponent's ships carry what they are built with, 川内改二 a 九八式夜偵.
  That has not been played.
- `nagato_broadside` is the same sortie in 梯形陣 (the fourth button of the formation screen)
  led by 長門改二 and 陸奥改二. The check wants one row of type 101 by the flagship with up
  to three hits, by day or, if the day did not make it, at night, and never a second; it
  fails when the attack does not come up (62 in 100 a turn). Six runs on 2026-10-12: five
  by day and one at night, without a page error.
- `enemy_night_carrier` sends 金剛 and three destroyers to 2-5, where a battleship with fewer
  than four destroyers is routed to C: its flagship 軽母ヌ級flagship attacks at night. The check
  wants an attack of hers in the night packet, not said to be flown (she has no night crew);
  the client plays it as abyssal aircraft (`night2`). It fails when the day battle ends it or
  leaves her heavily damaged. A fleet need not be six ships: the fair check counts them.
- `enemy_night_crew` sends two 古鷹 and four destroyers into 7-4 (南西海域 is the fourth tab,
  510, 680; 7-4 the fourth map, 930, 520) with `god_mode` on and `one_hit_kill` off (`UNHURT`):
  nothing hurts the fleet and it sinks next to nothing, so it gets four battles deep and the
  enemy there is whole. Two heavy cruisers are sent to C, four destroyers from there to E, and
  too little line of sight from J to K and so to M (cells 3, 17, 10, 19, 13). 追撃せず sits
  where 進撃 does, so the same taps carry the fleet past each battle that the day leaves
  open. At M it goes into the night against ヒ船団棲姫, who has a night crew of her own and
  night aircraft: the check wants an attack of hers there with `api_n_mother_list` 1. It fails
  when M is led by another (4 in 100). Three
  runs of three on 2026-10-13, the same cells each time, without a page error; the client
  plays her attack as aircraft over the fleet (`night1`). Since enemies cut in (2026-10-15)
  she makes 62 now and then (third run of three): sp 6, shown [1617, 1621, 1619], played as
  three bands, 艦戦, 艦爆 and 艦攻 (`night5`). The 艦爆 is her 深海戦爆複座鷹改, which forms no
  part of 62; the display takes a fighter, dive bombers and torpedo bombers whatever made it.
- `anti_air_cut_in` is the same fleet with 秋月 in the sixth place, sent to 2-5 (the extra
  operation sits where 6-5 does on its page, 700, 275), whose first battle meets carriers. The
  check is the fair one plus the `api_air_fire` she should send; it fails about 7 times in a
  hundred, when none of her three kinds fires. The cut-in itself is on `air1`–`air8`, taken
  a second apart from the sixteenth second of the battle.
- `escort_1_6` sends a light cruiser and five destroyers through 1-6 (the second banner of
  鎮守府海域's extra operations, 660, 420) with six arrivals already counted (`sunk:16:6`): A,
  E, the resource cell G, the air battle F, B and the goal N. The check wants that route,
  燃料 1000, the プレゼント箱 and 75 in the goal's answer, and the screenshots show the client
  playing it: the fuel balloon ×1000 and the last notch of the gauge (`goal3`), the gauge gone
  (`goal10`), 「特別戦果 +75」 (`bonus4`). Fuel in port went from 11000 to 12000. Four runs of
  four on 2026-10-13, without a page error.
- `recon_6_3` sends 秋津洲 with two 二式大艇, 阿武隈 with two 零観 and four destroyers through
  6-3: C, E, the reconnaissance cells G and H, the boss J. The check wants a great success by
  flying boat with one thing found at each, and headquarters to gain in port exactly what
  was found. `h4` has the client's balloon: 航空偵察 大成功!, the item ×3, 作戦完遂時にGet!
  The shots at G (`g1`–`g8`) held a frame from the first step instead; nothing was drawn
  between. A is a choice of route (#106): the start offers B and C, the client shows
  「艦隊針路選択可能!」 and 「艦隊の針路を選択できます。」 (`choice`), and the scenario taps
  C where `kcs2/resources/map/006/03_info.json` puts cell 3. The check wants the offer and
  `api_cell_id=3` in what the client sent (every request is saved beside its response, as
  `.request`). `leaving` has the ship under way to C with no compass.

## Limits

- Virtual time waits for requests only, not for image decoding or audio, and `Date` runs on it
  too: the client's clock ends a run minutes ahead of the server's, which a scenario about
  expeditions or repairs would have to mind. Between steps the page is frozen.
- Not a quality gate: it needs the 7 GB resource cache, the decoded `main.js`, Playwright
  and Chrome. Run it after a change the client has to play.
- The server holds the redb lock of `z/cache`; stop your own server first.
- A page error is the only sign of a broken animation. A sprite that is merely wrong shows
  only in a screenshot someone reads.
- The air corps scenario does not reach `cond_recovery` (it has no rest step) or the
  整備Lv screen.
- The quest scenario does not show a locked piece's warning (`api_invalid_flag`), and it
  cannot see what the client draws in the flagship's slots after a claim.
- A scenario takes 15 to 50 seconds on its own (`fresh_1_1` 14 s, `gunnery_cutin` 15 s,
  `transport_5_6` 48 s, measured 2026-10-10 on a loaded machine); `cargo build` in front of it
  is a few seconds when nothing changed. A check that takes minutes is not being slow, it is
  stuck: one run sat on the world-select page for 47 minutes, past every step deadline, in a
  browser call that never returned. `run.py` now stops a run with no result after `LIMIT`
  (300 s), names the step it stood at and the last KCSAPI call, and stops its server. It could
  not be made to happen again; start from that step if it does.
- The entry module id (32875) changes with a client build; `run.py` and
  `client-runtime.ts` say so when their patch no longer matches.
