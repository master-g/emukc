## ADDED Requirements

### Requirement: Default strategy uses decoder rules bundle
The `CacheListMakeStrategy::Default` strategy SHALL load the decoder rules bundle (`cache_rules.json` and sibling assets) and produce the same cache list output as `CacheListMakeStrategy::Rules`.

#### Scenario: Default produces identical output to Rules
- **WHEN** `build_cache_list_paths` is called with `CacheListMakeStrategy::Default`
- **THEN** the returned path set SHALL be identical to calling with `CacheListMakeStrategy::Rules`

#### Scenario: Default fails clearly when rules bundle is missing
- **WHEN** `build_cache_list_paths` is called with `Default` and `cache_rules.json` cannot be loaded
- **THEN** the function SHALL return an error indicating the missing asset

### Requirement: Greedy strategy wraps Rules plus holes report
The `CacheListMakeStrategy::Greedy` strategy SHALL delegate path generation to the `Rules` code path, then produce a holes report file if holes exist.

#### Scenario: Greedy produces Rules output plus holes report
- **WHEN** `make` is called with `Greedy` strategy
- **THEN** the cache list SHALL contain the same paths as `Rules` strategy
- **AND** a `holes_report.txt` file SHALL be generated if any holes are detected

#### Scenario: Greedy produces no holes report when no holes exist
- **WHEN** `make` is called with `Greedy` strategy and no holes are detected
- **THEN** no `holes_report.txt` file SHALL be generated

### Requirement: Legacy Default code paths removed
The hardcoded path generation branches that run without decoder assets SHALL be removed from `source/mod.rs`, `source/kcs/mod.rs`, and `source/kcs2/resources/mod.rs`.

#### Scenario: No code path generates cache lists without decoder assets
- **WHEN** any `CacheListMakeStrategy` variant (except `Minimal` and `Manifest`) is used
- **THEN** the decoder rules bundle SHALL be loaded and used for path generation

### Requirement: Comparison example baseline defaults to Manifest
The `decoder_cachelist_compare` example SHALL default `--baseline` to `manifest` instead of `default`.

#### Scenario: Default baseline flag is Manifest
- **WHEN** `decoder_cachelist_compare` is run without `--baseline`
- **THEN** the baseline strategy SHALL be `Manifest`
