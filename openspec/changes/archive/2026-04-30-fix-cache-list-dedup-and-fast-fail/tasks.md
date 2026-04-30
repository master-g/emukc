## 1. CacheListItem dedup fix

- [x] 1.1 Remove `id` field from `CacheListItem` in `crates/emukc_bootstrap/src/make_list/mod.rs` — delete `id` field, `_id` serde rename, and `next_id` counter from `CacheList`
- [x] 1.2 Change `CacheListItem` derives — remove `id` from equality: implement `Ord`/`PartialOrd` manually comparing `(path, version)` only, keep derive for `Debug`, `Clone`, `Serialize`, `Deserialize`
- [x] 1.3 Update `CacheList::add()` and `CacheList::add_unversioned()` — remove `id` assignment, simplify to construct `CacheListItem { path, version }`
- [x] 1.4 Update `CacheList::into_items()`, `into_path_set()`, `into_path_build_output()` — remove any `next_id` field references
- [x] 1.5 Fix all test code referencing `CacheListItem` ordering or `id` field — update assertions in `crates/emukc_bootstrap/src/make_list/mod.rs` tests

## 2. 404 fast-fail in fetch_from_remote

- [x] 2.1 Add `NotFound` variant to `emukc_cache::Error` enum (or similar) to distinguish 404 from other download errors
- [x] 2.2 Modify `fetch_from_url` in `crates/emukc_cache/src/kache.rs` to detect HTTP 404 and return the `NotFound` error variant
- [x] 2.3 Modify `fetch_from_remote` CDN loop to break immediately on `NotFound` error — return `FailedOnAllCdn` without trying remaining CDN nodes

## 3. Verification

- [x] 3.1 Run `cargo test -p emukc_bootstrap` — verify all make_list tests pass with dedup fix
- [x] 3.2 Run `cargo test -p emukc_cache` — verify cache tests pass with 404 fast-fail
- [x] 3.3 Run `cargo test --workspace` — verify no regressions
- [x] 3.4 Run `cargo clippy --workspace` and `cargo fmt --all` — verify clean lint
