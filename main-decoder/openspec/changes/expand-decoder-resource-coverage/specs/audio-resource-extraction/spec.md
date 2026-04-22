## ADDED Requirements

### Requirement: Extract SE resource IDs
The system SHALL scan modules for sound effect loading patterns (`playSE`, `SoundManager`, explicit `"se/"` paths) and extract all numeric SE IDs that the client requests.

#### Scenario: Discovers SE IDs matching hardcoded list
- **WHEN** the extractor processes the module graph
- **THEN** the output SE ID set covers at least the same IDs as the current `SE` hardcoded list (333 IDs: 101-118, 120, 201-258, 301-333)

#### Scenario: Discovers new SE IDs not in hardcoded list
- **WHEN** the game adds new sound effects
- **THEN** the extractor discovers the new IDs from updated main.js

### Requirement: Extract BGM resource IDs
The system SHALL scan modules for BGM loading patterns (`playBGM`, `"bgm/"` paths, BGM manager references) and extract categorized BGM IDs so Rust can generate `fanfare`, `port`, and `battle` paths without hardcoded category-specific lists.

#### Scenario: Discovers BGM IDs
- **WHEN** the extractor processes the module graph
- **THEN** the output includes categorized BGM data covering fanfare (1-5), port BGM, and battle BGM ranges

### Requirement: Extract voice resource patterns
The system SHALL scan modules for voice loading patterns (`"voice/"` paths, titlecall ranges, tutorial voice references) and extract voice IDs and ranges.

#### Scenario: Discovers titlecall ranges
- **WHEN** the extractor processes voice-related modules
- **THEN** the output includes titlecall_1 range (currently 1-103) and titlecall_2 range (currently 1-64)

#### Scenario: Discovers tutorial voice IDs
- **WHEN** the extractor processes tutorial modules
- **THEN** the output includes tutorial voice IDs matching current hardcoded list (15 files)

### Requirement: Output synced as JSON asset
The extractor output SHALL be written to `crates/emukc_bootstrap/assets/audio_resources.json` when `--sync-assets` flag is provided.

#### Scenario: JSON structure is valid
- **WHEN** the JSON asset is synced
- **THEN** it contains `seIds`, `bgm.fanfareIds`, `bgm.portIds`, `bgm.battleIds`, `voice.titlecall1Max`, `voice.titlecall2Max`, `voice.tutorialVoiceIds`, and `scriptVersion`
