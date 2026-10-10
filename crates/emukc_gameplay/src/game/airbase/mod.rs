use emukc_db::{
    entity::profile::{
        airbase::{base, plane as plane_db},
        map_record,
    },
    sea_orm::{ActiveValue, IntoActiveModel, QueryOrder, TransactionTrait, entity::prelude::*},
};
use emukc_model::{
    codex::Codex,
    kc2::MaterialCategory,
    profile::airbase::{
        AIRUNIT_MAX, Airbase, AirbaseAction, COND_DEPLOYED, COND_TICK_SECS, MAINTENANCE_LEVEL_MAX,
        PlaneInfo, PlaneState, SQUADRON_MAX, recovered_condition, recovery_per_tick,
        relocation_minutes, squadron_capacity,
    },
};
use emukc_time::chrono::{Duration, Utc};

use emukc_battle::{
    AirCorpsInput, AirRaidBase, AirSquadronInput, BattleAirBaseAttack, BattleAirRaid, BattleRng,
};

use crate::{err::GameplayError, gameplay::Ctx};

use super::map::get_map_records_impl;
use super::material::{deduct_material_impl, get_mat_impl};
use super::slot_item::{find_slot_item_impl, find_slot_items_by_id_impl, update_slot_item_impl};
use super::sortie::AirStrike;
use super::use_item::deduct_use_item_impl;
use plane::{get_planes_impl, squadrons_of};

mod plane;

/// Outcome of assigning, removing or resting squadrons.
#[derive(Debug, Clone)]
pub struct SetPlaneResult {
    /// `(api_base, api_bonus)` of the airbase after the change.
    pub distance: (i64, i64),
    /// Only the slots this call touched.
    pub updated: Vec<PlaneInfo>,
    /// Bauxite left, when the call spent any.
    pub after_bauxite: Option<i64>,
}

/// Fuel a resupply spends per aircraft replaced.
///
/// Both figures are the same for every aircraft type. Sources: wikiwiki's
/// 基地航空隊 page and <https://note.com/sukumo_inaudu/n/n3b6ad98713c2>, which
/// agree (a 15/18 squadron costs 9 fuel and 15 bauxite to fill).
pub const SUPPLY_FUEL_PER_PLANE: i64 = 3;
/// Bauxite a resupply spends per aircraft replaced.
pub const SUPPLY_BAUXITE_PER_PLANE: i64 = 5;

/// 設営隊, spent to add an air corps or raise an area's 整備Lv.
const USE_ITEM_CONSTRUCTION_CORPS: i64 = 73;
/// 航空特別増加食, spent to rest an air corps.
const USE_ITEM_AIR_RATION: i64 = 102;

/// Outcome of resupplying squadrons.
#[derive(Debug, Clone)]
pub struct SupplyResult {
    /// `(api_base, api_bonus)` of the airbase.
    pub distance: (i64, i64),
    /// The squadrons the call named.
    pub updated: Vec<PlaneInfo>,
    /// Fuel left.
    pub after_fuel: i64,
    /// Bauxite left.
    pub after_bauxite: i64,
}

impl Ctx {
    /// Unlock an airbase.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `area_id`: The area ID.
    /// - `rid`: The airbase ID.
    pub async fn unlock_airbase(
        &self,
        profile_id: i64,
        area_id: i64,
        rid: i64,
    ) -> Result<(), GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        unlock_airbase_impl(&tx, profile_id, area_id, rid).await?;

        tx.commit().await?;

