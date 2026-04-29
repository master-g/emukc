## 1. Decoder Semantic Completeness

- [x] 1.1 Add a regression test in `main-decoder` that exercises partial `banner_g` / `banner2_g` / `banner3_g` evidence and proves the emitted ship target family is not observed-complete.
- [x] 1.2 Update `main-decoder/src/cache-rules.ts` so target-family completeness is derived from decoder evidence instead of the presence of any banner-family signal.
- [x] 1.3 Verify complete ship target-family evidence still emits authoritative semantics and continues to suppress broad fallback where appropriate.

## 2. Rules-Mode Sound Fallback

- [x] 2.1 Add Rust regression coverage showing a decoder-complete `kcs/sound/*` family is emitted as rule-authored output without duplicate fallback-authored list items.
- [x] 2.2 Update `crates/emukc_bootstrap/src/make_list/source/kcs/mod.rs` to skip matching legacy sound fallback generators for complete decoder-covered families.
- [x] 2.3 Preserve fallback generation and fallback attribution for partial or unresolved sound-rule families.

## 3. Optional Sibling Asset Loading

- [x] 3.1 Add loader tests for malformed optional sibling decoder JSON next to a valid `cache_rules.json`.
- [x] 3.2 Update `crates/emukc_bootstrap/src/make_list/manifest/loader.rs` so absent, unreadable, and malformed optional sibling assets warn and continue.
- [x] 3.3 Keep malformed required rules assets fatal so invalid `cache_rules.json` still aborts bundle loading.

## 4. Verification

- [x] 4.1 Run `bun test` for the decoder regression coverage.
- [x] 4.2 Run `cargo test -p emukc_bootstrap make_list -- --test-threads=1`.
- [x] 4.3 Run `cargo fmt --check` and format the changed Rust files if needed.
- [x] 4.4 Note existing non-goal failures separately if `bun run check` or parallel `cargo test -p emukc_bootstrap make_list` still fail for the already-known issues.
