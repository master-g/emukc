## ADDED Requirements

### Requirement: Aggregate progress bar shows overall populate progress
The system SHALL display a progress bar showing total files processed, total files to process, percentage, and estimated time remaining.

#### Scenario: Normal populate progress
- **WHEN** populate is running on a TTY terminal
- **THEN** an aggregate progress bar is displayed with format `Populating cache ━━ pos/len (percent%, ETA: eta)`

#### Scenario: Non-TTY terminal
- **WHEN** populate is running without a TTY (piped or redirected)
- **THEN** no progress bars or spinners are displayed; logs write to file only

### Requirement: Stats bar shows concurrency and error count
The system SHALL display a stats line showing the number of currently active concurrent tasks, configured maximum concurrency, and cumulative error count.

#### Scenario: Active downloads in progress
- **WHEN** populate is running with 16 concurrent tasks configured and 12 are active and 2 errors have occurred
- **THEN** the stats bar displays `12/16 active │ 2 errors`

#### Scenario: All tasks completed
- **WHEN** populate finishes all tasks
- **THEN** the stats bar shows `0/N active │ E errors` before clearing

### Requirement: Per-task spinners show resource path of active downloads
The system SHALL display one spinner per in-flight download task showing the resource path being downloaded.

#### Scenario: Multiple concurrent downloads
- **WHEN** 4 tasks are actively downloading resources
- **THEN** 4 spinners are visible, each showing the `item.path` of the resource being fetched (e.g., `⠋ kcs2/resources/ship/banner/001.png`)

#### Scenario: Task completes successfully
- **WHEN** a download task finishes without error
- **THEN** the task's spinner is cleared from the display

### Requirement: Error spinners display failure details
The system SHALL show error information when a download task fails.

#### Scenario: Task fails with an error
- **WHEN** a download task encounters an error (e.g., 404, timeout)
- **THEN** the spinner for that task displays the resource path and error summary (e.g., `✗ kcs2/resources/equip/099.png (not found)`) and persists visible until populate completes

#### Scenario: Multiple errors accumulate
- **WHEN** multiple tasks fail during populate
- **THEN** each failed task's error spinner remains visible, and the error count in the stats bar reflects the total
