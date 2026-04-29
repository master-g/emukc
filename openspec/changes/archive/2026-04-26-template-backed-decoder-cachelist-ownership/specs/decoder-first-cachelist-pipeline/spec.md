## ADDED Requirements

### Requirement: Decoder-first authority accounting includes template-expanded output
The decoder-first cache-list pipeline SHALL classify paths expanded from complete decoder template-backed family descriptors as rule-authored output when all descriptor evidence and runtime input bindings are satisfied.

#### Scenario: Template-expanded output is decoder-authoritative
- **WHEN** decoder-first generation expands a complete template-backed family using decoder-provided template metadata and validated runtime inputs
- **THEN** the generated paths MUST be counted as rule-authored output in authority diagnostics
- **THEN** the same family MUST NOT be reported as fallback-dependent solely because legacy Rust generators contain an equivalent path formula

#### Scenario: Template-backed family remains unresolved
- **WHEN** decoder-first generation encounters a template-backed family whose descriptor, provenance, completeness mode, or input binding is partial or unresolved
- **THEN** the pipeline MUST keep that family in fallback territory for any paths not proven by decoder metadata
- **THEN** diagnostics MUST include an attributable residual key or family label for the fallback-authored output

### Requirement: Template-backed diagnostics remain sideband data
The decoder-first cache-list pipeline SHALL expose template-backed ownership diagnostics without changing the serialized cache-list item format.

#### Scenario: Generation emits template diagnostics
- **WHEN** decoder-first generation completes with template-expanded rule-authored output or template-backed fallback residuals
- **THEN** sideband diagnostics MUST include template family labels, completeness state, and residual fallback counts where available
- **THEN** serialized cache-list items MUST remain limited to the existing `_id`, `path`, and optional `version` fields