        Ok(())
    }

    /// Get airbases of a profile.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    pub async fn get_airbases(&self, profile_id: i64) -> Result<Vec<Airbase>, GameplayError> {
        let db = self.db.as_ref();
        let codex = self.codex.as_ref();
        let tx = db.begin().await?;

        ensure_airbases_impl(&tx, codex, profile_id).await?;
        settle_conditions_impl(&tx, profile_id).await?;

        let models = get_airbases_impl(&tx, profile_id).await?;

        let mut airbases = Vec::with_capacity(models.len());
        for model in models {
            airbases.push(load_airbase(&tx, codex, profile_id, model).await?);
        }

        tx.commit().await?;

        Ok(airbases)
    }

    /// Assign a squadron to an airbase slot, or clear the slot.
    ///
    /// `item_id` below zero clears the slot; the equipment goes back to the
    /// inventory untouched, since a squadron only ever borrows it. Moving a
    /// squadron between slots of the *same* airbase reports both slots, which
    /// is the 交換時は[2] case in `docs/apilist.txt`. Moving one between two
    /// airbases is a different endpoint — see [`Ctx::change_deployment_base`].
    ///
    /// A deployment spends bauxite: the equipment's `api_cost` for each
    /// aircraft of the squadron (一式陸攻 costs 12, and a live account paid 216
    /// for its eighteen). A move within the airbase spends nothing.
    pub async fn set_airbase_plane(
        &self,
        profile_id: i64,
        area_id: i64,
        base_id: i64,
        squadron_id: i64,
        item_id: i64,
    ) -> Result<SetPlaneResult, GameplayError> {
        let db = self.db.as_ref();
        let codex = self.codex.as_ref();

        if !(1..=SQUADRON_MAX).contains(&squadron_id) {
            return Err(GameplayError::WrongType(format!(
                "squadron {squadron_id} is outside 1..={SQUADRON_MAX}"
            )));
        }

        let tx = db.begin().await?;

        find_owned_airbase(&tx, profile_id, area_id, base_id).await?;
        settle_conditions_impl(&tx, profile_id).await?;

        let mut touched = vec![squadron_id];
        let mut after_bauxite = None;

        if item_id < 0 {
            relocate_squadron(&tx, profile_id, area_id, base_id, squadron_id).await?;
        } else {
            let item = find_slot_item_impl(&tx, item_id).await?;
            if item.profile_id != profile_id {
                return Err(GameplayError::EntryNotFound(format!(
                    "slot item {item_id} does not belong to profile {profile_id}"
                )));
            }
            if item.equip_on != 0 {
                return Err(GameplayError::WrongType(format!(
                    "slot item {item_id} is equipped on ship {}",
                    item.equip_on
                )));
            }

            let mst = codex.manifest.find_slotitem(item.mst_id).ok_or_else(|| {
                GameplayError::EntryNotFound(format!("slot item mst {} not found", item.mst_id))
            })?;
            let capacity = squadron_capacity(mst.api_type[2]).ok_or_else(|| {
                GameplayError::WrongType(format!(
                    "equipment {} cannot be assigned to a land base",
                    item.mst_id
                ))
            })?;

            // Already flying somewhere. Within this airbase it is a move: the
            // two slots trade squadrons as they are, strength and condition
            // included, and both are reported. Anywhere else the client should
            // have called change_deployment_base instead.
            let current = find_squadron_by_slot(&tx, profile_id, item_id).await?;
            let flying = current.as_ref().filter(|m| m.state == plane_db::Status::Assigned);
            if let Some(current) = flying {
                if current.area_id != area_id || current.rid != base_id {
                    return Err(GameplayError::WrongType(format!(
                        "slot item {item_id} is deployed to airbase {}/{}",
                        current.area_id, current.rid
                    )));
                }
                let source = current.squadron_id;
                if source != squadron_id {
                    touched.push(source);
                    let other =
                        find_assigned_squadron(&tx, profile_id, area_id, base_id, squadron_id)
                            .await?;
                    move_squadron(&tx, current.clone(), base_id, squadron_id).await?;
                    if let Some(other) = other {
                        move_squadron(&tx, other, base_id, source).await?;
                    }
                }
            } else {
                // A new deployment, which a squadron coming back from relocation
                // is too. Whatever flew in the slot goes into relocation.
                if let Some(relocating) = current {
                    relocating.delete(&tx).await?;
                }
                relocate_squadron(&tx, profile_id, area_id, base_id, squadron_id).await?;

                let am = plane_db::ActiveModel {
                    slot_id: ActiveValue::Set(item_id),
                    profile_id: ActiveValue::Set(profile_id),
                    area_id: ActiveValue::Set(area_id),
                    rid: ActiveValue::Set(base_id),
                    squadron_id: ActiveValue::Set(squadron_id),
                    state: ActiveValue::Set(plane_db::Status::Assigned),
                    condition: ActiveValue::Set(COND_DEPLOYED),
                    count: ActiveValue::Set(capacity),
                    max_count: ActiveValue::Set(capacity),
                    since: ActiveValue::Set(Some(Utc::now())),
                };
                am.insert(&tx).await?;

                let cost = mst.api_cost.unwrap_or(0) * capacity;
                let left =
                    deduct_material_impl(&tx, profile_id, &[(MaterialCategory::Bauxite, cost)])
                        .await?;
                after_bauxite = Some(left.bauxite);
            }
        }

        let occupied = get_planes_impl(&tx, profile_id, area_id, base_id).await?;
        let planes = squadrons_of(profile_id, area_id, base_id, occupied);
        let distance = distance_of(&tx, codex, &planes).await?;

        touched.sort_unstable();
        let updated = planes
            .into_iter()
            .filter(|plane| touched.contains(&plane.squadron_id))
            .collect::<Vec<_>>();

        tx.commit().await?;

        Ok(SetPlaneResult {
            distance,
            updated,
            after_bauxite,
        })
    }

    /// Swap a squadron between two airbases of the same area.
    ///
    /// The client only reaches here when the equipment already flies for
    /// another airbase *and* the destination slot is occupied, so this is a
    /// genuine exchange: the two squadrons trade places. Both airbases come
    /// back whole, which is the `api_base_items` the client expects.
    pub async fn change_deployment_base(
        &self,
        profile_id: i64,
        area_id: i64,
        base_id: i64,
        base_id_src: i64,
        squadron_id: i64,
        item_id: i64,
    ) -> Result<Vec<Airbase>, GameplayError> {
        let db = self.db.as_ref();
        let codex = self.codex.as_ref();

        if !(1..=SQUADRON_MAX).contains(&squadron_id) {
            return Err(GameplayError::WrongType(format!(
                "squadron {squadron_id} is outside 1..={SQUADRON_MAX}"
            )));
        }
        if base_id == base_id_src {
            return Err(GameplayError::WrongType(
                "a deployment change needs two different airbases".to_string(),
            ));
        }

        let tx = db.begin().await?;

        find_owned_airbase(&tx, profile_id, area_id, base_id).await?;
        find_owned_airbase(&tx, profile_id, area_id, base_id_src).await?;
        // The two air corps may be under different orders.
        settle_conditions_impl(&tx, profile_id).await?;

        let incoming = find_squadron_by_slot(&tx, profile_id, item_id).await?.ok_or_else(|| {
            GameplayError::EntryNotFound(format!("slot item {item_id} flies for no airbase"))
        })?;
        if incoming.area_id != area_id || incoming.rid != base_id_src {
            return Err(GameplayError::WrongType(format!(
                "slot item {item_id} flies for airbase {}/{}, not {area_id}/{base_id_src}",
                incoming.area_id, incoming.rid
            )));
        }
        if incoming.state != plane_db::Status::Assigned {
            return Err(GameplayError::WrongType(format!("slot item {item_id} is relocating")));
        }

        let outgoing =
            find_assigned_squadron(&tx, profile_id, area_id, base_id, squadron_id).await?;

        let source_squadron = incoming.squadron_id;
        move_squadron(&tx, incoming, base_id, squadron_id).await?;
        if let Some(outgoing) = outgoing {
            move_squadron(&tx, outgoing, base_id_src, source_squadron).await?;
        }

        let mut airbases = Vec::with_capacity(2);
        for rid in [base_id_src, base_id] {
            let model = find_owned_airbase(&tx, profile_id, area_id, rid).await?;
            airbases.push(load_airbase(&tx, codex, profile_id, model).await?);
        }

        tx.commit().await?;

        Ok(airbases)
    }

    /// Give airbases of one area their orders.
    ///
    /// The client sends every airbase it changed in one call, as two lists of
    /// the same length.
    pub async fn set_airbase_actions(
        &self,
        profile_id: i64,
        area_id: i64,
        orders: &[(i64, i64)],
    ) -> Result<(), GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        // What the squadrons regained so far, they regained under the old orders.
        settle_conditions_impl(&tx, profile_id).await?;

        for (base_id, kind) in orders {
            let action = i32::try_from(*kind).ok().and_then(base::Action::n).ok_or_else(|| {
                GameplayError::WrongType(format!("airbase action {kind} is not one of 0..=4"))
            })?;
            let mut am =
                find_owned_airbase(&tx, profile_id, area_id, *base_id).await?.into_active_model();
            am.action = ActiveValue::Set(action);
            am.update(&tx).await?;
        }

        tx.commit().await?;

        Ok(())
    }

    /// Rename an airbase.
    pub async fn rename_airbase(
        &self,
        profile_id: i64,
        area_id: i64,
        base_id: i64,
        name: &str,
    ) -> Result<(), GameplayError> {
        if name.is_empty() {
            return Err(GameplayError::WrongType("an airbase needs a name".to_string()));
        }

        let db = self.db.as_ref();
        let tx = db.begin().await?;

        let mut am =
            find_owned_airbase(&tx, profile_id, area_id, base_id).await?.into_active_model();
        am.name = ActiveValue::Set(name.to_string());
        am.update(&tx).await?;

        tx.commit().await?;

        Ok(())
    }

    /// Bring the named squadrons of an airbase back to full strength.
    ///
    /// Each aircraft replaced costs [`SUPPLY_FUEL_PER_PLANE`] fuel and
    /// [`SUPPLY_BAUXITE_PER_PLANE`] bauxite. A squadron that is full, empty or
    /// relocating is left alone.
    ///
    /// ponytail: all or nothing. What upstream does when the stock covers only
    /// part of the request has no source, so the call fails rather than fill
    /// some squadrons by a rule of our own.
    pub async fn supply_airbase(
        &self,
        profile_id: i64,
        area_id: i64,
        base_id: i64,
        squadron_ids: &[i64],
    ) -> Result<SupplyResult, GameplayError> {
        let db = self.db.as_ref();
        let codex = self.codex.as_ref();
        let tx = db.begin().await?;

        find_owned_airbase(&tx, profile_id, area_id, base_id).await?;
        settle_conditions_impl(&tx, profile_id).await?;

        let mut lost = 0;
        for model in get_planes_impl(&tx, profile_id, area_id, base_id).await? {
            if !squadron_ids.contains(&model.squadron_id)
                || model.state != plane_db::Status::Assigned
                || model.count >= model.max_count
            {
                continue;
            }
            lost += model.max_count - model.count;
            let full = model.max_count;
            let mut am = model.into_active_model();
            am.count = ActiveValue::Set(full);
            am.update(&tx).await?;
        }

        // Read the stock through the deduction even when nothing was lost: it
        // skips zero amounts and the client sets both counters from the answer.
        let left = deduct_material_impl(
            &tx,
            profile_id,
            &[
                (MaterialCategory::Fuel, lost * SUPPLY_FUEL_PER_PLANE),
                (MaterialCategory::Bauxite, lost * SUPPLY_BAUXITE_PER_PLANE),
            ],
        )
        .await?;

        let occupied = get_planes_impl(&tx, profile_id, area_id, base_id).await?;
        let planes = squadrons_of(profile_id, area_id, base_id, occupied);
        let distance = distance_of(&tx, codex, &planes).await?;
        let updated =
            planes.into_iter().filter(|plane| squadron_ids.contains(&plane.squadron_id)).collect();

        tx.commit().await?;

        Ok(SupplyResult {
            distance,
            updated,
            after_fuel: left.fuel,
            after_bauxite: left.bauxite,
        })
    }

    /// Add an air corps to an area, for one 設営隊.
    ///
    /// The area must already have its first one, which comes with the map, and
    /// holds at most `AIRUNIT_MAX`.
    pub async fn expand_airbase(
        &self,
        profile_id: i64,
        area_id: i64,
    ) -> Result<Airbase, GameplayError> {
        let db = self.db.as_ref();
        let codex = self.codex.as_ref();
        let tx = db.begin().await?;

        let owned = airbases_of_area(&tx, profile_id, area_id).await?;
        let Some(first) = owned.first() else {
            return Err(GameplayError::EntryNotFound(format!(
                "profile {profile_id} has no air corps in area {area_id} to expand"
            )));
        };
        let rid = i64::try_from(owned.len()).unwrap_or(AIRUNIT_MAX) + 1;
        if rid > AIRUNIT_MAX {
            return Err(GameplayError::WrongType(format!(
                "area {area_id} already holds {AIRUNIT_MAX} air corps"
            )));
        }
        let level = first.maintenance_level;

        deduct_use_item_impl(&tx, profile_id, USE_ITEM_CONSTRUCTION_CORPS, 1).await?;

        let mut am = unlock_airbase_impl(&tx, profile_id, area_id, rid).await?.into_active_model();
        am.maintenance_level = ActiveValue::Set(level);
        let model = am.update(&tx).await?;
        let airbase = load_airbase(&tx, codex, profile_id, model).await?;

        tx.commit().await?;

        Ok(airbase)
    }

    /// Raise an area's 整備Lv by one, for one 設営隊.
    ///
    /// ponytail: the level lives on the area's airbase rows, so an area the
    /// profile has no air corps in cannot be raised. Upstream allows an event
    /// area that is not open yet; that needs a per-area table, add it with the
    /// event maps.
    pub async fn expand_airbase_maintenance(
        &self,
        profile_id: i64,
        area_id: i64,
    ) -> Result<i64, GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        // The level changes the recovery rate from here on.
        settle_conditions_impl(&tx, profile_id).await?;
        let owned = airbases_of_area(&tx, profile_id, area_id).await?;
        let Some(first) = owned.first() else {
            return Err(GameplayError::EntryNotFound(format!(
                "profile {profile_id} has no air corps in area {area_id}"
            )));
        };
        let level = first.maintenance_level + 1;
        if level > MAINTENANCE_LEVEL_MAX {
            return Err(GameplayError::WrongType(format!(
                "area {area_id} is already at maintenance level {MAINTENANCE_LEVEL_MAX}"
            )));
        }

        deduct_use_item_impl(&tx, profile_id, USE_ITEM_CONSTRUCTION_CORPS, 1).await?;

        for model in owned {
            let mut am = model.into_active_model();
            am.maintenance_level = ActiveValue::Set(level);
            am.update(&tx).await?;
        }

        tx.commit().await?;

        Ok(level)
    }

    /// Rest an air corps with one 航空特別増加食: every squadron below a fresh
    /// deployment's condition goes back to it. The ration is spent even when
    /// nobody was tired.
    ///
    /// ponytail: how much a ration restores has no source; "back to untired"
    /// is this project's reading of the item.
    pub async fn recover_airbase_condition(
        &self,
        profile_id: i64,
        area_id: i64,
        base_id: i64,
    ) -> Result<SetPlaneResult, GameplayError> {
        let db = self.db.as_ref();
        let codex = self.codex.as_ref();
        let tx = db.begin().await?;

        find_owned_airbase(&tx, profile_id, area_id, base_id).await?;
        deduct_use_item_impl(&tx, profile_id, USE_ITEM_AIR_RATION, 1).await?;
        settle_conditions_impl(&tx, profile_id).await?;

        for model in get_planes_impl(&tx, profile_id, area_id, base_id).await? {
            if model.state == plane_db::Status::Assigned && model.condition < COND_DEPLOYED {
                let mut am = model.into_active_model();
                am.condition = ActiveValue::Set(COND_DEPLOYED);
                am.update(&tx).await?;
            }
        }

        let occupied = get_planes_impl(&tx, profile_id, area_id, base_id).await?;
        let planes = squadrons_of(profile_id, area_id, base_id, occupied);
        let distance = distance_of(&tx, codex, &planes).await?;
        let updated =
            planes.into_iter().filter(|p| matches!(p.state, PlaneState::Assigned)).collect();

        tx.commit().await?;

        Ok(SetPlaneResult {
            distance,
            updated,
            after_bauxite: None,
        })
    }

    /// The squadrons of an air corps after the recovery time has brought them,
    /// for the client's timed recovery poll.
    ///
    /// The client asks once per air corps each time it enters the sortie
    /// scene, and only while it holds a squadron it has not seen untired. It
    /// may hold a condition from before several sorties, so the answer is
    /// always the settled squadrons rather than "what changed just now".
    pub async fn recover_airbase_condition_with_time(
        &self,
        profile_id: i64,
        area_id: i64,
        base_id: i64,
    ) -> Result<SetPlaneResult, GameplayError> {
        let db = self.db.as_ref();
        let codex = self.codex.as_ref();
        let tx = db.begin().await?;

        find_owned_airbase(&tx, profile_id, area_id, base_id).await?;
        settle_conditions_impl(&tx, profile_id).await?;

        let occupied = get_planes_impl(&tx, profile_id, area_id, base_id).await?;
        let planes = squadrons_of(profile_id, area_id, base_id, occupied);
        let distance = distance_of(&tx, codex, &planes).await?;
        let updated =
            planes.into_iter().filter(|p| matches!(p.state, PlaneState::Assigned)).collect();

        tx.commit().await?;

        Ok(SetPlaneResult {
            distance,
            updated,
            after_bauxite: None,
        })
    }
}

