## ADDED Requirements

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
