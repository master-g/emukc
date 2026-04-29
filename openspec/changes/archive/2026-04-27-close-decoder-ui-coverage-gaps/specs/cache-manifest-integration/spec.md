## MODIFIED Requirements

### Requirement: Manifest generation consumes decoder audio and UI coverage assets
The decoder-driven cache-list generation path SHALL consume decoder audio and UI coverage assets to add currently missing non-ship/slot domains into the generated cache list. When decoder UI coverage assets enumerate concrete members for a family, the Rules path MUST emit those members as decoder-authored output before invoking legacy fallback, and fallback MUST remain responsible only for members not proven by the decoder bundle.

#### Scenario: Audio coverage assets are available
- **WHEN** decoder output includes audio coverage data for sound effects, BGM, or voice resources
- **THEN** the decoder-driven cache-list generation path MUST include those audio paths in the candidate cache list

#### Scenario: UI coverage assets are available
- **WHEN** decoder output includes UI coverage data for map, furniture, useitem, area, or world-select resources
- **THEN** the decoder-driven cache-list generation path MUST include those UI paths in the candidate cache list
- **THEN** the included decoder-covered UI members MUST be attributable as rule-authored candidate paths

#### Scenario: Legacy UI fallback overlaps decoder-covered members
- **WHEN** a legacy UI fallback generator produces a path already proven by the decoder UI coverage asset
- **THEN** the candidate cache list MUST preserve decoder-authored ownership for that path
- **THEN** comparison diagnostics MUST NOT count that overlapping path as fallback-authored output

### Requirement: Decoder-driven generation remains tolerant to partial coverage assets
The decoder-driven cache-list generation path SHALL tolerate missing or partial decoder coverage assets without aborting the entire generation run. Partial decoder UI assets MUST still be allowed to reclaim the concrete members they prove, while fallback remains responsible for uncovered residual members.

#### Scenario: Optional coverage asset is missing
- **WHEN** a decoder coverage asset for one domain is missing or unreadable
- **THEN** cache-list generation MUST log a warning for that domain
- **THEN** generation MUST continue for the remaining available decoder assets

#### Scenario: Sparse subset is unresolved
- **WHEN** a sparse category is marked `partial` or `unresolved` in the decoder coverage assets
- **THEN** cache-list generation MUST avoid claiming complete decoder coverage for that category
- **THEN** the generation path MUST fall back to the existing bootstrap behavior for that category or skip decoder-only expansion for it

#### Scenario: UI coverage asset provides partial concrete members
- **WHEN** a decoder UI asset proves concrete members for a map, furniture, useitem, area, or world-select family but does not prove the whole family
- **THEN** Rules-path generation MUST emit the proven members as decoder-authored output
- **THEN** fallback-generated residual members outside that proven set MUST remain attributable as fallback-authored output