/// Bring every flying squadron's condition up to now.
///
/// A squadron regains condition every [`COND_TICK_SECS`] at the rate of its air
/// corps' order and the area's 整備Lv. Nothing runs on a clock: the ticks since
/// `since` are applied whenever something is about to read a condition or
/// change the rate, so call this first in both cases.
pub(crate) async fn settle_conditions_impl<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    let now = Utc::now();
    let airbases = get_airbases_impl(c, profile_id).await?;
    let planes = plane_db::Entity::find()
        .filter(plane_db::Column::ProfileId.eq(profile_id))
        .filter(plane_db::Column::State.eq(plane_db::Status::Assigned))
        .all(c)
        .await?;

    for plane in planes {
        // A row from before the column existed starts counting now.
        let since = plane.since.unwrap_or(now);
        let ticks = (now - since).num_seconds() / COND_TICK_SECS;
        if plane.since.is_some() && ticks <= 0 {
            continue;
        }
        let rate = airbases
            .iter()
            .find(|base| base.area_id == plane.area_id && base.rid == plane.rid)
            .map_or(0, |base| {
                recovery_per_tick(AirbaseAction::from(base.action), base.maintenance_level)
            });
        let condition = recovered_condition(plane.condition, rate.max(1), ticks.max(0));

        let mut am = plane.into_active_model();
        am.condition = ActiveValue::Set(condition);
        am.since = ActiveValue::Set(Some(since + Duration::seconds(ticks.max(0) * COND_TICK_SECS)));
        am.update(c).await?;
    }

    Ok(())
}

