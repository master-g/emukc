## MODIFIED Requirements

### Requirement: Baseline strategy default
The comparison example SHALL use `Manifest` as the default baseline strategy, since `Default` now delegates to `Rules` and would produce a no-op comparison.

#### Scenario: Running without --baseline uses Manifest
- **WHEN** the comparison example is invoked without `--baseline`
- **THEN** it SHALL build the baseline path set using `CacheListMakeStrategy::Manifest`

## ADDED Requirements

### Requirement: Default-vs-Rules comparison warns as no-op
When the user passes `--baseline default --rules`, the comparison example SHALL emit a warning that both strategies produce identical output.

#### Scenario: User compares Default against Rules
- **WHEN** `--baseline default --rules` is passed
- **THEN** the example SHALL print a warning that `Default` now delegates to `Rules`
- **AND** continue with the comparison (which will show 100% overlap)
