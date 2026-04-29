## ADDED Requirements

### Requirement: Rules strategy generates covered `kcs/sound` families from decoder sound rules
The decoder-driven `Rules` cache-list generation path SHALL generate covered `kcs/sound/*` families from decoder-authored sound rules before consulting legacy Rust sound generators.

#### Scenario: Covered ship voice family is available in the decoder rule bundle
- **WHEN** the decoder rule bundle includes a covered ship voice family
- **THEN** the `Rules` path MUST generate the reachable ship voice paths from the decoder-authored rule plus existing manifest data
- **THEN** the generator MUST NOT depend on Rust-only formula tables for that covered family

#### Scenario: Covered special sound bucket is available in the decoder rule bundle
- **WHEN** the decoder rule bundle includes a covered non-ship sound bucket such as `kc9997`, `kc9998`, or `kc9999`
- **THEN** the `Rules` path MUST generate the reachable sound paths from the decoder-authored bucket rule
- **THEN** the legacy Rust bucket generator MUST NOT remain the primary source for the covered portion of that family

### Requirement: Sound fallback remains explicit for unresolved families
The decoder-driven cache-list generation path SHALL preserve existing Rust sound generators only for sound families that remain partial or unresolved in the decoder rule bundle.

#### Scenario: Sound family remains unresolved
- **WHEN** a sound family is marked partial or unresolved by the decoder rule bundle
- **THEN** the system MAY use the existing Rust sound generator for that family
- **THEN** any paths produced through that path MUST remain attributable as fallback-authored output
