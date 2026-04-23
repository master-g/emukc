## ADDED Requirements

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
