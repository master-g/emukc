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
- **THEN** it MUST generate the baseline list using the `Manifest` bootstrap strategy
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

### Requirement: Comparison report surfaces template-backed ownership
The comparison example SHALL report template-backed decoder ownership separately from generic rule-authored and fallback-authored totals when the candidate is built from a decoder rule bundle.

#### Scenario: Template-backed families are expanded
- **WHEN** a decoder-first comparison run expands one or more template-backed families as rule-authored output
- **THEN** the report MUST include grouped template-backed rule-authored counts by family or resource domain
- **THEN** the report MUST preserve the existing global rule-authored and fallback-authored totals

#### Scenario: Template-backed families remain fallback-dependent
- **WHEN** a decoder-first comparison run leaves one or more template-backed families partial, unresolved, or missing required runtime inputs
- **THEN** the report MUST include grouped fallback residuals for those template-backed families
- **THEN** the migration blocker summary MUST identify those residuals distinctly from non-template fallback prefixes

#### Scenario: Template residual reason is available
- **WHEN** the decoder-first pipeline reports a reason for a template-backed fallback residual
- **THEN** the comparison report MUST preserve that reason in the machine-readable report
- **THEN** the human-readable summary MUST identify the affected family and reason at the grouped blocker level

### Requirement: Migration readiness accounts for template-backed residuals
The comparison example SHALL treat unresolved template-backed families and fallback-authored residuals from template-backed domains as migration blockers until the report can prove decoder-authoritative ownership for the measured domains.

#### Scenario: Template residuals remain
- **WHEN** the candidate has full baseline recall but still contains fallback-authored template-backed residuals
- **THEN** the report MUST NOT mark the decoder-first candidate as migration-ready
- **THEN** the report MUST list the residual template family labels or required input gaps that prevent readiness

#### Scenario: Template residuals are resolved
- **WHEN** all measured template-backed families are generated from decoder-authoritative descriptors and validated runtime inputs with no fallback-authored residuals
- **THEN** the report MUST allow those families to be absent from the migration blocker list
- **THEN** supporting overlap, candidate-only, and authority totals MUST remain available for inspection

#### Scenario: Candidate preserves recall while residuals shrink
- **WHEN** a comparison run reduces template-backed fallback-authored residuals
- **THEN** the report MUST still include `baseline_only_count` and `candidate_only_count`
- **THEN** migration readiness MUST remain false if any measured template-backed residual blocker remains

### Requirement: Default-vs-Rules comparison warns as no-op
When the user passes `--baseline default --rules`, the comparison example SHALL emit a warning that both strategies produce identical output.

#### Scenario: User compares Default against Rules
- **WHEN** `--baseline default --rules` is passed
- **THEN** the example SHALL print a warning that `Default` now delegates to `Rules`
- **AND** continue with the comparison (which will show 100% overlap)

