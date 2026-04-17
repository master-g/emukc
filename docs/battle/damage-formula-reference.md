# KanColle Damage Formula Reference

> EmuKC battle formula specification — wiki truth vs current implementation.
> Sources: en.kancollewiki.net, kancolle.fandom.com
> Last updated: 2026-04-16

## 1. Damage Pipeline Overview

```
Basic Power → Pre-cap Modifiers → Cap → Post-cap Modifiers → Defense → Final Damage
     F₀            M_pre          cap()        M_post           A_hat         D
```

Final formula:

```
D = floor( ( floor(cap(F₀ × M_pre) × C) × M_post − A_hat ) × R )
```

Where:
- `F₀` = basic attack power (varies by attack type)
- `M_pre` = product of pre-cap modifiers (formation, engagement, damage state, night CI, ASW synergy)
- `cap()` = soft cap function
- `C` = critical modifier (1.5 normally)
- `M_post` = product of post-cap modifiers (artillery spotting, AP shell, contact, ammo)
- `A_hat` = defense power (armor with randomization)
- `R` = remaining ammo modifier

Minimum damage = 0 (miss) or 1 (hit). Overkill protection applies separately.

### Cap Function

```
cap(x, S) = floor(x)           if x ≤ S
            floor(S + sqrt(x − S))  otherwise
```

Per-attack-type caps:

| Attack Type | Cap (S) |
|---|---|
| Shelling | 220 |
| Carrier shelling | 220 |
| Torpedo | 180 |
| Airstrike | 170 (per bomber type) |
| ASW | 170 |
| Night battle | 360 |

### Defense Power

```
A_hat = floor(0.7 × A_t + 0.6 × random(0, floor(A_t) − 1)) − D_bonus
```

Where:
- `A_t` = defender's displayed armor stat
- `D_bonus` = armor penetration from specific depth charges (ASW only)
- Random component: uniform integer in `[0, floor(A_t) − 1]`

**Note**: This is a random range `[0.7A_t, 1.3A_t − 0.6]`, NOT a fixed multiplier.

---

## 2. Attack Type Formulas

### 2.1 Day Shelling (砲撃戦)

**Wiki formula:**

```
F₀ = firepower + 5                          (for non-CV)
F₀ = 55 + floor(1.5 × (FP + TP + floor(1.5 × DB) + improvement + CL_fit))  (for CV/CVL/CVB)
```

Pre-cap modifiers:
- Formation modifier (see table §3.1)
- Engagement modifier (see table §3.2)
- Damage state modifier (see §3.3)

Post-cap modifiers:
- Artillery spotting (DA=1.2×, AP CI=1.5×, etc.) — see §3.6
- Critical: 1.5×
- AP shell: 1.08/1.10/1.15× (see §3.7)
- Ammo: see §3.8

**Current code** (`calculate_shelling_damage`, core.rs:1225):
```rust
let attack_power = (attacker.ship.api_karyoku[0].max(0) as f64 + 5.0)
    * shelling_formation_modifier(formation_id);
let capped_power = apply_cap(attack_power * engagement.modifier(), 220.0) as f64;
let armor = defender.ship.api_soukou[0].max(0) as f64 * 0.7;
(capped_power - armor).floor().max(1.0) as i64
```

**Gaps:**
| # | Missing | Impact |
|---|---------|--------|
| S1 | Improvement bonus `Σ k_eq × √★` | Underestimates power for ★ equipment |
| S2 | CV special formula `55 + floor(1.5×(...))` | CV damage completely wrong |
| S3 | CL fit gun bonus `√単装 + 2√連装` | CL/CLT damage wrong with 14cm/15.2cm |
| S4 | Damage state modifier (chuuha×0.7, taiha×0.4) | No damage reduction when damaged |
| S5 | Defense randomization `0.7A + 0.6×rand(0,A−1)` | Uses fixed `A×0.7` instead |
| S6 | Artillery spotting post-cap modifiers | No DA/CI implementation |
| S7 | AP shell post-cap modifier | Not implemented |
| S8 | Critical hit (1.5×) | Not implemented |
| S9 | Ammo modifier | Not implemented |
| S10 | Scratch damage when attack < defense | Always minimum 1 instead |

