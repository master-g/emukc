## 1. Audit XP Paths

- [x] 1.1 Search codebase for all callers of `exp_to_ship_level` and all writes to `api_exp` / `api_lv` on ship entities
- [x] 1.2 Identify which paths lack the `!married && level >= 99` guard
- [x] 1.3 Document all XP-granting paths and their current guard status

## 2. Implement Level Cap

- [x] 2.1 Add `ship_level_cap(married: bool) -> i64` helper to level.rs (returns 99 or 175)
- [x] 2.2 Fix practice battle XP application to enforce level 99 cap for unmarried ships
- [x] 2.3 Fix any other XP paths identified in audit (quest rewards, exercises, etc.)
- [x] 2.4 Clamp level after `exp_to_ship_level` calls: `level = level.min(ship_level_cap(married))`

## 3. Testing

- [x] 3.1 Add test: unmarried ship at level 99 gains 0 XP from practice
- [x] 3.2 Add test: unmarried ship at level 98 gaining excess XP is clamped to 99
- [x] 3.3 Add test: married ship at level 99 can level up past 99
- [x] 3.4 Run `cargo test --test gameplay_tests` for integration pass
