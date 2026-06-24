---
date: 2026-06-24
topic: flagship-escort-shield
---

# Flagship Escort Shield (旗艦援護)

## Summary

Implement the flagship escort protection mechanic (旗艦援護/かばう), where an escort ship intercepts an attack aimed at the flagship, taking the damage instead. The server determines interception via formation-based probability, swaps the target to a healthy escort, and encodes the interception as a `.1` decimal suffix in `api_damage` for client animation.

---

## Problem Frame

Official server battle data (`/Users/mg/Downloads/kcsapi/battle*.txt`) contains fractional damage values (e.g. `198.1`, `6.1`) in `api_hougeki` `api_damage`. The KC client's `HougekiData.isShield()` method checks `damage % 1 > 0` to detect these values and renders a shield animation on the flagship (`getShieldTargetBanner` returns the fleet's flagship at index 0).

Our emulator currently has zero support for this mechanic. `api_damage` is `Vec<Vec<i64>>` (integers only), so we cannot encode the `.1` flag. The targeting logic never attempts interception. As a result, the client never plays the shield animation, and the flagship takes damage that should have been intercepted by escorts.

---

## Requirements

**Mechanic**

- R1. When an attack targets the flagship (fleet index 0), roll an interception check based on the friendly fleet's formation
- R2. Interception rates by formation: 単縦陣 45%, 複縦陣/梯形陣/単横陣 60%, 輪形陣/警戒陣 75%
- R3. Interception requires at least one non-flagship escort ship in green health (小破未満, HP > 75% max HP) on the defending fleet
- R4. If interception triggers, select a valid escort (green health, same surface/submarine category as flagship) as the new target; damage is calculated against that escort instead
- R5. Surface ships protect surface flagships; submarines protect submarine flagships — type matching is required

**Protocol encoding**

- R6. When interception triggers, the damage value in `api_damage` must carry a `.1` decimal suffix (e.g. `55` becomes `55.1`); the integer part is the actual damage, and the decimal part signals the shield flag to the client
- R7. `api_df_list` shows the interceptor's index (the actual damage taker), not the original flagship target
- R8. Applies to all hougeki phases (day shelling, night battle) and opening torpedo (`api_raigeki`) where the client's `isShield` / `isShield_f` / `isShield_e` methods check for fractional values

**Scope boundaries**

- R9. Both friendly and enemy fleets can trigger interception — the mechanic is bidirectional
- R10. Combined fleet (連合艦隊) escort fleet flagship does NOT receive protection during day battles — this restriction is out of scope until combined fleet sortie is implemented

---

## Acceptance Examples

- AE1. **Covers R1, R2, R3, R6.** Given a friendly fleet in 単縦陣 with a healthy escort, when the enemy attacks the friendly flagship, the RNG roll (deterministic in tests) triggers interception. `api_damage` for that attack shows `X.1` where X is the damage dealt to the escort. `api_df_list` shows the escort's index, not 0.
- AE2. **Covers R1, R2.** Given a friendly fleet in 単縦陣, the interception probability for any single flagship-targeted attack is 45%. With a seeded RNG, a specific sequence of attacks produces deterministic interception outcomes.
- AE3. **Covers R3.** Given a friendly fleet where all escorts are at 小破 or worse (HP ≤ 75%), interception cannot trigger even when the flagship is targeted.
- AE4. **Covers R5.** Given a fleet where the flagship is a surface ship but the only healthy escort is a submarine, interception does not trigger (type mismatch).
- AE5. **Covers R6, R8.** Given an opening torpedo phase where the enemy targets the friendly flagship and interception triggers, `api_fydam` carries a `.1` decimal suffix and `isShield_f` returns true.

---

## Success Criteria

- Official server battle replays show the shield animation when `.1` damage values are present; our emulator produces the same animation by encoding `.1` when interception triggers
- `api_damage` field can represent both integer and `.1`-suffixed values
- Interception logic is deterministic under seeded RNG, enabling test verification
- Both friendly and enemy fleets can intercept

---

## Scope Boundaries

- Combined fleet (連合艦隊) interception restrictions are deferred until combined fleet sortie is implemented
- Night battle Zuiun cut-in (瑞雲カットイン) mid-attack health-state change edge case (escort goes from green to 小破 between hit 1 and hit 2) is deferred for accuracy tuning
- The escort selection algorithm (random vs priority-based among eligible escorts) needs confirmation from additional official data or wiki sources

---

## Key Decisions

- **Damage field type change is required.** `api_damage` must change from `Vec<Vec<i64>>` to a type that supports `.1` decimal suffixes. This has cross-cutting impact on the battle packet types and all code that constructs or reads damage values.
- **Interception is resolved at targeting time, not at rendering time.** The server swaps the target before damage calculation, then encodes the result. The client only renders based on the `.1` flag.

---

## Dependencies / Assumptions

- The `api_damage` type change may interact with the si_list fix plan (`docs/plans/2026-06-24-001-fix-si-list-ci-string-type-plan.md`) since both touch `BattleHougeki` fields. Sequencing should be coordinated.
- Formation ID is available at battle time via `BattleContext.friendly_formation_id` / `enemy_formation_id`.
- Escort selection: the wiki states the rate depends only on formation (not number of healthy escorts), but does not specify WHICH escort is selected when multiple are eligible. Assumed random among eligible escorts pending confirmation.

---

## Outstanding Questions

### Resolve Before Planning

- **[Affects R4][Needs research]** When multiple eligible escorts exist, which one is selected? Random uniform, or first-in-slot-order? The wiki says rate depends only on formation, not escort count — but the selection method among multiple escorts is unspecified.

### Deferred to Planning

- **[Affects R6][Technical]** Should `api_damage` use `f64` or a newtype wrapper to preserve the `.1` semantics without floating-point precision risks? Planning should evaluate `f64` vs `i64` with a separate boolean shield flag.
- **[Affects R8][Technical]** The `api_raigeki` phase uses `api_fydam`/`api_eydam` fields, not `api_damage` inside hougeki — the encoding for raigeki shield flags may differ structurally.
