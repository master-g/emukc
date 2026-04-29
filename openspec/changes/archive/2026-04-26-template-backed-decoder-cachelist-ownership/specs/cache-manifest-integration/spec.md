## ADDED Requirements

### Requirement: Rules strategy expands decoder template-backed resource families
The decoder-driven `Rules` cache-list generation path SHALL expand decoder-emitted template-backed resource families using declared runtime input bindings before consulting legacy fallback generators for those families.

#### Scenario: Complete template and inputs are available
- **WHEN** the decoder bundle contains a complete template-backed family descriptor and all declared runtime inputs are available to bootstrap generation
- **THEN** the `Rules` path MUST expand the descriptor into cache-list paths using the decoder-provided path template
- **THEN** the expanded paths MUST be recorded as rule-authored output

#### Scenario: Template input binding is unavailable
- **WHEN** the decoder bundle contains a template-backed family descriptor but one or more declared runtime inputs cannot be loaded or validated
- **THEN** the `Rules` path MUST NOT mark that family as completely decoder-authored
- **THEN** generation MAY use existing fallback behavior for the affected family and MUST attribute those paths as fallback-authored output

### Requirement: Rules strategy suppresses broad fallback for complete template families
The decoder-driven `Rules` cache-list generation path SHALL suppress broad legacy fallback expansion for template-backed families whose descriptor and input bindings prove complete decoder-authoritative coverage.

#### Scenario: Complete template covers a family
- **WHEN** a template-backed family is complete and has been expanded from decoder metadata and validated runtime inputs
- **THEN** matching legacy fallback generators MUST NOT add the same family as fallback-authored output
- **THEN** duplicate path strings from fallback MUST NOT inflate fallback ownership for the covered family

#### Scenario: Template covers only a subset
- **WHEN** a template-backed family descriptor proves only a concrete subset or is marked partial
- **THEN** the `Rules` path MUST emit the proven subset as rule-authored output when possible
- **THEN** fallback MUST remain available only for the uncovered residual members and MUST keep fallback-authored attribution
