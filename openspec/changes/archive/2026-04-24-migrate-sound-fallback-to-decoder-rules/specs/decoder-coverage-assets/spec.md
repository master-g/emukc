## ADDED Requirements

### Requirement: Decoder distinguishes explicit audio assets from algorithmic sound-rule families
The decoder SHALL keep explicit audio asset extraction distinct from algorithmic `kcs/sound/*` rule extraction so decoder output makes clear which audio domains are direct asset groups and which are rule-driven sound families.

#### Scenario: Explicit audio asset is directly referenced
- **WHEN** decoded modules expose direct `se`, `bgm`, titlecall, tutorial voice, or explicit voice file references
- **THEN** the decoder MUST continue to emit those as explicit audio coverage assets
- **THEN** the decoder MUST NOT require an algorithmic sound rule just to represent a direct explicit asset path

#### Scenario: Algorithmic sound family is inferred
- **WHEN** decoded modules expose sound behavior that is better modeled as a semantic or algorithmic `kcs/sound/*` rule
- **THEN** the decoder MUST emit that family as a sound rule rather than flattening it into the explicit audio asset lists
- **THEN** the output MUST make the distinction between explicit audio coverage and rule-driven sound generation observable

### Requirement: Decoder emits sound-rule metadata needed for `kcs/sound` migration
The decoder SHALL emit the metadata needed to drive decoder-authored `kcs/sound/*` generation for covered sound families.

#### Scenario: Covered sound bucket or formula family is observed
- **WHEN** decoded modules expose a covered `kcs/sound/*` family
- **THEN** the decoder output MUST preserve the bucket identity, reachable voice IDs, and any semantic grouping needed for downstream cache-list generation
- **THEN** the emitted metadata MUST be stable enough to regenerate the same sound-rule bundle from the same decoded script version

#### Scenario: Sound-rule family is not fully derivable
- **WHEN** decoded modules expose only partial evidence for a sound-rule family
- **THEN** the decoder MUST mark that family partial or unresolved
- **THEN** the decoder MUST NOT silently claim complete algorithmic sound coverage for it
