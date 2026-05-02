## ADDED Requirements

### Requirement: Shelling phase participation by ship type
The system SHALL determine shelling phase eligibility based on ship type, not equipped items. All non-submarine surface ship types (DD, DE, CL, CLT, CT, CA, CAV, FBB, BB, BBV, AV, LHA, AO) SHALL participate in day shelling phase regardless of equipment. CV/CVL/CVB SHALL participate only when carrying at least one attack plane (dive bomber or torpedo bomber with slot > 0). SS/SSV SHALL NOT participate in shelling phase.

#### Scenario: DD with no equipment participates in shelling
- **WHEN** a DD has no guns or torpedoes equipped and is not sunk
- **THEN** the DD SHALL participate in day shelling phase with base attack power = firepower + 5

#### Scenario: DD with torpedo does not show torpedo attack in shelling
- **WHEN** a DD has only a torpedo equipped (no guns) and participates in shelling phase
- **THEN** the DD SHALL perform a normal shelling attack (api_at_type = 0), NOT a torpedo attack

#### Scenario: Submarine excluded from shelling
- **WHEN** an SS or SSV is not sunk
- **THEN** the ship SHALL NOT participate in day shelling phase

#### Scenario: CV without planes excluded from shelling
- **WHEN** a CV has zero dive bombers and zero torpedo bombers across all slots
- **THEN** the CV SHALL NOT participate in day shelling phase

#### Scenario: CV with attack planes participates in shelling
- **WHEN** a CV has at least one dive bomber or torpedo bomber with slot count > 0
- **THEN** the CV SHALL participate in day shelling phase

### Requirement: Closing torpedo participation by base torpedo stat
The system SHALL determine closing torpedo phase eligibility based on base torpedo stat (素の雷装 / `api_raisou[0]`), NOT ship type. Any ship with `api_raisou[0] > 0` SHALL participate in closing torpedo, regardless of ship type. Ships with `api_raisou[0] = 0` SHALL NOT participate. Ships with 中破 (moderate damage) or 大破 (heavy damage) SHALL NOT participate in closing torpedo.

**Source**: wikiwiki.jp/kancolle/戦闘について — "逆に言えば素の雷装値が1以上ならば艦種問わず雷撃戦に参加する"

#### Scenario: DD with base torpedo > 0 participates in closing torpedo
- **WHEN** a DD has `api_raisou[0] > 0` and is not 中破/大破
- **THEN** the DD SHALL participate in closing torpedo phase

#### Scenario: BB with base torpedo > 0 participates in closing torpedo
- **WHEN** Bismarck drei, Гангут, Conte di Cavour, or other BB with `api_raisou[0] > 0` is not 中破/大破
- **THEN** the BB SHALL participate in closing torpedo phase

#### Scenario: Ship with base torpedo = 0 excluded from closing torpedo
- **WHEN** a ship (regardless of ship type) has `api_raisou[0] = 0`
- **THEN** the ship SHALL NOT participate in closing torpedo phase
- **AND** this correctly excludes DE, LHA, AR, and 0-torpedo AV/AO/CT

#### Scenario: 中破 ship excluded from closing torpedo
- **WHEN** a ship is 中破 (moderately damaged, HP ≤ 50% of max)
- **THEN** the ship SHALL NOT participate in closing torpedo phase
- **NOTE**: This does NOT apply to opening torpedo (開幕雷撃は損傷度は問わず)

### Requirement: Opening torpedo participation by equipment and type
The system SHALL determine opening torpedo phase eligibility based on equipment (minisub/甲标的) and ship type. Ships with 特殊潜航艇 (minisub) equipped and `api_raisou[0] > 0` SHALL participate. CLT type ships SHALL participate. SS/SSV with level ≥ 10 SHALL participate without equipment. Damage state SHALL NOT prevent opening torpedo (開幕雷撃は損傷度は問わず発動する).

**Source**: wikiwiki.jp/kancolle/戦闘について — "基本的に特殊潜航艇装備が必要" / "Lv10以上の潜水艦は特殊潜航艇装備なしでも開幕雷撃可能"

#### Scenario: CLT participates in opening torpedo
- **WHEN** a CLT is not sunk, has `api_raisou[0] > 0`, and battle type includes torpedo phases
- **THEN** the CLT SHALL participate in opening torpedo phase regardless of equipment

#### Scenario: SS level ≥ 10 participates in opening torpedo
- **WHEN** an SS or SSV has level ≥ 10 and `api_raisou[0] > 0`
- **THEN** the ship SHALL participate in opening torpedo phase without equipment

#### Scenario: SS below level 10 does not opening torpedo without equipment
- **WHEN** an SS or SSV has level < 10 and no 甲标的 (minisub) equipped
- **THEN** the ship SHALL NOT participate in opening torpedo phase

#### Scenario: Ship with 甲标的 participates in opening torpedo
- **WHEN** any ship (e.g., ABKM改二, special CAV) has 特殊潜航艇 (甲标的/minisub) equipped and `api_raisou[0] > 0`
- **THEN** the ship SHALL participate in opening torpedo phase

### Requirement: Attack display type fallback
When a ship has no equipment matching display type categories, the system SHALL assign api_at_type = 0 (normal single attack) and use base ship stats for damage calculation. The system SHALL NOT skip the ship's attack turn due to missing equipment.

#### Scenario: Ship with no relevant equipment still attacks
- **WHEN** a surface ship has no guns, torpedoes, or relevant attack equipment
- **THEN** the ship SHALL still perform a shelling attack with api_at_type = 0 and damage based on base firepower + 5
