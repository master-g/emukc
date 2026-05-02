## ADDED Requirements

### Requirement: Map routing data validation
The Codex map data for each map SHALL include correct `next_cells` arrays that reflect the directed graph edges of the real KanColle map. When `routing_rules` are absent for a cell, `next_cells` SHALL serve as the definitive edge list.

#### Scenario: Map 1-3 next_cells correctness
- **WHEN** the Codex loads map 1-3 stage definitions
- **THEN** each cell's `next_cells` SHALL list only cells that are directly reachable per the real game's directed graph
- **THEN** no cell SHALL list a next_cell that is not an adjacent edge in the original game

#### Scenario: Fallback routing uses correct edges
- **WHEN** a cell has no routing_rules and multiple next_cells
- **THEN** the system SHALL select from valid adjacent cells only
- **THEN** the system SHALL NOT deterministically pick the first cell when multiple valid options exist
