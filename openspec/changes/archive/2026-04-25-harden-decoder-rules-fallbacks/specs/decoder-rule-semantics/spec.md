## ADDED Requirements

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
