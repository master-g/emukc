# Expedition unlock table

EmuKC opens every expedition from the start. `api_get_member/mission` lists all
`api_mst_mission` entries, so the client never shows one as locked.

## Why this is out of scope

The real game unlocks expeditions step by step, mostly by succeeding at earlier
expeditions and partly by clearing maps. None of that is in data the server
receives or ships with:

- `api_start2`'s `api_mst_mission` carries times, costs and rewards, but no
  unlock condition field.
- The structured third-party sources this project already consumes (kcanotify's
  expedition data, KC3Kai, tsunkit) describe requirements and rewards, not
  unlock chains.

The only remaining source is hand-reading wiki pages and maintaining a table
by hand, which would be unverifiable and drift with every new expedition. For a
private emulator, having every expedition open is an acceptable simplification.

The server side is cheap to add if a source appears: the client treats any
mission missing from the `api_get_member/mission` list as locked (state -1), so
a server-side table only needs to filter that list and refuse
`api_req_mission/start` for locked missions. Reopen when a structured,
verifiable unlock source exists.

## Prior requests

- #120: "Expedition unlock table"
