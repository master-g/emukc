## ADDED Requirements

### Requirement: All sortie cells start unpassed
When a sortie begins, `build_sortie_cell_data()` SHALL initialize all cells with `passed: false`, regardless of `cell_no`. The start cell is determined by the map definition, not by cell_no comparison.

#### Scenario: Non-start cells initialized as unpassed
- **WHEN** `build_sortie_cell_data()` processes map cells
- **THEN** every cell SHALL have `passed: false` in the returned `SortieCellData`

#### Scenario: Start cell determined by map definition
- **WHEN** the sortie starts on a map
- **THEN** the start cell is identified by the map's start cell designation, not by `cell_no == 0`
