## 1. Delete dead methods from SortieStore

- [x] 1.1 Delete `modify_active_sortie` method from `sortie_store.rs`
- [x] 1.2 Delete `with_pending_result_mut` method from `sortie_store.rs`
- [x] 1.3 Delete `with_pending_battle_mut` method from `sortie_store.rs`

## 2. Narrow allow(dead_code) scope

- [x] 2.1 Remove `#![allow(dead_code)]` from `sortie/mod.rs`
- [x] 2.2 Remove `EngagementType` from imports (unused) instead of adding allow — cleaner

## 3. Gate

- [x] 3.1 `cargo check --workspace` — 0 errors
- [x] 3.2 `cargo clippy --workspace` — no new warnings
- [x] 3.3 `cargo test -p emukc_gameplay` — all tests pass
- [x] 3.4 `cargo test --test gameplay_tests` — integration tests pass