/// Tire the squadrons of the air corps sent on a sortie: `(rid, condition lost)`.
///
/// Every flying squadron pays, whatever it then meets (wikiwiki: 攻撃結果や
/// 戦闘での勝利判定は影響しない, and a sortie that retreats before the attack
/// costs the same).
pub(crate) async fn tire_air_corps_impl<C>(
    c: &C,
    profile_id: i64,
    area_id: i64,
    costs: &[(i64, i64)],
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    settle_conditions_impl(c, profile_id).await?;

    for (rid, cost) in costs {
        for plane in get_planes_impl(c, profile_id, area_id, *rid).await? {
            if plane.state != plane_db::Status::Assigned {
                continue;
            }
            let condition = (plane.condition - cost).max(0);
            let mut am = plane.into_active_model();
            am.condition = ActiveValue::Set(condition);
            am.update(c).await?;
        }
    }

    Ok(())
}

/// The profile's air corps in one area, with squadrons and radius, in `rid` order.
pub(crate) async fn load_area_airbases_impl<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
    area_id: i64,
) -> Result<Vec<Airbase>, GameplayError>
where
    C: ConnectionTrait,
{
    let mut airbases = Vec::new();
    for model in airbases_of_area(c, profile_id, area_id).await? {
        airbases.push(load_airbase(c, codex, profile_id, model).await?);
    }

    Ok(airbases)
}

/// Fuel and ammunition one squadron spends on a sortie, whatever it then does.
///
/// wikiwiki 基地航空隊「出撃コスト」: a land attacker 1.5 fuel a plane (rounded
/// up) and 0.7 ammunition (rounded down); 大型陸上機 2 and 2; anything else 1
/// fuel and 0.6 ammunition (rounded up). Eighteen 陸攻 cost 27 and 12.
pub(crate) fn sortie_cost(equip_type: i64, count: i64) -> (i64, i64) {
    match equip_type {
        47 => ((count * 3 + 1) / 2, count * 7 / 10),
        53 => (count * 2, count * 2),
        _ => (count, (count * 6 + 9) / 10),
    }
}

