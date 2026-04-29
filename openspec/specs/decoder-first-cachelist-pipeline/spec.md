# decoder-first-cachelist-pipeline Specification

## Purpose
Define the decoder-first cache-list pipeline contract, including bundle loading, rule-vs-fallback authority accounting, and migration-readiness diagnostics.
## Requirements
### Requirement: Decoder-first generation accepts an explicit decoder rule bundle
The system SHALL support decoder-first cache-list generation from an explicit decoder bundle rooted at `cache_rules.json`, together with sibling decoder coverage assets from the same decoder output tree and the runtime manifest/version inputs already available to bootstrap generation.

#### Scenario: Explicit decoder rule bundle is provided
- **WHEN** a caller provides a `cache_rules.json` path from a decoder output `resources/` directory
- **THEN** the system MUST load that rules asset as the primary decoder bundle input
- **THEN** the system MUST resolve sibling decoder coverage assets from the same decoder output tree before consulting repo-tracked bootstrap assets

#### Scenario: Repo-tracked decoder bundle is used implicitly
- **WHEN** decoder-first generation is invoked without an explicit decoder output path
- **THEN** the system MUST load the repo-tracked decoder rule bundle from `crates/emukc_bootstrap/assets/`
- **THEN** the same decoder-first generation flow MUST remain available using the repo-tracked bundle inputs that are present

### Requirement: Decoder-first generation separates rule-authored output from fallback-authored output
The decoder-first cache-list pipeline SHALL classify generated output by authority stage so paths produced directly from decoder rules remain distinct from paths produced only by legacy fallback behavior.

#### Scenario: Decoder-covered family emits rule-authored output
- **WHEN** a ship, slot, audio, or UI family is covered by decoder bundle semantics
- **THEN** the generated paths for that covered family MUST be recorded as rule-authored output
- **THEN** broad legacy fallback MUST NOT re-expand that family outside the decoder rule's allowed scope

#### Scenario: Unresolved family emits fallback-authored output
- **WHEN** a family remains partial or unresolved in the decoder bundle
- **THEN** the system MAY use legacy fallback behavior to preserve generation continuity for that family
- **THEN** the resulting paths MUST be recorded as fallback-authored output with an attributable residual key or family label

### Requirement: Decoder-first generation exposes migration residuals without changing cache-list payload format
The decoder-first cache-list pipeline SHALL expose unresolved rule keys, fallback-dependent families, and authority totals as sideband diagnostics while preserving the existing serialized cache-list item format.

#### Scenario: Generation completes with residual fallback usage
- **WHEN** decoder-first generation finishes and one or more families still depend on fallback behavior
- **THEN** the diagnostics MUST include unresolved rule keys or grouped residual families for those paths
- **THEN** the serialized cache-list output MUST remain limited to the existing `_id`, `path`, and optional `version` fields

#### Scenario: Generation completes without decoder-authority blockers
- **WHEN** decoder-first generation finishes with no unresolved rule keys and no fallback-authored output for decoder-covered families
- **THEN** the diagnostics MUST indicate that decoder authority has no remaining blockers for the measured domains
- **THEN** the cache-list payload format MUST still remain unchanged

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

#### Scenario: Template-backed fallback has an explicit reason
- **WHEN** decoder-first generation leaves fallback-authored output for a template-backed family
- **THEN** authority diagnostics MUST identify whether the residual came from missing descriptor evidence, partial coverage mode, unavailable runtime input, or uncovered member residuals
- **THEN** migration-readiness checks MUST be able to use that reason without inspecting individual path strings

### Requirement: Template-backed diagnostics remain sideband data
The decoder-first cache-list pipeline SHALL expose template-backed ownership diagnostics without changing the serialized cache-list item format.

#### Scenario: Generation emits template diagnostics
- **WHEN** decoder-first generation completes with template-expanded rule-authored output or template-backed fallback residuals
- **THEN** sideband diagnostics MUST include template family labels, completeness state, and residual fallback counts where available
- **THEN** serialized cache-list items MUST remain limited to the existing `_id`, `path`, and optional `version` fields

#### Scenario: Diagnostics distinguish rule and fallback paths for the same family
- **WHEN** a template-backed family emits both proven rule-authored paths and residual fallback-authored paths
- **THEN** sideband diagnostics MUST report both counts under the same stable family label
- **THEN** serialized cache-list items MUST remain unchanged

