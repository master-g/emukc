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

#### Scenario: Shipgraph entries with sortno zero are excluded from friend_graph targets
- **WHEN** `graph_group_ship_ids_from_cache_rules()` resolves ship IDs for a friend_graph target (character_full, character_up, etc.)
- **THEN** shipgraph entries where `api_sortno == Some(0)` MUST be excluded from the friend_graph ID set
- **THEN** those entries MUST NOT produce character_full, character_full_dmg, character_up, or character_up_dmg paths

#### Scenario: Event ships not present in api_mst_ship are excluded via holes
- **WHEN** a shipgraph entry has `api_id >= 5000` but does not exist in `api_mst_ship`
- **THEN** the system MUST exclude that entry via the event_ship_holes mechanism
- **THEN** no character_full/character_up paths SHALL be generated for excluded event ships

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

### Requirement: Explicit path generation rejects directory-like paths without trailing slash
The explicit path generator SHALL reject paths that reference directories but lack a trailing slash character.

#### Scenario: Bare directory path without extension is filtered
- **WHEN** `generate_explicit_paths()` processes a path like `"resources/voice"` or `"resources/friendly_panel/e"`
- **THEN** the path MUST be recognized as a directory reference and excluded from the cache list
- **THEN** the path MUST NOT appear in the serialized cache-list output

#### Scenario: File path with extension is preserved
- **WHEN** `generate_explicit_paths()` processes a path like `"resources/stype/etext/sp001.png"`
- **THEN** the path MUST be included in the cache list as normal

### Requirement: Template area path expansion is scoped to observed area IDs
Template-backed area path families (airunit, airunit_extend_confirm) SHALL only generate paths for map areas known to have the corresponding resources.

#### Scenario: Decoder UI assets provide observed area IDs
- **WHEN** decoder UI resources contain observed airunit area IDs
- **THEN** template expansion MUST generate paths only for those observed IDs
- **THEN** areas without observed evidence (e.g., areas 001-005 for airunit) MUST NOT produce paths

#### Scenario: Decoder UI assets are absent
- **WHEN** decoder UI resources are not available
- **THEN** template expansion MUST fall back to the hardcoded area ID list from the unversioned fallback generator

### Requirement: Template gauge path expansion is scoped to maps with gauge files
Template-backed gauge path families SHALL only generate JSON paths for maps that actually have gauge resources on CDN.

#### Scenario: Template gauge expansion uses known gauge map set
- **WHEN** `add_template_gauge_paths()` expands the gauge template family
- **THEN** it MUST generate paths only for map IDs present in the known gauge map set (regular EO maps and event maps)
- **THEN** regular non-EO maps (e.g., 1-1, 2-1) MUST NOT produce gauge JSON paths

