## MODIFIED Requirements

### Requirement: Day Battle Simulation
When the player encounters a battle node, a day battle SHALL be simulated using
the battle subsystem. Implemented via SortieOps::sortie_battle.

#### Scenario: Normal day battle
- WHEN sortie_battle is called with a formation choice
- THEN enemy fleet composition is determined from the Codex map cell definition or fallback enemy builder
- THEN the battle simulation produces: aerial phase (optional), shelling phases, torpedo phase
- THEN battle damage is recorded in the SortieStore as a pending result
- THEN the response includes api_hourai_flag indicating which phases occurred
- THEN all damage fields in the response (api_damage, api_fydam/eydam, api_fdam/edam) SHALL contain raw (pre-clamp) damage values, allowing overkill display
- THEN HP tracking SHALL use effective (clamped) damage internally

#### Scenario: Air battle (airbattle)
- WHEN the cell event type calls for an airbattle
- THEN only the aerial phase is simulated
- THEN no shelling or torpedo phases occur

#### Scenario: Special battle types (ld_airbattle, ld_shooting)
- WHEN the cell event type calls for ld_airbattle or ld_shooting
- THEN the appropriate battle mode is used with its specific phase configuration
- THEN midnight battle is disabled for these types

#### Scenario: Enemy selection
- WHEN building the enemy fleet for a map cell
- THEN weighted node compositions are used to select from available enemy fleets in the Codex
- THEN fallback enemy fleets are used only when Codex enemy data is missing (degraded path)
