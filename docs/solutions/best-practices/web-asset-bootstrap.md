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
related_components: [emukc_network]
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
  to `z/cache/gadget_html5/js/kcs_const.js` during bootstrap.
- When the target file does not exist, download it from the first available
  CDN in `gadgets_cdn`.
- When the target file already exists and `--overwrite` is not passed, skip
  the download and log at debug level.

**Download main.js.**

- Download `kcs2/js/main.js` from the configured `game_cdn` to
  `z/cache/kcs2/js/main.js` during bootstrap.
- When the target file does not exist, download it from the configured
  `game_cdn`.
- When the target file already exists and `--overwrite` is not passed, skip
  the download and log at debug level.

**Download version.json.**

- Download `kcs2/version.json` from the configured `game_cdn` to
  `z/cache/kcs2/version.json` during bootstrap.
- When the target file does not exist, download it from the configured
  `game_cdn`.
- When the target file already exists and `--overwrite` is not passed, skip
  the download and log at debug level.

**Graceful degradation when CDN is unconfigured.**

- When `gadgets_cdn` is an empty list, skip the `kcs_const.js` download and
  emit a warn-level log stating that `gadgets_cdn` must be configured.
- When `game_cdn` is an empty list, skip the `main.js` and `version.json`
  downloads and emit a warn-level log stating that `game_cdn` must be
  configured.

**Force update restores web assets.**

- On `cargo run -- bootstrap --force-update` with CDN configured, first
  delete the old `kcs_const.js` and `version.json`, then re-download all
  three files.

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
  it to `z/cache/kcs2/js/main.js`.
- `gadgets_cdn` empty: bootstrap skips `kcs_const.js`, logs a warn, and
  continues with the Codex build.
- `--skip-web-assets`: no network calls; only the Codex build runs.

## Related

- `docs/solutions/best-practices/resource-manifest.md`
- `docs/solutions/best-practices/bootstrap-guide.md`