/// Charge the sortie of the air corps `rids` of one area to the stock.
///
/// A stock that cannot cover it pays what it has and the air corps flies
/// anyway, which is what upstream does (same wikiwiki section).
pub(crate) async fn charge_air_sortie_impl<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
    area_id: i64,
    rids: &[i64],
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    let (mut fuel, mut ammo) = (0, 0);
    for rid in rids {
        let planes: Vec<plane_db::Model> = get_planes_impl(c, profile_id, area_id, *rid)
            .await?
            .into_iter()
            .filter(|plane| plane.state == plane_db::Status::Assigned && plane.count > 0)
            .collect();
        let slot_ids: Vec<i64> = planes.iter().map(|plane| plane.slot_id).collect();
        let items = find_slot_items_by_id_impl(c, &slot_ids).await?;
        for plane in &planes {
            let equip_type = items
                .iter()
                .find(|item| item.id == plane.slot_id)
                .and_then(|item| codex.manifest.find_slotitem(item.mst_id))
                .map_or(0, |mst| mst.api_type[2]);
            let (squadron_fuel, squadron_ammo) = sortie_cost(equip_type, plane.count);
            fuel += squadron_fuel;
            ammo += squadron_ammo;
        }
    }

    let stock = get_mat_impl(c, profile_id).await?;
    deduct_material_impl(
        c,
        profile_id,
        &[
            (MaterialCategory::Fuel, fuel.min(stock.fuel)),
            (MaterialCategory::Ammo, ammo.min(stock.ammo)),
        ],
    )
    .await?;

    Ok(())
}

/// The air corps sent against a node, as the battle takes them: each with the
/// squadrons that still fly and one wave for every time it was pointed here.
///
/// `cell_nos` are all the cell numbers of the node: a node reached by several
/// edges has one per edge, and the client names whichever it drew the spot for.
pub(crate) async fn striking_air_corps_impl<C>(
    c: &C,
    profile_id: i64,
    area_id: i64,
    strikes: &[AirStrike],
    cell_nos: &[i64],
) -> Result<Vec<AirCorpsInput>, GameplayError>
where
    C: ConnectionTrait,
{
    let mut air_corps = Vec::new();
    for strike in strikes {
        let waves = strike.cells.iter().filter(|cell| cell_nos.contains(cell)).count();
        if waves == 0 {
            continue;
        }
        let planes: Vec<plane_db::Model> = get_planes_impl(c, profile_id, area_id, strike.base_rid)
            .await?
            .into_iter()
            .filter(|plane| plane.state == plane_db::Status::Assigned && plane.count > 0)
            .collect();
        let slot_ids: Vec<i64> = planes.iter().map(|plane| plane.slot_id).collect();
        let items = find_slot_items_by_id_impl(c, &slot_ids).await?;
        let squadrons: Vec<AirSquadronInput> = planes
            .iter()
            .filter_map(|plane| {
                let item = items.iter().find(|item| item.id == plane.slot_id)?;
                Some(AirSquadronInput {
                    squadron_id: plane.squadron_id,
                    mst_id: item.mst_id,
                    count: plane.count,
                    alv: item.aircraft_lv,
                })
            })
            .collect();
        if !squadrons.is_empty() {
            air_corps.push(AirCorpsInput {
                base_rid: strike.base_rid,
                waves,
                squadrons,
            });
        }
    }

    Ok(air_corps)
}

/// Write back what the air corps have left after a battle: for each one, the
/// counts its last attack ended with.
pub(crate) async fn record_strike_losses_impl<C>(
    c: &C,
    profile_id: i64,
    area_id: i64,
    attacks: &[BattleAirBaseAttack],
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    for (index, attack) in attacks.iter().enumerate() {
        if attacks[index + 1..].iter().any(|later| later.api_base_id == attack.api_base_id) {
            continue;
        }
        for (squadron_id, count) in &attack.remaining {
            plane_db::Entity::update_many()
                .col_expr(plane_db::Column::Count, Expr::value(*count))
                .filter(plane_db::Column::ProfileId.eq(profile_id))
                .filter(plane_db::Column::AreaId.eq(area_id))
                .filter(plane_db::Column::Rid.eq(attack.api_base_id))
                .filter(plane_db::Column::SquadronId.eq(*squadron_id))
                .filter(plane_db::Column::State.eq(plane_db::Status::Assigned))
                .exec(c)
                .await?;
        }
    }

    reset_emptied_squadrons_impl(
        c,
        profile_id,
        area_id,
        attacks.iter().map(|attack| attack.api_base_id),
    )
    .await
}

/// A squadron shot down to nothing starts its proficiency over (`kcsim.js` 3944).
async fn reset_emptied_squadrons_impl<C>(
    c: &C,
    profile_id: i64,
    area_id: i64,
    base_rids: impl IntoIterator<Item = i64>,
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    let emptied = plane_db::Entity::find()
        .filter(plane_db::Column::ProfileId.eq(profile_id))
        .filter(plane_db::Column::AreaId.eq(area_id))
        .filter(plane_db::Column::Rid.is_in(base_rids))
        .filter(plane_db::Column::State.eq(plane_db::Status::Assigned))
        .filter(plane_db::Column::Count.eq(0))
        .all(c)
        .await?;
    for plane in emptied {
        update_slot_item_impl(c, plane.slot_id, None, Some(0), None).await?;
    }

    Ok(())
}

/// Damage to one base from which a raid destroys aircraft on the ground.
const GROUND_LOSS_DAMAGE: i64 = 50;

/// The air corps of an area as a raid finds them, in `rid` order: every one is a target, and
/// the ones ordered to defend send their squadrons up.
pub(crate) async fn raided_air_corps_impl<C>(
    c: &C,
    profile_id: i64,
    area_id: i64,
) -> Result<Vec<AirRaidBase>, GameplayError>
where
    C: ConnectionTrait,
{
    let mut bases = Vec::new();
    for model in airbases_of_area(c, profile_id, area_id).await? {
        let planes: Vec<plane_db::Model> = get_planes_impl(c, profile_id, area_id, model.rid)
            .await?
            .into_iter()
            .filter(|plane| plane.state == plane_db::Status::Assigned)
            .collect();
        let slot_ids: Vec<i64> = planes.iter().map(|plane| plane.slot_id).collect();
        let items = find_slot_items_by_id_impl(c, &slot_ids).await?;
        bases.push(AirRaidBase {
            base_rid: model.rid,
            defending: model.action == base::Action::Defense,
            squadrons: planes
                .iter()
                .filter_map(|plane| {
                    let item = items.iter().find(|item| item.id == plane.slot_id)?;
                    Some(AirSquadronInput {
                        squadron_id: plane.squadron_id,
                        mst_id: item.mst_id,
                        count: plane.count,
                        alv: item.aircraft_lv,
                    })
                })
                .collect(),
        });
    }

    Ok(bases)
}

