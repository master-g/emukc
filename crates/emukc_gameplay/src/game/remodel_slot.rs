//! Improvement arsenal (改修工廠) operations.
//!
//! Entering the arsenal requires 明石 or 明石改 as flagship of fleet 1; the
//! second ship decides which recipes are on offer today. Rules are documented
//! in `crates/emukc_model/src/codex/remodel_slot.rs`.

use emukc_db::{
    entity::profile::item::slot_item,
    sea_orm::{TransactionTrait, entity::prelude::*},
};
use emukc_model::codex::remodel_slot::{
    AKASHI_KAI_MST_ID, AKASHI_MST_ID, MAX_STARS, RemodelRecipe, recover_success_rate,
    remodel_success_rate,
};
use emukc_model::{prelude::*, profile::material::Material};
use emukc_time::chrono::{Datelike, Utc};

use crate::err::GameplayError;
use crate::gameplay::Ctx;

use super::fleet::get_fleet_ships_impl;
use super::material::{deduct_material_impl, get_mat_impl};
use super::slot_item::{add_slot_item_impl, find_slot_item_impl, update_slot_item_impl};
use super::use_item::deduct_use_item_impl;

/// One row of the improvement candidate list.
#[derive(Debug, Clone)]
pub struct RemodelSlotListEntry {
    pub recipe_id: i64,
    /// `mst_id` of the equipment this recipe improves.
    pub slot_item_id: i64,
    pub req_fuel: i64,
    pub req_ammo: i64,
    pub req_steel: i64,
    pub req_bauxite: i64,
    /// 開発資材.
    pub req_buildkit: i64,
    /// 改修資材.
    pub req_remodelkit: i64,
}

/// Costs of a specific improvement attempt, shown before the player commits.
#[derive(Debug, Clone)]
pub struct RemodelSlotDetail {
    pub req_buildkit: i64,
    pub req_remodelkit: i64,
    /// Cost when the player pays to guarantee success.
    pub certain_buildkit: i64,
    pub certain_remodelkit: i64,
    /// Equipment consumed by the attempt, `0` when none.
    pub req_slot_id: i64,
    pub req_slot_num: i64,
    pub req_slot_id2: Option<i64>,
    pub req_slot_num2: Option<i64>,
    pub req_useitem_id: Option<i64>,
    pub req_useitem_num: Option<i64>,
    pub req_useitem_id2: Option<i64>,
    pub req_useitem_num2: Option<i64>,
    /// Whether this attempt upgrades the equipment into a different one.
    pub change_flag: bool,
}

/// Outcome of an improvement attempt.
#[derive(Debug, Clone)]
pub struct RemodelSlotResult {
    pub success: bool,
    /// `[before, after]` equipment `mst_id`.
    pub remodel_id: [i64; 2],
    pub after_material: Material,
    /// The updated equipment. Absent on failure.
    pub after_slot: Option<KcApiSlotItem>,
    /// Instance ids of the equipment eaten by the attempt.
    pub used_slot_ids: Vec<i64>,
    pub voice_ship_id: i64,
    pub voice_id: i64,
}

/// Outcome of a level reset attempt.
#[derive(Debug, Clone)]
pub struct RemodelSlotRecoverResult {
    pub success: bool,
    /// The equipment, back at ★0. Absent on failure.
    pub after_slot: Option<KcApiSlotItem>,
}

/// Flagship and second ship of fleet 1, as the arsenal sees them.
struct ArsenalSecretaries {
    /// True when the flagship is 明石改 rather than plain 明石.
    flagship_is_kai: bool,
    /// `mst_id` of the second ship, or `None` when the fleet has only one.
    second_mst_id: Option<i64>,
    /// Instance id of the second ship, for the remodel voice line.
    second_ship_id: i64,
}

