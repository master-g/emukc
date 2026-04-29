## ADDED Requirements

### Requirement: Comparison example consumes the full decoder rule bundle
The comparison example SHALL treat an explicit `cache_rules.json` input as the root of a decoder bundle so candidate generation can use sibling decoder coverage assets from the same decoder run.

#### Scenario: Rules path points into decoder output resources
- **WHEN** the comparison example is run with a `--rules` path under a decoder output `resources/` directory
- **THEN** the candidate generation path MUST derive sibling decoder coverage assets from that same decoder output tree
- **THEN** the comparison run MUST evaluate the candidate using the full available decoder bundle instead of `cache_rules.json` alone

#### Scenario: Optional sibling asset is missing next to rules path
- **WHEN** one or more optional sibling decoder coverage assets are absent next to the provided `--rules` path
- **THEN** the comparison example MUST report which bundle assets were unavailable
- **THEN** the comparison run MUST still proceed with the decoder bundle data that was successfully loaded

### Requirement: Comparison report includes authority breakdown for decoder-first candidates
The comparison example SHALL report how much of the candidate cache list was produced directly from decoder rules versus legacy fallback when the candidate is built from the decoder-first pipeline.

#### Scenario: Decoder-first comparison run completes successfully
- **WHEN** the candidate cache list is built from an explicit decoder rule bundle
- **THEN** the report MUST include counts for rule-authored candidate paths and fallback-authored candidate paths
- **THEN** the report MUST include grouped fallback residual prefixes or family labels

#### Scenario: Candidate has no fallback-authored residuals
- **WHEN** the candidate cache list is produced entirely from decoder-authoritative coverage for the measured domains
- **THEN** the report MUST show zero fallback-authored candidate paths
- **THEN** the fallback residual section MUST be empty

### Requirement: Comparison report surfaces migration blockers for default-switch planning
The comparison example SHALL emit a migration summary that makes remaining decoder-first blockers explicit for future bootstrap default-switch planning.

#### Scenario: Candidate still has migration blockers
- **WHEN** the comparison finds baseline-only paths, unresolved rule keys, or fallback-dependent decoder families
- **THEN** the report MUST list those blocker categories explicitly
- **THEN** the report MUST NOT present the candidate as migration-ready

#### Scenario: Candidate has no migration blockers
- **WHEN** the candidate has full baseline recall and no unresolved or fallback-dependent decoder-covered families
- **THEN** the report MUST surface that the decoder-first pipeline has no remaining migration blockers for the measured domains
- **THEN** the report MUST keep the supporting overlap and delta metrics available for inspection
