## MODIFIED Requirements

### Requirement: Decoder emits semantic rules for non-ship sound buckets
The system SHALL represent decoder-derived non-ship `kcs/sound/*` families such as `9997`, `9998`, and `9999` as sound rules when those families are directly observable in decoded `main.js`. For bucket members that are decoder-observable, the emitted rules MUST be strong enough that Rust-owned bucket generators stop being the primary source for those covered members.

#### Scenario: Explicit sound bucket is referenced in decoded modules
- **WHEN** decoded modules call sound playback against a bucket such as `9998` or `9999`
- **THEN** the decoder MUST emit rule data that preserves the bucket identity and reachable voice IDs
- **THEN** downstream generation MUST be able to generate those sound paths without consulting Rust-owned fallback tables for the covered bucket entries

#### Scenario: Random-choice sound bucket is used
- **WHEN** decoded modules use random-choice playback for a sound bucket with multiple possible voice IDs
- **THEN** the decoder MUST preserve the reachable voice IDs for cache-list generation purposes
- **THEN** the emitted sound rule MUST not undercount reachable IDs because of the runtime randomization wrapper

#### Scenario: Bucket evidence is present but current extraction is incomplete
- **WHEN** decoded modules contain bucket-specific sound playback evidence for `kc9997`, `kc9998`, or `kc9999`
- **THEN** the emitted rule MUST include the decoder-observable reachable members of that bucket instead of leaving the bucket effectively empty
- **THEN** any remaining unproven bucket members MUST stay partial or unresolved rather than forcing the entire family back to Rust-owned fallback

### Requirement: Unresolved sound families remain explicit
The system SHALL mark sound-rule families as partial or unresolved when the decoder cannot derive them safely from decoded evidence. Partial status MUST describe real decoder limits rather than extractor blind spots for decoder-observable members.

#### Scenario: Sound family cannot be proven completely
- **WHEN** the decoder cannot derive complete sound coverage for a `kcs/sound/*` family from decoded modules
- **THEN** the emitted sound rule MUST be marked partial or unresolved
- **THEN** downstream generation MUST treat that family as fallback territory instead of claiming decoder-authoritative sound coverage

#### Scenario: Decoder-observable sound members exist
- **WHEN** the decoded script exposes a proper subset of reachable members for a `kcs/sound/*` family
- **THEN** the emitted sound rule MUST preserve that observed subset even if the family remains partial overall
- **THEN** downstream comparison and generation MUST be able to distinguish the covered subset from the residual fallback-owned remainder
