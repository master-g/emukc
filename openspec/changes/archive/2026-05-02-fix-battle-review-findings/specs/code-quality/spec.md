## ADDED Requirements

### Requirement: SortieRepository trait methods SHALL not collide with SortieStore inherent method names

The `SortieStore` inherent methods used internally by the trait implementation SHALL follow a `_sortie` suffix convention to avoid shadowing `SortieRepository` trait methods of the same name.

#### Scenario: Trait method delegation is explicit
- **WHEN** `SortieRepository` is implemented for `SortieStore`
- **THEN** each trait method body SHALL delegate to a distinctly-named inherent method (e.g., `get_pending_battle_sortie`), or directly access the hashmap

### Requirement: Dead code SHALL be removed from production paths

Functions only referenced by `#[cfg(test)]` code SHALL either be gated with `#[cfg(test)]` or moved into the test module.

#### Scenario: enemy_slot_ids is not compiled in production
- **WHEN** building without `--test`
- **THEN** `fn enemy_slot_ids(&BattleShipInput)` SHALL NOT exist in `game/sortie.rs`

### Requirement: Unused imports SHALL be removed

Each module SHALL import only the symbols it directly references.

#### Scenario: practice/mod.rs imports are minimal
- **WHEN** `practice/mod.rs` is compiled
- **THEN** `BattleType`, `EngagementType`, and `CryptoRng` SHALL NOT appear in its import list
