//! One-shot scenario / state builder.
//!
//! Puts a fresh profile into a declared target state in a single call, skipping
//! the manual create-account → PvP-for-exp → sortie-to-unlock → repair loop. It
//! composes the existing gameplay traits ([`ShipOps`], [`MaterialOps`],
//! [`FleetOps`]) plus the KTD-5 direct map clear/unlock setter, so the
//! `battle sim` CLI and the integration tests share one builder.
//!
//! The builder operates over an existing profile (created via the usual
//! sign-up / new-profile / start-game flow); it seeds ship, material, fleet, and
//! map state only.

use emukc_db::sea_orm::TransactionTrait;
use emukc_model::kc2::{KcApiShip, MaterialCategory, level};

use crate::{
    err::GameplayError,
    game::{FleetOps, MaterialOps, ShipOps, clear_and_unlock_map_impl, unlock_map_impl},
    gameplay::HasContext,
};

/// A single ship to place in the scenario fleet.
#[derive(Debug, Clone)]
pub struct ShipSpec {
    /// Ship manifest (master) id.
    pub mst_id: i64,
    /// Target level.
    pub level: i64,
    /// Optional current-HP override (e.g., a damaged flagship).
    pub hp: Option<i64>,
    /// Optional fuel override.
    pub fuel: Option<i64>,
    /// Optional ammo override.
    pub ammo: Option<i64>,
}

impl ShipSpec {
    /// A ship at the given master id and level, with no stat overrides.
    pub fn new(mst_id: i64, level: i64) -> Self {
        Self {
            mst_id,
            level,
            hp: None,
            fuel: None,
            ammo: None,
        }
    }

    /// Set a current-HP override (builder style).
    #[must_use]
    pub fn with_hp(mut self, hp: i64) -> Self {
        self.hp = Some(hp);
        self
    }
}

/// A declarative target state applied over a fresh profile.
#[derive(Debug, Clone, Default)]
pub struct Scenario {
    /// Ships placed into fleet 1, in order.
    pub fleet: Vec<ShipSpec>,
    /// Materials to add.
    pub materials: Vec<(MaterialCategory, i64)>,
    /// Maps to unlock without clearing.
    pub unlock_maps: Vec<i64>,
    /// Maps to mark cleared (in dependency order); each cascades unlock to its
    /// dependents.
    pub clear_maps: Vec<i64>,
}

impl Scenario {
    /// A fresh fleet able to sortie 1-1 (which is unlocked by default).
    pub fn fresh_1_1() -> Self {
        Self {
            fleet: vec![ShipSpec::new(951, 1), ShipSpec::new(951, 1)],
            materials: default_materials(),
            unlock_maps: vec![],
            clear_maps: vec![],
        }
    }

    /// A leveled six-ship fleet with maps 1-1..1-4 cleared, so 2-1 (the
    /// mid-boss area) becomes sortie-able through the full prerequisite chain.
    pub fn leveled_for_mid_boss() -> Self {
        Self {
            fleet: vec![ShipSpec::new(951, 30); 6],
            materials: default_materials(),
            unlock_maps: vec![],
            clear_maps: vec![11, 12, 13, 14],
        }
    }
}

fn default_materials() -> Vec<(MaterialCategory, i64)> {
    vec![
        (MaterialCategory::Fuel, 10000),
        (MaterialCategory::Ammo, 10000),
        (MaterialCategory::Steel, 10000),
        (MaterialCategory::Bauxite, 10000),
        (MaterialCategory::Bucket, 100),
    ]
}

/// Apply a scenario to an existing profile, returning the created ship ids (in
/// fleet order).
///
/// Ships, materials, and fleet assignment go through the public gameplay trait
/// methods; map unlock/clear goes through the KTD-5 minimal setter in a single
/// transaction.
pub async fn apply_scenario<C>(
    ctx: &C,
    profile_id: i64,
    scenario: &Scenario,
) -> Result<Vec<i64>, GameplayError>
where
    C: HasContext + ?Sized,
{
    if !scenario.materials.is_empty() {
        ctx.add_material(profile_id, &scenario.materials).await?;
    }

    let mut ship_ids = Vec::with_capacity(scenario.fleet.len());
    for spec in &scenario.fleet {
        let mut ship = ctx.add_ship(profile_id, spec.mst_id).await?;
        apply_ship_spec(&mut ship, spec);
        ctx.update_ship(&ship).await?;
        ship_ids.push(ship.api_id);
    }

    if !ship_ids.is_empty() {
        let mut slots = [-1_i64; 6];
        for (slot, id) in slots.iter_mut().zip(ship_ids.iter()) {
            *slot = *id;
        }
        ctx.update_fleet_ships(profile_id, 1, &slots).await?;
    }

    if !scenario.unlock_maps.is_empty() || !scenario.clear_maps.is_empty() {
        let codex = ctx.codex();
        let tx = ctx.db().begin().await?;
        for &map_id in &scenario.unlock_maps {
            unlock_map_impl(&tx, codex, profile_id, map_id).await?;
        }
        for &map_id in &scenario.clear_maps {
            clear_and_unlock_map_impl(&tx, codex, profile_id, map_id).await?;
        }
        tx.commit().await?;
    }

    Ok(ship_ids)
}

/// Set level (and a consistent exp triple) plus any HP/fuel/ammo overrides on a
/// freshly-added ship. Stats are not re-scaled to level — the level is set, not
/// the derived combat stats (sufficient for sortie eligibility and AE4).
fn apply_ship_spec(ship: &mut KcApiShip, spec: &ShipSpec) {
    if spec.level > 1 {
        let exp_now = level::ship_level_required_exp(spec.level);
        let (_, next_exp) = level::exp_to_ship_level(exp_now);
        ship.api_lv = spec.level;
        ship.api_exp = [exp_now, next_exp, 0];
    }
    if let Some(hp) = spec.hp {
        ship.api_nowhp = hp;
    }
    if let Some(fuel) = spec.fuel {
        ship.api_fuel = fuel;
    }
    if let Some(ammo) = spec.ammo {
        ship.api_bull = ammo;
    }
}