/// Take what a raid cost: the aircraft the defenders lost in the air, stores in proportion to
/// the damage, and aircraft on the ground of every base hit hard. Sets `api_lost_kind`.
///
/// wikiwiki 基地航空隊「基地への空襲」: fuel or bauxite, `damage × 0.9 + 0.1` rounded; a base
/// that took 50 or more loses 1 to 4 aircraft from its first squadron down, never the last
/// one of a squadron, unless it was ordered to shelter. Whether stores are lost at all is
/// random upstream at an unpublished rate; here any damage costs them.
pub(crate) async fn settle_air_raid_impl<C>(
    c: &C,
    profile_id: i64,
    area_id: i64,
    bases: &mut [AirRaidBase],
    raid: &mut BattleAirRaid,
    rng: &mut impl BattleRng,
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    let sheltered: Vec<i64> = airbases_of_area(c, profile_id, area_id)
        .await?
        .into_iter()
        .filter(|model| model.action == base::Action::Evasion)
        .map(|model| model.rid)
        .collect();

    let mut planes_lost = false;
    for (base, &damage) in bases.iter_mut().zip(&raid.base_damage) {
        if damage < GROUND_LOSS_DAMAGE || sheltered.contains(&base.base_rid) {
            continue;
        }
        let mut owed = rng.roll_range(1, 5);
        for squadron in &mut base.squadrons {
            let taken = owed.min(squadron.count - 1).max(0);
            squadron.count -= taken;
            owed -= taken;
            planes_lost |= taken > 0;
        }
    }
    for base in bases.iter() {
        for squadron in &base.squadrons {
            plane_db::Entity::update_many()
                .col_expr(plane_db::Column::Count, Expr::value(squadron.count))
                .filter(plane_db::Column::ProfileId.eq(profile_id))
                .filter(plane_db::Column::AreaId.eq(area_id))
                .filter(plane_db::Column::Rid.eq(base.base_rid))
                .filter(plane_db::Column::SquadronId.eq(squadron.squadron_id))
                .filter(plane_db::Column::State.eq(plane_db::Status::Assigned))
                .exec(c)
                .await?;
        }
    }

    let damage: i64 = raid.base_damage.iter().sum();
    let mut stores_lost = false;
    if damage > 0 {
        let stock = get_mat_impl(c, profile_id).await?;
        let (category, held) = if rng.roll_range(0, 2) == 0 {
            (MaterialCategory::Fuel, stock.fuel)
        } else {
            (MaterialCategory::Bauxite, stock.bauxite)
        };
        let lost = ((damage as f64 * 0.9 + 0.1).round() as i64).min(held);
        if lost > 0 {
            deduct_material_impl(c, profile_id, &[(category, lost)]).await?;
            stores_lost = true;
        }
    }

    raid.api_lost_kind = match (stores_lost, planes_lost) {
        (true, false) => 1,
        (true, true) => 2,
        (false, true) => 3,
        (false, false) => 4,
    };

    Ok(())
}

/// The profile's air corps in one area, in `rid` order.
async fn airbases_of_area<C>(
    c: &C,
    profile_id: i64,
    area_id: i64,
) -> Result<Vec<base::Model>, GameplayError>
where
    C: ConnectionTrait,
{
    let models = base::Entity::find()
        .filter(base::Column::ProfileId.eq(profile_id))
        .filter(base::Column::AreaId.eq(area_id))
        .order_by_asc(base::Column::Rid)
        .all(c)
        .await?;

    Ok(models)
}

/// Fetch one airbase, rejecting anything the profile does not own.
async fn find_owned_airbase<C>(
    c: &C,
    profile_id: i64,
    area_id: i64,
    rid: i64,
) -> Result<base::Model, GameplayError>
where
    C: ConnectionTrait,
{
    base::Entity::find()
        .filter(base::Column::ProfileId.eq(profile_id))
        .filter(base::Column::AreaId.eq(area_id))
        .filter(base::Column::Rid.eq(rid))
        .one(c)
        .await?
        .ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "profile {profile_id} has no airbase {rid} in area {area_id}"
            ))
        })
}

/// The squadron an equipment currently flies in, if any.
async fn find_squadron_by_slot<C>(
    c: &C,
    profile_id: i64,
    slot_id: i64,
) -> Result<Option<plane_db::Model>, GameplayError>
where
    C: ConnectionTrait,
{
    let model = plane_db::Entity::find_by_id(slot_id)
        .filter(plane_db::Column::ProfileId.eq(profile_id))
        .one(c)
        .await?;

    Ok(model)
}

/// The squadron flying in one slot, if any. A slot may also hold squadrons
/// still relocating out of it; those are not it.
async fn find_assigned_squadron<C>(
    c: &C,
    profile_id: i64,
    area_id: i64,
    rid: i64,
    squadron_id: i64,
) -> Result<Option<plane_db::Model>, GameplayError>
where
    C: ConnectionTrait,
{
    let model = plane_db::Entity::find()
        .filter(plane_db::Column::ProfileId.eq(profile_id))
        .filter(plane_db::Column::AreaId.eq(area_id))
        .filter(plane_db::Column::Rid.eq(rid))
        .filter(plane_db::Column::SquadronId.eq(squadron_id))
        .filter(plane_db::Column::State.eq(plane_db::Status::Assigned))
        .one(c)
        .await?;

    Ok(model)
}

/// Send the squadron flying in one slot into relocation, if there is one.
///
/// Removing a squadron does not empty the slot at once. The live server answers
/// `api_state: 2` with `api_slotid` still set and the radius unchanged, and only
/// later reports `0 / 0` — the 配置転換 the client tracks through
/// `api_port/port`'s `api_base_convert_slot`.
async fn relocate_squadron<C>(
    c: &C,
    profile_id: i64,
    area_id: i64,
    rid: i64,
    squadron_id: i64,
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    plane_db::Entity::update_many()
        .col_expr(plane_db::Column::State, Expr::value(plane_db::Status::Reassigning))
        .col_expr(plane_db::Column::Since, Expr::value(Some(Utc::now())))
        .filter(plane_db::Column::ProfileId.eq(profile_id))
        .filter(plane_db::Column::AreaId.eq(area_id))
        .filter(plane_db::Column::Rid.eq(rid))
        .filter(plane_db::Column::SquadronId.eq(squadron_id))
        .filter(plane_db::Column::State.eq(plane_db::Status::Assigned))
        .exec(c)
        .await?;

    Ok(())
}

