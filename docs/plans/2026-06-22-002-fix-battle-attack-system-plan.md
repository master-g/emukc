---
title: "fix: Battle Attack System — Phase Participation Rules and Enemy Overkill"
type: fix
date: 2026-06-22
origin: openspec/changes/fix-battle-attack-system (translated during openspec sunset)
---

# fix: Battle Attack System

## Summary

Fix three interrelated bugs in battle attack eligibility and damage application, all verified against wikiwiki.jp/kancolle/戦闘について:

1. **Shelling display type** uses an equipment checklist (`DAY_SURFACE_DISPLAY_TYPES`) as the participation gate, causing a DD with only a torpedo to render a torpedo attack animation during shelling. The real rule is ship-type-gated participation; equipment only selects display type and adds stats.
2. **Closing torpedo participation** restricts to a hardcoded ship type whitelist that excludes BBs with base torpedo > 0 (Bismarck drei, Гангут, etc.) and includes types with base torpedo = 0 (DE, LHA). The actual rule is `api_raisou[0] > 0` for any ship type.
3. **Enemy overkill** — `apply_damage` caps ALL damage to current HP, preventing overkill display against enemy ships in sortie. Sortie enemies should go negative; practice stays capped.

## Reconciliation (2026-06-22)

Read-only audit of all 27 tasks against current code (`crates/emukc_battle/src/`).

| Status | Count |
|--------|-------|
| Done | 9 |
| Partial | 0 |
| Not done | 18 |

**Done — U1 fully shipped (3/3):** `can_shell_day_ship` (targeting.rs:257) is ship-type based (SS/SSV excluded, CV conditional on planes, all others always). `DayAttackType::Normal = 0` (day_cutin.rs:14) is the fallback via `normal_attack()`. Damage uses base stats + equipment bonuses, not equipment-gated participation.

**Done — U3 partially (2/3):** equipment check for 甲標的 preserved (targeting.rs:225), CLT always eligible. **Missing:** SS/SSV level ≥ 10 gate — currently ALL SS/SSV are unconditionally eligible regardless of level.

**Done — U4 partially (4/6):** `is_friendly`/`is_sortie` fields exist (runtime.rs:32,35). Practice capping, friendly sinking protection, and practice friendly capping all verified present. **Missing:** enemy sortie overkill — `apply_damage` (runtime.rs:87) still caps ALL damage via `raw_damage.min(self.current_hp)`; downstream negative-HP audit (4.3) not performed.

**Not done — U2 (0/4):** `can_closing_torpedo_ship` (targeting.rs:227) still has the 12-entry ship-type `matches!` whitelist. The stat gate (`api_raisou[0] <= 0 → false`) IS present, but the whitelist on top blocks BBs with base torpedo > 0 (Bismarck drei etc.) and includes types with base torpedo = 0 (DE, LHA) as no-ops.

**Not done — U5 (0/11):** no regression tests for any of the four fix areas.

### Execution residual (18 tasks)

The genuinely remaining work, re-prioritized:

1. **U2 closing torpedo whitelist removal (4 tasks)** — remove the `matches!` whitelist in `can_closing_torpedo_ship`; verify ship-type coverage. Self-contained, no deps.
2. **U3.2 SS/SSV level ≥ 10 gate (1 task)** — add level check to the SS/SSV branch in `can_opening_torpedo_ship`.
3. **U4.1 enemy sortie overkill + U4.3 audit (2 tasks)** — add `!is_friendly && is_sortie` skip in `apply_damage`; audit MVP/win-rank/battle-result for negative HP tolerance.
4. **U5 tests (11 tasks)** — regression tests for all four fix areas.

U1 needs no further work — it was already correct.

## Problem Frame

The battle simulation in `crates/emukc_gameplay/src/game/battle/core.rs` (~4.5k lines) determines attack eligibility and damage application. Three defects exist:

1. **Attack display type selection** uses `DAY_SURFACE_DISPLAY_TYPES` — an equipment list — to choose the display type for shelling phase. If a DD has no gun but a torpedo, the torpedo display type is selected, causing the client to render a torpedo attack animation during shelling phase.
2. **Closing torpedo participation** checks BOTH ship type AND `api_raisou[0] > 0`. The ship type whitelist includes DE/LHA/CT/AO (all have base torpedo = 0, so excluded by the stat check anyway, but logically wrong) and excludes BBs with base torpedo > 0 (Bismarck drei, Гангут, Conte di Cavour, etc.).
3. **Damage application** in `apply_damage` caps effective damage to current HP for ALL targets (friendly and enemy, sortie and practice), preventing overkill display against enemy ships in sortie.

