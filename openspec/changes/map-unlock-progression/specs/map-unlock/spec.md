## ADDED Requirements

### Requirement: Map prerequisites defined in codex
The system SHALL define per-map prerequisite data in the Codex (`MapCatalog`), mapping each regular map ID to the map ID that must be cleared before it becomes available. Map 1-1 SHALL have no prerequisite (always available).

#### Scenario: Same-area sequential unlock
- **WHEN** a map N-M exists in area N
- **THEN** the prerequisite for N-M is N-(M-1) (e.g., 1-2 requires 1-1, 1-3 requires 1-2)

#### Scenario: Cross-area unlock
- **WHEN** area N's final map (N-4) is cleared
- **THEN** area (N+1)'s first map ((N+1)-1) becomes available (e.g., 1-4 → 2-1, 2-4 → 3-1)

#### Scenario: First map always available
- **WHEN** map 1-1 is queried
- **THEN** it has no prerequisite and is always unlocked

### Requirement: Per-player unlock state tracked in database
The system SHALL track map unlock state per player via an `unlocked` boolean on the `map_record` entity. Only unlocked maps SHALL appear in `api_get_member/mapinfo` responses.

#### Scenario: New account initialization
- **WHEN** a new profile is created
- **THEN** only map 1-1 has `unlocked = true`; all other maps have `unlocked = false`

#### Scenario: Unlocked maps returned in mapinfo
- **WHEN** `api_get_member/mapinfo` is called
- **THEN** only maps with `unlocked = true` are included in the response

#### Scenario: Locked maps hidden from mapinfo
- **WHEN** a map has `unlocked = false`
- **THEN** it SHALL NOT appear in the `api_get_member/mapinfo` response

### Requirement: Sortie gated by unlock status
The system SHALL reject sortie requests to locked maps via `api_req_map/start`.

#### Scenario: Sortie to unlocked map succeeds
- **WHEN** a player starts a sortie to a map with `unlocked = true`
- **THEN** the sortie begins normally

#### Scenario: Sortie to locked map rejected
- **WHEN** a player attempts to start a sortie to a map with `unlocked = false`
- **THEN** the system returns an error response (`api_result = -1`)

### Requirement: Unlock cascade on map clear
The system SHALL automatically unlock dependent maps when a prerequisite map is cleared, and return the newly unlocked map IDs via `api_next_map_ids` in the battle result response.

#### Scenario: Map clear unlocks next map in same area
- **WHEN** a player clears map 1-1 (boss defeated, first clear)
- **THEN** map 1-2 is unlocked and `api_next_map_ids` contains `[12]`

#### Scenario: Map clear unlocks first map of next area
- **WHEN** a player clears map 1-4 (area boss)
- **THEN** map 2-1 is unlocked and `api_next_map_ids` contains `[21]`

#### Scenario: Already-unlocked map not re-reported
- **WHEN** a player clears a map whose dependents are already unlocked
- **THEN** `api_next_map_ids` is absent or empty from the response

#### Scenario: Multiple maps unlocked by single clear
- **WHEN** clearing a map unlocks more than one dependent map (rare edge case)
- **THEN** all newly unlocked map IDs appear in `api_next_map_ids`

### Requirement: Existing account migration preserves access
The system SHALL migrate existing accounts by setting `unlocked = true` for all maps that are already cleared or whose prerequisites are satisfied.

#### Scenario: Existing account with cleared maps
- **WHEN** an existing account is migrated and has maps 1-1 through 3-4 cleared
- **THEN** all maps in areas 1-3 are set to `unlocked = true`

#### Scenario: Existing account with partial progress
- **WHEN** an existing account has cleared 1-1 but not 1-2
- **THEN** map 1-1 and 1-2 are unlocked (1-2's prerequisite satisfied), but 1-3 and later are locked
