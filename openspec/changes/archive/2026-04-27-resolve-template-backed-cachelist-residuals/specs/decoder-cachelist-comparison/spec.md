## MODIFIED Requirements

### Requirement: Comparison report surfaces template-backed ownership
The comparison example SHALL report template-backed decoder ownership separately from generic rule-authored and fallback-authored totals when the candidate is built from a decoder rule bundle.

#### Scenario: Template-backed families are expanded
- **WHEN** a decoder-first comparison run expands one or more template-backed families as rule-authored output
- **THEN** the report MUST include grouped template-backed rule-authored counts by family or resource domain
- **THEN** the report MUST preserve the existing global rule-authored and fallback-authored totals

#### Scenario: Template-backed families remain fallback-dependent
- **WHEN** a decoder-first comparison run leaves one or more template-backed families partial, unresolved, or missing required runtime inputs
- **THEN** the report MUST include grouped fallback residuals for those template-backed families
- **THEN** the migration blocker summary MUST identify those residuals distinctly from non-template fallback prefixes

#### Scenario: Template residual reason is available
- **WHEN** the decoder-first pipeline reports a reason for a template-backed fallback residual
- **THEN** the comparison report MUST preserve that reason in the machine-readable report
- **THEN** the human-readable summary MUST identify the affected family and reason at the grouped blocker level

### Requirement: Migration readiness accounts for template-backed residuals
The comparison example SHALL treat unresolved template-backed families and fallback-authored residuals from template-backed domains as migration blockers until the report can prove decoder-authoritative ownership for the measured domains.

#### Scenario: Template residuals remain
- **WHEN** the candidate has full baseline recall but still contains fallback-authored template-backed residuals
- **THEN** the report MUST NOT mark the decoder-first candidate as migration-ready
- **THEN** the report MUST list the residual template family labels or required input gaps that prevent readiness

#### Scenario: Template residuals are resolved
- **WHEN** all measured template-backed families are generated from decoder-authoritative descriptors and validated runtime inputs with no fallback-authored residuals
- **THEN** the report MUST allow those families to be absent from the migration blocker list
- **THEN** supporting overlap, candidate-only, and authority totals MUST remain available for inspection

#### Scenario: Candidate preserves recall while residuals shrink
- **WHEN** a comparison run reduces template-backed fallback-authored residuals
- **THEN** the report MUST still include `baseline_only_count` and `candidate_only_count`
- **THEN** migration readiness MUST remain false if any measured template-backed residual blocker remains
