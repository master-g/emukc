# cache-manifest-integration Specification

## Purpose
Define how decoder-produced manifest, category, coverage, and rules assets integrate with Rust bootstrap cache-list generation.
## Requirements
### Requirement: Ship resource path generation
The system SHALL generate cache list paths for resolved ship entries using `SuffixUtils` and the same path templates as the existing `make_list/source/kcs2/resources/ship.rs`. When decoder semantic rules exist for a ship target family, the system SHALL use those semantic rules to decide which target categories and selector scopes are valid. Only ship target families without applicable decoder semantic rules MAY fall back to the legacy damage-variant mapping table.

#### Scenario: Damaged-only ship target does not expand into broad legacy variants
- **WHEN** a ship entry belongs to a decoder-covered family whose semantic rule marks the effective target as damaged-only
- **THEN** the generated cache-list paths MUST include only the canonical damaged target categories allowed by that semantic rule
- **THEN** the generator MUST NOT expand that family into undamaged or unrelated sibling variants through the legacy fallback table

#### Scenario: Variant-expandable ship target still emits its allowed family
- **WHEN** a ship entry belongs to a decoder-covered family whose semantic rule allows a base target plus a constrained set of damage variants
- **THEN** the generated cache-list paths MUST include the canonical base target and only the variant targets allowed by that semantic rule
- **THEN** ship selector scope such as friendly, abyssal, or graph-driven grouping MUST remain constrained to the decoder rule

#### Scenario: Family without decoder semantic rule uses legacy fallback behavior
- **WHEN** a ship target family has no applicable decoder semantic rule
- **THEN** path generation SHALL continue using the existing static variant mapping behavior
- **THEN** output for that family SHALL remain identical to the current fallback implementation

### Requirement: Manifest generation uses decoder category groups for deterministic ship and slot gaps
The decoder-driven cache-list generation path SHALL use decoder category-group assets to generate deterministic ship and slot categories that are visible in decoder outputs but not represented as concrete manifest ship/slot entries.

#### Scenario: Ship category is present in decoder categories but absent from manifest entries
- **WHEN** decoder assets show a deterministic ship category such as `power_up` in the decoder category groups
- **THEN** the decoder-driven cache-list generation path MUST generate the corresponding ship paths using the same templates as the existing bootstrap implementation

#### Scenario: Slot category is present in decoder categories but absent from manifest entries
- **WHEN** decoder assets show a deterministic slot category such as `card_t` in the decoder category groups
- **THEN** the decoder-driven cache-list generation path MUST generate the corresponding slot paths using the same templates as the existing bootstrap implementation

### Requirement: Manifest generation constrains sparse ship and slot categories with decoder subsets
The decoder-driven cache-list generation path SHALL use decoder sparse-subset assets to constrain categories whose membership is not universal across all friendly ships or all slotitems.

#### Scenario: Sparse ship subset constrains special-resource generation
- **WHEN** the decoder sparse-subset asset provides an observed ship subset for a sparse category such as `special`, `card_round`, or `reward_*`
- **THEN** cache-list generation MUST limit output for that category to the observed subset instead of expanding the category across all friendly ships

#### Scenario: Sparse ship subset constrains sp_remodel generation
- **WHEN** the decoder sparse-subset asset provides independent subsets for `sp_remodel` image assets and remodel-message assets
- **THEN** cache-list generation MUST apply those subsets separately so `sp_remodel` output is not expanded to unrelated ships

### Requirement: Manifest generation consumes decoder audio and UI coverage assets
The decoder-driven cache-list generation path SHALL consume decoder audio and UI coverage assets to add currently missing non-ship/slot domains into the generated cache list. When decoder UI coverage assets enumerate concrete members for a family, the Rules path MUST emit those members as decoder-authored output before invoking legacy fallback, and fallback MUST remain responsible only for members not proven by the decoder bundle.

#### Scenario: Audio coverage assets are available
- **WHEN** decoder output includes audio coverage data for sound effects, BGM, or voice resources
- **THEN** the decoder-driven cache-list generation path MUST include those audio paths in the candidate cache list

#### Scenario: UI coverage assets are available
- **WHEN** decoder output includes UI coverage data for map, furniture, useitem, area, or world-select resources
- **THEN** the decoder-driven cache-list generation path MUST include those UI paths in the candidate cache list
- **THEN** the included decoder-covered UI members MUST be attributable as rule-authored candidate paths

#### Scenario: Legacy UI fallback overlaps decoder-covered members
- **WHEN** a legacy UI fallback generator produces a path already proven by the decoder UI coverage asset
- **THEN** the candidate cache list MUST preserve decoder-authored ownership for that path
- **THEN** comparison diagnostics MUST NOT count that overlapping path as fallback-authored output

### Requirement: Decoder-driven generation remains tolerant to partial coverage assets
The decoder-driven cache-list generation path SHALL tolerate missing or partial decoder coverage assets without aborting the entire generation run. Partial decoder UI assets MUST still be allowed to reclaim the concrete members they prove, while fallback remains responsible for uncovered residual members.

