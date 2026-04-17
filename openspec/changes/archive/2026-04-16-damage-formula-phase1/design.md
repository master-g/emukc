## Context

All battle damage in EmuKC flows through `core.rs` in `emukc_gameplay`. Five `calculate_*_damage()` functions compute final damage per attack, but all use simplified approximations:
- Fixed armor multiplier instead of randomized defense
- No damage state penalty (chuuha/taiha ships fight at full power)
- Scratch damage never triggers (always minimum 1)
- Torpedo uses `TP + 5` when real formula uses `TP + improvement_bonus`

These four fixes are foundational — every subsequent formula improvement (improvement bonus, critical hits, artillery spotting) builds on correct defense calculation and damage state handling.

## Goals / Non-Goals

**Goals:**
- Defense randomization for all attack types (shelling, torpedo, airstrike, ASW, night)
- Damage state modifier as pre-cap multiplier for shelling, torpedo, ASW
- Scratch damage trigger when capped power < defense
- Torpedo basic power fix (remove +5)

**Non-Goals:**
- Improvement bonus (requires star-level data, phase 2)
- CV special formula, CL fit gun bonus (phase 2)
- Critical hits, artillery spotting, AP shell (phase 3)
- Night recon, carrier night attack (phase 5)
- Any API response format changes

## Decisions

### D1: Extract `calculate_defense_power()` as shared function

Create a single `calculate_defense_power(random, armor_stat, defense_type) -> f64` function replacing the inline `armor × k` in each damage calculator.

**Defense types and coefficients:**

| Attack Type | Old Code | Wiki Formula |
|---|---|---|
| Shelling / Night / ASW | `A × 0.7` | `floor(0.7×A + 0.6×rand(0, floor(A)−1))` |
| Torpedo | `A × 0.55` | Same randomized formula, no separate coefficient |
| Airstrike | `A × 0.6` | Same randomized formula, no separate coefficient |

**Wait** — wiki uses a single defense formula `0.7×A + 0.6×rand(0, floor(A)−1)` for ALL attack types. The `×0.55` and `×0.6` in current code are wrong simplifications. The random component already provides variance.

**Alternative considered**: Keep per-type armor coefficients. Rejected — wiki shows one defense formula for all attack types.

### D2: `damage_state_modifier()` returns pre-cap multiplier

New function `damage_state_modifier(current_hp, max_hp, attack_phase) -> f64`:

| State | HP ratio | Shelling | Torpedo | ASW |
|---|---|---|---|---|
| Chuuha | 25%–75% | 0.7 | 0.8 | 0.7 |
| Taiha | <25% | 0.4 | 0.0 | 0.4 |

Torpedo taiha = 0.0 means ship cannot torpedo at taiha. This is already partially enforced by `can_opening_torpedo_ship()` / `can_closing_torpedo_ship()` which check HP, but the damage modifier provides belt-and-suspenders coverage.

Applied after formation + engagement, before cap:
```
pre_cap = basic_power × formation × engagement × damage_state
```

### D3: Scratch damage replaces `max(1)` floor

Current code: `(capped_power - armor).floor().max(1.0)`

Change to:
```
if capped_power < defense:
    scratch_damage(random, target.current_hp)
else:
    (capped_power - defense).floor()
```

This applies to all attack types. Night battle already uses scratch for submarine targets — extend the pattern.

### D4: Torpedo removes `+5` constant

`calculate_torpedo_damage`: change `(TP + 5) × formation` to `TP × formation`.

The `+5` constant is shelling-only. Torpedo basic power is just torpedo stat plus improvement bonus (which will be added in phase 2).

## Risks / Trade-offs

**[Defense RNG makes tests non-deterministic]** → Existing tests that assert exact damage values will break. Mitigation: use seeded RNG in tests (already supported via `BattleRandom::new(Some(seed))`). Adjust expected value ranges to account for defense variance.

**[Damage state makes chuuha/taiha ships weaker]** → This is correct behavior. Current tests may have ships dealing too much damage at low HP. Mitigation: update test expectations.

**[Torpedo power decrease from removing +5]** → Slight torpedo nerf. Correct per wiki. The `+5` was mistakenly applied to torpedo when it's shelling-only.
