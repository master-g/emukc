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

## Limits

- Not a quality gate: it needs the 7 GB resource cache, the decoded `main.js`, Playwright
  and Chrome. Run it after a change the client has to play.
- The server holds the redb lock of `z/cache`; stop your own server first.
- A page error is the only sign of a broken animation. A sprite that is merely wrong shows
  only in a screenshot someone reads.
- The entry module id (32875) changes with a client build; `run.py` and
  `client-runtime.ts` say so when their patch no longer matches.