/// Release every squadron whose relocation is over, and name the equipment of
/// those still in it.
///
/// A relocation lasts [`relocation_minutes`] of the area's 整備Lv and then
/// needs a visit to the port (wikiwiki 基地航空隊: 12分経過のち一旦母港を経由する),
/// so the port view is the one caller. The live sample fits: the slot answered
/// `api_state: 2` at 18:03 and `0 / 0` at 18:16. A row from before `since`
/// existed has no start and is released at once, as it used to be.
///
/// Returns the equipment still waiting and the equipment released by this call.
pub(crate) async fn settle_relocations_impl<C>(
    c: &C,
    profile_id: i64,
) -> Result<(Vec<i64>, Vec<i64>), GameplayError>
where
    C: ConnectionTrait,
{
    let now = Utc::now();
    let airbases = get_airbases_impl(c, profile_id).await?;
    let relocating = plane_db::Entity::find()
        .filter(plane_db::Column::ProfileId.eq(profile_id))
        .filter(plane_db::Column::State.eq(plane_db::Status::Reassigning))
        .order_by_asc(plane_db::Column::SlotId)
        .all(c)
        .await?;

    let mut waiting = Vec::new();
    let mut released = Vec::new();
    for plane in relocating {
        let level = airbases
            .iter()
            .find(|base| base.area_id == plane.area_id)
            .map_or(0, |base| base.maintenance_level);
        let over = plane
            .since
            .is_none_or(|since| now - since >= Duration::minutes(relocation_minutes(level)));
        if over {
            released.push(plane.slot_id);
            plane.delete(c).await?;
        } else {
            waiting.push(plane.slot_id);
        }
    }

    Ok((waiting, released))
}

/// Re-home a squadron without disturbing its strength or condition.
async fn move_squadron<C>(
    c: &C,
    model: plane_db::Model,
    rid: i64,
    squadron_id: i64,
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    let mut am = model.into_active_model();
    am.rid = ActiveValue::Set(rid);
    am.squadron_id = ActiveValue::Set(squadron_id);
    am.update(c).await?;

    Ok(())
}

/// One airbase with its squadrons and radius filled in.
async fn load_airbase<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
    model: base::Model,
) -> Result<Airbase, GameplayError>
where
    C: ConnectionTrait,
{
    let occupied = get_planes_impl(c, profile_id, model.area_id, model.rid).await?;
    let planes = squadrons_of(profile_id, model.area_id, model.rid, occupied);
    let (base_range, bonus_range) = distance_of(c, codex, &planes).await?;

    Ok(Airbase {
        id: model.id,
        area_id: model.area_id,
        rid: model.rid,
        action: model.action.into(),
        base_range,
        bonus_range,
        name: model.name,
        maintenance_level: model.maintenance_level,
        planes,
    })
}

/// Combat radius of an airbase: `api_base` plus `api_bonus`.
///
/// The base radius is the shortest one among the assigned squadrons — an
/// airbase can only reach as far as its least capable plane — and an empty
/// airbase reaches nowhere. The bonus comes from reconnaissance planes and is
/// not modelled yet.
///
/// ponytail: bonus is a flat 0 until squadron assignment lands and there is
/// something to compute it from; `docs/apilist.txt:2849-2851` defines it as a
/// separate field precisely so it can be added without touching the base.
async fn distance_of<C>(
    c: &C,
    codex: &Codex,
    planes: &[PlaneInfo],
) -> Result<(i64, i64), GameplayError>
where
    C: ConnectionTrait,
{
    let slot_ids: Vec<i64> = planes
        .iter()
        .filter(|p| matches!(p.state, PlaneState::Assigned))
        .map(|p| p.slot_id)
        .collect();
    if slot_ids.is_empty() {
        return Ok((0, 0));
    }

    // `plane_info` keys squadrons by the slot item's instance id, so the
    // manifest radius is two hops away: instance -> mst id -> api_distance.
    let items = find_slot_items_by_id_impl(c, &slot_ids).await?;
    let base = items
        .iter()
        .filter_map(|item| {
            codex
                .manifest
                .find_slotitem(item.mst_id)
                .and_then(|mst| mst.api_distance)
                .filter(|distance| *distance > 0)
        })
        .min()
        .unwrap_or(0);

    Ok((base, 0))
}

/// Give the profile the air corps its unlocked maps entitle it to.
///
/// An area that has any unlocked map declaring `airbase_count` gets its first
/// air corps. Further ones are bought with 設営隊 through
/// `api_req_air_corps/expand_base`, up to `AIRUNIT_MAX` — the map's
/// `airbase_count` caps how many may *sortie* there, not how many are owned.
pub(crate) async fn ensure_airbases_impl<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    let records = get_map_records_impl(c, profile_id).await?;

    for area_id in areas_entitled_to_air_corps(codex, &records) {
        let owned = base::Entity::find()
            .filter(base::Column::ProfileId.eq(profile_id))
            .filter(base::Column::AreaId.eq(area_id))
            .count(c)
            .await?;

        if owned == 0 {
            unlock_airbase_impl(c, profile_id, area_id, 1).await?;
        }
    }

    Ok(())
}

/// Areas whose unlocked maps entitle the profile to an air corps.
fn areas_entitled_to_air_corps(codex: &Codex, records: &[map_record::Model]) -> Vec<i64> {
    let catalog = codex.map_catalog();

    let mut areas: Vec<i64> = records
        .iter()
        .filter(|record| record.unlocked)
        .filter_map(|record| catalog.maps.get(&record.map_id))
        .filter(|definition| definition.airbase_count.unwrap_or(0) > 0)
        .map(|definition| definition.maparea_id)
        .collect();
    areas.sort_unstable();
    areas.dedup();

    areas
}

/// The name upstream gives an air corps: 第一基地航空隊, 第二…, 第三….
fn default_name(rid: i64) -> String {
    let ordinal = match rid {
        1 => "\u{4E00}",
        2 => "\u{4E8C}",
        3 => "\u{4E09}",
        _ => return format!("\u{7B2C}{rid}\u{57FA}\u{5730}\u{822A}\u{7A7A}\u{968A}"),
    };
    format!("\u{7B2C}{ordinal}\u{57FA}\u{5730}\u{822A}\u{7A7A}\u{968A}")
}

