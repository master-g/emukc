## ADDED Requirements

### Requirement: Comparison report surfaces sound-rule migration progress explicitly
The comparison example SHALL report sound-domain decoder-first migration progress explicitly when the candidate is built from the decoder rule bundle.

#### Scenario: Decoder-first comparison run includes sound residuals
- **WHEN** the comparison run produces fallback-authored sound paths
- **THEN** the report MUST surface the relevant `kcs/sound/*` residual families in a grouped section or migration-blocker summary
- **THEN** the sound fallback share MUST remain visible without requiring manual inspection of the full path list

#### Scenario: Decoder-first comparison run materially reduces sound fallback
- **WHEN** the candidate reduces sound-domain fallback usage compared with the prior decoder-first baseline
- **THEN** the report MUST preserve enough sound-domain detail for the user to verify that the reduction came from the sound migration work rather than unrelated changes

### Requirement: Comparison report distinguishes explicit audio coverage from algorithmic sound-rule coverage
The comparison example SHALL make it possible to distinguish explicit audio asset coverage from algorithmic `kcs/sound/*` rule coverage in the decoder-first candidate analysis.

#### Scenario: Candidate contains both explicit audio assets and sound-rule output
- **WHEN** the candidate includes `se` / `bgm` / titlecall style assets plus `kcs/sound/*` sound-rule families
- **THEN** the report MUST preserve grouped output that lets the user tell which residuals belong to explicit audio assets and which belong to algorithmic sound families
- **THEN** migration analysis MUST remain actionable at the sound-domain level
