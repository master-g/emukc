## ADDED Requirements

### Requirement: Log output coexists with MultiProgress bars
The `log_with_mp` function SHALL accept an `Option<MultiProgress>` and a closure. When `Some(mp)`, it SHALL call `mp.suspend(f)`. When `None`, it SHALL call `f()` directly.

#### Scenario: TTY with MultiProgress
- **WHEN** `log_with_mp` is called with `Some(mp)` and a closure that logs
- **THEN** the closure executes inside `mp.suspend()`, preventing output collision with progress bars

#### Scenario: Non-TTY without MultiProgress
- **WHEN** `log_with_mp` is called with `None` and a closure that logs
- **THEN** the closure executes directly

### Requirement: Aggregate bar never exceeds 100%
The aggregate progress bar in `populate` SHALL have its length extended to include retry items before pass 2 begins, so the bar position never exceeds the bar length.

#### Scenario: Pass 2 starts after partial pass 1 failures
- **WHEN** pass 1 completes with N failures and pass 2 begins
- **THEN** the aggregate bar length is set to `total_files + N` before pass 2 processes any items

### Requirement: Failure collection is panic-safe
The `run_pass` function SHALL collect failures without `Arc::try_unwrap().unwrap()`, using lock-based clone instead.

#### Scenario: Task failure during populate
- **WHEN** a task fails and its failure is recorded
- **THEN** the failure is collected via `Mutex::lock().unwrap().clone()`, not `Arc::try_unwrap`