### 2.2 Torpedo (雷撃戦)

**Wiki formula:**

```
F₀ = torpedo + improvement_bonus
```

Where `improvement_bonus = Σ(k_eq × √★)` for torpedo equipment.
`k_eq` for torpedoes = 1.2 per ★ level.

Pre-cap modifiers: formation, engagement, damage state (chuuha×0.8, taiha×0)

Post-cap: critical 1.5×, ammo

**Current code** (`calculate_torpedo_damage`, core.rs:1238):
```rust
let attack_power = (attacker.ship.api_raisou[0].max(0) as f64 + 5.0)
    * torpedo_formation_modifier(formation_id);
let capped_power = apply_cap(attack_power * engagement.modifier(), 180.0) as f64;
let armor = defender.ship.api_soukou[0].max(0) as f64 * 0.55;
```

**Gaps:**
| # | Missing | Impact |
|---|---------|--------|
| T1 | Improvement bonus `1.2 × √★` per torpedo | Underestimates torpedo power |
| T2 | Damage state (chuuha×0.8, taiha=0) | Damaged ships torpedo at full power |
| T3 | Defense uses fixed `A×0.55` not random | Defense always same value |
| T4 | Critical hit | Not implemented |
| T5 | Ammo modifier | Not implemented |
| T6 | Basic power should be `TP + improvement`, not `TP + 5` | Wrong base for opening/closing torpedo |

**Note**: `+5` constant is for shelling only. Torpedo basic power is just `TP + improvement_bonus`.

### 2.3 Airstrike (航空戦/航空攻撃)

**Wiki formula:**

Per bomber type (torpedo bombers, dive bombers, seaplane bombers separately):

```
F₀ = type_multiplier × Σ(stat_i × √(onslot_i)) + 25 + improvement_bonus
```

Where:
- Torpedo bomber: `stat = api_raig`, `type_multiplier = 0.8` or `1.5` (random per slot)
- Dive bomber: `stat = api_baku`, `type_multiplier = 1.0`
- Seaplane bomber: same as dive bomber

Pre-cap: contact modifier (1.12–1.2× based on contact plane accuracy)

Post-cap: critical 1.5×

**Current code** (`calculate_airstrike_damage`, core.rs:1073):
```rust
let total_bomb_power: f64 = attacker_ships.iter()
    .flat_map(|ship| ship.slot_items.iter().zip(ship.ship.api_onslot))
    .filter_map(|(slot_item, onslot)| {
        // ... filters for airstrike types
        let stat = if is_torpedo_bomber { mst.api_raig } else { mst.api_baku };
        Some(stat * (onslot as f64).sqrt())
    }).sum();
let raw_power = total_bomb_power + 25.0;
let capped = apply_cap(raw_power, 170.0) as f64;
let armor = defender.ship.api_soukou[0].max(0) as f64 * 0.6;
```

**Gaps:**
| # | Missing | Impact |
|---|---------|--------|
| A1 | Per-type calculation (should be separate for TB/DB) | Blends TB+DB into one pool |
| A2 | Torpedo bomber ×0.8/×1.5 random multiplier | TB always uses base stat |
| A3 | Contact modifier (×1.12–1.2) | Missing pre-cap bonus |
| A4 | Improvement bonus | Missing |
| A5 | Defense uses fixed `A×0.6` not random | Always same defense |
| A6 | Per-plane-type cap application | Single cap for all types |

### 2.4 Anti-Submarine Warfare (対潜攻撃)

**Wiki formula:**

```
F₀ = (√(base_asw) × 2 + √(equip_asw) × 1.5 + type_bonus) × synergy × armor_pen
```

Where:
- `base_asw` = ship ASW − equipment ASW (modernization + innate)
- `equip_asw` = Σ(equipment ASW stats)
- `type_bonus` = +13 (depth charge) or +8 (aircraft ASW)
- `synergy` = ASW synergy modifier (see §3.4)
- `armor_pen` = `1 + √(specific_DC_asw − 2) × 0.25` for Hedgehog/T3 DC, etc.

