## MODIFIED Requirements

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

## ADDED Requirements

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