The wikiwiki source makes the correct rules unambiguous: closing torpedo is gated by base torpedo stat (`素の雷装値が1以上ならば艦種問わず`), opening torpedo requires minisub equipment for non-SS/CLT ships, and shelling participation is ship-type based (SS excluded, CV conditional on planes, all others always).

## Requirements

- **R1. Shelling participation is ship-type based.** SS/SSV cannot shell (except with 特二式内火艇 vs installations — edge case deferred). CV/CVL/CVB shell only with 艦攻 or 艦爆 equipped and not fully shot down. All other surface ships always shell, regardless of equipment.
- **R2. Attack display type fallback.** When a ship has no relevant equipment, assign `api_at_type = 0` (normal single attack) and use base firepower for damage calculation. Ship type determines participation; equipment determines display type and adds to stats.
- **R3. Closing torpedo eligibility uses base torpedo stat only.** `api_raisou[0] > 0` AND not 中破/大破 → eligible. No ship type whitelist. Damage state gates closing torpedo but NOT opening torpedo (開幕雷撃は損傷度は問わず発動する).
- **R4. Opening torpedo eligibility preserves equipment gate.** 特殊潜航艇 (minisub/甲标的) required for non-submarine ships. SS/SSV level ≥ 10 can opening torpedo without equipment. CLT always eligible.
- **R5. Enemy overkill in sortie.** Enemy ships in sortie battles receive uncapped damage (HP can go negative). `BattleRuntimeShip` already has `is_friendly` and `is_sortie` fields (core.rs:213) — change capping logic, not signature.
- **R6. Practice and friendly damage unchanged.** Practice enemy damage capped at current HP. Friendly sortie sinking protection (轟沈ストッパー) unchanged. Practice friendly damage capped at current HP.

## Non-goals

- Night battle attack type overhaul (separate concern, can be addressed later)
- ASW attack type changes (already ship-type-based)
- Changing the sinking protection logic for friendly ships
- Equipment improvement bonus changes (separate system)
- SS shelling exception with 特二式内火艇 vs installations (edge case, deferred)

## Key Technical Decisions

### KTD1. Corrected phase participation rules (wikiwiki-verified)

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

This rule produces correct results for all ship types: DD/CL/CLT/CA/CAV and SS/SSV (most have base torpedo > 0 → Yes); BBs with base torpedo (Bismarck drei, Гангут, Conte di Cavour, 金剛型第三改装, Norge級 → Yes); DE/LHA/AR/most BB/BBV/FBB/most CV/CVL/CVB (base torpedo = 0 → No); AV (千歳改/甲, 瑞穂, 日進 → Yes; 秋津洲, Commandant Teste → No); AO (速吸改 → Yes; 速吸未改 → No); CT (香取, 鹿岛 → No).

**Opening torpedo eligibility** — equipment + type based:

| Condition | Eligible |
|-----------|----------|
| SS/SSV, level ≥ 10, base torpedo > 0 | Yes |
| CLT type | Yes |
| Any ship with 特殊潜航艇 (minisub/甲标的) equipped, base torpedo > 0 | Yes |
| All other ships | No |

Damage state does NOT prevent opening torpedo.

**Rationale**: The wikiwiki makes it clear that closing torpedo is fundamentally gated by base torpedo stat, not ship type. Ship type *correlates* with base torpedo but is not the determinant. Using base torpedo stat as the gate correctly handles all edge cases (BBs with torpedo, AV with/without, etc.). Opening torpedo requires equipment (甲标的) for non-submarine ships, with CLT and high-level SS/SSV as the only equipment-free exceptions.

### KTD2. Attack display type fallback

**Decision**: When a ship has no relevant equipment for display type selection, assign `api_at_type = 0` (normal single attack) and use base firepower for damage calculation. Ship type determines whether the ship participates; equipment determines the display type and adds to stats.

**Rationale**: Real KanColle allows bare-ship attacks with minimal power. The shelling display type selection should use available equipment as modifiers on top of ship-type-gated participation, not as participation gates themselves.

### KTD3. Uncapped enemy damage in sortie

**Decision**: In `apply_damage`, change the effective damage capping logic:

