# decoder-rule-semantics Specification

## Purpose
Define decoder-authored ship and slot semantic rules that narrow cache-list generation beyond raw manifest extraction.

## Requirements

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

### Requirement: Ship semantic completeness controls fallback suppression
The system SHALL distinguish complete decoder ship target-family semantics from partial target observations before downstream generation treats those semantics as authoritative.

#### Scenario: Partial banner-family evidence remains fallback-safe
- **WHEN** decoded `main.js` evidence contains one or more `banner_g`, `banner2_g`, or `banner3_g` signals but does not prove complete semantic coverage for the banner target family
- **THEN** the decoder rule output MUST mark that family partial or unresolved instead of observed-complete
- **THEN** downstream cache-list generation MUST preserve legacy variant fallback for the unproven remainder of that family

#### Scenario: Complete target-family evidence suppresses broad fallback
- **WHEN** decoded `main.js` evidence proves the complete semantic scope for a ship target family
- **THEN** the decoder rule output MUST identify that family as complete with the allowed target semantics and selector scope
- **THEN** downstream cache-list generation MUST treat those semantics as authoritative for that family

#### Scenario: Hardcoded semantic cases do not imply decoder completeness
- **WHEN** a decoder implementation contains static semantic case definitions for known target families
- **THEN** those static definitions MUST NOT be emitted as observed-complete solely because any member of the family was observed
- **THEN** emitted completeness MUST reflect decoder evidence for the family, not the presence of Rust- or TypeScript-authored fallback constants

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
