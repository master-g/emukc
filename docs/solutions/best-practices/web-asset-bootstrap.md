---
title: "Web asset bootstrap download contract"
date: 2026-06-22
category: best-practices
module: emukc_bootstrap
problem_type: tooling_decision
component: tooling
severity: medium
applies_when:
  - "Running cargo run -- bootstrap to fetch kcs_const.js, main.js, and version.json"
  - "Handling missing CDN configuration or --force-update / --skip-web-assets flags"
tags: [bootstrap, web-assets, kcs-const, main-js, version-json, cdn]
related_components: [emukc_network, emukc_cache]
---

# Web asset bootstrap download contract

## Context

The bootstrap flow downloads three web assets — `kcs_const.js`, `main.js`,
and `version.json` — from configured CDNs into the local cache before the
Codex build runs. This contract documents the download targets, the
graceful-degradation behavior when CDNs are unconfigured, and the
`--force-update` / `--skip-web-assets` flag semantics.

## Guidance

**Download kcs_const.js.**

- Download `gadget_html5/js/kcs_const.js` from the configured `gadgets_cdn`
  to `<cache_root>/gadget_html5/js/kcs_const.js` during bootstrap.
- When the target file does not exist, download it from the first available
  CDN in `gadgets_cdn`.
- When the target file already exists and `--overwrite` is not passed, skip
  the download and log at debug level.

**Download main.js.**

- Fetch `kcs2/js/main.js` through the resource cache (`Kache`) at the client
  version `kcs_const.js` names (`scriptVesion`), from the configured `game_cdn`.
- A new version keeps the replaced `main.js` in the cache history under the
  version it was recorded at, and is downloaded even when it is as long as the
  old one (only png and mp3 take the cache's same-length check). An unchanged version
  is not downloaded again.
- When the target file already exists and `--overwrite` is not passed, skip
  the download.
- The cache database is opened for this, so the server must not be running.

**Download version.json.**

- Download `kcs2/version.json` from the configured `game_cdn` to
  `<cache_root>/kcs2/version.json` during bootstrap.
- When the target file does not exist, download it from the configured
  `game_cdn`.
- When the target file already exists and `--overwrite` is not passed, skip
  the download and log at debug level.

**Graceful degradation when CDN is unconfigured.**

- When `gadgets_cdn` or `game_cdn` is an empty list, download none of the
  three, emit a warn-level log naming the list that must be configured, and
  fail with `WebAssetUnavailable` (the cache needs both lists).

**Force update restores web assets.**

- On `cargo run -- bootstrap --force-update` with CDN configured, fetch
  `kcs_const.js` and `version.json` anew, and `main.js` when its version
  changed.

**Skip web assets flag.**

- Provide a `--skip-web-assets` CLI flag.
- When passed, skip all web asset downloads and run only the existing Codex
  build flow.

## Why This Matters

These three assets are the inputs to the decoder and the Codex build. The
graceful-degradation rule lets bootstrap proceed (building the Codex from
already-cached data) instead of hard-failing when a CDN is intentionally
unconfigured — important for offline or CI environments. The skip flag lets
repeat runs avoid network entirely when only the Codex build is needed.

## When to Apply

- When modifying the bootstrap download sequence.
- When debugging a missing `main.js` or `version.json` after a fresh clone.

## Examples

- Fresh clone, `game_cdn` configured, `main.js` absent: bootstrap downloads
  it to `<cache_root>/kcs2/js/main.js`.
- `gadgets_cdn` empty: bootstrap builds the Codex, then logs a warn and fails
  at the web assets without downloading any of them.
- A new client: `main.js` of the old version stays in the cache history, and
  `kcs_const.js` and `version.json` are replaced.
- `--skip-web-assets`: no network calls; only the Codex build runs.

## Related

- `docs/solutions/best-practices/resource-manifest.md`
- `docs/solutions/best-practices/bootstrap-guide.md`
