## 1. Move example entrypoints out of tests

- [x] 1.1 Create the top-level `examples/` directory and move `tests/model/load.rs`, `tests/bootstrap/download.rs`, `tests/dump_fs_tree/dump_tree.rs`, and `tests/kache/test.rs` into matching files under `examples/`
- [x] 1.2 Update `/Users/mg/github/emukc/Cargo.toml` `[[example]]` paths so `model_loader`, `bootstrap_download`, `dump_tree`, and `kache_test` point at `examples/...` without changing their public names

## 2. Re-establish test-only boundaries

- [x] 2.1 Confirm `tests/` contains only integration tests, fixtures, and test-specific support files after the move
- [x] 2.2 Search the repository for stale references to the old `tests/...` example paths and update tracked references that should now point to `examples/...`

## 3. Refresh contributor guidance

- [x] 3.1 Update `/Users/mg/github/emukc/tests/README.md` so it documents `tests/` as test-only and points runnable samples to `examples/`
- [x] 3.2 Update any adjacent documentation or comments that still describe the mixed `tests/` layout

## 4. Verify developer workflows

- [x] 4.1 Run `cargo test` to confirm the test layout cleanup did not break integration-test discovery
- [x] 4.2 Run representative example commands such as `cargo run --example bootstrap_download -- --help` or another non-destructive invocation to confirm the moved examples are still wired correctly
