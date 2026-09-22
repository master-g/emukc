---
title: "Investigating the live API with your own account"
date: 2026-09-22
category: best-practices
module: emukc
problem_type: tooling_decision
component: tooling
severity: medium
applies_when:
  - "A response shape cannot be settled from apilist.txt or the decoded client"
  - "Implementing an endpoint whose state machine is not documented anywhere"
  - "Deciding whether a field is optional, nullable, or simply absent"
  - "Cross-checking codex data against what the official server actually serves"
tags: [live-api, investigation, snapshot, session, port-api-key, read-only]
related_components: [emukc_crypto, main-decoder, scripts]
---

# Investigating the live API with your own account

## Context

This repo has three sources of truth about the protocol and they answer
different questions:

| Source | Answers |
| --- | --- |
| `docs/apilist.txt` | what a field is called and roughly means |
| the decoded client (`main-decoder/out/main.decoded.js`) | how the client *consumes* a field, and what it refuses |
| a live response | what the server *actually sends*, including state machines nobody wrote down |

The third one was unused until 2026-09-22. One session against the live server
settled four things that neither of the other two could: the questlist tab
semantics, the two-phase squadron removal, the `api_port` signature, and the
per-plane bauxite cost. It also cost 216 bauxite and left a squadron in a
half-removed state for a while, so the order of operations below is not
ceremony.

## The session rule

**The token lives exactly as long as the game page.** Close the page and the
next request fails, whatever it is. Plan a round of requests to run while the
page is open, and never split a read/write pair across a page close — that is
how the squadron ended up stranded mid-transfer.

`api_result: 100` with 「ブラウザを再起動し再ログインしてください」 has **two
unrelated causes** and the message does not distinguish them:

1. the session really is dead (page closed), or
2. the request is missing a parameter that endpoint requires.

`api_port/port` is case 2: it also wants `api_sort_key=5`, `spi_sort_order=2`
(the upstream field really is spelled `spi_`) and `api_port`, a signature over
the member id, the current second and three random draws. Do not read a 100 as
a session error before checking the client's `_connect` for that endpoint.

## Endpoint classes

- **Read-only, verified working** — everything in `READ_ONLY` in
  `scripts/fetch_live_api.py`: `require_info`, `mapinfo`, `slot_item`, `deck`,
  `material`, `ndock`, `kdock`, `mission`, `useitem`, `preset_deck`,
  `practice`, `remodel_slotlist`, `api_start2/getData`, and `questlist` per tab.
- **Read-only but signed** — `api_port/port`. The key comes from
  `emukc_crypto::PortApiKey` (authoritative) or the mirror in the script; both
  are pinned to the same five vectors so they cannot drift.
- **Gone** — `api_get_member/base_air_corps` answers 404 on the live server
  although apilist lists it. Its data lives in `mapinfo`.
- **Writes** — `set_plane`, `supply`, sorties, practice. Each one changes the
  account. Treat every one as a decision, not a step.

## Workflow

1. Open the game, copy any request out of the browser's network tab (it carries
   `api_token`).
2. `export EMUKC_LIVE_TOKEN=... EMUKC_LIVE_HOST=https://<server>.kancolle-server.com`
3. `python3 scripts/fetch_live_api.py --member-id <id>` — one JSON per endpoint
   under `z/snapshot/<date>/`, which is gitignored. The member id is in
   `api_port/port`'s `api_basic`; without it the script skips port.
4. **Read the current state before any write.** The response to a write is only
   interpretable against what was there before — the bauxite measurement worked
   only because `mapinfo` showed the slot had already emptied.
5. Read it back after the write, and restore whatever you disturbed in the same
   session.

## Traps this turned up

- **Squadron removal is two-phase.** `set_plane` with `api_item_id: -1` answers
  `api_state: 2` and *keeps* `api_slotid`; the radius does not change. Some time
  later it settles to `api_state: 0, api_slotid: 0` and the equipment returns to
  the inventory. While in between, port carries
  `api_plane_info.api_base_convert_slot: [<slot id>]`.
- **`api_plane_info` is optional.** With nothing mid-transfer, port omits the
  whole object rather than sending an empty one.
- **An empty list can be `null`.** `questlist` with `api_tab_id=9` and no
  active quests answers `api_list: null`, not `[]`.
- **The server filters `questlist` by tab.** Each tab returns only its own
  `api_type`; `api_count` is that tab's count, not the total.
- **Official responses send minimums, not ranges.** `remodel_slotlist`'s
  `api_req_buildkit`/`api_req_remodelkit` are the codex's `dev_mat_min` /
  `screw_min`.
- **Empty fleet slots are `{api_id: -1}`** with no other fields, the same shape
  as an empty air corps slot.

## What is still unverified

Battle responses. Every rule in `docs/battle/rules.md`, the golden transcript
and the validation gate rest on self-generated payloads. A practice battle
(`api_req_practice/*`) would produce a genuine one at no risk of sinking, which
is the cheapest way to close that gap. A sortie adds nothing a practice battle
does not, except the sinking.

Also open: whether the 12 bauxite per plane is a constant or varies by aircraft
type (only the land-based bomber was measured), and what makes `api_c_list`
appear on a quest row — it was absent from all 105 rows sampled.
