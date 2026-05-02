## MODIFIED Requirements

### Requirement: Sortie State Machine
A sortie SHALL be a stateful progression through a map, managed by an in-memory
SortieStore keyed to the profile. Implemented via SortieOps.

#### Scenario: Sortie start
- **WHEN** a sortie is started for a profile with a valid fleet on a valid map area/stage
- **THEN** any existing active sortie, pending result, and pending battle for the profile SHALL be removed from the SortieStore before creating new state
- **THEN** the fleet's fuel and ammo are reduced by the map's consumption rate
- **THEN** a new ActiveSortieState is created with map cell data
- **THEN** the starting cell is determined by the map definition
- **THEN** the response includes cell_data with `api_passed: 0` for ALL cells (none visited yet)
- **THEN** the response includes map area/stage identifiers and initial cell position

#### Scenario: Sortie start with unavailable fleet
- **WHEN** the selected fleet is already in a sortie or on an expedition
- **THEN** the operation fails

#### Scenario: Sortie start with invalid map
- **WHEN** the specified map area or stage does not exist in the Codex
- **THEN** the operation fails

#### Scenario: Sortie start with sunk ships in fleet
- **WHEN** any ship in the selected fleet has HP of 0 (sunk)
- **THEN** the operation fails

#### Scenario: Sortie start with locked map
- **WHEN** the specified map has `unlocked = false` for the player
- **THEN** the operation fails with an error response

#### Scenario: Practice session does not leak into sortie
- **WHEN** a practice battle completes and its results are processed
- **THEN** all practice-related entries (pending_battle, pending_result) SHALL be removed from the SortieStore
- **THEN** starting a sortie after practice SHALL NOT carry over practice enemy data

## ADDED Requirements

### Requirement: Map 1-3 routing follows directed graph
Map 1-3 cell routing SHALL follow the directed graph edges defined in the Codex map data. When advancing to a next cell, the system SHALL only move to cells that are valid edges from the current cell, respecting routing rules when present and falling back to next_cells only when rules are absent.

#### Scenario: 1-3 routing uses correct edges
- **WHEN** a fleet advances through map 1-3
- **THEN** each cell transition SHALL correspond to a valid edge in the map's directed graph
- **THEN** the fleet SHALL NOT skip cells or jump to non-adjacent cells

#### Scenario: 1-3 routing rules are present in codex
- **WHEN** the codex loads map 1-3 data
- **THEN** routing_rules SHALL be populated for cells that have branching paths
- **THEN** next_cells SHALL reflect the correct directed graph edges
