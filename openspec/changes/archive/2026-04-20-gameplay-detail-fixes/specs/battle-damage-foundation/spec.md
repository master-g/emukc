## MODIFIED Requirements

### Requirement: Defense power uses randomized formula

The system SHALL calculate defense power using the formula `floor(0.7 × A_t + 0.6 × random(0, floor(A_t) − 1))` where `A_t` is the defender's armor stat, for all attack types (shelling, torpedo, airstrike, ASW, night battle).

The random component SHALL be a uniform integer in `[0, floor(A_t) − 1]`.

The defense calculation SHALL be extracted into a shared `calculate_defense_power()` function used by all damage calculators.

#### Scenario: Shelling attack against armored target
- **WHEN** a ship with armor stat 80 is attacked by shelling
- **THEN** defense power is `floor(0.7 × 80 + 0.6 × rand(0, 79))` yielding a value in range `[56, 103]`

#### Scenario: Torpedo attack against armored target
- **WHEN** a ship with armor stat 50 is attacked by torpedo
- **THEN** defense power uses the same randomized formula (NOT `A × 0.55`)

#### Scenario: Airstrike against armored target
- **WHEN** a ship with armor stat 60 is attacked by airstrike
- **THEN** defense power uses the same randomized formula (NOT `A × 0.6`)

#### Scenario: Armor stat of 1
- **WHEN** a ship has armor stat 1
- **THEN** defense power is `floor(0.7 + 0)` = 0 (no random range when `floor(A) − 1 = 0`)

## ADDED Requirements

### Requirement: Airstrike Stage 3 split into dive and torpedo bombing phases

During aerial combat Stage 3, the bombing phase SHALL be split into two sequential sub-phases:
1. **Dive bombing phase**: each slot equipped with dive bomber type aircraft (CarrierBasedDiveBomber, SeaBasedBomber, JetFighterBomber, JetAttacker — uses `api_baku` stat) independently selects a random alive target from the opposing fleet and deals damage
2. **Torpedo bombing phase**: each slot equipped with torpedo bomber type aircraft (CarrierBasedTorpedoBomber — uses `api_raig` stat) independently selects a random alive target from the opposing fleet and deals damage

Each slot SHALL perform its own independent target selection. Multiple slots MAY select the same target (damage accumulates) or different targets.

Per-slot damage SHALL use the formula: `equipment_stat × √(remaining_planes) + 25`, capped at 170, then reduced by target defense power.

#### Scenario: CV with 2 dive bomber slots and 1 torpedo bomber slot against 6 enemies
- **WHEN** a CV has 2 dive bomber slots and 1 torpedo bomber slot, and the enemy fleet has 6 alive ships
- **THEN** the dive bombing phase performs 2 independent target selections (one per dive bomber slot)
- **THEN** the torpedo bombing phase performs 1 independent target selection (one per torpedo bomber slot)
- **THEN** total of 3 attacks are resolved, potentially hitting 1–3 different ships

#### Scenario: Multiple slots select same target
- **WHEN** two or more bomber slots happen to select the same target
- **THEN** all slot attacks apply damage to that target, accumulating total damage

#### Scenario: Enemy airstrike against friendly fleet
- **WHEN** the enemy fleet has bomber-capable ships
- **THEN** the same dive/torpedo split and per-slot targeting applies to the enemy's bombing phase
- **THEN** `api_fdam` reflects accumulated damage from all enemy bomber slots

#### Scenario: No bomber slots on either side
- **WHEN** no friendly or enemy ships have bomber aircraft equipped
- **THEN** Stage 3 produces zero damage as before

#### Scenario: Only torpedo bombers equipped
- **WHEN** a ship has only torpedo bomber slots (no dive bombers)
- **THEN** only the torpedo bombing phase executes, dive bombing phase produces no damage

#### Scenario: Only dive bombers equipped
- **WHEN** a ship has only dive bomber slots (no torpedo bombers)
- **THEN** only the dive bombing phase executes, torpedo bombing phase produces no damage