Pre-cap: formation, engagement, damage state

Post-cap: critical, ammo

**OASW conditions** (先制対潜):
- DE: ASW ≥ 60 + sonar
- DD/CL/CT/CLT/AO: ASW ≥ 100 + sonar
- CVL: ASW ≥ 65 + ASW aircraft
- CVB: ASW ≥ 100 + ASW aircraft
- BBV: ASW ≥ 100 + large sonar + ASW aircraft
- **Missing**: Isuzu K2, Tatsuta K2 unconditional OASW

**Current code** (`calculate_asw_damage`, core.rs:1368):
```rust
let raw_power = (base_asw.sqrt() * 2.0 + equip_asw.sqrt() * 1.5 + type_bonus) * synergy;
let modified = raw_power * asw_formation_modifier(formation_id) * engagement.modifier();
let capped = apply_cap(modified, 170.0) as f64;
let armor = defender.ship.api_soukou[0].max(0) as f64 * 0.7;
```

**Gaps:**
| # | Missing | Impact |
|---|---------|--------|
| W1 | ASW armor penetration (`√(DC_asw − 2) × 0.25`) | No DC-specific armor reduction |
| W2 | Damage state modifier | Missing |
| W3 | Defense randomization | Fixed `A×0.7` |
| W4 | Special unconditional OASW (Isuzu K2, Tatsuta K2) | Missing ship-specific OASW |
| W5 | Critical hit | Not implemented |
| W6 | Ammo modifier | Not implemented |

**Already correct:**
- Basic power formula `√base×2 + √equip×1.5 + bonus` ✓
- ASW synergy values (1.4375, 1.265, 1.15, 1.1) ✓
- Formation modifiers (diamond 1.2, echelon 1.1, abreast 1.3) ✓
- OASW threshold conditions for standard ship types ✓

### 2.5 Night Battle (夜戦)

**Wiki formula:**

```
F₀ = firepower + torpedo + improvement_bonus + night_recon_bonus
```

Where:
- `improvement_bonus = Σ(k_eq × √★)` for all equipment
- `night_recon_bonus` = +5/+7/+9 based on 夜偵 proficiency (only if AS+)

Night CI/DA are **pre-cap** modifiers:

| Attack Type | Multiplier | Hits | Wiki Name |
|---|---|---|---|
| Normal | 1.0× | 1 | 通常攻撃 |
| Double Attack | 1.2× | 2 | 連撃 |
| MainMainMain CI | 2.0× | 1 | 主砲CI |
| MainMainSec CI | 1.75× | 1 | 主副CI |
| TorpTorpTorp CI | 1.3× | 2 | 雷撃CI |
| MainTorpRadar CI | 1.625× | 1 | 主魚電CI |

Additional CI types not yet implemented:

| Attack Type | Multiplier | Hits | Wiki Name |
|---|---|---|---|
| Carrier Night Air Attack CI | varies | varies | 夜攻/夜戦CI |
| MainTorpTorp CI | 1.3× | 2 | 主魚魚CI |

**Current code** (`calculate_night_damage`, core.rs:1251):
```rust
let attack_power = (attacker.ship.api_karyoku[0].max(0)
    + attacker.ship.api_raisou[0].max(0) + 5) as f64;
let capped_power = apply_cap(attack_power, 360.0) as f64;
let armor = defender.ship.api_soukou[0].max(0) as f64 * 0.7;
```

**Gaps:**
| # | Missing | Impact |
|---|---------|--------|
| N1 | Improvement bonus `Σ k_eq × √★` | Missing star-level bonus |
| N2 | Night recon bonus (+5/+7/+9) | Missing fleet firepower buff |
| N3 | Defense randomization | Fixed `A×0.7` |
| N4 | Carrier night attack formula | Entirely missing |
| N5 | MainTorpTorp CI (1.3×, 2 hits) | Missing CI variant |

