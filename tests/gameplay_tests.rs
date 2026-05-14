//! Gameplay integration tests

#[path = "gameplay_tests/map/mod.rs"]
mod map;

#[path = "gameplay_tests/quest/mod.rs"]
mod quest;

#[path = "gameplay_tests/useitem_material_sync.rs"]
mod useitem_material_sync;

#[path = "gameplay_tests/remodel_hp_restore.rs"]
mod remodel_hp_restore;

#[path = "gameplay_tests/level_cap_exp.rs"]
mod level_cap_exp;

#[path = "gameplay_tests/remodel_preserve_fields.rs"]
mod remodel_preserve_fields;
