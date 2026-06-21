---
title: "cache populate distinguishes version rollback from genuine download failure"
date: 2026-06-22
category: best-practices
module: emukc_bootstrap
problem_type: best_practice
component: development_workflow
severity: medium
applies_when:
  - "Documenting or troubleshooting cache populate retry behavior"
  - "Deciding whether a populate failure should be retried or skipped"
tags: [cache-populate, version-rollback, retry, error-classification, troubleshooting]
related_components: [emukc_cache]
---

# cache populate distinguishes version rollback from genuine download failure

## Context

`cache populate` can fail to write a resource for two very different reasons:
the on-disk cached version is NEWER than the manifest version (a version
rollback — a no-op, not a defect), or the download genuinely failed (network
error, HTTP 404). Treating both identically — either retrying the rollback or
skipping the real failure — wastes work or hides broken downloads. The
classification must also be explainable to the user in documentation, not
inferred from log messages.

## Guidance

`cache populate` SHALL distinguish "version rollback" failures (which are
SKIPPED without retry) from genuine download failures (which are RETRIED in
pass 2).

This classification SHALL be based on a **typed error variant**, NOT on an
error-message substring match. A version rollback surfaces as a distinct typed
error (e.g. `InvalidFileVersion`) that the populate loop pattern-matches; it
must not be detected by stringifying an error and searching for a substring.

`BOOTSTRAP.md` SHALL document this classification in its troubleshooting
section. Specifically, when a user encounters `skipping N items with version
rollback` in populate output, the documentation MUST explain that this means
the on-disk version is newer than the manifest version (a no-op, not a
failure), and MUST state that retried failures are genuinely failed downloads
(likely network or 404).

## Why This Matters

Substring-based error classification is fragile: a wording change in an error
message silently breaks the retry/skip decision and mis-categorizes failures.
Documenting the classification in `BOOTSTRAP.md` lets users self-diagnose the
common "skipping N items with version rollback" line instead of filing it as a
bug. Retrying version rollbacks wastes a full network round-trip per item for no
effect; skipping genuine 404s hides broken resources.

## When to Apply

- When adding a new failure mode to populate — decide typed-variant retry vs
  skip and document it.
- When updating the `BOOTSTRAP.md` troubleshooting section.
- When reviewing populate error handling — confirm classification is on the
  typed variant, not on message text.

## Examples

Pass 1 fetches a resource and the cache returns `InvalidFileVersion` (on-disk
v2 vs manifest v1). Populate logs "skipping (version rollback)" and does NOT
add it to the pass-2 retry list. A genuine `FailedOnAllCdn` (all CDN nodes 404
or network error) IS added to the pass-2 retry list.

## Related

- `cache-make-list-versioning.md` — the make-list-side version handling that
  prevents most rollbacks from reaching populate.
- `bootstrap-guide.md` — the troubleshooting section this classification is
  documented in.