**Already correct:**
- CI types: 主主主(2.0), 主主副(1.75), 魚魚魚(1.3×2), 主魚電(1.625), 連撃(1.2×2) ✓
- CI detection from equipment loadout ✓
- Night cap = 360 ✓

### 2.6 Carrier Night Air Attack (夜間航空攻撃)

**Wiki formula (for CV night attack):**

```
F₀ = K_a × (FP + TP + DB × K_b + improvement) + night_recon_bonus
```

Where:
- `K_a = 3` if ship has night-capable aircraft, else not eligible
- `K_b = 0.45` for night aircraft (零戦62型(爆戦/岩井隊長機), etc.), `0.3` for others
- Night-capable aircraft types: Night Fighter, Night Torpedo Bomber, specific equipment IDs

This is an entirely separate formula from normal night battle and needs its own implementation.

**Current code**: Not implemented. CVs likely default to normal night damage or skip night battle.

---

## 3. Modifier Tables

### 3.1 Formation Modifiers

**Shelling:**

| Formation | ID | Shelling | Torpedo | ASW |
|---|---|---|---|---|
| Line Ahead | 1 | 1.0 | 1.0 | 1.0 |
| Double Line | 2 | 0.8 | 0.8 | 1.0 |
| Diamond | 3 | 0.7 | 0.7 | 1.2 |
| Echelon | 4 | 0.85 | 0.85 | 1.1 |
| Line Abreast | 5 | 0.6 | 0.6 | 1.3 |

**Current code**: ✓ Matches for shelling, torpedo, and ASW.

### 3.2 Engagement Modifiers

| Engagement | ID | Modifier |
|---|---|---|
| Parallel (同航戦) | 1 | 1.0 |
| Head-on (反航戦) | 2 | 0.8 |
| T-advantage (T字戦有利) | 3 | 1.2 |
| T-disadvantage (T字戦不利) | 4 | 0.6 |

**Current code**: Uses `engagement.modifier()` — assumed correct.

### 3.3 Damage State Modifier

Applied as pre-cap multiplier based on attacker's HP ratio at start of battle:

| State | HP% | Shelling | Torpedo | ASW |
|---|---|---|---|---|
| Normal | >75% | 1.0 | 1.0 | 1.0 |
| Chuuha (中破) | 25–75% | 0.7 | 0.8 | 0.7 |
| Taiha (大破) | <25% | 0.4 | 0 (cannot attack) | 0.4 |

**Current code**: Not implemented.

### 3.4 ASW Synergy Modifiers

| Sonar Type | DC Projector | Depth Charge | Synergy |
|---|---|---|---|
| Small Sonar | Yes | Yes | 1.4375 |
| Large Sonar | Yes | Yes | 1.265 |
| Any Sonar | — | Yes | 1.15 |
| — | Yes | Yes | 1.1 |
| None | — | — | 1.0 |

**Current code**: ✓ Correct values implemented in `asw_synergy_modifier()`.

**Note**: "DC Projector" vs "Depth Charge" distinction requires checking specific item IDs, not just `KcSlotItemType3::DepthCharge`. Current code treats all depth charges as both projector and charge (simplified).

### 3.5 Improvement Bonus (改修強化)

Applies to basic attack power:

```
improvement_bonus = Σ(k_eq × √★) across all equipment
```

Where `k_eq` varies by equipment type and attack phase:

| Equipment | Day Shelling | Torpedo | Night Battle | ASW |
|---|---|---|---|---|
| Small/Medium Main Gun | 1.0 | — | 1.0 | — |
| Large Main Gun | 1.5 | — | 1.5 | — |
| Secondary Gun | varies | — | varies | — |
| Torpedo | — | 1.2 | 1.2 | — |
| Dive Bomber | varies | — | — | — |
| Sonar | — | — | — | 1.0 |
| Depth Charge | — | — | — | 1.0 |

**Current code**: Not implemented.

### 3.6 Artillery Spotting (弾着観測射撃)

Day battle post-cap modifiers. Requires air superiority (AS+) and at least one recon aircraft.

