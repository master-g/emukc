---
title: "Equipment improvement (★) bonuses for day shelling, torpedo, and night battle"
date: 2026-06-22
category: architecture-patterns
module: emukc_battle
problem_type: architecture_pattern
component: service_object
severity: medium
applies_when:
  - "Modifying day shelling, torpedo, or night battle basic power calculation"
  - "Adding or adjusting equipment star-level improvement bonus formulas"
tags: [equipment, improvement, star-bonus, shelling, torpedo, night-battle, damage-formula]
related_components: [emukc_model]
---

# Equipment improvement (★) bonuses for day shelling, torpedo, and night battle

## Context

Equipment star-level (★) improvement modifies basic attack power in the battle
damage formula. This contract defines the bonus formulas for day shelling,
torpedo, and night battle, and the equipment type weights. Migrated from
`openspec/specs/equipment-improvement-bonus/spec.md`.

## Guidance

### Day shelling improvement bonus

The system SHALL add equipment star-level (★) improvement bonuses to day
shelling basic power. For each equipped weapon, the bonus is
`√(★) × type_weight`, where `type_weight` depends on equipment type:

- Small caliber main gun: 1.0
- Medium caliber main gun: 1.0
- Large caliber main gun: 1.0
- Secondary gun: 1.0
- Torpedo: 1.0
- Seaplane bomber / carrier-based dive bomber / carrier-based torpedo bomber:
  1.0

The total improvement bonus SHALL be added to `firepower + 5` before
formation/engagement/damage-state modifiers. With all equipment at ★0, the
bonus is 0 and basic power is unchanged.

### Torpedo improvement bonus

The system SHALL add star-level improvement bonuses to torpedo basic power.
For each equipped torpedo (type: Torpedo, SubmarineTorpedo), the bonus is
`★ × 1.2`. The total SHALL be added to `torpedo_stat` before
formation/engagement/damage-state modifiers.

### Night battle improvement bonus

The system SHALL add star-level improvement bonuses to night battle basic
power. The bonus formula matches day shelling: `√(★)` per equipment. The total
SHALL be added to `firepower + torpedo + 5` before cap.

## Why This Matters

These bonuses are applied to *basic power* before the formation/engagement/
damage-state modifiers, so they compound with later multipliers. Omitting them
or applying the wrong type weight makes improved equipment underperform
relative to the real game, which is especially impactful for optimized
DD/torpedo and night-battle builds.

## When to Apply

- When modifying day shelling, torpedo, or night battle basic power
  calculation.
- When adding a new equipment type that should contribute an improvement
  bonus.

## Examples

- DD with a ★10 small caliber main gun: day bonus `√10 ≈ 3.16`.
- Ship with a ★5 torpedo: torpedo bonus `5 × 1.2 = 6.0`.
- Submarine with a ★8 submarine torpedo: bonus `8 × 1.2 = 9.6`.
- Night battle with a ★4 gun + ★6 torpedo: bonus `√4 + √6 = 2.0 + 2.45 =
  4.45`.

## Related

- `docs/solutions/architecture-patterns/battle-damage-foundation.md` — base
  damage formula these bonuses feed into.
- `docs/solutions/architecture-patterns/night-battle-sinking-protection.md` —
  night battle sinking rules.
