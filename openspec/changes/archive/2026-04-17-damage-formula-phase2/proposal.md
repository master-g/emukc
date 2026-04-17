## Why

Day battle, torpedo, night battle, and ASW damage formulas lack 改修強化 (equipment improvement bonuses), CV special formula, CL 軽砲補正, 夜偵 contact bonus, and 爆雷投射機/depth charge armor reduction. Current `calculate_shelling_damage`, `calculate_torpedo_damage`, `calculate_night_damage`, and `calculate_asw_damage` use only base stats + formation/engagement/damage-state modifiers. This produces damage values that diverge significantly from the actual game, limiting battle fidelity now that enemy stats are accurate (Track 1 complete).

## What Changes

- **Day shelling 改修強化**: Add equipment star-level bonus to `calculate_shelling_damage`. Formula: `Σ(√star × type_weight)` per equipped weapon.
- **CV special formula**: Ships with CV/CVL/CVB type use `1.5 × torpedo_bomber_count + 55` instead of `firepower + 5` when dive/torpedo bombers equipped.
- **CL 軽砲補正**: Light cruisers (CL/CLT) get `√single_mount_count + 2 × √twin_mount_count` bonus to basic power from small/medium caliber guns.
- **Torpedo 改修強化**: `calculate_torpedo_damage` adds `torpedo_star × 1.2` per torpedo equipment.
- **Night battle 改修強化**: `calculate_night_damage` adds equipment star bonuses to basic power.
- **夜偵 contact bonus**: Night battle adds +5/+7/+9 based on air superiority state when night recon aircraft equipped.
- **ASW 爆雷投射機 armor reduction**: Depth charge projectors apply `√(equip_asw − 2)` armor reduction to submarine targets.

## Capabilities

### New Capabilities
- `equipment-improvement-bonus`: Equipment star-level (★) improvement power bonuses for day shelling, torpedo, and night battle formulas.

### Modified Capabilities
- `battle-damage-foundation`: Extends existing damage spec with CV special formula, CL 軽砲補正, 夜偵 contact bonus, and ASW armor reduction requirements.

## Impact

- `crates/emukc_gameplay/src/game/battle/core.rs`: Primary change target. All four `calculate_*_damage` functions gain equipment-aware parameters.
- `crates/emukc_model/src/kc2/`: `KcApiSlotItem` already has `api_level` (star level). No model changes needed.
- Tests: New unit tests for each formula element. Existing damage tests unaffected (basic power path unchanged when star=0).
- No API response format changes. Damage values change but response structure stays identical.
- Non-goal: Submarine-specific armor correction (× 0.7/× 0.55), combined fleet formulas, support expedition formulas.
