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
   `god_mode` and `one_hit_kill` on, and a config on port 27777 without TLS. The user's
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
- Animations run in real time: 5-6 with four battles takes about four minutes.
- A preset need not be about a battle. `air_corps_6_4` opens 中部海域 as far as 6-4 and
  leaves bombers and 設営隊 in the inventory; its steps deploy a squadron, order a sortie,
  buy a second air corps and close the panel. The client only sends
  `api_req_air_corps/set_action` when the panel closes (a click outside it), not when the
  order is changed. Its sim target is 1-1, because every entry of `PRESETS` is also
  battled by the sim gate.
- That run found the cache list short of `slot/airunit_banner`, `airunit_fairy` and
  `airunit_name` for 一式陸攻 (169): the list keeps one equipment per plane picture
  (`airunit_slot_ids`, 57 of them), but the client loads them by the equipment's own id and
  the origin has 169's. Not fixed yet; how many of the other planes the origin has is
  unknown until it is asked.

## Limits

- Not a quality gate: it needs the 7 GB resource cache, the decoded `main.js`, Playwright
  and Chrome. Run it after a change the client has to play.
- The server holds the redb lock of `z/cache`; stop your own server first.
- A page error is the only sign of a broken animation. A sprite that is merely wrong shows
  only in a screenshot someone reads.
- The air corps scenario cannot reach `supply` or `cond_recovery`: nothing costs a squadron
  aircraft or morale until the sortie side exists. The 整備Lv screen is not driven either.
- The entry module id (32875) changes with a client build; `run.py` and
  `client-runtime.ts` say so when their patch no longer matches.