| Target | Mode | Damage Behavior |
|--------|------|-----------------|
| Enemy | Sortie | Full raw damage, HP can go negative |
| Enemy | Practice | Cap to current HP |
| Friendly | Sortie | Sinking protection (existing) |
| Friendly | Practice | Cap to current HP |

`BattleRuntimeShip` already has `is_friendly` and `is_sortie` fields (`crates/emukc_gameplay/src/game/battle/core.rs:213`). No signature change needed — modify the internal logic to skip capping when `!self.is_friendly && self.is_sortie`.

**Rationale**: In sortie, the client shows overkill damage against enemies. In practice, HP is preserved. The context fields already exist on `BattleRuntimeShip`.

### KTD4. Wikiwiki audit completed (2026-05-02)

Source: wikiwiki.jp/kancolle/戦闘について (last modified: 2026-03-31). Key findings verified: closing torpedo rule (`素の雷装値 ≥ 1`, any ship type, NOT ship type whitelist); opening torpedo rule (特殊潜航艇 equipment required for non-SS/CLT ships); shelling (CV need 艦攻/艦爆, all other surface ships always participate); SS shelling exception (特二式内火艇 vs installations — deferred).

## High-Level Technical Design

Three function-level changes in `crates/emukc_gameplay/src/game/battle/core.rs`, each surgical:

1. **Shelling display type (`day_attack_display_ids`)** — replace the equipment-checklist participation gate with ship-type gate. Equipment still selects display type when present; fallback to `api_at_type = 0` with base firepower + 5 when absent.
2. **Closing torpedo (`can_closing_torpedo_ship`)** — remove the ship type whitelist, keep only `api_raisou[0] > 0` + not sunk + not 中破/大破.
3. **Opening torpedo (`can_opening_torpedo_ship`)** — keep the 甲标的 equipment check (do NOT remove it), add SS/SSV level ≥ 10 exception, keep CLT always eligible.
4. **Damage application (`apply_damage`)** — skip the `raw_damage.min(self.current_hp)` capping when `!self.is_friendly && self.is_sortie`.

No signature changes to any public function. `BattleRuntimeShip` already carries the `is_friendly`/`is_sortie` context fields needed by KTD3.

## Implementation Units

### U1. Shelling Display Type Fix

- **Goal:** Fix shelling participation to be ship-type based (not equipment-checklist based), and add the `api_at_type = 0` fallback when no relevant equipment is present.
- **Requirements:** R1, R2.
- **Dependencies:** none.
- **Files:**
  - `crates/emukc_gameplay/src/game/battle/core.rs` — primary file, `day_attack_display_ids` and shelling phase participation.
  - `crates/emukc_gameplay/src/game/battle/sortie.rs` — sortie battle handlers.
- **Tasks:**
  - [x] 1.1 Change `day_attack_display_ids` to use ship type for participation gate, not equipment-only checklist
  - [x] 1.2 Implement attack display type fallback: when no relevant equipment, assign `api_at_type = 0` with base firepower + 5 (design D2)
  - [x] 1.3 Ensure shelling damage formula uses base ship stats plus equipment bonuses, not equipment-gated
- **Patterns to follow:** the ship-type tables in KTD1 (shelling eligibility row).
- **Test scenarios:**
  - **Happy:** DD with no equipment shelling shows `api_at_type = 0`, not torpedo attack.
  - **Happy:** CV with attack planes participates in shelling.
  - **Error:** CV without planes excluded from shelling.
  - **Edge:** DD with only torpedo shows normal shelling attack (not torpedo animation).
- **Verification:** tasks 1.1–1.3 complete; existing battle tests pass.

### U2. Closing Torpedo Fix

- **Goal:** Replace the ship type whitelist with base torpedo stat as the sole closing torpedo gate, per wikiwiki.
- **Requirements:** R3.
- **Dependencies:** none (parallelizable with U1).
- **Files:**
  - `crates/emukc_gameplay/src/game/battle/core.rs` — `can_closing_torpedo_ship`.
- **Tasks:**
  - [ ] 2.1 In `can_closing_torpedo_ship`, remove the ship type whitelist — keep only `api_raisou[0] > 0` + not sunk + not 中破/大破
  - [ ] 2.2 Verify DE, LHA, AR, CV/CVL/CVB, most BB are correctly excluded (base torpedo = 0)
  - [ ] 2.3 Verify BB with base torpedo > 0 (Bismarck drei, Гангут, Conte di Cavour, 金剛型第三改装, Norge級) are now included
  - [ ] 2.4 Verify AV/AO/CT correctly follow base torpedo stat (not ship type): 千歳改/甲 included vs 秋津洲 excluded
