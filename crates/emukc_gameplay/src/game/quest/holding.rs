//! The equipment a quest takes at its claim: whether it is in hand, and taking it.
//!
//! Two kinds of condition name equipment. A model conversion wants it on the
//! flagship of the first fleet; a 準備 condition wants it lying unequipped.
//! Both are read against the player's state at the moment of asking, like
//! composition conditions, and never through counters.

use std::collections::BTreeSet;

use emukc_db::{
    entity::profile::{
        item::slot_item,
        quest::progress,
        ship::{self},
    },
    sea_orm::{ActiveModelTrait, ActiveValue, IntoActiveModel, entity::prelude::*},
};
use emukc_model::{
    codex::Codex,
    kc2::start2::ApiMstSlotitem,
    thirdparty::{
        Kc3rdQuestCondition, Kc3rdQuestConditionConsumption, Kc3rdQuestConditionShip,
        Kc3rdQuestConditionSlotItem, Kc3rdQuestConditionSlotItemType, Kc3rdQuestRequirement,
        matcher::ship_matches_mst_id,
    },
};

use crate::{
    err::GameplayError,
    game::{
        fleet::find_fleet,
        picturebook::add_slot_item_to_picturebook_impl,
        ship::recalculate_ship_status_with_model,
        slot_item::{find_slot_items_by_id_impl, get_unset_slot_items_impl},
    },
};

/// 熟練度 `>>`.
const AIRCRAFT_LV_MAX: i64 = 7;

/// What a quest's equipment conditions would take, resolved to instances.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Taken {
    /// Instances on the flagship.
    pub equipped: Vec<i64>,
    /// Instances lying unequipped.
    pub loose: Vec<i64>,
    /// The 改修 level a conversion carries over to its reward.
    pub kept_stars: Option<i64>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Holding {
    /// Everything is in hand.
    Held(Taken),
    /// Something is not there.
    Missing,
    /// Everything is there, but a piece on the flagship is locked. The client
    /// then shows the quest as done and refuses the click (`api_invalid_flag`).
    Locked,
}

/// The part of a profile the equipment conditions look at.
pub(super) struct Snapshot {
    flagship: Option<ship::Model>,
    /// The flagship's equipment with its slot, 1 to 5, and 6 for the extra slot.
    equipped: Vec<(i64, slot_item::Model)>,
    /// Unlocked equipment nothing holds, cheapest to give up first.
    loose: Vec<slot_item::Model>,
}

pub(super) async fn load_snapshot<C>(c: &C, profile_id: i64) -> Result<Snapshot, GameplayError>
where
    C: ConnectionTrait,
{
    let flagship_id = find_fleet(c, profile_id, 1).await?.ship_1;
    let flagship = if flagship_id > 0 {
        ship::Entity::find_by_id(flagship_id).one(c).await?
    } else {
        None
    };

    let mut equipped = Vec::new();
    if let Some(ship) = &flagship {
        let slots = slots_of(ship);
        for item in find_slot_items_by_id_impl(c, &slots).await? {
            if let Some(idx) = slots.iter().position(|id| *id == item.id) {
                equipped.push((idx as i64 + 1, item));
            }
        }
        equipped.sort_by_key(|(slot, _)| *slot);
    }

    // A locked piece is never handed in (wikiwiki 任務: 低改修値のもの優先).
    let mut loose = get_unset_slot_items_impl(c, profile_id).await?;
    loose.retain(|item| !item.locked);
    loose.sort_by_key(|item| (item.level, item.aircraft_lv, item.id));

    Ok(Snapshot {
        flagship,
        equipped,
        loose,
    })
}

fn slots_of(ship: &ship::Model) -> [i64; 6] {
    [ship.slot_1, ship.slot_2, ship.slot_3, ship.slot_4, ship.slot_5, ship.slot_ex]
}

fn fits(item: &slot_item::Model, wanted: &Kc3rdQuestConditionSlotItem) -> bool {
    let Kc3rdQuestConditionSlotItemType::Equipment(ids) = &wanted.item_type else {
        // No quest names a type here; the data would have to change first.
        return false;
    };
    ids.contains(&item.mst_id)
        && item.level >= wanted.stars
        && (!wanted.fully_skilled || item.aircraft_lv >= AIRCRAFT_LV_MAX)
}