impl Ctx {
    /// List every improvement recipe available right now.
    ///
    /// Availability is decided by the second ship of fleet 1 and the current
    /// weekday. Returns an empty list when the flagship is not 明石(改).
    pub async fn remodel_slot_list(
        &self,
        profile_id: i64,
    ) -> Result<Vec<RemodelSlotListEntry>, GameplayError> {
        let db = self.db.as_ref();
        let codex = self.codex.as_ref();

        let secretaries = resolve_secretaries(db, profile_id).await?;
        let Some(second_mst_id) = secretaries.second_mst_id else {
            return Ok(Vec::new());
        };

        let weekday = Utc::now().weekday().num_days_from_monday();

        let entries = codex
            .remodel_recipes()
            .into_iter()
            .filter(|recipe| recipe.allows_secretary(second_mst_id, weekday))
            .map(|recipe| {
                // The client shows the entry-level cost here; the real cost for
                // a given equipment comes from the detail call.
                let per_level = recipe.consumption_at(0);
                RemodelSlotListEntry {
                    recipe_id: recipe.recipe_id,
                    slot_item_id: recipe.slot_item_id,
                    req_fuel: recipe.base_consumption.fuel,
                    req_ammo: recipe.base_consumption.ammo,
                    req_steel: recipe.base_consumption.steel,
                    req_bauxite: recipe.base_consumption.bauxite,
                    req_buildkit: per_level.map_or(0, |c| c.dev_mat_min),
                    req_remodelkit: per_level.map_or(0, |c| c.screw_min),
                }
            })
            .collect();

        Ok(entries)
    }

    /// Costs of improving `slot_id` through `recipe_id`.
    pub async fn remodel_slot_detail(
        &self,
        profile_id: i64,
        recipe_id: i64,
        slot_id: i64,
    ) -> Result<RemodelSlotDetail, GameplayError> {
        let db = self.db.as_ref();
        let codex = self.codex.as_ref();

        let recipe = codex.find_remodel_recipe(recipe_id)?;
        let item = find_owned_item(db, profile_id, slot_id, &recipe, false).await?;

        let consumption = recipe.consumption_at(item.level).ok_or_else(|| {
            GameplayError::WrongType(format!(
                "slot item {slot_id} at star {} has no recipe stage",
                item.level
            ))
        })?;

        let slot_costs = consumption.slot_item_consumption.as_deref().unwrap_or_default();
        let item_costs = consumption.use_item_consumption.as_deref().unwrap_or_default();

        Ok(RemodelSlotDetail {
            req_buildkit: consumption.dev_mat_min,
            req_remodelkit: consumption.screw_min,
            certain_buildkit: consumption.dev_mat_max,
            certain_remodelkit: consumption.screw_max,
            req_slot_id: slot_costs.first().map_or(0, |c| c.id),
            req_slot_num: slot_costs.first().map_or(0, |c| c.count),
            req_slot_id2: slot_costs.get(1).map(|c| c.id),
            req_slot_num2: slot_costs.get(1).map(|c| c.count),
            req_useitem_id: item_costs.first().map(|c| c.id),
            req_useitem_num: item_costs.first().map(|c| c.count),
            req_useitem_id2: item_costs.get(1).map(|c| c.id),
            req_useitem_num2: item_costs.get(1).map(|c| c.count),
            change_flag: item.level >= MAX_STARS && recipe.upgrade_to.is_some(),
        })
    }

    /// Run an improvement attempt.
    ///
    /// On success the equipment gains a star, or becomes its variant when it
    /// was already at ★10. On failure the equipment is left untouched — but
    /// resources, development/improvement materials and the consumed equipment
    /// are spent either way. Use items are refunded on failure, matching the
    /// upstream rule that items are not consumed by a failed attempt.
    pub async fn remodel_slot(
        &self,
        profile_id: i64,
        recipe_id: i64,
        slot_id: i64,
        certain: bool,
    ) -> Result<RemodelSlotResult, GameplayError> {
        let db = self.db.as_ref();
        let codex = self.codex.as_ref();

        let recipe = codex.find_remodel_recipe(recipe_id)?;
        let secretaries = resolve_secretaries(db, profile_id).await?;

        let tx = db.begin().await?;

        let item = find_owned_item(&tx, profile_id, slot_id, &recipe, false).await?;
        let stars_before = item.level;
        let consumption = recipe.consumption_at(stars_before).ok_or_else(|| {
            GameplayError::WrongType(format!(
                "slot item {slot_id} at star {stars_before} has no recipe stage"
            ))
        })?;

        let (dev_mat, screw) = if certain {
            (consumption.dev_mat_max, consumption.screw_max)
        } else {
            (consumption.dev_mat_min, consumption.screw_min)
        };

        deduct_material_impl(
            &tx,
            profile_id,
            &[
                (MaterialCategory::Fuel, recipe.base_consumption.fuel),
                (MaterialCategory::Ammo, recipe.base_consumption.ammo),
                (MaterialCategory::Steel, recipe.base_consumption.steel),
                (MaterialCategory::Bauxite, recipe.base_consumption.bauxite),
                (MaterialCategory::DevMat, dev_mat),
                (MaterialCategory::Screw, screw),
            ],
        )
        .await?;

        // Equipment required by the recipe is eaten whether or not the attempt
        // succeeds, so it is consumed before the roll.
        let used_slot_ids = consume_required_items(
            &tx,
            profile_id,
            slot_id,
            consumption.slot_item_consumption.as_deref().unwrap_or_default(),
        )
        .await?;

        let success = certain
            || roll_success(remodel_success_rate(stars_before, secretaries.flagship_is_kai));

        // Items are only spent on a successful attempt.
        if success {
            for cost in consumption.use_item_consumption.as_deref().unwrap_or_default() {
                deduct_use_item_impl(&tx, profile_id, cost.id, cost.count).await?;
            }
        }

        let (after_mst_id, after_slot) = if success {
            apply_success(&tx, codex, profile_id, &item, &recipe).await?
        } else {
            (item.mst_id, None)
        };

        let after_material = get_mat_impl(&tx, profile_id).await?;

        tx.commit().await?;

        Ok(RemodelSlotResult {
            success,
            remodel_id: [item.mst_id, after_mst_id],
            after_material: after_material.into(),
            after_slot,
            used_slot_ids,
            voice_ship_id: secretaries.second_ship_id,
            voice_id: 0,
        })
    }