#### Scenario: Optional coverage asset is missing
- **WHEN** a decoder coverage asset for one domain is missing or unreadable
- **THEN** cache-list generation MUST log a warning for that domain
- **THEN** generation MUST continue for the remaining available decoder assets

#### Scenario: Sparse subset is unresolved
- **WHEN** a sparse category is marked `partial` or `unresolved` in the decoder coverage assets
- **THEN** cache-list generation MUST avoid claiming complete decoder coverage for that category
- **THEN** the generation path MUST fall back to the existing bootstrap behavior for that category or skip decoder-only expansion for it

#### Scenario: UI coverage asset provides partial concrete members
- **WHEN** a decoder UI asset proves concrete members for a map, furniture, useitem, area, or world-select family but does not prove the whole family
- **THEN** Rules-path generation MUST emit the proven members as decoder-authored output
- **THEN** fallback-generated residual members outside that proven set MUST remain attributable as fallback-authored output

### Requirement: Slot alias targets use decoder normalization semantics before universal slot expansion
The decoder-driven cache-list generation path SHALL apply decoder-authored slot normalization semantics before any universal slotitem expansion for alias families such as `item_on2` and `item_up2`.

#### Scenario: Normalized alias family emits only constrained slot paths
- **WHEN** a decoder semantic rule defines how a slot alias family maps from observed runtime slot selectors or normalization behavior
- **THEN** cache-list generation MUST emit paths only for the normalized slot IDs permitted by that rule
- **THEN** the generator MUST NOT treat that alias family as a universal slotitem category

#### Scenario: Unresolved alias family preserves fallback safety
- **WHEN** a slot alias family remains partial or unresolved in decoder semantic rules
- **THEN** cache-list generation MUST preserve existing fallback behavior for that family
- **THEN** the system MUST continue generation without claiming precise decoder coverage for that alias family

### Requirement: Rules strategy consumes sibling decoder coverage assets
The decoder-driven `Rules` cache-list generation path SHALL consume sibling decoder coverage assets from the same decoder bundle as `cache_rules.json` so non-ship/slot coverage and deterministic category extensions are preserved when using the rules path. Optional sibling coverage assets MUST be treated as fallback territory when they are absent, unreadable, or malformed, without aborting the entire rules bundle load.

#### Scenario: Rules path points to decoder output resources
- **WHEN** a caller builds a candidate cache list from a `cache_rules.json` file under a decoder output `resources/` directory
- **THEN** the generation path MUST load sibling decoder coverage assets from that same directory
- **THEN** audio, UI, and deterministic category extensions available in that decoder bundle MUST be applied to the generated candidate cache list

#### Scenario: Optional sibling coverage asset is missing or unreadable
- **WHEN** one or more optional sibling coverage assets next to `cache_rules.json` are absent or unreadable
- **THEN** rules-driven generation MUST continue with the remaining decoder bundle data
- **THEN** the missing asset MUST be treated as explicit fallback territory instead of being assumed complete silently

#### Scenario: Optional sibling coverage asset is malformed
- **WHEN** one or more optional sibling coverage assets next to `cache_rules.json` contain malformed JSON or otherwise fail to decode
- **THEN** rules-driven generation MUST continue with the remaining decoder bundle data
- **THEN** the malformed asset MUST be treated as explicit fallback territory instead of aborting the rules bundle load

### Requirement: Decoder-covered families suppress broad fallback expansion
The decoder-driven cache-list generation path SHALL treat decoder-covered families as authoritative and SHALL only invoke broad legacy expansion for families that remain partial or unresolved in the decoder bundle.

#### Scenario: Covered ship or slot family is generated from decoder semantics
- **WHEN** a ship or slot family such as `banner*`, `item_up2`, or `item_on2` is covered by an observed decoder rule
- **THEN** generation MUST use that decoder rule as the authoritative selector for the family
- **THEN** legacy universal expansion MUST NOT add sibling paths outside the rule's allowed set

#### Scenario: Family remains unresolved and falls back safely
- **WHEN** a ship or slot family is marked partial or unresolved by the decoder bundle
- **THEN** generation MAY use the existing fallback behavior for that family
- **THEN** any paths produced through that fallback MUST remain attributable as fallback-authored output

### Requirement: Rules strategy generates covered `kcs/sound` families from decoder sound rules
The decoder-driven `Rules` cache-list generation path SHALL generate covered `kcs/sound/*` families from decoder-authored sound rules before consulting legacy Rust sound generators. Residual fallback MUST narrow as decoder bucket coverage improves.

#### Scenario: Covered ship voice family is available in the decoder rule bundle
- **WHEN** the decoder rule bundle includes a covered ship voice family
- **THEN** the `Rules` path MUST generate the reachable ship voice paths from the decoder-authored rule plus existing manifest data
- **THEN** the generator MUST NOT depend on Rust-only formula tables for that covered family

