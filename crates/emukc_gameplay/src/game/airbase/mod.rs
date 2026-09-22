use emukc_db::{
    entity::profile::{
        airbase::{base, plane as plane_db},
        map_record,
    },
    sea_orm::{ActiveValue, IntoActiveModel, QueryOrder, TransactionTrait, entity::prelude::*},
};
use emukc_model::{
    codex::Codex,
    profile::airbase::{Airbase, PlaneInfo, PlaneState, SQUADRON_MAX, squadron_capacity},
};

use crate::{err::GameplayError, gameplay::Ctx};

use super::map::get_map_records_impl;
use super::slot_item::{find_slot_item_impl, find_slot_items_by_id_impl};
use plane::{get_planes_impl, squadrons_of};

mod plane;

/// Outcome of assigning or removing a squadron.
#[derive(Debug, Clone)]
pub struct SetPlaneResult {
    /// `(api_base, api_bonus)` of the airbase after the change.
    pub distance: (i64, i64),
    /// Only the slots this call touched.
    pub updated: Vec<PlaneInfo>,
    /// Bauxite left, when the call spent any.
    pub after_bauxite: Option<i64>,
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
        settle_relocations_impl(&tx, profile_id).await?;

        let models = get_airbases_impl(&tx, profile_id).await?;

        let mut airbases = Vec::with_capacity(models.len());
        for model in models {
            let occupied = get_planes_impl(&tx, profile_id, model.area_id, model.rid).await?;
            let planes = squadrons_of(profile_id, model.area_id, model.rid, occupied);
            let (base_range, bonus_range) = distance_of(&tx, codex, &planes).await?;

            airbases.push(Airbase {
                id: model.id,
                area_id: model.area_id,
                rid: model.rid,
                action: model.action.into(),
                base_range,
                bonus_range,
                name: model.name,
                maintenance_level: model.maintenance_level,
                planes,
            });
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
    /// ponytail: assignment tops the squadron up for free. Upstream spends
    /// bauxite here (`api_after_bauxite` exists on the response), but the cost
    /// per aircraft has no published source and belongs with the resupply cost
    /// the plan defers to U4 — so this returns `after_bauxite: None`, which the
    /// client reads as "nothing was spent". Wire both to the same figure once
    /// U4 settles it.
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

        let mut touched = vec![squadron_id];

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

            // Already flying somewhere. Within this airbase it is a move and
            // both slots are reported; anywhere else the client should have
            // called change_deployment_base instead.
            if let Some(current) = find_squadron_by_slot(&tx, profile_id, item_id).await? {
                let here = current.area_id == area_id && current.rid == base_id;
                // A relocating squadron flies for nobody, so it may land
                // anywhere; one still assigned belongs to its airbase.
                if !here && current.state != plane_db::Status::Reassigning {
                    return Err(GameplayError::WrongType(format!(
                        "slot item {item_id} is deployed to airbase {}/{}",
                        current.area_id, current.rid
                    )));
                }
                if here && current.squadron_id != squadron_id {
                    touched.push(current.squadron_id);
                }
                current.delete(&tx).await?;
            }

            clear_squadron(&tx, profile_id, area_id, base_id, squadron_id).await?;

            let am = plane_db::ActiveModel {
                slot_id: ActiveValue::Set(item_id),
                profile_id: ActiveValue::Set(profile_id),
                area_id: ActiveValue::Set(area_id),
                rid: ActiveValue::Set(base_id),
                squadron_id: ActiveValue::Set(squadron_id),
                state: ActiveValue::Set(plane_db::Status::Assigned),
                condition: ActiveValue::Set(1),
                count: ActiveValue::Set(capacity),
                max_count: ActiveValue::Set(capacity),
            };
            am.insert(&tx).await?;
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
            after_bauxite: None,
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

        let incoming = find_squadron_by_slot(&tx, profile_id, item_id).await?.ok_or_else(|| {
            GameplayError::EntryNotFound(format!("slot item {item_id} flies for no airbase"))
        })?;
        if incoming.area_id != area_id || incoming.rid != base_id_src {
            return Err(GameplayError::WrongType(format!(
                "slot item {item_id} flies for airbase {}/{}, not {area_id}/{base_id_src}",
                incoming.area_id, incoming.rid
            )));
        }

        let outgoing = plane_db::Entity::find()
            .filter(plane_db::Column::ProfileId.eq(profile_id))
            .filter(plane_db::Column::AreaId.eq(area_id))
            .filter(plane_db::Column::Rid.eq(base_id))
            .filter(plane_db::Column::SquadronId.eq(squadron_id))
            .one(&tx)
            .await?;

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

/// Empty one slot, if anything is in it.
async fn clear_squadron<C>(
    c: &C,
    profile_id: i64,
    area_id: i64,
    rid: i64,
    squadron_id: i64,
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    plane_db::Entity::delete_many()
        .filter(plane_db::Column::ProfileId.eq(profile_id))
        .filter(plane_db::Column::AreaId.eq(area_id))
        .filter(plane_db::Column::Rid.eq(rid))
        .filter(plane_db::Column::SquadronId.eq(squadron_id))
        .exec(c)
        .await?;

    Ok(())
}

/// Send one slot's squadron into relocation, if anything is in it.
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
        .filter(plane_db::Column::ProfileId.eq(profile_id))
        .filter(plane_db::Column::AreaId.eq(area_id))
        .filter(plane_db::Column::Rid.eq(rid))
        .filter(plane_db::Column::SquadronId.eq(squadron_id))
        .exec(c)
        .await?;

    Ok(())
}

/// Let every finished relocation empty its slot.
///
/// ponytail: settles on the next airbase read rather than on a clock. The one
/// live measurement only bounds the real delay — the slot answered `api_state: 2`
/// at 18:03 and `0 / 0` at 18:16 — and nothing upstream publishes the duration,
/// so a timer would be an invented number. Reading the airbases is what the
/// client does when it opens the 出撃 menu, which is exactly where the sample
/// saw the slot already empty. Swap in a real cooldown once the duration has a
/// source; that needs a timestamp column on `plane_info`.
async fn settle_relocations_impl<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    plane_db::Entity::delete_many()
        .filter(plane_db::Column::ProfileId.eq(profile_id))
        .filter(plane_db::Column::State.eq(plane_db::Status::Reassigning))
        .exec(c)
        .await?;

    Ok(())
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
        name: ActiveValue::Set(format!("\u{7B2C}{rid}\u{57FA}\u{5730}\u{822A}\u{7A7A}\u{968A}")),
        maintenance_level: ActiveValue::Set(1),
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
