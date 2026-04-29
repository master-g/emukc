## ADDED Requirements

### Requirement: Cache rules asset emits canonical ship semantic scope
The decoder SHALL emit ship semantic rule data in `cache_rules.json` for target families whose generation behavior cannot be represented correctly by raw manifest entries alone.

#### Scenario: Ship target family needs semantic disambiguation
- **WHEN** decoded `main.js` usage distinguishes between base, damaged-only, or group-scoped ship targets inside the same family
- **THEN** `cache_rules.json` MUST encode the canonical semantic behavior for that family
- **THEN** the emitted rule MUST preserve enough selector scope information for downstream generation to distinguish friendly, abyssal, graph-driven, or sparse-subset behavior

#### Scenario: Decoder cannot prove full ship semantic scope
- **WHEN** the decoder cannot derive complete semantic scope for a ship target family directly from decoded `main.js`
- **THEN** `cache_rules.json` MUST mark that semantic rule as partial or unresolved
- **THEN** the decoder MUST NOT synthesize complete ship semantic scope by copying Rust-authored fallback tables

### Requirement: Cache rules asset emits slot normalization semantics
The decoder SHALL emit slot semantic rule data in `cache_rules.json` for normalization-driven or alias slot target families whose behavior depends on runtime selector mapping rather than universal slotitem membership.

#### Scenario: Slot alias family is runtime-normalized
- **WHEN** decoded `main.js` usage shows a slot target family such as `item_up2` or `item_on2` is produced through runtime normalization or alias behavior
- **THEN** `cache_rules.json` MUST encode the normalization and selector constraints needed to reproduce that family precisely
- **THEN** the emitted rule MUST be consumable without treating the family as a universal slotitem category

#### Scenario: Slot normalization cannot be resolved completely
- **WHEN** the decoder cannot fully derive a slot alias family's selector or normalization behavior
- **THEN** `cache_rules.json` MUST mark that family as partial or unresolved
- **THEN** the decoder MUST NOT claim complete slot semantic precision for that family

### Requirement: Decoder semantic rules remain decoder-authored artifacts
The decoder SHALL derive ship and slot semantic rule outputs from decoded `main.js` evidence and SHALL NOT use Rust-authored path rule constants as the source of truth for semantic meaning.

#### Scenario: Semantic rule can be derived from decoded runtime evidence
- **WHEN** decoded modules expose enough call structure, grouping behavior, or normalization behavior to infer a semantic rule
- **THEN** the decoder MUST emit that rule from decoder-observed evidence with provenance
- **THEN** the rule MUST be stable when regenerated from the same decoded artifact set

#### Scenario: Semantic rule cannot be derived from decoded runtime evidence
- **WHEN** decoded modules do not expose enough evidence to derive a semantic rule safely
- **THEN** the decoder MUST leave that semantic rule partial or unresolved
- **THEN** the decoder MUST NOT backfill semantic truth by parsing Rust fallback constants
