## 1. Investigation

- [x] 1.1 Verify CT repair time modifier from wikiwiki (ship_type_mod for 練習巡洋艦)
- [x] 1.2 Determine if "fleet has CT → reduced dock time" mechanic exists in real KanColle

## 2. Remodel HP Fix

- [x] 2.1 Add `new_ship.api_nowhp = new_ship.api_maxhp` at end of `remodel_impl` after `cal_ship_status`
- [x] 2.2 Add test: ship at partial HP is fully healed after remodel
- [x] 2.3 Add test: ship at full HP is fully healed after remodel with higher max HP

## 3. CT Dock Time Fix

- [x] 3.1 Update CT ship_type_mod in repair calculation if wikiwiki shows different value than 1.0
- [x] 3.2 Implement fleet-CT repair time reduction if the mechanic is confirmed (follow-up)
- [x] 3.3 Add test: CT repair time uses correct modifier

## 4. Verification

- [x] 4.1 Run `cargo test --test gameplay_tests` for integration pass
- [x] 4.2 Run `cargo clippy --workspace`