| Type | Equipment | Multiplier | Hits |
|---|---|---|---|
| Double Attack (DA) | Main×2 + Recon | 1.2× | 2 |
| AP Shell CI | Main + AP + Recon | 1.5× | 1 |
| Main+Secondary CI | Main + Secondary + Recon | 1.3× | 2 |
| Main+Radar CI | Main + Radar + Recon | 1.2× | 2 |
| Main+AP+Secondary CI | Main + AP + Secondary + Recon | 1.35× | 2 |
| Main×2+Secondary CI | Main×2 + Secondary + Recon | 1.5× | 1 |

**Current code**: Not implemented.

### 3.7 AP Shell Modifier

Post-cap multiplier when attacker has AP Shell equipped and target is armored:

| Attacker | Target Type | Modifier |
|---|---|---|
| Large Gun + AP Shell | Heavy armor (CA, BB, etc.) | 1.08 |
| Large Gun + AP Shell | Regular armor | 1.10 |
| Large Gun + AP Shell | Light armor (DD, etc.) | 1.15 |

**Current code**: Not implemented.

### 3.8 Ammunition Modifier

Post-cap multiplier based on remaining ammunition percentage:

```
ammo_mod = floor(remaining_ammo / 50) / 100    if remaining < 50%
           1.0                                   otherwise
```

| Ammo Remaining | Modifier |
|---|---|
| ≥50% | 1.0 |
| 40–49% | 0.8 |
| 30–39% | 0.6 |
| 20–29% | 0.4 |
| 10–19% | 0.2 |
| 0–9% | 0.0 (scratch only) |

**Current code**: Not implemented.

### 3.9 Contact Modifier (接触)

Pre-cap airstrike modifier when aerial contact is established:

| Contact Plane Accuracy | Modifier |
|---|---|
| 0 | 1.12 |
| 1 | 1.15 |
| 2 | 1.17 |
| 3 | 1.20 |

**Current code**: Not implemented.

### 3.10 Scratch Damage (割合ダメージ)

When attack power < defense power:

```
D = floor(0.06 × H_t + 0.08 × random(0, H_t − 1))
```

Where `H_t` = target's current HP.

**Current code** (`roll_scratch_damage`, core.rs:368):
```rust
((current_hp as f64) * 0.06 + (random_part as f64) * 0.08).floor().max(1.0) as i64
```

✓ Correct formula for scratch damage itself. However, it is only applied in night battle against submarines — day shelling/torpedo use `max(1)` instead of checking attack < defense.

### 3.11 Overkill Protection (轟沈ストッパー)

Applies to friendly sortie ships only:

```
if effective_damage ≥ current_hp AND (is_flagship OR NOT was_taiha_at_entry):
    replace with proportional damage
    D_proportional = floor(H/2) + floor(rand(0, H) × 0.3)
    clamped to [0, current_hp − 1]
```

Where `H` = HP at node entry (`entry_hp`).

**Current code** (core.rs:194-214): ✓ Correctly implemented.

---

## 4. Gap Summary by Priority

### High Priority (affects all battles)

| # | Gap | Attack Types Affected | Implementation Effort |
|---|-----|----------------------|----------------------|
| 1 | Defense randomization | All | Low — replace fixed `A×k` with random formula |
| 2 | Damage state modifier | Shelling, Torpedo, ASW | Low — pre-cap multiplier based on HP% |
| 3 | Improvement bonus | All | Medium — requires star-level data from equipment |
| 4 | Scratch damage trigger | Shelling, Torpedo | Low — check if attack < defense |
| 5 | Ammo modifier | All | Low — post-cap multiplier |

### Medium-High Priority (affects specific scenarios)

| # | Gap | Attack Types Affected | Implementation Effort |
|---|-----|----------------------|----------------------|
| 6 | CV special shelling formula | Carrier shelling only | Medium — new basic power calculation |
| 7 | Artillery spotting | Day shelling | Medium — equipment detection + trigger rates |
| 8 | Critical hit | All | Medium — trigger rate + 1.5× modifier |
| 9 | Contact modifier | Airstrike only | Low — pre-cap multiplier |
| 10 | AP shell modifier | Day shelling | Low — post-cap multiplier |

