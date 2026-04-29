## MODIFIED Requirements

### Requirement: UI coverage asset captures explicit cache-list file groups
The decoder SHALL emit a UI coverage asset that records explicit file groups for non-ship/slot UI domains required by cache-list generation, including map, furniture, useitem, area, and world-select resources. For decoder-observable migration-critical UI families, the asset MUST emit concrete members instead of leaving those groups empty, and MUST preserve partial or unresolved coverage modes when the decoded script does not prove full membership.

#### Scenario: Map and furniture files are observable
- **WHEN** decoded modules contain explicit map or furniture resource paths, file groups, deterministic literal references, or decoder-observable construction patterns
- **THEN** the UI coverage asset MUST store those paths or file groups in a structure that preserves the original domain grouping
- **THEN** decoder-observable map and furniture members MUST be available to downstream cache-list generation as concrete asset members

#### Scenario: Useitem card and underline resources are distinct
- **WHEN** decoded modules expose `useitem/card` and `useitem/card_` style resources through direct paths, literal ID sets, or deterministic construction patterns
- **THEN** the UI coverage asset MUST represent those groups separately so bootstrap generation can preserve the current cache-list layout
- **THEN** the emitted asset MUST NOT leave both groups empty when decoder-observable IDs exist

#### Scenario: Area resources are observable
- **WHEN** decoded modules expose `area/sally`, `area/airunit`, or `area/airunit_extend_confirm` resources through direct paths, literal ID sets, or deterministic construction patterns
- **THEN** the UI coverage asset MUST preserve those concrete members under stable area domain keys
- **THEN** the emitted asset MUST keep unresolved area groups partial or unresolved instead of claiming complete coverage without decoder evidence

#### Scenario: World-select resources are observable
- **WHEN** decoded modules expose `worldselect` resources through direct paths, literal filenames, or deterministic construction patterns
- **THEN** the UI coverage asset MUST preserve those concrete files under a stable world-select domain key
- **THEN** the emitted asset MUST make decoder-observable world-select files available for Rules-path cache-list generation
