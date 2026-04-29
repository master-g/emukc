## MODIFIED Requirements

### Requirement: Audio coverage asset captures cache-list audio domains
The decoder SHALL emit an audio coverage asset that records directly observable cache-list audio domains needed by bootstrap generation, including sound-effect IDs, categorized BGM IDs, and voice-related ranges or explicit file stems. The emitted asset MUST materially cover the currently dominant fallback families before those families are considered decoder-authored by the Rules path.

#### Scenario: Sound effect and BGM data are visible in decoded modules
- **WHEN** decoded modules expose numeric sound-effect IDs or categorized BGM IDs through explicit paths, call arguments, inline tables, or other decoder-observable runtime structures
- **THEN** the audio coverage asset MUST store those IDs under stable domain keys usable by bootstrap cache-list generation
- **THEN** the asset MUST NOT leave categorized `port`, `battle`, or `fanfare` BGM coverage empty when decoder-observable evidence for those families exists in the decoded script

#### Scenario: Voice domain is represented by ranges or file stems
- **WHEN** decoded modules expose titlecall ranges, tutorial voice stems, or other voice file groupings without listing every file individually
- **THEN** the audio coverage asset MUST preserve the observable range or stem form instead of flattening it into invented IDs
- **THEN** the emitted asset MUST make the reachable titlecall and explicit voice-file families available strongly enough to reduce the current fallback-owned titlecall residuals

### Requirement: UI coverage asset captures explicit cache-list file groups
The decoder SHALL emit a UI coverage asset that records explicit file groups for non-ship/slot UI domains required by cache-list generation, including map, furniture, useitem, area, and world-select resources. The asset MUST capture concrete members for the highest-value residual families that are still generated primarily from Rust fallback logic.

#### Scenario: Map and furniture files are observable
- **WHEN** decoded modules contain explicit map or furniture resource paths, file groups, or deterministic file references
- **THEN** the UI coverage asset MUST store those paths or file groups in a structure that preserves the original domain grouping
- **THEN** the emitted asset MUST provide concrete coverage for the decoder-observable map and furniture families that currently dominate the fallback residual report

#### Scenario: Useitem card and underline resources are distinct
- **WHEN** decoded modules expose `useitem/card` and `useitem/card_` style resources
- **THEN** the UI coverage asset MUST represent those groups separately so bootstrap generation can preserve the current cache-list layout
- **THEN** the emitted asset MUST not leave both groups empty when decoder-observable IDs exist

#### Scenario: Area and world-select resources are observable
- **WHEN** decoded modules expose `area/sally`, `area/airunit`, `area/airunit_extend_confirm`, or `worldselect` resources
- **THEN** the UI coverage asset MUST preserve those concrete members under stable domain keys
- **THEN** the emitted asset MUST be sufficient to shrink current fallback-owned area or world-select residuals for the covered families

### Requirement: Sparse ship and slot subsets are emitted with explicit completeness metadata
The decoder SHALL emit a structured coverage asset bundle for sparse ship and slot categories whose membership is directly observable in decoded `main.js`. Each extracted subset MUST include the category key, resource domain, observed IDs, provenance, and an explicit coverage mode that distinguishes complete observation from partial or unresolved observation. Categories with decoder-observable literal evidence MUST no longer remain unresolved merely because the current extractor fails to connect that evidence to the emitted subset.

#### Scenario: Observable sparse ship subset is extracted
- **WHEN** decoded modules contain an explicit enumerable subset for ship resources such as `special`, `sp_remodel/*`, `card_round`, or `reward_*`
- **THEN** the emitted coverage asset MUST include that subset under a stable category key
- **THEN** the asset MUST record the observed ship IDs and provenance identifying which decoder modules produced the subset

#### Scenario: Runtime-driven subset remains unresolved
- **WHEN** a ship or slot category is only referenced through runtime-driven identifiers and the decoder cannot enumerate a stable subset directly from decoded `main.js`
- **THEN** the emitted coverage asset MUST mark that category as `partial` or `unresolved`
- **THEN** the decoder MUST NOT synthesize completeness by copying Rust constants or CDN-derived IDs into the asset
