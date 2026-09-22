//! Squadrons belonging to an airbase.

use emukc_db::{
    entity::profile::airbase::plane,
    sea_orm::{QueryOrder, entity::prelude::*},
};
use emukc_model::profile::airbase::{PlaneInfo, PlaneState, SQUADRON_MAX};

use crate::err::GameplayError;

/// Read every occupied squadron of one airbase, ordered by squadron id.
pub(crate) async fn get_planes_impl<C>(
    c: &C,
    profile_id: i64,
    area_id: i64,
    rid: i64,
) -> Result<Vec<plane::Model>, GameplayError>
where
    C: ConnectionTrait,
{
    let models = plane::Entity::find()
        .filter(plane::Column::ProfileId.eq(profile_id))
        .filter(plane::Column::AreaId.eq(area_id))
        .filter(plane::Column::Rid.eq(rid))
        .order_by_asc(plane::Column::SquadronId)
        .all(c)
        .await?;

    Ok(models)
}

/// Fill the gaps so every one of an airbase's `SQUADRON_MAX` slots is reported.
///
/// The `plane_info` table is keyed by the slot item's instance id, so an empty
/// slot has no row to key — an unoccupied squadron is simply absent. The client
/// draws a fixed number of rows and never infers the count from the response,
/// so the missing ones are synthesised here rather than stored.
pub(crate) fn squadrons_of(
    profile_id: i64,
    area_id: i64,
    rid: i64,
    occupied: Vec<plane::Model>,
) -> Vec<PlaneInfo> {
    (1..=SQUADRON_MAX)
        .map(|squadron_id| {
            occupied.iter().find(|m| m.squadron_id == squadron_id).map_or_else(
                || PlaneInfo {
                    id: profile_id,
                    area_id,
                    rid,
                    slot_id: 0,
                    squadron_id,
                    state: PlaneState::Unassigned,
                    condition: 0,
                    count: 0,
                    max_count: 0,
                },
                |m| m.clone().into(),
            )
        })
        .collect()
}
