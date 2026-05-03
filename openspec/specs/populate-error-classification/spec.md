## ADDED Requirements

### Requirement: BOOTSTRAP.md documents structured retry classification

BOOTSTRAP.md SHALL state that `cache populate` distinguishes "version rollback" failures (which are skipped without retry) from genuine download failures (which are retried in pass 2), and SHALL note that this classification is based on a typed error variant rather than an error message substring.

#### Scenario: User reads troubleshooting section

- **WHEN** a user encounters `skipping N items with version rollback` in populate output
- **THEN** the BOOTSTRAP.md troubleshooting section SHALL explain this means the on-disk version is newer than the manifest version (a no-op, not a failure)
- **THEN** the documentation SHALL state that retried failures are genuinely failed downloads (likely network or 404)
