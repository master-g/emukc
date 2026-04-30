## ADDED Requirements

### Requirement: Error spinners SHALL be cleared on failure
When a populate task fails, the per-task spinner SHALL call `finish_and_clear()` so no permanent line remains on the terminal. The error information SHALL be stored in an in-memory failure collection for later display.

#### Scenario: Task fails during populate
- **WHEN** a cache item fetch fails
- **THEN** the spinner for that item is cleared from the terminal and the failure (path + error reason) is recorded in a failure collection

#### Scenario: All tasks succeed
- **WHEN** all cache item fetches succeed
- **THEN** no failures are recorded and all spinners are cleared

### Requirement: Populate SHALL continue after individual task failures
The populate function SHALL NOT terminate on the first task error. All tasks in the current pass SHALL be allowed to complete. Failures SHALL be collected, not propagated.

#### Scenario: Some tasks fail in first pass
- **WHEN** 5 out of 1000 tasks fail during the first pass
- **THEN** all 1000 tasks complete, 995 succeed, 5 failures are recorded

#### Scenario: IO error reading list file
- **WHEN** the list file cannot be read or parsed
- **THEN** the populate function SHALL return an error immediately (this is a fatal infrastructure error, not a task failure)

### Requirement: Failed items SHALL be retried once
After the first pass completes, all failed items SHALL be retried in a second pass with the same concurrency settings. Items that succeed on retry are counted as recovered.

#### Scenario: Transient failure recovered on retry
- **WHEN** a task fails in pass 1 due to a transient network error
- **THEN** the same item is retried in pass 2 and succeeds (recovered)

#### Scenario: Persistent failure after retry
- **WHEN** a task fails in pass 1 and also fails in pass 2
- **THEN** the item is recorded as a final failure

#### Scenario: No failures in first pass
- **WHEN** all tasks succeed in the first pass
- **THEN** no second pass is executed

### Requirement: End-of-run summary SHALL be printed
After all passes complete, a structured summary SHALL be printed showing: total items, successful items, items retried, items recovered, items ultimately failed, elapsed time, and a list of final failures with paths and error reasons.

#### Scenario: Run with some failures
- **WHEN** populate completes with 2 final failures
- **THEN** summary shows total, OK count, retry count, recovered count, failed count, elapsed time, and lists the 2 failed file paths with error reasons

#### Scenario: Fully successful run
- **WHEN** populate completes with zero failures
- **THEN** summary shows total count, OK count, zero failures, and elapsed time (no failure list)

### Requirement: Populate SHALL return error when final failures exist
If any items fail after retry, `populate()` SHALL return `Err`. If all items succeed (including after retry), it SHALL return `Ok(())`.

#### Scenario: All items succeed
- **WHEN** all items succeed in pass 1 or pass 2
- **THEN** `populate()` returns `Ok(())`

#### Scenario: Items remain failed after retry
- **WHEN** 2 items fail in both pass 1 and pass 2
- **THEN** `populate()` returns `Err` with a summary message