### Medium Priority (specific ships/situations)

| # | Gap | Attack Types Affected | Implementation Effort |
|---|-----|----------------------|----------------------|
| 11 | CL fit gun bonus | CL/CLT shelling | Medium — equipment-specific additive bonus |
| 12 | Night recon bonus | Night battle | Low — fleet-wide pre-cap bonus |
| 13 | ASW armor penetration | ASW only | Medium — item-ID-specific armor reduction |
| 14 | Special OASW (Isuzu K2 etc.) | ASW only | Low — ship-ID-based condition |
| 15 | Torpedo bomber ×0.8/×1.5 | Airstrike only | Low — random per slot |
| 16 | Carrier night air attack | Night battle | Large — entirely new formula |
| 17 | Per-type airstrike cap | Airstrike only | Medium — separate calculations |
| 18 | ASW synergy DC projector ID check | ASW only | Low — check specific item IDs |

---

## 5. Implementation Order Suggestion

Based on dependency chains and impact:

```
Phase 1: Foundation fixes (all battles immediately better)
  ├─ Defense randomization
  ├─ Damage state modifier
  ├─ Scratch damage trigger (attack < defense)
  └─ Torpedo basic power fix (remove +5)

Phase 2: Equipment-driven power (requires star-level access)
  ├─ Improvement bonus (Σ k_eq × √★)
  ├─ CV special shelling formula
  └─ CL fit gun bonus

Phase 3: Post-cap modifiers (visible damage spikes)
  ├─ Critical hit system
  ├─ Artillery spotting
  ├─ AP shell modifier
  └─ Ammo modifier

Phase 4: Airstrike fidelity
  ├─ Per-type airstrike calculation
  ├─ Contact modifier
  ├─ Torpedo bomber ×0.8/×1.5
  └─ Airstrike improvement bonus

Phase 5: Night battle fidelity
  ├─ Night recon bonus
  ├─ Night improvement bonus
  ├─ MainTorpTorp CI variant
  └─ Carrier night air attack

Phase 6: ASW refinement
  ├─ ASW armor penetration
  ├─ Special unconditional OASW
  └─ DC projector item ID distinction
```

---

## 6. Formation + Engagement Modifier Application Point

Current code applies formation and engagement differently per attack type:

| Attack Type | Formation | Engagement | Code |
|---|---|---|---|
| Shelling | Pre-cap ✓ | Pre-cap ✓ | `× form × eng` then cap |
| Torpedo | Pre-cap ✓ | Pre-cap ✓ | `× form × eng` then cap |
| ASW | Pre-cap ✓ | Pre-cap ✓ | `× asw_form × eng` then cap |
| Airstrike | N/A | N/A | No formation/engagement |
| Night | None | None | No formation/engagement |

Wiki confirms: formation + engagement are pre-cap for shelling/torpedo/ASW. Night battle has no formation/engagement modifier. Airstrike has none either. Current application point is correct.

---

## 7. Key Formula Constants Reference

| Constant | Value | Context |
|---|---|---|
| Shelling base | `FP + 5` | Non-CV shelling |
| Shelling cap | 220 | Day shelling soft cap |
| Torpedo cap | 180 | Opening/closing torpedo |
| Airstrike cap | 170 | Per bomber type |
| ASW cap | 170 | Anti-submarine |
| Night cap | 360 | Night battle |
| ASW aircraft bonus | +8 | Type bonus for aircraft ASW |
| ASW depth charge bonus | +13 | Type bonus for depth charge ASW |
| Scratch coefficient | `0.06×H + 0.08×rand(0,H−1)` | Proportional damage |
| Overkill coefficient | `floor(H/2) + floor(rand(0,H)×0.3)` | Sinking protection |
| Critical multiplier | 1.5 | Post-cap |
| DA multiplier | 1.2 | Day/Night double attack |
