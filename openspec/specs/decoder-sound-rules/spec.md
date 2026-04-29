# decoder-sound-rules Specification

## Purpose
Define decoder-authored semantic and algorithmic rules for `kcs/sound/*` cache-list generation.
## Requirements
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

### Requirement: Sound rule coverage declares fallback disposition
The decoder SHALL emit enough coverage state for each sound-rule family so downstream generation can decide whether matching legacy sound fallback remains necessary.

#### Scenario: Complete sound-rule family suppresses matching fallback
- **WHEN** decoded `main.js` evidence proves complete coverage for a `kcs/sound/*` family represented by a decoder sound rule
- **THEN** the emitted sound rule MUST identify that family as complete for cache-list generation
- **THEN** downstream generation MUST be able to skip the matching legacy Rust sound fallback generator for that family

#### Scenario: Partial sound-rule family preserves fallback territory
- **WHEN** decoded `main.js` evidence proves only part of a `kcs/sound/*` family represented by a decoder sound rule
- **THEN** the emitted sound rule MUST identify the covered members and mark the family partial or unresolved
- **THEN** downstream generation MUST preserve legacy fallback for the unproven remainder

#### Scenario: Covered and fallback sound output remain distinguishable
- **WHEN** a sound family contains both decoder-covered and fallback-covered members
- **THEN** the emitted sound-rule metadata MUST allow downstream comparison to distinguish rule-authored output from fallback-authored residual output
- **THEN** decoder-covered members MUST NOT need to be regenerated by broad legacy fallback solely to preserve path coverage

