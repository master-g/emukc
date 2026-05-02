## 1. Auto-fix via clippy

- [x] 1.1 Run `cargo clippy --fix --lib -p emukc_gameplay` to apply 12 auto-fixes
- [x] 1.2 Manually delete `insert_active_sortie` from `sortie_store.rs` (clippy warns but won't auto-delete pub methods)
- [x] 1.3 Add `#[allow(dead_code)]` to `SortieNightBattleSession::profile_id`

## 2. Gate

- [x] 2.1 `cargo check --workspace` — 0 errors
- [x] 2.2 `cargo clippy -p emukc_gameplay` — 0 actionable warnings (only missing docs remain)
- [x] 2.3 `cargo test -p emukc_gameplay` — all tests pass
- [x] 2.4 `cargo test --test gameplay_tests` — integration tests pass
