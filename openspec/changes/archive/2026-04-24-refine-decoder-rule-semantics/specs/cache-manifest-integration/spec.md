## MODIFIED Requirements

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

## ADDED Requirements

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

