## Context

`emukc_bootstrap::make_list::CacheList` uses a `BTreeSet<CacheListItem>` as its backing store. `CacheListItem` derives `Ord` on all fields `(id, path, version)`. Since `id` is a monotonically increasing counter (`next_id += 1` per `add()` call), the set never deduplicates — two items with the same path but different `id` values coexist.

This causes two downstream problems:
1. The generated `cache_resources.nedb` file inflates to 395,850 lines for 69,702 unique paths.
2. `populate` attempts to download every line, wasting time on duplicates and on resources that don't exist on CDN.

Separately, `emukc_cache::kache::fetch_from_remote()` iterates through all 20 CDN nodes even after receiving a definitive 404. Since all CDN nodes are CloudFront with identical content, 404 on one implies 404 on all.

## Goals / Non-Goals

**Goals:**
- Produce cache lists with zero path duplicates (same path+version appears at most once).
- Fail fast on 404 responses in `fetch_from_remote` — stop trying other CDN nodes immediately.
- Maintain backward compatibility of the `.nedb` line format (JSONL with `path` and optional `version`).

**Non-Goals:**
- Adding pre-download HEAD batch validation.
- Changing populate concurrency model or retry strategy.
- Changing `batch_check_exists` behavior (already correct).
- Optimizing which paths are generated (reducing false positives at the make_list level).

## Decisions

### Decision 1: Remove `id` field from `CacheListItem`

**Choice**: Delete the `id` field entirely, derive `Ord`/`PartialOrd`/`Eq`/`PartialEq`/`Hash` on `(path, version)`.

**Alternative considered**: Keep `id` but implement custom `Ord` ignoring it. Rejected because `id` serves no purpose after the fix and leaving it invites future bugs.

**Rationale**: `id` was originally intended as a line number for the output file. Since the file is consumed line-by-line with no index lookups, the field is unused. Removing it simplifies the struct and makes dedup automatic via `BTreeSet`.

**Impact**: Any test code referencing `item.id` or asserting on ordering by `id` must be updated. The `serde` rename `_id` field disappears from JSON output — consumers that parse `_id` will need to ignore its absence. Current consumers (`populate.rs`) deserialize with `serde_json::from_str` and only use `path` and `version`, so no breakage.

### Decision 2: Use `BTreeSet` with path-based dedup directly

**Choice**: Keep `BTreeSet<CacheListItem>` but now `CacheListItem` compares by `(path, version)` only. Calling `add()` with a duplicate path will silently replace (or be a no-op via `insert`).

**Alternative considered**: Switch to `HashSet<String>` for paths and store `(path, version)` pairs separately. Rejected because it loses the structured item representation.

**Rationale**: `BTreeSet::insert` returns `bool` indicating whether the item was new. We can use this for diagnostics (counting dedup hits) if desired.

### Decision 3: 404 fast-fail in `fetch_from_remote`

**Choice**: In the CDN loop inside `fetch_from_remote()`, if `fetch_from_url` returns an error indicating the HTTP response was 404, break out of the loop immediately and return `FailedOnAllCdn`.

**Alternative considered**: Return a distinct error variant `NotFoundOnCdn` to distinguish from "CDN unreachable". Rejected as unnecessary for now — callers treat all download failures the same.

**Rationale**: All CDN nodes are CloudFront front-ends for the same S3 origin. Content is identical across nodes. A 404 response is definitive, not transient. The current behavior of trying all 20 nodes on 404 wastes ~20× time per non-existent resource.

**Implementation**: Add a check in `fetch_from_remote` — after `fetch_from_url` returns `Err`, inspect the error. If it's a 404, break. This requires `fetch_from_url` (or the download layer) to propagate the HTTP status code. Currently `fetch_from_url` calls `download::Request::execute()` which may not distinguish 404 from other errors. The simplest approach: use `self.client.head(&url).send()` first, check for 404, and only proceed to GET if HEAD returns 200.

**Revised simpler approach**: In `fetch_from_remote`, before calling `fetch_from_url`, do a lightweight HEAD request. If HEAD returns 404, skip this CDN and return `FailedOnAllCdn` immediately (since all CDNs are identical). If HEAD returns 200, proceed to GET. If HEAD fails/times out, try next CDN.

Wait — that's essentially "Option A" from explore. The user chose Option B: **if the GET download returns 404, fail immediately without trying other CDNs**.

Final approach: Modify `fetch_from_url` to detect 404 responses and return a specific error variant. In `fetch_from_remote`, match on this variant and break the CDN loop.

## Risks / Trade-offs

- **[Risk] CDN inconsistency**: If CDN nodes diverge (e.g., cache propagation delay), fast-fail could miss a resource that exists on another node. → Mitigation: All nodes are CloudFront with a single S3 origin. Divergence is extremely unlikely in practice.
- **[Risk] Breaking `_id` consumers**: Any external tooling parsing the `_id` field from `.nedb` files will break. → Mitigation: No known external consumers. Internal code (`populate.rs`) ignores `_id`.
- **[Trade-off] Losing insertion order**: With dedup on `BTreeSet`, the output file order changes from insertion-order to sorted-by-path order. → Acceptable: `populate` processes items independently; order doesn't matter.