- **Patterns to follow:** the closing torpedo eligibility table in KTD1.
- **Test scenarios:**
  - **Happy:** DD with base torpedo > 0 participates in closing torpedo.
  - **Happy:** BB with base torpedo > 0 (Bismarck drei) participates in closing torpedo.
  - **Error:** Ship with base torpedo = 0 (DE, LHA) excluded from closing torpedo.
  - **Error:** 中破 ship excluded from closing torpedo.
- **Verification:** tasks 2.1–2.4 complete; verification tasks confirm expected ship-type coverage.

### U3. Opening Torpedo Fix

- **Goal:** Preserve the 甲标的 equipment gate for non-submarine ships and add the SS/SSV level ≥ 10 equipment-free exception.
- **Requirements:** R4.
- **Dependencies:** none (parallelizable with U1, U2).
- **Files:**
  - `crates/emukc_gameplay/src/game/battle/core.rs` — `can_opening_torpedo_ship`.
- **Tasks:**
  - [x] 3.1 Keep equipment check for 特殊潜航艇 (minisub/甲标的) in `can_opening_torpedo_ship` — do NOT remove it
  - [ ] 3.2 Add SS/SSV level ≥ 10 requirement for equipment-free opening torpedo
  - [x] 3.3 Keep CLT type as always eligible for opening torpedo (with `api_raisou[0] > 0`)
- **Patterns to follow:** the opening torpedo eligibility table in KTD1.
- **Test scenarios:**
  - **Happy:** ABKM改二 with 甲标的 participates in opening torpedo (equipment-based).
  - **Happy:** CLT participates in opening torpedo.
  - **Error:** SS level < 10 does NOT opening torpedo without 甲标的.
  - **Error:** SS level ≥ 10 opens torpedo without equipment.
- **Verification:** tasks 3.1–3.3 complete; equipment gate intact, SS level exception added.

### U4. Damage Application Fix

- **Goal:** Allow overkill damage against enemy ships in sortie battles (HP goes negative), while keeping practice and friendly damage capped.
- **Requirements:** R5, R6.
- **Dependencies:** none (parallelizable with U1–U3, but U4.3 audits downstream consumers that may interact with the other units).
- **Files:**
  - `crates/emukc_gameplay/src/game/battle/core.rs` — `apply_damage` (the capping logic, near `BattleRuntimeShip` fields at line 213).
  - `crates/emukc_gameplay/src/game/battle/sortie.rs` — verify sortie handlers.
  - `crates/emukc_gameplay/src/game/battle/practice.rs` — verify practice handlers (no regression).
  - `crates/emukc_gameplay/src/game/sortie_result.rs` — HP snapshot for battle results (verify negative HP tolerance).
- **Tasks:**
  - [ ] 4.1 In `apply_damage`, change enemy sortie capping: when `!self.is_friendly && self.is_sortie`, skip `raw_damage.min(self.current_hp)` — allow HP to go negative
  - [x] 4.2 `BattleRuntimeShip` already has `is_friendly` and `is_sortie` fields (core.rs:213) — no signature change needed
  - [ ] 4.3 Audit downstream consumers of HP for enemy ships: verify MVP calculation, `calculate_win_rank`, and battle result handlers tolerate negative HP
  - [x] 4.4 Verify practice battles still cap enemy damage at current HP
  - [x] 4.5 Verify friendly sinking protection unchanged for sortie
  - [x] 4.6 Verify practice friendly damage still capped at current HP
- **Patterns to follow:** KTD3 damage-behavior table (Enemy Sortie → no cap; all others → cap or sinking protection).
- **Test scenarios:**
  - **Happy:** enemy overkill damage in sortie (HP goes negative).
  - **Error:** enemy damage capped in practice.
  - **Edge:** friendly sinking protection unchanged for sortie.
  - **Edge:** practice friendly damage capped at current HP.
- **Verification:** tasks 4.1–4.6 complete; downstream consumers (MVP, win rank, battle result) tolerate negative enemy HP.

### U5. Testing

