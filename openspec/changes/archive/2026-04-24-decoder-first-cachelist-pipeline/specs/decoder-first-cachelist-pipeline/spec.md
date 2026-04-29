## ADDED Requirements

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
