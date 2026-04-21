## ADDED Requirements

### Requirement: Stable output schema versioning
The resource manifest output format SHALL use a versioned schema. Field additions SHALL be backward-compatible — new fields SHALL have default values so existing Rust consumers continue working.

#### Scenario: New field added in future version
- **WHEN** a new field is added to manifest entries (e.g., a new entry kind)
- **THEN** Rust consumers built against the previous schema SHALL deserialize successfully, using defaults for the new field

#### Scenario: Version field present
- **WHEN** the manifest is generated
- **THEN** the `version` field SHALL be present and set to an integer value
- **THEN** the Rust consumer SHALL check this field and warn on unexpected versions
