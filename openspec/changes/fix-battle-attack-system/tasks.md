## 1. Shelling Display Type Fix

- [ ] 1.1 Change `day_attack_display_ids` to use ship type for participation gate, not equipment-only checklist
- [ ] 1.2 Implement attack display type fallback: when no relevant equipment, assign `api_at_type = 0` with base firepower + 5 (design D2)
- [ ] 1.3 Ensure shelling damage formula uses base ship stats plus equipment bonuses, not equipment-gated

## 2. Closing Torpedo Fix

- [ ] 2.1 In `can_closing_torpedo_ship`, remove the ship type whitelist — keep only `api_raisou[0] > 0` + not sunk + not 中破/大破
- [ ] 2.2 Verify DE, LHA, AR, CV/CVL/CVB, most BB are correctly excluded (base torpedo = 0)
- [ ] 2.3 Verify BB with base torpedo > 0 (Bismarck drei, Гангут, Conte di Cavour, 金剛型第三改装, Norge級) are now included
- [ ] 2.4 Verify AV/AO/CT correctly follow base torpedo stat (not ship type): 千歳改/甲 included vs 秋津洲 excluded

## 3. Opening Torpedo Fix

- [ ] 3.1 Keep equipment check for 特殊潜航艇 (minisub/甲标的) in `can_opening_torpedo_ship` — do NOT remove it
- [ ] 3.2 Add SS/SSV level ≥ 10 requirement for equipment-free opening torpedo
- [ ] 3.3 Keep CLT type as always eligible for opening torpedo (with `api_raisou[0] > 0`)

## 4. Damage Application Fix

- [ ] 4.1 In `apply_damage`, change enemy sortie capping: when `!self.is_friendly && self.is_sortie`, skip `raw_damage.min(self.current_hp)` — allow HP to go negative
- [ ] 4.2 `BattleRuntimeShip` already has `is_friendly` and `is_sortie` fields (core.rs:213) — no signature change needed
- [ ] 4.3 Audit downstream consumers of HP for enemy ships: verify MVP calculation, `calculate_win_rank`, and battle result handlers tolerate negative HP
- [ ] 4.4 Verify practice battles still cap enemy damage at current HP
- [ ] 4.5 Verify friendly sinking protection unchanged for sortie
- [ ] 4.6 Verify practice friendly damage still capped at current HP

## 5. Testing

- [ ] 5.1 Add tests: DD with no equipment shelling shows `api_at_type = 0`, not torpedo attack
- [ ] 5.2 Add tests: DD with only torpedo equipped shows normal shelling attack in shelling phase
- [ ] 5.3 Add tests: BB with base torpedo > 0 participates in closing torpedo
- [ ] 5.4 Add tests: DE with base torpedo = 0 excluded from closing torpedo
- [ ] 5.5 Add tests: ABKM改二 with 甲标的 participates in opening torpedo (equipment-based)
- [ ] 5.6 Add tests: SS level < 10 does NOT opening torpedo without 甲标的
- [ ] 5.7 Add tests: enemy overkill damage in sortie (HP goes negative)
- [ ] 5.8 Add tests: enemy damage capped in practice
- [ ] 5.9 Add tests: CV without planes excluded from shelling
- [ ] 5.10 Run existing battle tests to verify no regression
- [ ] 5.11 Run `cargo test --test gameplay_tests` for integration test pass
