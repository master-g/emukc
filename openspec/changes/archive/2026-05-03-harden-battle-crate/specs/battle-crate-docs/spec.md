## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: RNG cross-phase continuity documented

The `simulate_day` function SHALL have a doc comment explaining that the `rng` parameter is consumed sequentially across all battle phases (kouku, OASW, opening torpedo, shelling 1, shelling 2, closing torpedo), so the same seed always produces a deterministic full battle.

#### Scenario: simulate_day doc comment explains RNG continuity

- **WHEN** a developer reads the doc comment of `simulate_day` in `simulation/mod.rs`
- **THEN** the comment SHALL state that RNG state carries across phases
- **THEN** the comment SHALL note that changing phase order or adding/removing phases changes all subsequent random outcomes

### Requirement: Air Stage2 simplification documented

The kouku Stage2 anti-air fire calculation in `simulation/kouku.rs` SHALL have a `// NOTE:` comment explaining that it uses a linear approximation (`total_aa / 400 × plane_count`) instead of the real per-ship AA with slot-level shootdowns, and that this is a known deviation from KanColle's actual formula.

#### Scenario: Stage2 code has simplification note

- **WHEN** a developer reads the Stage2 section of `simulate_kouku` in `simulation/kouku.rs`
- **THEN** a `// NOTE:` comment SHALL be present
- **THEN** the comment SHALL describe the approximation and the real formula it replaces

### Requirement: Formation modifier deduplication

The `shelling_formation_modifier` and `torpedo_formation_modifier` functions in `damage.rs` SHALL be replaced by a single `formation_modifier` function. The `asw_formation_modifier` SHALL remain separate because its values differ.

#### Scenario: Single formation_modifier function exists

- **WHEN** `damage.rs` is inspected
- **THEN** a function named `formation_modifier` SHALL exist
- **THEN** `shelling_formation_modifier` SHALL NOT exist
- **THEN** `torpedo_formation_modifier` SHALL NOT exist

#### Scenario: asw_formation_modifier remains separate

- **WHEN** `damage.rs` is inspected
- **THEN** `asw_formation_modifier` SHALL exist as a separate function with its own values (Diamond=1.2, Echelon=1.1, Line Abreast=1.3)