    /// Reset an improved equipment back to ★0.
    ///
    /// One 工廠資源 is spent whether or not the attempt lands; the 開発資材 the
    /// player put in are only spent on success, which is the order the client
    /// applies them in. The equipment keeps its instance id — the client feeds
    /// `api_after_slot` into the slot it already holds — so a ★10 variant stays
    /// the variant and is not turned back into what it was improved from.
    pub async fn remodel_slot_recover(
        &self,
        profile_id: i64,
        recipe_id: i64,
        slot_id: i64,
        dev_mat: i64,
    ) -> Result<RemodelSlotRecoverResult, GameplayError> {
        let db = self.db.as_ref();
        let codex = self.codex.as_ref();

        if dev_mat < 1 {
            return Err(GameplayError::WrongType(format!(
                "a level reset needs at least one 開発資材, got {dev_mat}"
            )));
        }

        let recipe = codex.find_remodel_recipe(recipe_id)?;
        // Entering the reset menu goes through the arsenal, so the same
        // flagship rule applies; the weekday secretary rules do not.
        resolve_secretaries(db, profile_id).await?;

        let tx = db.begin().await?;

        // ponytail: the client's list also hides equipment held by a fleet that
        // is out on an expedition. That is a UI courtesy, not an invariant —
        // resetting stars cannot break an expedition already under way.
        let item = find_owned_item(&tx, profile_id, slot_id, &recipe, true).await?;

        if item.level < 1 {
            return Err(GameplayError::WrongType(format!("slot item {slot_id} is already at ★0")));
        }

        deduct_use_item_impl(&tx, profile_id, KcUseItemType::ArsenalResource as i64, 1).await?;

        let success = roll_success(recover_success_rate(dev_mat));

        let after_slot = if success {
            deduct_material_impl(&tx, profile_id, &[(MaterialCategory::DevMat, dev_mat)]).await?;

            let updated = update_slot_item_impl(&tx, item.id, Some(0), None, None).await?;
            Some(KcApiSlotItem {
                api_id: updated.id,
                api_slotitem_id: updated.mst_id,
                api_locked: updated.locked as i64,
                api_level: updated.level,
                api_alv: (updated.aircraft_lv > 0).then_some(updated.aircraft_lv),
            })
        } else {
            None
        };

        tx.commit().await?;

        Ok(RemodelSlotRecoverResult {
            success,
            after_slot,
        })
    }
}

/// Read fleet 1 and check the arsenal is open at all.
async fn resolve_secretaries<C>(c: &C, profile_id: i64) -> Result<ArsenalSecretaries, GameplayError>
where
    C: ConnectionTrait,
{
    let ships = get_fleet_ships_impl(c, profile_id, 1).await?;

    let flagship = ships
        .first()
        .ok_or_else(|| GameplayError::WrongType("fleet 1 has no flagship".to_string()))?;

    let flagship_is_kai = match flagship.mst_id {
        AKASHI_MST_ID => false,
        AKASHI_KAI_MST_ID => true,
        other => {
            return Err(GameplayError::WrongType(format!(
                "the improvement arsenal needs 明石 or 明石改 as flagship, found {other}"
            )));
        }
    };

    let second = ships.get(1);

    Ok(ArsenalSecretaries {
        flagship_is_kai,
        second_mst_id: second.map(|s| s.mst_id),
        second_ship_id: second.map_or(0, |s| s.id),
    })
}

