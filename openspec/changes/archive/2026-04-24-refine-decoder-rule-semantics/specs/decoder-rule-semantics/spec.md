## ADDED Requirements

### Requirement: Ship semantic rules define canonical variant scope
The system SHALL represent decoder-derived ship rule semantics as canonical target behavior, including whether a target is base, damaged-only, or variant-expandable, plus the ship selector scope that may generate that target.

#### Scenario: Damaged-only target is represented explicitly
- **WHEN** decoded `main.js` usage shows a ship target family such as `banner_g`, `banner2_g`, or `banner3_g` is only valid in damaged form
- **THEN** the decoder rule output MUST encode that family as damaged-only semantic behavior
- **THEN** downstream cache-list generation MUST NOT infer undamaged sibling targets from the raw target name alone

#### Scenario: Group-scoped target preserves friendly versus abyssal boundaries
- **WHEN** decoded runtime usage for a ship target family differs between friendly ships, abyssal ships, or graph-driven ship groups
- **THEN** the decoder rule output MUST preserve those selector boundaries explicitly
- **THEN** downstream cache-list generation MUST be able to emit the allowed group without expanding into disallowed ship groups

### Requirement: Slot semantic rules define normalization-scoped alias families
The system SHALL represent decoder-derived slot rule semantics for normalization-driven target families so that alternate slot targets are modeled as constrained aliases of observed runtime selectors rather than universal slotitem categories.

#### Scenario: Normalized alternate slot target is constrained to observed runtime semantics
- **WHEN** decoded `main.js` usage shows a target family such as `item_up2` or `item_on2` is produced from a specific runtime slot selector or normalization rule
- **THEN** the decoder rule output MUST preserve that selector and normalization behavior explicitly
- **THEN** downstream cache-list generation MUST NOT expand that target family across all slotitems solely because the raw target exists

#### Scenario: Unresolved slot normalization remains explicit
- **WHEN** the decoder cannot fully derive the normalization or selector scope for a slot alias family
- **THEN** the decoder rule output MUST mark that family as partial or unresolved
- **THEN** downstream generation MUST treat it as a fallback case instead of claiming precise decoder semantics