#### Scenario: Covered special sound bucket is available in the decoder rule bundle
- **WHEN** the decoder rule bundle includes a covered non-ship sound bucket such as `kc9997`, `kc9998`, or `kc9999`
- **THEN** the `Rules` path MUST generate the reachable sound paths from the decoder-authored bucket rule
- **THEN** the legacy Rust bucket generator MUST only fill the residual uncovered members of that family instead of remaining the primary source for the whole bucket

### Requirement: Rules strategy suppresses duplicate sound fallback for covered families
The decoder-driven `Rules` cache-list generation path SHALL avoid running broad legacy sound fallback generators for sound families that the decoder rule bundle marks complete, while preserving fallback for partial or unresolved families.

#### Scenario: Complete decoder sound family skips matching fallback generator
- **WHEN** the decoder rule bundle contains a complete sound rule for a `kcs/sound/*` family
- **THEN** the `Rules` path MUST generate that family's reachable paths from the decoder-authored rule
- **THEN** the matching legacy Rust sound fallback generator MUST NOT insert the same family as fallback-authored output

#### Scenario: Partial decoder sound family keeps residual fallback
- **WHEN** the decoder rule bundle marks a `kcs/sound/*` family partial or unresolved
- **THEN** the `Rules` path MUST preserve legacy sound fallback for the unproven remainder of that family
- **THEN** paths generated by that fallback MUST remain attributable as fallback-authored output

#### Scenario: Duplicate paths do not inflate fallback ownership
- **WHEN** a decoder sound rule and a legacy sound fallback generator can produce the same path string
- **THEN** the `Rules` path MUST prevent decoder-covered complete families from being inserted again as fallback-authored list items
- **THEN** comparison output MUST NOT report fallback ownership for paths whose family is complete in decoder sound rules

### Requirement: Sound fallback remains explicit for unresolved families
The decoder-driven cache-list generation path SHALL preserve existing Rust sound generators only for sound families that remain partial or unresolved in the decoder rule bundle. Fallback attribution MUST remain narrow enough to show which members are still outside decoder authority.

#### Scenario: Sound family remains unresolved
- **WHEN** a sound family is marked partial or unresolved by the decoder rule bundle
- **THEN** the system MAY use the existing Rust sound generator for the uncovered portion of that family
- **THEN** any paths produced through that path MUST remain attributable as fallback-authored output

#### Scenario: Sound family is partially covered
- **WHEN** the decoder rule bundle covers only part of a `kcs/sound/*` family
- **THEN** the covered paths MUST be emitted as rule-authored output
- **THEN** the residual fallback section MUST report only the uncovered remainder of that family

### Requirement: Rules strategy expands decoder template-backed resource families
The decoder-driven `Rules` cache-list generation path SHALL expand decoder-emitted template-backed resource families using declared runtime input bindings before consulting legacy fallback generators for those families.

#### Scenario: Complete template and inputs are available
- **WHEN** the decoder bundle contains a complete template-backed family descriptor and all declared runtime inputs are available to bootstrap generation
- **THEN** the `Rules` path MUST expand the descriptor into cache-list paths using the decoder-provided path template
- **THEN** the expanded paths MUST be recorded as rule-authored output

#### Scenario: Template input binding is unavailable
- **WHEN** the decoder bundle contains a template-backed family descriptor but one or more declared runtime inputs cannot be loaded or validated
- **THEN** the `Rules` path MUST NOT mark that family as completely decoder-authored
- **THEN** generation MAY use existing fallback behavior for the affected family and MUST attribute those paths as fallback-authored output

#### Scenario: Template family has partial member coverage
- **WHEN** the decoder bundle contains a partial template-backed family descriptor with a proven subset of expandable paths
- **THEN** the `Rules` path MUST emit the proven subset as rule-authored output when the required inputs for that subset are available
- **THEN** fallback MUST remain responsible only for residual members outside the proven decoder subset

### Requirement: Rules strategy suppresses broad fallback for complete template families
The decoder-driven `Rules` cache-list generation path SHALL suppress broad legacy fallback expansion for template-backed families whose descriptor and input bindings prove complete decoder-authoritative coverage.

#### Scenario: Complete template covers a family
- **WHEN** a template-backed family is complete and has been expanded from decoder metadata and validated runtime inputs
- **THEN** matching legacy fallback generators MUST NOT add the same family as fallback-authored output
- **THEN** duplicate path strings from fallback MUST NOT inflate fallback ownership for the covered family

#### Scenario: Template covers only a subset
- **WHEN** a template-backed family descriptor proves only a concrete subset or is marked partial
- **THEN** the `Rules` path MUST emit the proven subset as rule-authored output when possible
- **THEN** fallback MUST remain available only for the uncovered residual members and MUST keep fallback-authored attribution

#### Scenario: Complete map or gauge template overlaps legacy generators
- **WHEN** decoder template expansion emits map or gauge paths that a legacy generator can also produce
- **THEN** the candidate cache list MUST preserve rule-authored ownership for the decoder-expanded paths
- **THEN** comparison diagnostics MUST NOT count those overlapping paths as fallback-authored residuals

