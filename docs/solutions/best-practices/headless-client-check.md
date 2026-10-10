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
  is the one that reaches the night battle.
- `anti_air_cut_in` is the same fleet with 秋月 in the sixth place, sent to 2-5 (the extra
  operation sits where 6-5 does on its page, 700, 275), whose first battle meets carriers. The
  check is the fair one plus the `api_air_fire` she should send; it fails about 7 times in a
  hundred, when none of her three kinds fires. The cut-in itself is on `air1`–`air8`, taken
  a second apart from the sixteenth second of the battle.

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
