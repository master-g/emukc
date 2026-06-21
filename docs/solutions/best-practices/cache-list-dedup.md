---
title: "CacheList deduplication, identity, and 404 fast-fail semantics"
date: 2026-06-22
category: best-practices
module: emukc_bootstrap
problem_type: tooling_decision
component: tooling
severity: medium
applies_when:
  - "Modifying CacheList add/dedup behavior or CacheListItem identity"
  - "Implementing or reviewing fetch_from_remote CDN fallback logic"
tags: [cachelist, dedup, identity, cdn, fallback, http-404]
related_components: [emukc_cache, emukc_network]
---

# CacheList deduplication, identity, and 404 fast-fail semantics

## Context

The cache list (`CacheList` / `CacheListItem`) drives the bulk resource
download. Three interlocking design decisions govern its correctness: how
items deduplicate (so the same resource is not downloaded twice), what fields
define item identity (a sequential `id` would break dedup and inflate list
size), and how the remote fetcher reacts to HTTP 404 versus transient errors
(retrying a hard 404 across every CDN node wastes time; failing a transient 5xx
on the first node loses a recoverable download).

## Guidance

### Deduplication by path + version

`CacheList` SHALL treat two items with the same `path` AND the same `version`
as identical. Adding an item whose path+version combination is already present
SHALL be a no-op (the existing entry is kept).

- Same path, same version, added twice → exactly one entry.
- Same path, DIFFERENT version → two entries (one per version), because the
  versioned resource is distinct.
- Same path, `None` version, added twice → exactly one entry (two `None`
  versions collapse to one).

### Identity excludes a sequential id

`CacheListItem` equality and ordering SHALL be based on `(path, version)` ONLY.
A sequential `id` field SHALL NOT exist on the struct. When a `CacheListItem` is
serialized to JSON, the output contains `path` and optionally `version`, but no
`_id` field. A sequential id would break `(path, version)` dedup semantics and
leak an internal counter into the serialized cache list.

### 404 fast-fail; preserve fallback for non-404 errors

When `fetch_from_remote` receives an HTTP 404 from any CDN node, it SHALL return
`FailedOnAllCdn` immediately, WITHOUT attempting the remaining CDN nodes. A 404
means the resource does not exist on the origin; trying the other nodes is
guaranteed waste.

The existing CDN fallback behavior (trying multiple CDN nodes) SHALL be
PRESERVED for connection errors, timeouts, and server errors (5xx). Only HTTP
404 triggers the immediate failure. A timeout on the first node with a 200 on
the second node MUST still succeed from the second node; a non-404 error on one
node MUST fall through to the next.

## Why This Matters

Dedup bugs silently double or halve the download workload and, when combined
with a `_id` field, make list diffs and comparisons meaningless. Treating a 404
as retriable burns one full CDN-node round-trip per node for a resource that can
never be fetched; treating a transient timeout as fatal drops a download that a
single retry would have recovered. Keeping the 404-vs-transient split at the
fetch layer (and documenting it) makes populate both faster and more complete.

## When to Apply

- When changing `CacheList::add` or `CacheListItem` fields.
- When modifying `fetch_from_remote` CDN fallback logic.
- When adding a new error variant to the fetch path — classify it as 404-like
  (fast-fail) or transient (fallback) explicitly.

## Examples

`add("...0003_9511....png", Some("1"))` twice → one entry. Then
`add("...0003_9511....png", Some("2"))` → now two entries. Serialized JSON shows
`path` + `version`, no `_id`. Fetch hits a 404 on node A → returns
`FailedOnAllCdn` without touching node B. Fetch times out on node A → tries
node B → 200 → returns the bytes.

## Related

- `cache-make-list-versioning.md` — how versions are assigned before they reach
  `CacheList`.
- `populate-error-classification.md` — how populate classifies the failures
  `fetch_from_remote` returns.
