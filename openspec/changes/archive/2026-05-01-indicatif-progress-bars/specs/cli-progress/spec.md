## ADDED Requirements

### Requirement: Progress bar for cache populate
The `cache populate` command SHALL display a progress bar showing completed count, total count, percentage, and estimated time remaining. The progress bar SHALL update in-place without scrolling.

#### Scenario: Populating cache resources
- **WHEN** `cache populate` runs with a list of N resources
- **THEN** a single progress bar displays `completed/N`, percentage, and ETA, updating as each resource completes

#### Scenario: Non-TTY environment
- **WHEN** `cache populate` runs and stdout is not a terminal (piped or CI)
- **THEN** no progress bar is drawn and existing `info!` log output serves as progress indication

### Requirement: Progress bar for cache make-list
The `cache make-list` command in greedy mode SHALL display a progress bar showing checked count, total count, found count, check rate, and ETA.

#### Scenario: Generating cache list in greedy mode
- **WHEN** `cache make-list` runs with greedy strategy enabled
- **THEN** a progress bar displays checked/total, found count, checks/s rate, and ETA

#### Scenario: Non-TTY environment for make-list
- **WHEN** `cache make-list` runs and stdout is not a terminal
- **THEN** no progress bar is drawn and `info!` log output is used instead

### Requirement: Phase-labeled progress for bootstrap
The `bootstrap` command SHALL display phase-labeled progress output, showing which phase is active and overall progress within that phase.

#### Scenario: Bootstrap with multiple phases
- **WHEN** `bootstrap` runs through its download, parse, web assets, and save phases
- **THEN** each phase transition displays a labeled header and any per-phase progress bars

#### Scenario: Bootstrap web asset download
- **WHEN** bootstrap downloads web assets (kcs_const.js, main.js, version.json)
- **THEN** each asset download shows filename and completion status without clobbering phase output

### Requirement: Concurrent download visibility
Download operations that use concurrent tasks (e.g., `download_all` with multiple resources) SHALL show aggregate progress with per-resource detail.

#### Scenario: Multiple concurrent resource downloads
- **WHEN** multiple resources are downloading concurrently
- **THEN** an aggregate progress bar shows overall completion and individual download names are visible

### Requirement: Log output does not clobber progress bars
`info!` and `warn!` log lines emitted during progress-displaying operations SHALL NOT break or corrupt the active progress bar display.

#### Scenario: Warning emitted during populate
- **WHEN** a `warn!` log is emitted while the populate progress bar is active
- **THEN** the log line prints cleanly above the progress bar and the bar continues updating normally

#### Scenario: Info log during bootstrap download
- **WHEN** an `info!` log reports download completion during bootstrap
- **THEN** the log line appears without corrupting the progress bar state
