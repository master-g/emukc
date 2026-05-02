## Context

The battle simulation in `crates/emukc_gameplay/src/game/battle/core.rs` (~4.5k lines) determines attack eligibility and damage application. Currently:

1. **Attack display type selection** uses `DAY_SURFACE_DISPLAY_TYPES` — an equipment list — to choose the display type for shelling phase. If a DD has no gun but a torpedo, the torpedo display type is selected, causing the client to render a torpedo attack animation during shelling phase.
2. **Closing torpedo participation** checks BOTH ship type AND `api_raisou[0] > 0`. The ship type whitelist includes DE/LHA/CT/AO (all have base torpedo = 0, so excluded by the stat check anyway, but logically wrong) and excludes BBs with base torpedo > 0 (Bismarck drei, Гангут, etc.).
3. **Damage application** in `apply_damage` caps effective damage to current HP for ALL targets (friendly and enemy, sortie and practice), preventing overkill display against enemy ships in sortie.

In real KanColle (source: wikiwiki.jp/kancolle/戦闘について):

**Shelling (砲撃戦):**
- SS/SSV cannot shell (except with 特二式内火艇 vs installation targets)
- CV/CVL/CVB need 艦攻 or 艦爆 equipped and not fully shot down
- All other surface ships always shell, regardless of equipment

**Closing Torpedo (雷撃戦):**
- Rule: base torpedo stat (素の雷装 / `api_raisou[0]`) ≥ 1 → can participate
- **"逆に言えば素の雷装値が1以上ならば艦種問わず雷撃戦に参加する"** — any ship type qualifies if base torpedo ≥ 1
- This includes BBs with base torpedo (Bismarck drei, Гангут, Conte di Cavour, etc.)
- 中破 or 大破 → cannot participate

**Opening Torpedo (開幕雷撃):**
- **"基本的に特殊潜航艇装備が必要"** — minisub (甲标的) equipment is required for non-submarine ships
- SS/SSV with level ≥ 10 can opening torpedo without equipment
- CLT can opening torpedo (inherent)
- Ships with minisub equipped (ABKM K2, special CAVs, etc.) can also opening torpedo

## Goals / Non-Goals

**Goals:**
- Shelling display type selected by ship type + equipment availability (not equipment checklist)
- Shelling damage formula uses base stats + equipment bonuses (not equipment-gated)
- Closing torpedo eligibility: base torpedo stat > 0 (remove restrictive ship type whitelist)
- Opening torpedo eligibility: minisub equipment OR CLT type OR SS/SSV level ≥ 10
- Enemy ships in sortie receive uncapped damage (excess/overkill visible)
- Practice and friendly sortie damage remain capped with sinking protection

**Non-Goals:**
- Night battle overhaul (separate change)
- ASW attack type changes (already ship-type-based)
- Changing sinking protection logic
- Equipment improvement bonus changes

## Decisions

### D1: Corrected phase participation rules (wikiwiki-verified)

**Shelling eligibility** — ship type based:
| Ship Type | Can Shell | Notes |
|-----------|-----------|-------|
| DD, DE, CL, CLT, CT, CA, CAV, FBB, BB, BBV, AV, LHA, AO | Yes | Always |
| CV, CVL, CVB | Conditional | Requires >0 艦攻/艦爆 not all shot down |
| SS, SSV | No | Except with 特二式内火艇 vs installations |

**Closing torpedo eligibility** — base torpedo stat based:
| Condition | Eligible |
|-----------|----------|
| `api_raisou[0]` (base torpedo) > 0 AND not 中破/大破 | Yes |
| `api_raisou[0]` = 0 | No |

