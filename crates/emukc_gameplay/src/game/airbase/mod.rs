use emukc_db::{
    entity::profile::{
        airbase::{base, plane as plane_db},
        map_record,
    },
    sea_orm::{ActiveValue, QueryOrder, TransactionTrait, entity::prelude::*},
};
use emukc_model::{
    codex::Codex,
    profile::airbase::{Airbase, PlaneInfo},
};

use crate::{err::GameplayError, gameplay::Ctx};

use super::map::get_map_records_impl;
use super::slot_item::find_slot_items_by_id_impl;
use plane::{get_planes_impl, squadrons_of};

mod plane;

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
    let slot_ids: Vec<i64> = planes.iter().map(|p| p.slot_id).filter(|id| *id != 0).collect();
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
