## ADDED Requirements

### Requirement: Rules strategy consumes sibling decoder coverage assets
The decoder-driven `Rules` cache-list generation path SHALL consume sibling decoder coverage assets from the same decoder bundle as `cache_rules.json` so non-ship/slot coverage and deterministic category extensions are preserved when using the rules path.

#### Scenario: Rules path points to decoder output resources
- **WHEN** a caller builds a candidate cache list from a `cache_rules.json` file under a decoder output `resources/` directory
- **THEN** the generation path MUST load sibling decoder coverage assets from that same directory
- **THEN** audio, UI, and deterministic category extensions available in that decoder bundle MUST be applied to the generated candidate cache list

#### Scenario: Optional sibling coverage asset is missing
- **WHEN** one or more optional sibling coverage assets next to `cache_rules.json` are absent or unreadable
- **THEN** rules-driven generation MUST continue with the remaining decoder bundle data
- **THEN** the missing asset MUST be treated as explicit fallback territory instead of being assumed complete silently

### Requirement: Decoder-covered families suppress broad fallback expansion
The decoder-driven cache-list generation path SHALL treat decoder-covered families as authoritative and SHALL only invoke broad legacy expansion for families that remain partial or unresolved in the decoder bundle.

#### Scenario: Covered ship or slot family is generated from decoder semantics
- **WHEN** a ship or slot family such as `banner*`, `item_up2`, or `item_on2` is covered by an observed decoder rule
- **THEN** generation MUST use that decoder rule as the authoritative selector for the family
- **THEN** legacy universal expansion MUST NOT add sibling paths outside the rule's allowed set

#### Scenario: Family remains unresolved and falls back safely
- **WHEN** a ship or slot family is marked partial or unresolved by the decoder bundle
- **THEN** generation MAY use the existing fallback behavior for that family
- **THEN** any paths produced through that fallback MUST remain attributable as fallback-authored output
