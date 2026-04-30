## 1. Add `log_with_mp` helper

- [x] 1.1 Add `pub fn log_with_mp(mp: &Option<MultiProgress>, f: impl FnOnce())` to `crates/emukc_bootstrap/src/progress.rs`
- [x] 1.2 Replace all 8 `if let Some(mp) = mp.as_ref() { mp.suspend(|| { ... }) } else { ... }` instances in `crates/emukc_bootstrap/src/download.rs` with `log_with_mp(&mp, || { ... })` calls
- [x] 1.3 Fix mixed tab/space indentation in the replaced blocks

## 2. Fix aggregate bar overflow

- [x] 2.1 In `crates/emukc_bootstrap/src/populate.rs`, before pass 2 `run_pass` call, add `pb.set_length(total_files as u64 + retry_count as u64)` on the aggregate bar

## 3. Harden failure collection

- [x] 3.1 In `crates/emukc_bootstrap/src/populate.rs` `run_pass`, replace `Arc::try_unwrap(failures).unwrap().into_inner().unwrap()` with `failures.lock().unwrap().clone()`

## 4. Add invariant assertions

- [x] 4.1 Add `debug_assert_eq!(active_count.load(Ordering::Relaxed), 0)` after each `run_pass` call in `populate`

## 5. Verify

- [x] 5.1 Run `cargo build` — no compile errors
- [x] 5.2 Run `cargo clippy -p emukc_bootstrap` — no new warnings
