//! The escort-success cell (船団護衛成功) that is the goal of 1-6.
//!
//! See `docs/solutions/architecture-patterns/reconnaissance-and-escort-cells.md`.

use emukc_crypto::rng;
use emukc_db::sea_orm::{ActiveModelTrait, ActiveValue, ConnectionTrait, IntoActiveModel};
use emukc_model::codex::{
    Codex,
    map::{CellItem, MapDefinition, MapStageDefinition},
};
use emukc_time::chrono::Utc;

use crate::{
    err::GameplayError,
    game::{
        basic::{add_war_result_impl, eo_war_result_counts},
        map::{check_and_unlock_dependencies_impl, find_map_record_impl, monthly_reset_due},
        use_item::add_use_item_impl,
    },
};

use super::{SortieEscortSuccess, SortieItemGet};

/// `api_event_id` of an escort-success cell.
pub(super) const EVENT_ID: i64 = 8;

/// プレゼント箱, given with the arrival that empties the gauge.
const PRESENT_BOX: i64 = 60;
/// The ranking points (戦果) of that arrival, kept with the month's until 22:00 JST on its
/// last day.
const RANK_POINTS: i64 = 75;
/// The fuel of that arrival.
const LAST_ARRIVAL_FUEL: i64 = 1000;

const BASIC: [CellItem; 4] = [CellItem::Fuel, CellItem::Ammo, CellItem::Steel, CellItem::Bauxite];

/// Settle the fleet's arrival at an escort-success cell: one more of the arrivals that
/// empty the map's gauge, and a resource. The resource waits for the port; the caller
/// puts it by.
///
/// wikiwiki 鎮守府海域 1-6: 「100単位の資材」, 「多いものでは1000」, 燃料 1000 with the
/// arrival that empties the gauge, and 「大幅に減少」 after it.
// ponytail: which resource and how much are drawn evenly, by the hundred up to 1000 while
// the gauge stands and by the ten up to 100 afterwards; the source has no distribution.
pub(super) async fn arrive<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
    definition: &MapDefinition,
    stage: &MapStageDefinition,
) -> Result<SortieEscortSuccess, GameplayError>
where
    C: ConnectionTrait,
{
    let now = Utc::now();
    let gain = |unit| SortieItemGet {
        item: BASIC[rng::usize(0..BASIC.len())],
        amount: unit * rng::i64_inclusive(1..=10),
    };
    let plain = |gain| SortieEscortSuccess {
        gain,
        bonus_use_item: None,
        rank_points: 0,
    };
    let record = find_map_record_impl(c, profile_id, definition.map_id).await?;
    let required = stage.required_defeat_count.or(definition.required_defeat_count);
    let (false, Some(required)) = (record.cleared, required) else {
        return Ok(plain(gain(10)));
    };

    let arrivals = record.defeat_count.unwrap_or_default() + 1;
    let last_fuel = SortieItemGet {
        item: CellItem::Fuel,
        amount: LAST_ARRIVAL_FUEL,
    };
    // Last month's gauge, the sortie having left before 00:00 JST on the 1st: nothing of it
    // is kept, and the arrival that would empty it brings its fuel alone
    // (sakamoto-kouji.com 2019/08/04, tried on 2020-03-01).
    if monthly_reset_due(record.last_reset_at, now) {
        return Ok(plain(if arrivals < required {
            gain(100)
        } else {
            last_fuel
        }));
    }
    let mut am = record.into_active_model();
    am.defeat_count = ActiveValue::Set(Some(arrivals.min(required)));
    if arrivals < required {
        am.update(c).await?;
        return Ok(plain(gain(100)));
    }
    am.cleared = ActiveValue::Set(true);
    am.last_cleared_at = ActiveValue::Set(Some(now));
    am.update(c).await?;
    // As any map cleared for the first time: what waited on it opens.
    check_and_unlock_dependencies_impl(c, codex, profile_id, definition.map_id).await?;
    add_use_item_impl(c, profile_id, PRESENT_BOX, 1).await?;
    // ponytail: from 22:00 the 75 is still sent; what the game shows then is unknown.
    if eo_war_result_counts(now) {
        add_war_result_impl(c, profile_id, RANK_POINTS).await?;
    }
    Ok(SortieEscortSuccess {
        gain: last_fuel,
        bonus_use_item: Some((PRESENT_BOX, 1)),
        rank_points: RANK_POINTS,
    })
}
