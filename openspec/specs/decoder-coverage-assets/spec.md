# decoder-coverage-assets Specification

## Purpose
TBD - created by archiving change expand-decoder-cachelist-coverage. Update Purpose after archive.

## Requirements
### Requirement: Sparse ship and slot subsets are emitted with explicit completeness metadata
The decoder SHALL emit a structured coverage asset bundle for sparse ship and slot categories whose membership is directly observable in decoded `main.js`. Each extracted subset MUST include the category key, resource domain, observed IDs, provenance, and an explicit coverage mode that distinguishes complete observation from partial or unresolved observation.

#### Scenario: Observable sparse ship subset is extracted
- **WHEN** decoded modules contain an explicit enumerable subset for ship resources such as `special`, `sp_remodel/*`, `card_round`, or `reward_*`
- **THEN** the emitted coverage asset MUST include that subset under a stable category key
- **THEN** the asset MUST record the observed ship IDs and provenance identifying which decoder modules produced the subset

#### Scenario: Runtime-driven subset remains unresolved
- **WHEN** a ship or slot category is only referenced through runtime-driven identifiers and the decoder cannot enumerate a stable subset directly from decoded `main.js`
- **THEN** the emitted coverage asset MUST mark that category as `partial` or `unresolved`
- **THEN** the decoder MUST NOT synthesize completeness by copying Rust constants or CDN-derived IDs into the asset

### Requirement: Audio coverage asset captures cache-list audio domains
The decoder SHALL emit an audio coverage asset that records directly observable cache-list audio domains needed by bootstrap generation, including sound-effect IDs, categorized BGM IDs, and voice-related ranges or explicit file stems.

#### Scenario: Sound effect and BGM data are visible in decoded modules
- **WHEN** decoded modules expose numeric sound-effect IDs or categorized BGM IDs through explicit paths, call arguments, or inline tables
- **THEN** the audio coverage asset MUST store those IDs under stable domain keys usable by bootstrap cache-list generation

#### Scenario: Voice domain is represented by ranges or file stems
- **WHEN** decoded modules expose titlecall ranges, tutorial voice stems, or other voice file groupings without listing every file individually
- **THEN** the audio coverage asset MUST preserve the observable range or stem form instead of flattening it into invented IDs

### Requirement: UI coverage asset captures explicit cache-list file groups
The decoder SHALL emit a UI coverage asset that records explicit file groups for non-ship/slot UI domains required by cache-list generation, including map, furniture, useitem, area, and world-select resources.

#### Scenario: Map and furniture files are observable
- **WHEN** decoded modules contain explicit map or furniture resource paths or file groups
- **THEN** the UI coverage asset MUST store those paths or file groups in a structure that preserves the original domain grouping

#### Scenario: Useitem card and underline resources are distinct
- **WHEN** decoded modules expose `useitem/card` and `useitem/card_` style resources
- **THEN** the UI coverage asset MUST represent those groups separately so bootstrap generation can preserve the current cache-list layout

### Requirement: Coverage assets are emitted as decoder outputs and syncable bootstrap assets
The decoder pipeline SHALL write coverage assets into the normal decoder output tree and SHALL support syncing those same assets into `crates/emukc_bootstrap/assets/` for repo-tracked workflows.

#### Scenario: Decoder run without bootstrap sync
- **WHEN** the decoder runs with coverage extraction enabled but without an asset sync flag
- **THEN** the coverage assets MUST be written under the decoder output directory
- **THEN** the pipeline MUST NOT modify repo-tracked bootstrap assets

#### Scenario: Decoder run with bootstrap sync
- **WHEN** the decoder runs with the asset sync workflow enabled
- **THEN** the same coverage assets MUST be written to the decoder output directory and synced into `crates/emukc_bootstrap/assets/`
