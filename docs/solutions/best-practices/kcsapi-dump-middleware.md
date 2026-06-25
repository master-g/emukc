---
title: "KCSAPI dump middleware: opt-in dev capture whose output carries api_token and full profile"
date: 2026-06-25
category: best-practices
module: emukc
problem_type: operational_safety
component: middleware
severity: medium
applies_when:
  - "Enabling EMUKC_KCSAPI_DUMP / make serve-dump to capture KCSAPI traffic"
  - "Choosing a dump output path, or sharing/committing a captured dump"
  - "Reordering the kcsapi route_layer stack"
tags: [kcsapi, dump, middleware, dev-tooling, security, api-token, gitignore]
related_components: []
---

# KCSAPI dump middleware: opt-in dev capture whose output carries api_token and full profile

The mechanics are self-documented in `src/bin/net/router/kcsapi/mod.rs:107-120`
(env `EMUKC_KCSAPI_DUMP`: unset/empty = off; `1`/`on`/`true` = write
`.data/logs/kcsapi_dump.jsonl`; any other value = output path). This note only
records what that doc comment does **not** say.

## Guidance

### The dump is sensitive data, unredacted

Each JSONL record stores the **raw** request form body and the **raw**
`svdata=`-prefixed response. That means a dump contains the player's
`api_token` (in request bodies) and complete profile/game state (in responses),
verbatim — there is no redaction. Treat a dump file like a credential.

### Default path is safe; custom paths are on you

The default `.data/logs/kcsapi_dump.jsonl` is safe because `/.data` is
gitignored (`.gitignore:19`). The "any other value = path" mode lets you point
the dump anywhere — if you point it inside a tracked directory you will commit
tokens and profile data. Keep custom paths under `.data/` or another ignored
location, and never share a raw dump.

### Enable via `make serve-dump`

`make serve-dump` (`Makefile:54`, `DUMP ?= .data/logs/kcsapi_dump.jsonl`) is the
intended entry point — it just sets `EMUKC_KCSAPI_DUMP=$(DUMP)` before
`cargo run`. Override the file with `make serve-dump DUMP=...`.

### It's a single-user dev tool, and it's the outermost route_layer

Per its own `ponytail:` note it does blocking append per request with no
rotation or locking — fine for one dev, not for multi-client capture. It is the
last-registered `route_layer` (`mod.rs:72`), i.e. the **outermost** wrapper, so
it captures the final, **uncompressed** response after mocking/content-type
layers. Reordering the route_layer stack changes what it sees and can break the
readable, uncompressed capture.

## When to Apply

- Before sharing or committing anything captured: it has tokens — scrub or keep
  it local.
- When picking a non-default `EMUKC_KCSAPI_DUMP` path: keep it ignored.
- When touching the `route_layer` stack in `kcsapi/mod.rs`: keep `dump_middleware`
  outermost so it dumps uncompressed final responses.

## Related

- `docs/solutions/architecture-patterns/battle-protocol-validator-boundary.md`
  — a captured battle response is exactly the input `battle validate` consumes.