/// A named ship stands for itself and its later remodels: the data lists 鳳翔
/// where 鳳翔改 serves as well, and 伊勢改二 where 伊勢 does not.
fn secretary_matches(codex: &Codex, cond: &Kc3rdQuestConditionShip, mst_id: i64) -> bool {
    match cond {
        Kc3rdQuestConditionShip::Ship(ids) => ids.iter().any(|id| {
            *id == mst_id || codex.ship_and_after(*id).is_ok_and(|v| v.contains(&mst_id))
        }),
        _ => ship_matches_mst_id(cond, Some(codex), mst_id),
    }
}

/// Resolve the equipment conditions among `conditions`. `None` when there are none.
pub(super) fn holding(
    codex: &Codex,
    snapshot: &Snapshot,
    conditions: &[Kc3rdQuestCondition],
) -> Option<Holding> {
    let mut taken = Taken::default();
    let mut used: BTreeSet<i64> = BTreeSet::new();
    let mut any = false;
    let mut missing = false;
    let mut locked = false;

    for condition in conditions {
        match condition {
            Kc3rdQuestCondition::ModelConversion(conversion) => {
                any = true;
                let mst_id = snapshot.flagship.as_ref().map(|ship| ship.mst_id);
                let is = |cond: &Kc3rdQuestConditionShip| {
                    mst_id.is_some_and(|id| secretary_matches(codex, cond, id))
                };
                let wanted = conversion.secretary.as_ref().is_none_or(is);
                let banned = conversion.banned_secretary.as_ref().is_some_and(is);
                if !wanted || banned {
                    missing = true;
                }

                for slot in conversion.slots.iter().flatten() {
                    // `pos` counts slots from 1; 0 is any slot of the flagship.
                    let mut candidates = snapshot.equipped.iter().filter(|(pos, item)| {
                        (slot.pos == 0 || slot.pos == *pos)
                            && !used.contains(&item.id)
                            && fits(item, &slot.item)
                    });
                    let free = candidates.clone().find(|(_, item)| !item.locked);
                    match free.or_else(|| candidates.next()) {
                        Some((_, item)) => {
                            used.insert(item.id);
                            locked |= item.locked;
                            taken.equipped.push(item.id);
                            if slot.keep_stars {
                                taken.kept_stars = Some(item.level);
                            }
                        }
                        None => missing = true,
                    }
                }
            }
            Kc3rdQuestCondition::Consumption(
                Kc3rdQuestConditionConsumption::SlotItemConsumption(items),
            ) => {
                any = true;
                for wanted in items {
                    let found: Vec<i64> = snapshot
                        .loose
                        .iter()
                        .filter(|item| !used.contains(&item.id) && fits(item, wanted))
                        .take(wanted.amount.max(0) as usize)
                        .map(|item| item.id)
                        .collect();
                    if (found.len() as i64) < wanted.amount {
                        missing = true;
                    }
                    used.extend(&found);
                    taken.loose.extend(found);
                }
            }
            _ => {}
        }
    }

    if !any {
        return None;
    }
    Some(if missing {
        Holding::Missing
    } else if locked {
        Holding::Locked
    } else {
        Holding::Held(taken)
    })
}

pub(super) fn conditions_of(requirements: &Kc3rdQuestRequirement) -> &[Kc3rdQuestCondition] {
    match requirements {
        Kc3rdQuestRequirement::And(conditions)
        | Kc3rdQuestRequirement::OneOf(conditions)
        | Kc3rdQuestRequirement::Sequential(conditions) => conditions,
    }
}

/// Hold back the quests whose counters are done but whose equipment is not in
/// hand, and let them through again once it is.
///
/// Counters alone carry such a quest to `Completed` (`is_satisfied` counts these
/// conditions as met), so this runs after every other writer, when the quest
/// list is read. A quest held back shows 80% (wikiwiki 任務: 達成率80%となり
/// 再廃棄の必要はない). One with only a lock in the way stays done: the
/// client shows it as such and blocks the click itself.
pub(super) async fn validate_equipment_quests<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    let quests = progress::Entity::find()
        .filter(progress::Column::ProfileId.eq(profile_id))
        .filter(progress::Column::Status.eq(progress::Status::Activated))
        .filter(
            progress::Column::Progress
                .is_in([progress::Progress::Completed, progress::Progress::Eighty]),
        )
        .all(c)
        .await?;

    let mut snapshot = None;
    for quest in quests {
        let Some(mst) = codex.quest.get(&quest.quest_id) else {
            continue;
        };
        let conditions = conditions_of(&mst.requirements);
        if !conditions.iter().any(names_equipment) {
            continue;
        }
        if snapshot.is_none() {
            snapshot = Some(load_snapshot(c, profile_id).await?);
        }
        let Some(snapshot) = snapshot.as_ref() else {
            continue;
        };
        let in_hand = !matches!(holding(codex, snapshot, conditions), Some(Holding::Missing));

        let next = match (quest.progress, in_hand) {
            (progress::Progress::Completed, false) => progress::Progress::Eighty,
            (progress::Progress::Eighty, true) => {
                let stored: Vec<Kc3rdQuestCondition> =
                    serde_json::from_value(quest.requirements.clone())?;
                // A composition condition is never met by counters; its own
                // validation decides for those quests.
                if stored.iter().all(Kc3rdQuestCondition::is_satisfied) {
                    progress::Progress::Completed
                } else {
                    continue;
                }
            }
            _ => continue,
        };
        let mut am = quest.into_active_model();
        am.progress = ActiveValue::Set(next);
        am.update(c).await?;
    }

    Ok(())
}