- **Goal:** Add regression tests for all four fix areas and verify no existing battle test regresses.
- **Requirements:** R1–R6.
- **Dependencies:** U1–U4 (tests exercise the fixed logic).
- **Files:**
  - `tests/gameplay_tests/` — new test cases for each scenario below.
- **Tasks:**
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
- **Patterns to follow:** existing battle tests in `tests/gameplay_tests/` and `crates/emukc_gameplay/src/game/battle/`.
- **Test scenarios:** enumerated in tasks 5.1–5.9 (each maps to one acceptance scenario below).
- **Verification:** tasks 5.1–5.11 complete; all new tests pass; existing battle tests green.

## Behavioral notes

This plan was translated from `openspec/changes/fix-battle-attack-system/`. The openspec `specs/` deltas are captured as follows:

- **`battle-attack-type` (ADDED requirements):** This capability is newly introduced by this change. It codifies the four shelling/closing-torpedo/opening-torpedo/display-fallback requirements (R1–R4 above) plus the full WHEN/THEN scenario set. After the openspec sunset (migration plan U5), this contract has no checked-in home — it lives in this plan's Requirements + Implementation Units until captured into `docs/solutions/architecture-patterns/` during the change's own implementation (a new `battle-attack-type.md` should be created there when this plan ships).

- **`battle-damage-foundation` (MODIFIED requirements):** The damage-application-with-mode-dependent-capping requirement modifies the existing damage foundation. That foundation was already migrated to `docs/solutions/architecture-patterns/battle-damage-foundation.md` (migration plan U1). The note in that doc — *"this enemy-overkill requirement is MODIFIED by the fix-battle-attack-system change — currently capped to current HP pending that change"* — is resolved when this plan's U4 ships. Update that doc's enemy-overkill clause at that time.

## Acceptance / Done

The change is complete when all hold:

- A1. U1–U5 each landed; working tree clean.
- A2. `cargo fmt --check`, `cargo clippy --workspace -- -W warnings`, `cargo test --test gameplay_tests` all green.
- A3. Shelling participation is ship-type based (SS excluded, CV conditional on planes, all others always).
- A4. Closing torpedo uses `api_raisou[0] > 0` (no ship type whitelist).
- A5. Opening torpedo preserves 甲标的 equipment gate + adds SS level ≥ 10 exception.
- A6. Enemy sortie damage is uncapped (overkill visible); practice and friendly damage unchanged.
- A7. `docs/solutions/architecture-patterns/battle-damage-foundation.md` enemy-overkill note updated to reflect the shipped behavior.

## Risks & Dependencies

- **Large refactor surface.** `core.rs` is ~4.6k lines. Mitigate with targeted function replacements (`day_attack_display_ids`, `can_closing_torpedo_ship`, `can_opening_torpedo_ship`, `apply_damage`), not a file rewrite.
- **Practice/regression.** Changes to shared battle code may break practice battles. Mitigate with existing practice tests and new test cases (U5 tasks 5.8, 5.11).
- **Client desync.** Changing attack display types may confuse the game client. Mitigate by matching original server behavior as verified by wikiwiki (KTD4).
- **Base torpedo stat reliability.** Relies on `api_raisou[0]` correctly reflecting 素の雷装 for both friendly and enemy ships. Verify enemy ship data fidelity in codex during U2 verification.
- **Negative HP downstream.** Allowing enemy HP to go negative may break consumers that assume HP ≥ 0. Task 4.3 audits MVP calculation, `calculate_win_rank`, and battle result handlers explicitly.

## Sources / Research

- wikiwiki.jp/kancolle/戦闘について (last modified: 2026-03-31) — the authoritative source for all phase participation rules, audited 2026-05-02 (KTD4).
- `crates/emukc_gameplay/src/game/battle/core.rs` — primary implementation file (`day_attack_display_ids`, `can_closing_torpedo_ship`, `can_opening_torpedo_ship`, `apply_damage`, `BattleRuntimeShip`).
- `crates/emukc_gameplay/src/game/battle/sortie.rs` — sortie battle handlers.
- `crates/emukc_gameplay/src/game/battle/practice.rs` — practice battle handlers.
- `crates/emukc_gameplay/src/game/sortie_result.rs` — HP snapshot for battle results.
- `docs/solutions/architecture-patterns/battle-damage-foundation.md` — the migrated damage foundation contract this change MODIFIES (enemy-overkill clause).
