# decoder-cachelist-comparison Specification

## Purpose
TBD - created by archiving change decoder-cachelist-comparison-example. Update Purpose after archive.
## Requirements
### Requirement: Comparison example accepts explicit decoder manifest input
The repository SHALL provide a runnable example that accepts a decoder-produced resource manifest path as input and uses that manifest to build a candidate cache list without overwriting the repo-tracked bootstrap assets.

#### Scenario: Candidate manifest path provided
- **WHEN** a user runs the comparison example with a valid decoder manifest path
- **THEN** the example MUST load that manifest as the candidate source
- **THEN** the example MUST NOT modify `crates/emukc_bootstrap/assets/resource_manifest.json`

#### Scenario: Candidate manifest path missing or invalid
- **WHEN** the example is run with a missing file or an invalid manifest payload
- **THEN** the example MUST fail with a clear validation error before generating comparison output

### Requirement: Comparison example builds both candidate and baseline lists
The comparison example SHALL generate a candidate cache list from the explicit decoder manifest input and SHALL also generate a baseline cache list from the current bootstrap strategy in the same run.

#### Scenario: Default baseline run
- **WHEN** the example is run without an explicit baseline strategy override
- **THEN** it MUST generate the baseline list using the current `Default` bootstrap strategy
- **THEN** it MUST generate the candidate list using the decoder manifest override with the existing Rust cache-list generation infrastructure

#### Scenario: Alternate baseline strategy selected
- **WHEN** the example is run with a supported baseline strategy override
- **THEN** it MUST build the baseline list with that strategy
- **THEN** the candidate generation path MUST still use the explicit decoder manifest input

### Requirement: Comparison report is path-set based
The comparison example SHALL compare cache lists using unique resource `path` values rather than `_id` or line order and SHALL emit a report that includes overlap and delta metrics.

#### Scenario: Report contains summary metrics
- **WHEN** a comparison run completes successfully
- **THEN** the report MUST include candidate count, baseline count, intersection count, only-baseline count, only-candidate count, and at least one percentage-based coverage metric

#### Scenario: Report groups deltas by path prefix
- **WHEN** the comparison report includes paths missing from either side
- **THEN** the report MUST group those deltas by resource prefix or category so the user can quickly identify the highest-impact domains

### Requirement: Decoder manifest is available as a normal decoder output artifact
The decoder pipeline SHALL make the resource manifest available as an output artifact suitable for direct external consumption by the comparison example.

#### Scenario: Decoder writes output artifacts
- **WHEN** `main-decoder` runs with resource-manifest extraction enabled
- **THEN** it MUST write the resource manifest to the decoder output area in addition to any optional bootstrap asset sync

#### Scenario: Comparison example consumes decoder output directly
- **WHEN** a user points the example at the decoder output manifest artifact
- **THEN** the example MUST be able to run the comparison workflow without requiring a prior sync into bootstrap assets

### Requirement: Comparison example consumes the full decoder output asset bundle
The comparison example SHALL accept decoder output artifacts as a bundle so candidate generation can use `resource_manifest.json` together with sibling decoder coverage assets produced in the same decoder run.

#### Scenario: Decoder manifest path points into decoder output resources
- **WHEN** the comparison example is run with a manifest path under a decoder output resources directory
- **THEN** the example MUST derive sibling decoder coverage assets from that same decoder output tree
- **THEN** the example MUST build the candidate cache list from the full available decoder asset bundle without requiring a bootstrap-asset sync first

#### Scenario: Optional sibling asset is missing
- **WHEN** one or more optional decoder coverage assets are absent next to the manifest path
- **THEN** the example MUST report which optional assets were unavailable
- **THEN** the comparison run MUST still proceed with the decoder assets that were successfully loaded

### Requirement: Comparison report includes domain-level coverage breakdown
The comparison example SHALL report coverage deltas by resource domain in addition to the existing global path-set overlap metrics.

#### Scenario: Comparison run completes successfully
- **WHEN** a comparison run finishes
- **THEN** the report MUST include domain-level baseline count, candidate count, and overlap metrics for the major cache-list domains such as ship, slot, sound, map, furniture, BGM, useitem, and voice

#### Scenario: Sparse categories remain over- or under-covered
- **WHEN** the candidate cache list significantly over-generates or under-generates a sparse ship or slot category
- **THEN** the report MUST surface that category in a grouped delta section so decoder extraction work can target the highest-impact gaps
