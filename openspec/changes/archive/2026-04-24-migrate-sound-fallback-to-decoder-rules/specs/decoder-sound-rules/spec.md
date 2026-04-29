## ADDED Requirements

### Requirement: Decoder emits semantic rules for formula-driven ship voice families
The system SHALL represent decoder-derived ship voice generation rules as semantic sound rules rather than relying on Rust-authored formulas alone.

#### Scenario: Ship voice family is derivable from decoded runtime behavior
- **WHEN** decoded `main.js` modules expose ship voice playback behavior that combines a ship voice bucket, ship identity, and voice-id families
- **THEN** the decoder MUST emit a sound rule that preserves the semantic generation model for that family
- **THEN** downstream cache-list generation MUST be able to reproduce the reachable `kcs/sound/kc*/` ship voice paths from that rule plus existing manifest data

#### Scenario: Special ship voice subsets exist
- **WHEN** decoded runtime behavior distinguishes special ship voice families such as repair voices or special-CG voice groups
- **THEN** the decoder sound rule output MUST preserve those subsets explicitly
- **THEN** downstream generation MUST NOT rely on Rust-only hardcoded lists to discover those subsets for covered families

### Requirement: Decoder emits semantic rules for non-ship sound buckets
The system SHALL represent decoder-derived non-ship `kcs/sound/*` families such as `9997`, `9998`, and `9999` as sound rules when those families are directly observable in decoded `main.js`.

#### Scenario: Explicit sound bucket is referenced in decoded modules
- **WHEN** decoded modules call sound playback against a bucket such as `9998` or `9999`
- **THEN** the decoder MUST emit rule data that preserves the bucket identity and reachable voice IDs
- **THEN** downstream generation MUST be able to generate those sound paths without consulting Rust-owned fallback tables for the covered bucket entries

#### Scenario: Random-choice sound bucket is used
- **WHEN** decoded modules use random-choice playback for a sound bucket with multiple possible voice IDs
- **THEN** the decoder MUST preserve the reachable voice IDs for cache-list generation purposes
- **THEN** the emitted sound rule MUST not undercount reachable IDs because of the runtime randomization wrapper

### Requirement: Unresolved sound families remain explicit
The system SHALL mark sound-rule families as partial or unresolved when the decoder cannot derive them safely from decoded evidence.

#### Scenario: Sound family cannot be proven completely
- **WHEN** the decoder cannot derive complete sound coverage for a `kcs/sound/*` family from decoded modules
- **THEN** the emitted sound rule MUST be marked partial or unresolved
- **THEN** downstream generation MUST treat that family as fallback territory instead of claiming decoder-authoritative sound coverage
