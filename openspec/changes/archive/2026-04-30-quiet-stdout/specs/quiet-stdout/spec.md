## ADDED Requirements

### Requirement: Builder quiet stdout mode
`emukc_log::Builder` SHALL accept a `quiet_stdout` flag that, when set to true, causes `build()` and `build_simple()` to skip registering the stdout fmt layer.

#### Scenario: Quiet mode with file appender
- **WHEN** `Builder::with_quiet_stdout(true)` is called and a file appender is configured
- **THEN** `build()` SHALL register only the file layer, not the stdout layer

#### Scenario: Quiet mode without file appender
- **WHEN** `Builder::with_quiet_stdout(true)` is called and no file appender is configured
- **THEN** `build()` SHALL register no layers (logs discarded)

#### Scenario: Quiet mode false (default)
- **WHEN** `with_quiet_stdout` is not called or called with `false`
- **THEN** `build()` SHALL register the stdout layer as it does currently

### Requirement: CLI subcommands suppress stdout logs
The CLI entry point SHALL enable quiet stdout mode for subcommands that use indicatif progress bars.

#### Scenario: Bootstrap command
- **WHEN** user runs the `bootstrap` subcommand
- **THEN** the log builder SHALL be configured with `with_quiet_stdout(true)`

#### Scenario: Cache populate command
- **WHEN** user runs `cache populate`
- **THEN** the log builder SHALL be configured with `with_quiet_stdout(true)`

#### Scenario: Other commands
- **WHEN** user runs any other subcommand (serve, battle, etc.)
- **THEN** stdout logging SHALL behave normally (no quiet mode)
