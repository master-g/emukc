## Purpose
Material (resource) management for EmuKC. Covers the 8 material categories,
caps, addition/deduction with atomicity, regeneration, and persistence.

## Requirements

### Requirement: Material Categories and Representation
The system SHALL track 8 material categories: fuel, ammo, steel, bauxite,
instant repair (torch/bucket), instant construction (torch), development
material (devmat), and improvement material (screw). Implemented via MaterialOps.

#### Scenario: Material representation
- WHEN materials are retrieved for a profile via get_materials
- THEN all 8 categories are present with integer values
- THEN values are non-negative

### Requirement: Material Caps
Each material category SHALL have a maximum capacity determined by server
configuration and the player's HQ level. Caps MUST be applied via Codex game config.

#### Scenario: Adding materials within cap
- WHEN materials are added via add_material_impl and the result stays within the category cap
- THEN the full amount is added

#### Scenario: Adding materials exceeding cap
- WHEN materials are added and the result would exceed the cap for any category
- THEN the value is clamped to the cap (no overflow)

#### Scenario: Cap application
- WHEN add_material_impl completes
- THEN apply_hard_cap is called on the material model using Codex game configuration

### Requirement: Material Deduction
Materials SHALL be deducted atomically for construction, crafting, resupply,
and other operations. All categories MUST have sufficient stock or none are deducted.

#### Scenario: Successful deduction
- WHEN sufficient materials exist for all requested categories via deduct_material_impl
- THEN materials are deducted and the resulting material state is returned

#### Scenario: Insufficient materials
- WHEN any requested category has insufficient quantity
- THEN the operation fails with an Insufficient error indicating the category, current stock, and requested amount
- THEN no materials are deducted (the check happens before any mutation)

#### Scenario: Zero or negative deduction amounts are skipped
- WHEN a deduction request includes amounts <= 0 for some categories
- THEN those categories are skipped (no error, no mutation)

### Requirement: Material Regeneration
Fuel, ammo, steel, and bauxite SHALL regenerate over time based on HQ level
and server configuration. Regeneration MUST happen on port entry via update_materials.

#### Scenario: Material regeneration on update
- WHEN update_materials is called for a profile
- THEN apply_self_replenish is called using the profile's HQ level and elapsed time
- THEN regenerated values are clamped to the material cap
- THEN the updated material state is persisted

### Requirement: Material Initialization
New profiles SHALL receive a starting set of materials from Codex game configuration.

#### Scenario: Initial materials
- WHEN a profile is first initialized
- THEN materials are created from codex.game_cfg.material.new_material()

### Requirement: Material Persistence
Material changes SHALL be persisted via the SeaORM material entity under entity::profile::material.

#### Scenario: Transactional material operations
- WHEN an _impl function modifies materials within a transaction
- THEN changes are only committed when the enclosing transaction commits

#### Scenario: Material record uniqueness
- WHEN material records are queried
- THEN exactly one record exists per profile_id (enforced by database)
