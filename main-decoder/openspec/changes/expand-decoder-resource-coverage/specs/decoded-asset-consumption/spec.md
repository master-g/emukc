## ADDED Requirements

### Requirement: Load resource_categories.json at compile time
The Rust `emukc_bootstrap` crate SHALL load `assets/resource_categories.json` via `include_str!` and deserialize it into a typed struct. The data SHALL be available to `make_list` source modules.

#### Scenario: Categories replace hardcoded arrays
- **WHEN** `make_list` generates ship resources in Default strategy
- **THEN** ship generation groups are read from the loaded JSON instead of hardcoded category arrays like `vec!["album_status", "banner", ...]`

#### Scenario: Slot category groups replace hardcoded arrays
- **WHEN** `make_list` generates slot resources in Default strategy
- **THEN** slot generation groups are read from the loaded JSON instead of the current inline category arrays in `slot.rs`

### Requirement: Partial `resource_id_sets.json` does not replace exhaustive Rust baselines in this change
The Rust `emukc_bootstrap` crate SHALL treat `resource_id_sets.json` as advisory-only for now. Because the asset is constrained to what decoded `main.js` makes directly observable, it SHALL NOT replace the existing exhaustive ship/slot ID baselines in this change.

#### Scenario: Ship ID baselines stay in place
- **WHEN** `make_list` generates special/remodel/reward ship resources
- **THEN** it continues using the existing Rust baselines (`SPECIAL_SHIPS`, `SP_REMODEL_SHIPS`, `SP_REMODEL_MES`, `CARD_ROUNDS`, `REWARDS`) rather than assuming `resource_id_sets.json` is exhaustive

#### Scenario: `BTXT_FLAT_IDS` stays in place
- **WHEN** `make_list` generates `btxt_flat` slot resources
- **THEN** it continues using the existing `BTXT_FLAT_IDS` baseline rather than assuming `resource_id_sets.json` is exhaustive

### Requirement: Load audio_resources.json at compile time
The Rust `emukc_bootstrap` crate SHALL load `assets/audio_resources.json` via `include_str!` and use it in `unversioned.rs` and `bgm.rs`.

#### Scenario: SE IDs replace hardcoded list
- **WHEN** `make_list` generates SE resources
- **THEN** SE IDs are read from the loaded JSON instead of the hardcoded `SE` static list (333 IDs)

#### Scenario: Voice ranges replace hardcoded ranges
- **WHEN** `make_list` generates voice resources
- **THEN** titlecall ranges are read from the loaded JSON instead of hardcoded `1..=103` and `1..=64`

#### Scenario: Tutorial voice stems replace hardcoded list
- **WHEN** `make_list` generates tutorial voice resources
- **THEN** tutorial voice file stems are read from the loaded JSON instead of the hardcoded tutorial voice list

### Requirement: Load ui_resources.json at compile time
The Rust `emukc_bootstrap` crate SHALL load `assets/ui_resources.json` via `include_str!` and use it in `map.rs`, `furniture.rs`, `use_item.rs`, and `unversioned.rs`.

#### Scenario: Map resources use JSON data
- **WHEN** `make_list` generates map resources
- **THEN** explicit map files come from loaded JSON instead of the hardcoded default map file list

#### Scenario: Use item IDs replace hardcoded list
- **WHEN** `make_list` generates use item resources
- **THEN** both `card` and `card_` ids come from loaded JSON instead of hardcoded lists

#### Scenario: Area resources use JSON data
- **WHEN** `make_list` generates area resources in `unversioned.rs`
- **THEN** `area/sally` and `area/airunit` ids come from loaded JSON instead of hardcoded lists

#### Scenario: World select resources use JSON data
- **WHEN** `make_list` generates world select resources in `unversioned.rs`
- **THEN** file names come from loaded JSON instead of hardcoded lists

### Requirement: Graceful fallback when JSON assets are empty or missing fields
The Rust consumption layer SHALL not panic if a JSON asset field is empty or contains fewer entries than expected. It SHALL log a warning and continue with whatever data is available.

#### Scenario: Empty SE list
- **WHEN** `audio_resources.json` has an empty `seIds` array
- **THEN** `make_list` logs a warning and skips SE resource generation without panicking

#### Scenario: Missing new category
- **WHEN** a new ship category appears in the JSON that Rust doesn't know how to generate paths for
- **THEN** Rust logs a warning and skips that category

### Requirement: Resource list coverage is non-regressing
After switching to JSON-driven resource lists, the generated cache list SHALL contain at least as many entries as the previous hardcoded version for each migrated resource domain.

#### Scenario: Ship resource count
- **WHEN** generating with Default strategy using JSON data
- **THEN** the total ship resource count is >= the count produced by the previous hardcoded version

#### Scenario: Slot resource count
- **WHEN** generating with Default strategy using JSON data
- **THEN** the total slot resource count is >= the count produced by the previous hardcoded version

#### Scenario: Audio resource count
- **WHEN** generating with Default strategy using JSON data
- **THEN** the total migrated audio resource count is >= the count produced by the previous hardcoded version

#### Scenario: UI resource count
- **WHEN** generating with Default strategy using JSON data
- **THEN** the total migrated UI resource count is >= the count produced by the previous hardcoded version
