## ADDED Requirements

### Requirement: Zero clippy warnings for emukc_battle
`cargo clippy --workspace` SHALL produce zero warnings originating from `crates/emukc_battle/`.

#### Scenario: Clean clippy run
- **WHEN** `cargo clippy --workspace` is executed
- **THEN** no warnings reference any file under `crates/emukc_battle/`

### Requirement: Self-documenting types module
The `types` module SHALL use `#[allow(missing_docs)]` to suppress missing-doc warnings on data structures whose field names are self-explanatory (mirroring KanColle API naming).

#### Scenario: Types module allows missing docs
- **WHEN** a struct in `types.rs` has no doc comment
- **THEN** clippy does not emit a `missing_docs` warning for that struct or its fields

### Requirement: Documented public enums and functions
All public enums (`BattleType`, `EngagementType`, `AirState`, `BattleOutcome`) and public functions (`simulate_day`, `simulate_night`, `calculate_mvp`, `calculate_win_rank`, `apply_cap`) SHALL have `///` doc comments describing their purpose.

#### Scenario: Public enum has doc comment
- **WHEN** a public enum in `emukc_battle` is inspected
- **THEN** it has a `///` doc comment of at least one line

#### Scenario: Public function has doc comment
- **WHEN** a public function in `emukc_battle` is inspected
- **THEN** it has a `///` doc comment of at least one line

### Requirement: Dead code annotated with allow and TODO
Unused constants and functions reserved for future features SHALL be annotated with `#[allow(dead_code)]` and a `// TODO:` comment indicating the feature that will use them.

#### Scenario: Unused constant preserved with annotation
- **WHEN** a constant in `targeting.rs` is not referenced by current code
- **THEN** it has `#[allow(dead_code)]` attribute and a `// TODO:` comment

#### Scenario: Unused function preserved with annotation
- **WHEN** a function in `damage.rs` or `targeting.rs` is not called by current code
- **THEN** it has `#[allow(dead_code)]` attribute and a `// TODO:` comment

### Requirement: Doc backticks fixed
Doc comments SHALL use backticks around type names and code identifiers per rustdoc convention.

#### Scenario: Crate-level doc uses backticks
- **WHEN** the crate root doc comment references a type name
- **THEN** the type name is wrapped in backticks