fn names_equipment(condition: &Kc3rdQuestCondition) -> bool {
    matches!(
        condition,
        Kc3rdQuestCondition::ModelConversion(_)
            | Kc3rdQuestCondition::Consumption(
                Kc3rdQuestConditionConsumption::SlotItemConsumption(_)
            )
    )
}

/// Take the equipment away. Nothing is refunded and nothing counts as scrapped:
/// handing equipment to a quest is not 廃棄.
///
/// With `convert_to` (a model and its 改修 level), the first piece taken from
/// the flagship is not removed but becomes that model where it sits. The client
/// re-reads equipment after a conversion and not the ship
/// (`main.decoded.js:104036`), so the instance must survive in its slot.
/// Returns whether a piece was converted.
pub(super) async fn take<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
    taken: &Taken,
    convert_to: Option<(i64, i64)>,
) -> Result<bool, GameplayError>
where
    C: ConnectionTrait,
{
    let converted = convert_to.zip(taken.equipped.first().copied());
    if let Some(((mst_id, stars), item_id)) = converted {
        let mst = codex.find::<ApiMstSlotitem>(&mst_id)?;
        let mut am = slot_item::ActiveModel {
            id: ActiveValue::Unchanged(item_id),
            ..Default::default()
        };
        am.mst_id = ActiveValue::Set(mst_id);
        am.type3 = ActiveValue::Set(mst.api_type[2]);
        am.level = ActiveValue::Set(stars);
        am.aircraft_lv = ActiveValue::Set(0);
        am.update(c).await?;
        add_slot_item_to_picturebook_impl(c, profile_id, mst.api_sortno).await?;
    }
    let gone = |id: &i64| converted.is_none_or(|(_, kept)| kept != *id);

    if !taken.equipped.is_empty() {
        let flagship_id = find_fleet(c, profile_id, 1).await?.ship_1;
        let mut ship = ship::Entity::find_by_id(flagship_id).one(c).await?.ok_or_else(|| {
            GameplayError::EntryNotFound(format!("ship with id {flagship_id} not found"))
        })?;
        for slot in [
            &mut ship.slot_1,
            &mut ship.slot_2,
            &mut ship.slot_3,
            &mut ship.slot_4,
            &mut ship.slot_5,
            &mut ship.slot_ex,
        ] {
            if taken.equipped.contains(slot) && gone(slot) {
                *slot = -1;
            }
        }
        recalculate_ship_status_with_model(c, codex, &ship).await?.update(c).await?;
    }

    let ids: Vec<i64> = taken.equipped.iter().chain(&taken.loose).copied().filter(gone).collect();
    if !ids.is_empty() {
        slot_item::Entity::delete_many()
            .filter(slot_item::Column::ProfileId.eq(profile_id))
            .filter(slot_item::Column::Id.is_in(ids))
            .exec(c)
            .await?;
    }

    Ok(converted.is_some())
}

/// Of `quest_ids`, the ones whose equipment is all there with a locked piece on
/// the flagship: the quests the client must refuse to hand in.
pub(crate) async fn quests_behind_a_lock_impl<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
    quest_ids: &[i64],
) -> Result<Vec<i64>, GameplayError>
where
    C: ConnectionTrait,
{
    let named: Vec<(i64, &[Kc3rdQuestCondition])> = quest_ids
        .iter()
        .filter_map(|id| Some((*id, conditions_of(&codex.quest.get(id)?.requirements))))
        .filter(|(_, conditions)| conditions.iter().any(names_equipment))
        .collect();
    if named.is_empty() {
        return Ok(Vec::new());
    }

    let snapshot = load_snapshot(c, profile_id).await?;
    Ok(named
        .into_iter()
        .filter(|(_, conditions)| holding(codex, &snapshot, conditions) == Some(Holding::Locked))
        .map(|(id, _)| id)
        .collect())
}
