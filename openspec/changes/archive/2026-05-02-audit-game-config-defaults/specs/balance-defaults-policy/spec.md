## ADDED Requirements

### Requirement: Numeric balance defaults match stock behavior

The `Default` impl of every numeric balance struct in `crates/emukc_model/src/codex/game_config.rs` SHALL produce values that match stock KanColle behavior, except where explicitly documented otherwise. Numeric multipliers (XP boost, time factor, cost factor) SHALL default to `1.0` (the no-op multiplier) unless an openspec proposal explicitly justifies a different baseline.

#### Scenario: ExpConfig default has no XP boost

- **WHEN** `ExpConfig::default()` is called
- **THEN** the returned value SHALL have `ct_exp_boost == 1.0`
- **THEN** the returned value SHALL have `practice_exp_boost == 1.0`

#### Scenario: DockingConfig default has no time/cost adjustment

- **WHEN** `DockingConfig::default()` is called
- **THEN** the returned value SHALL have `time_factor == 1.0`
- **THEN** the returned value SHALL have `cost_factor == 1.0`

#### Scenario: Regression test enforces the contract

- **WHEN** `cargo test -p emukc_model` runs
- **THEN** a test in `crates/emukc_model/src/codex/game_config.rs` SHALL assert each of the above default values
- **THEN** any future change that flips these values silently SHALL produce a test failure

### Requirement: Boolean QoL defaults are documented

The `Default` impl of `PicturebookConfig` (and any future similar bool-only QoL struct) SHALL carry a docstring explaining the rationale for the chosen default values. The docstring SHALL state explicitly when the default differs from stock KanColle behavior.

#### Scenario: PicturebookConfig::default has rationale

- **WHEN** a developer reads `crates/emukc_model/src/codex/game_config.rs`
- **THEN** the `Default` impl of `PicturebookConfig` SHALL be preceded by or carry a doc comment stating that EmuKC is a single-player emulator and the picture book defaults to fully unlocked
- **THEN** the docstring SHALL note the override path (`[game.picturebook]` in `emukc.config.toml`)

### Requirement: Balance default changes follow the policy

The project SHALL document a "Balance defaults policy" in `CLAUDE.md` requiring that any change to a numeric `Default` value in `crates/emukc_model/src/codex/` (XP multipliers, drop rates, repair times, material caps) be:

1. In its own commit, separate from infrastructure or refactor work.
2. Prefixed `feat(balance):` or `chore(balance):` in the commit message.
3. Accompanied by a commit body listing the previous value(s).
4. Linked to an openspec proposal.
5. Covered by a regression test asserting the new value.

#### Scenario: Policy is discoverable

- **WHEN** a contributor reads `CLAUDE.md` at the project root
- **THEN** a section titled "Balance defaults policy" SHALL be present
- **THEN** the section SHALL enumerate rules 1 through 5 above

#### Scenario: Numeric balance change without dedicated commit

- **WHEN** a code review observes a numeric default flip inside a commit whose subject does not begin with `feat(balance):` or `chore(balance):`
- **THEN** the review SHALL block merge until the change is split into a dedicated commit

## REMOVED Requirements

### Requirement: ct_exp_boost defaults to 250.0

**Reason**: Introduced silently in `16c112f` (a bootstrap-rollback fix commit) without proposal or rationale. A 250× XP multiplier for Command Ship flagships is not stock KanColle behavior and was almost certainly a debug-time tweak.

**Migration**: Users who want the prior 250× behavior SHALL set `ct_exp_boost = 250.0` explicitly in the `[exp]` section of their `emukc.config.toml`. The configurable knob is preserved; only the implicit default reverts to `1.0`.