/// Fetch the equipment being improved, rejecting anything the recipe does not
/// apply to.
async fn find_owned_item<C>(
    c: &C,
    profile_id: i64,
    slot_id: i64,
    recipe: &RemodelRecipe,
    allow_equipped: bool,
) -> Result<slot_item::Model, GameplayError>
where
    C: ConnectionTrait,
{
    let item = find_slot_item_impl(c, slot_id).await?;

    if item.profile_id != profile_id {
        return Err(GameplayError::EntryNotFound(format!(
            "slot item {slot_id} does not belong to profile {profile_id}"
        )));
    }
    if item.mst_id != recipe.slot_item_id {
        return Err(GameplayError::WrongType(format!(
            "recipe {} improves equipment {}, not {}",
            recipe.recipe_id, recipe.slot_item_id, item.mst_id
        )));
    }
    if !allow_equipped && item.equip_on != 0 {
        return Err(GameplayError::WrongType(format!(
            "slot item {slot_id} is equipped on ship {}",
            item.equip_on
        )));
    }

    Ok(item)
}

/// Delete the unimproved equipment a recipe demands, and report what was eaten.
///
/// Only ★0 and unlocked equipment qualifies, and the equipment being improved
/// can never feed itself.
async fn consume_required_items<C>(
    c: &C,
    profile_id: i64,
    improving_id: i64,
    costs: &[Kc3rdSlotItemImproveItemConsumption],
) -> Result<Vec<i64>, GameplayError>
where
    C: ConnectionTrait,
{
    let mut eaten = Vec::new();

    for cost in costs {
        let candidates = slot_item::Entity::find()
            .filter(slot_item::Column::ProfileId.eq(profile_id))
            .filter(slot_item::Column::MstId.eq(cost.id))
            .filter(slot_item::Column::Level.eq(0))
            .filter(slot_item::Column::Locked.eq(false))
            .filter(slot_item::Column::EquipOn.eq(0))
            .filter(slot_item::Column::Id.ne(improving_id))
            .all(c)
            .await?;

        if (candidates.len() as i64) < cost.count {
            return Err(GameplayError::Insufficient(format!(
                "need {} unimproved copies of equipment {}, have {}",
                cost.count,
                cost.id,
                candidates.len()
            )));
        }

        for victim in candidates.into_iter().take(cost.count as usize) {
            eaten.push(victim.id);
            victim.delete(c).await?;
        }
    }

    Ok(eaten)
}

/// Apply a successful attempt: one more star, or the variant swap at ★10.
async fn apply_success<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
    item: &slot_item::Model,
    recipe: &RemodelRecipe,
) -> Result<(i64, Option<KcApiSlotItem>), GameplayError>
where
    C: ConnectionTrait,
{
    if item.level >= MAX_STARS {
        let Some((variant_mst_id, initial_stars)) = recipe.upgrade_to else {
            return Err(GameplayError::WrongType(format!(
                "recipe {} cannot improve past ★{MAX_STARS}",
                recipe.recipe_id
            )));
        };

        // The upgrade replaces the equipment rather than mutating it, so the
        // client sees a new instance id in `api_after_slot`.
        item.clone().delete(c).await?;
        let created =
            add_slot_item_impl(c, codex, profile_id, variant_mst_id, initial_stars, 0).await?;

        return Ok((
            variant_mst_id,
            Some(KcApiSlotItem {
                api_id: created.id,
                api_slotitem_id: created.mst_id,
                api_locked: created.locked as i64,
                api_level: created.level,
                api_alv: (created.aircraft_lv > 0).then_some(created.aircraft_lv),
            }),
        ));
    }

    let updated = update_slot_item_impl(c, item.id, Some(item.level + 1), None, None).await?;

    Ok((
        updated.mst_id,
        Some(KcApiSlotItem {
            api_id: updated.id,
            api_slotitem_id: updated.mst_id,
            api_locked: updated.locked as i64,
            api_level: updated.level,
            api_alv: (updated.aircraft_lv > 0).then_some(updated.aircraft_lv),
        }),
    ))
}

/// Roll against a percentage success rate.
fn roll_success(rate: i64) -> bool {
    if rate >= 100 {
        return true;
    }
    if rate <= 0 {
        return false;
    }
    i64::from(emukc_crypto::rng::u32(0..100)) < rate
}
