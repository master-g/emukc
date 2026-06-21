---
title: "CLI progress bars for cache and bootstrap commands"
date: 2026-06-22
category: best-practices
module: emukc_bootstrap
problem_type: best_practice
component: development_workflow
severity: low
applies_when:
  - "Adding or modifying progress display in cache or bootstrap CLI commands"
  - "Emitting log lines during a progress-displaying operation"
tags: [cli, progress-bar, indicatif, tty, bootstrap, cache-populate]
related_components: [emukc_cache]
---

# CLI progress bars for cache and bootstrap commands

## Context

The `bootstrap`, `cache make-list`, and `cache populate` commands are
long-running operations (manifest download, list generation, bulk resource
download). Without a progress indicator the user has no feedback on completion,
rate, or ETA, and cannot tell whether the process is stalled. The progress
display must also coexist with `info!`/`warn!` log lines without corrupting
either output, and must degrade gracefully in non-TTY environments (piped
output, CI) where in-place bars cannot render.

## Guidance

The `cache populate` command SHALL display a progress bar showing completed
count, total count, percentage, and estimated time remaining. The bar SHALL
update in-place without scrolling the terminal.

The `cache make-list` command in greedy mode SHALL display a progress bar
showing checked count, total count, found count, check rate (checks/s), and
ETA.

The `bootstrap` command SHALL display phase-labeled progress output: each
phase transition (download, parse, web assets, save) shows a labeled header,
and per-phase progress bars where applicable. Web-asset downloads
(`kcs_const.js`, `main.js`, `version.json`) show filename and completion
status without clobbering phase output.

Download operations that use concurrent tasks (e.g. `download_all` with
multiple resources) SHALL show an aggregate progress bar with per-resource
detail visible.

**TTY gating:** when stdout is not a terminal (piped or CI), NO progress bar
SHALL be drawn. Existing `info!` log output serves as the progress indication
in that case. This applies to both `cache populate` and `cache make-list`.

**Log/progress coexistence:** `info!` and `warn!` log lines emitted during a
progress-displaying operation SHALL NOT break or corrupt the active progress
bar. A warning must print cleanly above the bar; the bar must continue updating
normally afterward.

## Why This Matters

A progress bar that scrolls the terminal, collides with log lines, or renders
garbage in CI actively degrades the user experience versus no bar at all.
Rendering artifacts in piped output can break automation that parses command
output. Gating the bar on TTY and routing logs through the progress layer's
suspend mechanism keeps both the interactive and automated paths clean.

## When to Apply

- When adding a new long-running CLI step to bootstrap or cache.
- When introducing a concurrent download path that should show aggregate
  progress.
- When emitting `info!`/`warn!` from code that runs under an active progress
  bar — route through the log/progress coexistence mechanism (see Related).

## Examples

`cache populate` with N resources renders a single bar: `completed/N`,
percentage, ETA, updating per resource. In a CI pipe the same run emits only
`info!` lines. A `warn!` during populate prints above the bar without breaking
it.

## Related

- `progress-logging-helper.md` — the `log_with_mp` / aggregate-bar-length
  / panic-safe-failure invariants that make log/progress coexistence work.
