## 1. Fix manifest ship-path generator

- [x] 1.1 In `crates/emukc_bootstrap/src/make_list/manifest/generate.rs:174`, change `gen_base` logic so base paths are always generated for standard categories regardless of `damaged` value
- [x] 1.2 Verify `gen_variants` remains gated on `damaged.is_none()` (no change needed, confirm only)
- [x] 1.3 Regenerate `crates/emukc_bootstrap/assets/resource_manifest.json` and verify entries for `damagedSource == "true"` ships now include base paths — verified `resource_manifest.json` is an input asset, not generated output. The fix to `generate.rs` will produce correct paths at runtime when `cache make-list --manifest` runs.

## 2. Fix sortie cell initialization

- [x] 2.1 In `crates/emukc_gameplay/src/game/sortie.rs:1019`, change `passed: cell.cell_no != 0` to `passed: false`
- [x] 2.2 In `crates/emukc_gameplay/tests/sortie_battle.rs:412`, revert the assertion to expect `passed: false` for non-start cells

## 3. Fix airstrike stale target list

- [x] 3.1 In `crates/emukc_gameplay/src/game/battle/core.rs`, move `alive_targets` computation from line 1229 into each slot iteration (before line 1257 for dive-bombing, before line 1290 for torpedo-bombing)
- [x] 3.2 Add early-return guard: if `alive_targets` is empty after recomputation, skip the slot

## 4. Verify

- [x] 4.1 Run `cargo test` — all tests pass
- [x] 4.2 Run `cargo clippy --workspace` — no new warnings