This rule produces correct results for all ship types:
- DD, CL, CLT, CA, CAV: most have base torpedo > 0 → Yes
- SS, SSV: base torpedo > 0 → Yes
- Bismarck drei, Гангут, Conte di Cavour, 金剛型第三改装, Norge級: base torpedo > 0 → Yes
- DE, LHA, AR, most BB/BBV/FBB, most CV/CVL/CVB: base torpedo = 0 → No
- AV: 千歳改/甲, 瑞穂, 日進 (base torpedo > 0) → Yes; 秋津洲, Commandant Teste (base torpedo = 0) → No
- AO: 速吸改 (base torpedo > 0) → Yes; 速吸未改 (base torpedo = 0) → No
- CT: 香取, 鹿岛 (base torpedo = 0) → No

**Opening torpedo eligibility** — equipment + type based:
| Condition | Eligible |
|-----------|----------|
| SS/SSV, level ≥ 10, base torpedo > 0 | Yes |
| CLT type | Yes |
| Any ship with 特殊潜航艇 (minisub/甲标的) equipped, base torpedo > 0 | Yes |
| All other ships | No |

Damage state does NOT prevent opening torpedo (開幕雷撃は損傷度は問わず発動する).

**Rationale**: The wikiwiki makes it clear that closing torpedo is fundamentally gated by base torpedo stat, not ship type. The ship type list in the wiki is a convenience guide to which types TYPICALLY have torpedo, but the actual rule is `素の雷装値が1以上ならば艦種問わず`. Ship type *correlates* with base torpedo but is not the determinant. Using base torpedo stat as the gate correctly handles all edge cases (BBs with torpedo, AV with/without, etc.). Opening torpedo requires equipment (甲标的) for non-submarine ships, with CLT and high-level SS/SSV as the only equipment-free exceptions.

### D2: Attack display type fallback

**Decision**: When a ship has no relevant equipment for display type selection, assign `api_at_type = 0` (normal single attack) and use base firepower for damage calculation. Ship type determines whether the ship participates; equipment determines the display type and adds to stats.

**Rationale**: Real KanColle allows bare-ship attacks with minimal power. The shelling display type selection should use available equipment as modifiers on top of ship-type-gated participation, not as participation gates themselves.

### D3: Uncapped enemy damage in sortie

**Decision**: In `apply_damage`, change the effective damage capping logic:

| Target | Mode | Damage Behavior |
|--------|------|-----------------|
| Enemy | Sortie | Full raw damage, HP can go negative |
| Enemy | Practice | Cap to current HP |
| Friendly | Sortie | Sinking protection (existing) |
| Friendly | Practice | Cap to current HP |

`BattleRuntimeShip` already has `is_friendly` and `is_sortie` fields (core.rs:213). No signature change needed — modify the internal logic to skip capping when `!self.is_friendly && self.is_sortie`.

**Rationale**: In sortie, the client shows overkill damage against enemies. In practice, HP is preserved. The context fields already exist on `BattleRuntimeShip`.

### D4: Wikiwiki audit completed (2026-05-02)

Source: wikiwiki.jp/kancolle/戦闘について (last modified: 2026-03-31)

Key findings verified:
- Closing torpedo rule: `素の雷装値 ≥ 1` (any ship type) — NOT ship type whitelist
- Opening torpedo rule: 特殊潜航艇 equipment required for non-SS/CLT ships
- Shelling: CV need 艦攻/艦爆 equipped (not all shot down), all other surface ships always participate
- SS shelling exception: 特二式内火艇 vs installations (edge case, deferred)

## Risks / Trade-offs

- **[Risk] Large refactor surface**: core.rs is ~4.6k lines → Mitigate with targeted function replacements, not file rewrite
- **[Risk] Practice/regression**: Changes to shared battle code may break practice battles → Mitigate with existing practice tests and new test cases
- **[Risk] Client desync**: Changing attack display types may confuse the game client → Mitigate by matching original server behavior as verified by wikiwiki
- **[Risk] Base torpedo stat reliability**: Relies on `api_raisou[0]` correctly reflecting 素の雷装 for both friendly and enemy ships → Verify enemy ship data fidelity in codex