pub(crate) async fn unlock_airbase_impl<C>(
    c: &C,
    profile_id: i64,
    area_id: i64,
    rid: i64,
) -> Result<base::Model, GameplayError>
where
    C: ConnectionTrait,
{
    let model = base::Entity::find()
        .filter(base::Column::ProfileId.eq(profile_id))
        .filter(base::Column::AreaId.eq(area_id))
        .filter(base::Column::Rid.eq(rid))
        .one(c)
        .await?;

    if let Some(model) = model {
        return Ok(model);
    }

    let am = base::ActiveModel {
        id: ActiveValue::NotSet,
        profile_id: ActiveValue::Set(profile_id),
        area_id: ActiveValue::Set(area_id),
        rid: ActiveValue::Set(rid),
        action: ActiveValue::Set(base::Action::Idle),
        base_range: ActiveValue::Set(0),
        bonus_range: ActiveValue::Set(0),
        name: ActiveValue::Set(default_name(rid)),
        maintenance_level: ActiveValue::Set(0),
    };

    let m = am.insert(c).await?;

    Ok(m)
}

pub(crate) async fn get_airbases_impl<C>(
    c: &C,
    profile_id: i64,
) -> Result<Vec<base::Model>, GameplayError>
where
    C: ConnectionTrait,
{
    let models = base::Entity::find()
        .filter(base::Column::ProfileId.eq(profile_id))
        .order_by_asc(base::Column::Id)
        .all(c)
        .await?;

    Ok(models)
}

pub(super) async fn init<C>(_c: &C, _profile_id: i64) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    Ok(())
}

pub(super) async fn wipe<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    base::Entity::delete_many().filter(base::Column::ProfileId.eq(profile_id)).exec(c).await?;
    plane_db::Entity::delete_many()
        .filter(plane_db::Column::ProfileId.eq(profile_id))
        .exec(c)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use emukc_db::entity::profile::map_record::SelectedRank;

    use super::*;

    fn record(map_id: i64, unlocked: bool) -> map_record::Model {
        map_record::Model {
            id: map_id,
            profile_id: 1,
            map_id,
            cleared: false,
            last_cleared_at: None,
            last_reset_at: None,
            defeat_count: None,
            current_hp: None,
            gauge_index: 0,
            stage_id: None,
            selected_rank: SelectedRank::NotSet,
            event_state: None,
            unlocked,
        }
    }

    #[tokio::test]
    async fn a_squadron_shot_down_to_nothing_starts_its_proficiency_over() {
        use emukc_db::entity::profile::item::slot_item;

        let db = emukc_db::prelude::new_mem_db().await.unwrap();
        let account = emukc_db::entity::user::account::ActiveModel {
            name: ActiveValue::Set("emptied".into()),
            secret: ActiveValue::Set(String::new()),
            create_time: ActiveValue::Set(Utc::now()),
            last_login: ActiveValue::Set(Utc::now()),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        let pid = emukc_db::entity::profile::default_active_model(account.uid, "emptied")
            .insert(&db)
            .await
            .unwrap()
            .id;
        // Two full-proficiency 一式陸攻 in airbase 6/1: squadron 1 has nothing left, squadron 2 five.
        for squadron_id in 1..=2 {
            let item = slot_item::ActiveModel {
                profile_id: ActiveValue::Set(pid),
                mst_id: ActiveValue::Set(169),
                type3: ActiveValue::Set(47),
                locked: ActiveValue::Set(false),
                level: ActiveValue::Set(0),
                aircraft_lv: ActiveValue::Set(7),
                aircraft_exp: ActiveValue::Set(120),
                equip_on: ActiveValue::Set(0),
                ..Default::default()
            }
            .insert(&db)
            .await
            .unwrap();
            plane_db::ActiveModel {
                slot_id: ActiveValue::Set(item.id),
                profile_id: ActiveValue::Set(pid),
                area_id: ActiveValue::Set(6),
                rid: ActiveValue::Set(1),
                squadron_id: ActiveValue::Set(squadron_id),
                state: ActiveValue::Set(plane_db::Status::Assigned),
                condition: ActiveValue::Set(COND_DEPLOYED),
                count: ActiveValue::Set(if squadron_id == 1 {
                    0
                } else {
                    5
                }),
                max_count: ActiveValue::Set(18),
                since: ActiveValue::Set(None),
            }
            .insert(&db)
            .await
            .unwrap();
        }

        reset_emptied_squadrons_impl(&db, pid, 6, [1]).await.unwrap();

        let levels: Vec<(i64, i64)> = slot_item::Entity::find()
            .order_by_asc(slot_item::Column::Id)
            .all(&db)
            .await
            .unwrap()
            .iter()
            .map(|item| (item.aircraft_lv, item.aircraft_exp))
            .collect();
        assert_eq!(levels, [(0, 0), (7, 120)]);
    }

    /// The figures wikiwiki gives as examples.
    #[test]
    fn a_sortie_costs_what_the_source_says() {
        assert_eq!(sortie_cost(47, 18), (27, 12), "陸攻");
        assert_eq!(sortie_cost(53, 9), (18, 18), "大型陸上機");
        assert_eq!(sortie_cost(9, 4), (4, 3), "偵察機");
        assert_eq!(sortie_cost(48, 18), (18, 11), "the rest");
        assert_eq!(sortie_cost(47, 17), (26, 11), "1.5 rounds up, 0.7 down");
    }

    /// 6-4 and 6-5 are the only regular maps that declare an airbase, and both
    /// sit in area 6 — so the area is entitled once, not once per map.
    #[test]
    fn only_unlocked_airbase_maps_entitle_an_area() {
        let codex =
            emukc_model::codex::Codex::load_without_cache_source("../../.data/codex").unwrap();

        assert_eq!(
            areas_entitled_to_air_corps(&codex, &[record(11, true), record(64, false)]),
            Vec::<i64>::new(),
            "1-1 declares no airbase and 6-4 is still locked"
        );
        assert_eq!(areas_entitled_to_air_corps(&codex, &[record(64, true)]), vec![6]);
        assert_eq!(
            areas_entitled_to_air_corps(&codex, &[record(64, true), record(65, true)]),
            vec![6],
            "two maps in the same area entitle it once"
        );
    }
}
