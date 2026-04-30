## Why

`CacheListItem` derives `Ord` on `(id, path, version)` where `id` is a monotonically increasing counter. This means the `BTreeSet<CacheListItem>` in `CacheList` never deduplicates entries with the same path — every `add()` call inserts a new element. The generated cache list file contains 395,850 lines for only 69,702 unique paths (5.7× inflation). During `populate`, each duplicate triggers a full download attempt that hits all 20 CDN nodes before failing, causing 16 concurrent slots to stall for minutes on resources that don't exist.

## What Changes

- **Fix `CacheListItem` dedup**: Change `Ord`/`PartialOrd` to compare by `(path, version)` only, ignoring `id`. This makes `BTreeSet` correctly deduplicate entries with identical paths.
- **Add 404 fast-fail in `fetch_from_remote`**: When any CDN returns HTTP 404 for a resource, immediately return `FailedOnAllCdn` without trying remaining CDNs. CloudFront CDN nodes serve identical content, so 404 on one means 404 on all.
- **Remove `id` field from `CacheListItem`**: The `id` field serves no purpose after dedup is path-based. Remove it to prevent future misuse.

## Capabilities

### New Capabilities

- `cache-list-dedup`: Ensures `CacheList` produces unique path entries by deduplicating on `(path, version)`.

### Modified Capabilities

- `resource-manifest`: `fetch_from_remote` in `emukc_cache` gains 404 fast-fail behavior, changing from "try all CDNs" to "fail immediately on 404".

## Non-goals

- Pre-download HEAD validation of all list entries (batch pre-check) — deferred to future work.
- Changes to `populate` retry logic or concurrency model — out of scope.
- Changes to `batch_check_exists` — already works correctly for HEAD checks.

## Impact

- `emukc_bootstrap/src/make_list/mod.rs`: `CacheListItem` struct, `Ord` derive, `CacheList::add()` method.
- `emukc_cache/src/kache.rs`: `fetch_from_remote()` method — add early return on 404.
- `emukc_bootstrap` tests: any tests that assert on `CacheListItem` ordering or `id` values.
- Cache list output files (`cache_resources.nedb`): will shrink from ~396K to ~70K lines. No format change (just fewer lines).
- `populate` performance: dramatically faster for lists containing non-existent resources.
