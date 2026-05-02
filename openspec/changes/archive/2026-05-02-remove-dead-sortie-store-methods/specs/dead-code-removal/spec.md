## ADDED Requirements

### Requirement: SortieStore SHALL NOT contain unused methods

Methods on `SortieStore` that have zero call sites SHALL be removed.

#### Scenario: Dead closure-based methods are absent
- **WHEN** `sortie_store.rs` is compiled
- **THEN** `modify_active_sortie`, `with_pending_result_mut`, and `with_pending_battle_mut` SHALL NOT exist

### Requirement: allow(dead_code) SHALL be scoped to specific items

Blanket `#![allow(dead_code)]` on modules SHALL be replaced with targeted `#[allow(dead_code)]` on specific items.

#### Scenario: sortie/mod.rs has no blanket allow
- **WHEN** `sortie/mod.rs` is compiled
- **THEN** `#![allow(dead_code)]` SHALL NOT be present; only item-level `#[allow(dead_code)]` annotations remain
