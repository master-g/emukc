## 1. Strategy Dispatch Refactor

- [x] 1.1 In `source/mod.rs::make()`, change `Default` and `Greedy` branches to load the rules bundle and delegate to the `Rules` code path (identical to the existing `Rules` branch), with `Greedy` additionally generating a holes report
- [x] 1.2 Remove the `Manifest` branch in `source/mod.rs` — it can remain as a separate strategy or be folded into `Rules` with a flag; keep `Manifest` as-is for backward compatibility
- [x] 1.3 In `source/kcs/mod.rs::make()`, remove the non-`Rules` else branch (legacy `kc9997::make`, `kc9998::make`, `kc9999::make`, `purchase::make`, `voice::make` without decoder rules)
- [x] 1.4 Remove `source/kcs2/resources/mod.rs::make()` (the legacy function for Default/Greedy without decoder assets)

## 2. Comparison Example Update

- [x] 2.1 In `examples/decoder_cachelist_compare.rs`, change `BaselineStrategy::Default` default to `BaselineStrategy::Manifest`
- [x] 2.2 Add a warning when `--baseline default --rules` is used, noting that both strategies now produce identical output

## 3. Test Updates

- [x] 3.1 Update `build_cache_list_paths_with_manifest_path_matches_repo_manifest_strategy` test to account for Default now using Rules
- [x] 3.2 Verify `cargo test -p emukc_bootstrap` passes with the refactored strategies
- [x] 3.3 Run `cargo run --example decoder_cachelist_compare -- --rules crates/emukc_bootstrap/assets/cache_rules.json` and confirm Default-vs-Rules shows 100% overlap with zero baseline-only paths

## 4. Cleanup

- [x] 4.1 Remove any now-dead code in `source/kcs/` and `source/kcs2/resources/` (legacy functions only called from removed branches)
- [x] 4.2 Run `cargo clippy --workspace` and `cargo fmt --all`
