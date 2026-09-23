use std::collections::BTreeMap;

use emukc_db::{
    entity::profile::{airbase::plane as plane_db, item::slot_item},
    sea_orm::{ActiveValue, TransactionTrait, TryIntoModel, entity::prelude::*},
};
use emukc_model::{prelude::*, profile::slot_item::SlotItem};

use crate::{
    err::GameplayError,
    game::{
        material::add_material_impl,
        quest::observe::{GameplayOutcome, observe},
    },
    gameplay::Ctx,
};

use super::airbase::settle_relocations_impl;
use super::picturebook::add_slot_item_to_picturebook_impl;

/// ドラム缶(輸送用). Expeditions count the canisters, sortie routing counts the
/// ships carrying one; both key on this master id.
pub(crate) const DRUM_CANISTER_MST_ID: i64 = 75;

/// What holds a piece of equipment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlotItemOccupant {
    /// Fitted to this ship.
    Ship(i64),
    /// Flown by a squadron of this airbase.
    Airbase {
        area_id: i64,
        rid: i64,
    },
}

impl std::fmt::Display for SlotItemOccupant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ship(ship_id) => write!(f, "equipped on ship {ship_id}"),
            Self::Airbase {
                area_id,
                rid,
            } => write!(f, "deployed to airbase {area_id}/{rid}"),
        }
    }
}

/// What holds each of `item_ids`; an item missing from the map is free.
///
/// A squadron that has finished relocating no longer holds its plane, so the
/// profile's relocations settle first — the same read-time rule the airbases
/// follow, without which a released plane would stay locked until the player
/// next opened the sortie menu.
pub(crate) async fn slot_item_occupants_impl<C>(
    c: &C,
    profile_id: i64,
    item_ids: &[i64],
) -> Result<BTreeMap<i64, SlotItemOccupant>, GameplayError>
where
    C: ConnectionTrait,
{
    settle_relocations_impl(c, profile_id).await?;

    let mut occupants = BTreeMap::new();
    if item_ids.is_empty() {
        return Ok(occupants);
    }
    let on_ships = slot_item::Entity::find()
        .filter(slot_item::Column::ProfileId.eq(profile_id))
        .filter(slot_item::Column::Id.is_in(item_ids.to_owned()))
        .filter(slot_item::Column::EquipOn.gt(0))
        .all(c)
        .await?;
    for item in on_ships {
        occupants.insert(item.id, SlotItemOccupant::Ship(item.equip_on));
    }
    let in_squadrons = plane_db::Entity::find()
        .filter(plane_db::Column::ProfileId.eq(profile_id))
        .filter(plane_db::Column::SlotId.is_in(item_ids.to_owned()))
        .all(c)
        .await?;
    for plane in in_squadrons {
        occupants.insert(
            plane.slot_id,
            SlotItemOccupant::Airbase {
                area_id: plane.area_id,
                rid: plane.rid,
            },
        );
    }

    Ok(occupants)
}

/// Reject when any of `item_ids` is held by a ship or a squadron.
pub(crate) async fn ensure_slot_items_free_impl<C>(
    c: &C,
    profile_id: i64,
    item_ids: &[i64],
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    match slot_item_occupants_impl(c, profile_id, item_ids).await?.into_iter().next() {
        Some((item_id, occupant)) => {
            Err(GameplayError::WrongType(format!("slot item {item_id} is {occupant}")))
        }
        None => Ok(()),
    }
}

/// Keep only the items nothing holds.
pub(crate) async fn retain_free_slot_items_impl<C>(
    c: &C,
    profile_id: i64,
    items: &mut Vec<slot_item::Model>,
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    let ids = items.iter().map(|item| item.id).collect::<Vec<_>>();
    let occupants = slot_item_occupants_impl(c, profile_id, &ids).await?;
    items.retain(|item| !occupants.contains_key(&item.id));
    Ok(())
}

impl Ctx {
    /// Add slot item to a profile.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `mst_id`: The slot item manifest ID.
    /// - `stars`: The stars of the item.
    /// - `alv`: The aircraft level of the item.
    pub async fn add_slot_item(
        &self,
        profile_id: i64,
        mst_id: i64,
        stars: i64,
        alv: i64,
    ) -> Result<KcApiSlotItem, GameplayError> {
        let codex = self.codex.as_ref();

        let db = self.db.as_ref();
        let tx = db.begin().await?;

        let m = add_slot_item_impl(&tx, codex, profile_id, mst_id, stars, alv).await?;

        tx.commit().await?;

        Ok(KcApiSlotItem {
            api_id: m.id,
            api_slotitem_id: mst_id,
            api_locked: 0,
            api_level: stars,
            api_alv: (alv > 0).then_some(alv),
        })
    }

    /// Find slot item from a profile.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `id`: The slot item instance ID.
    pub async fn find_slot_item(&self, id: i64) -> Result<KcApiSlotItem, GameplayError> {
        let db = self.db.as_ref();
        let m = find_slot_item_impl(db, id).await?;
        let slot_item: SlotItem = m.into();

        Ok(slot_item.into())
    }

    /// Get all slot items from a profile.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    pub async fn get_slot_items(
        &self,
        profile_id: i64,
    ) -> Result<Vec<KcApiSlotItem>, GameplayError> {
        let db = self.db.as_ref();
        let ms = get_slot_items_impl(db, profile_id).await?;

        let slot_items: Vec<SlotItem> = ms.into_iter().map(std::convert::Into::into).collect();
        let slot_items: Vec<KcApiSlotItem> =
            slot_items.into_iter().map(std::convert::Into::into).collect();

        Ok(slot_items)
    }

    /// Update slot item.
    ///
    /// # Parameters
    ///
    /// - `id`: The slot item instance ID.
    /// - `stars`: The stars of the item.
    /// - `alv`: The aircraft level of the item.
    /// - `equip_on`: The ship instance ID the item is equipped on.
    pub async fn update_slot_item(
        &self,
        id: i64,
        stars: Option<i64>,
        alv: Option<i64>,
        equip_on: Option<i64>,
    ) -> Result<KcApiSlotItem, GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        let m = update_slot_item_impl(&tx, id, stars, alv, equip_on).await?;

        tx.commit().await?;

        Ok(KcApiSlotItem {
            api_id: m.id,
            api_slotitem_id: m.mst_id,
            api_locked: m.locked as i64,
            api_level: m.level,
            api_alv: (m.aircraft_lv > 0).then_some(m.aircraft_lv),
        })
    }

    /// Get all unset slot items from a profile.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    pub async fn get_unset_slot_items(
        &self,
        profile_id: i64,
    ) -> Result<Vec<KcApiSlotItem>, GameplayError> {
        let db = self.db.as_ref();
        let ms = get_unset_slot_items_impl(db, profile_id).await?;

        let slot_items: Vec<SlotItem> = ms.into_iter().map(std::convert::Into::into).collect();
        let slot_items: Vec<KcApiSlotItem> =
            slot_items.into_iter().map(std::convert::Into::into).collect();

        Ok(slot_items)
    }

    /// Get unset slot items by types.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `type3`: The item types.
    pub async fn get_unset_slot_items_by_types(
        &self,
        profile_id: i64,
        type3: &[i64],
    ) -> Result<BTreeMap<i64, Vec<i64>>, GameplayError> {
        let db = self.db.as_ref();

        let item_ids = get_unset_slot_items_by_types_impl(db, profile_id, type3).await?;

        Ok(item_ids)
    }

    /// Toggle slot item locked status.
    ///
    /// for now (5.9.4.0) this can only lock or unlock the item that is not equipped on any ship.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID (for ownership verification).
    /// - `item_id`: The slot item instance ID.
    pub async fn toggle_slot_item_locked(
        &self,
        profile_id: i64,
        item_id: i64,
    ) -> Result<KcApiSlotItem, GameplayError> {
        let codex = self.codex.as_ref();
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        let m = toggle_slot_item_locked_impl(&tx, profile_id, item_id).await?;

        // If this item is equipped on a ship, recalculate that ship's has_locked_euqip
        if m.equip_on > 0 {
            let result = crate::game::ship::find_ship_impl(&tx, m.equip_on).await?;
            if let Some((ship, _)) = result {
                crate::game::ship::recalculate_ship_status_with_model(&tx, codex, &ship)
                    .await?
                    .update(&tx)
                    .await?;
            }
        }

        let m: SlotItem = m.into();

        tx.commit().await?;

        Ok(m.into())
    }

    /// Destroy slot items.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `item_ids`: The slot item instance IDs.
    pub async fn destroy_items(
        &self,
        profile_id: i64,
        item_ids: &[i64],
    ) -> Result<Vec<(MaterialCategory, i64)>, GameplayError> {
        let codex = self.codex.as_ref();
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        // Checked here and not in `destroy_items_impl`: scrapping a ship goes
        // through that `_impl` on purpose, taking its own equipment with it.
        ensure_slot_items_free_impl(&tx, profile_id, item_ids).await?;
        let (scrapped_materials, outcomes) =
            destroy_items_impl(&tx, codex, profile_id, item_ids).await?;

        observe(&tx, codex, profile_id, &outcomes).await?;

        tx.commit().await?;

        Ok(scrapped_materials)
    }
}

/// Add slot item to a profile.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
/// - `mst_id`: The item master ID.
/// - `sortno`: The item's sort number.
/// - `stars`: The stars of the item.
/// - `alv`: The aircraft level of the item.
pub async fn add_slot_item_impl<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
    mst_id: i64,
    stars: i64,
    alv: i64,
) -> Result<slot_item::Model, GameplayError>
where
    C: ConnectionTrait,
{
    let mst = codex.find::<ApiMstSlotitem>(&mst_id)?;
    let am = slot_item::ActiveModel {
        id: ActiveValue::NotSet,
        profile_id: ActiveValue::Set(profile_id),
        mst_id: ActiveValue::Set(mst_id),
        type3: ActiveValue::Set(mst.api_type[2]),
        locked: ActiveValue::Set(false),
        level: ActiveValue::Set(stars),
        aircraft_lv: ActiveValue::Set(alv),
        equip_on: ActiveValue::Set(0),
    };

    let model = am.save(c).await?;

    // add slot item to picture book
    add_slot_item_to_picturebook_impl(c, profile_id, mst.api_sortno).await?;

    Ok(model.try_into_model()?)
}

pub async fn find_slot_item_impl<C>(c: &C, id: i64) -> Result<slot_item::Model, GameplayError>
where
    C: ConnectionTrait,
{
    let record = slot_item::Entity::find_by_id(id)
        .one(c)
        .await?
        .ok_or_else(|| GameplayError::EntryNotFound(format!("slot item {id} not found")))?;

    Ok(record)
}

pub async fn find_slot_items_by_id_impl<C>(
    c: &C,
    ids: &[i64],
) -> Result<Vec<slot_item::Model>, GameplayError>
where
    C: ConnectionTrait,
{
    let records = slot_item::Entity::find()
        .filter(slot_item::Column::Id.is_in(ids.to_owned()))
        .all(c)
        .await?;

    Ok(records)
}

pub(crate) async fn update_slot_item_impl<C>(
    c: &C,
    id: i64,
    stars: Option<i64>,
    alv: Option<i64>,
    equip_on: Option<i64>,
) -> Result<slot_item::Model, GameplayError>
where
    C: ConnectionTrait,
{
    let model = slot_item::Entity::find()
        .filter(slot_item::Column::Id.eq(id))
        .one(c)
        .await?
        .ok_or_else(|| GameplayError::EntryNotFound(format!("slot item {id} not found")))?;

    let mut am: slot_item::ActiveModel = model.into();

    if let Some(stars) = stars {
        am.level = ActiveValue::Set(stars);
    }

    if let Some(alv) = alv {
        am.aircraft_lv = ActiveValue::Set(alv);
    }

    if let Some(equip_on) = equip_on {
        am.equip_on = ActiveValue::Set(equip_on);
    }

    let m = am.save(c).await?;

    Ok(m.try_into_model()?)
}

pub(crate) async fn get_slot_items_impl<C>(
    c: &C,
    profile_id: i64,
) -> Result<Vec<slot_item::Model>, GameplayError>
where
    C: ConnectionTrait,
{
    let records = slot_item::Entity::find()
        .filter(slot_item::Column::ProfileId.eq(profile_id))
        .all(c)
        .await?;

    Ok(records)
}

pub(crate) async fn get_unset_slot_items_impl<C>(
    c: &C,
    profile_id: i64,
) -> Result<Vec<slot_item::Model>, GameplayError>
where
    C: ConnectionTrait,
{
    let mut records = slot_item::Entity::find()
        .filter(slot_item::Column::ProfileId.eq(profile_id))
        .filter(slot_item::Column::EquipOn.lte(0))
        .all(c)
        .await?;
    retain_free_slot_items_impl(c, profile_id, &mut records).await?;

    Ok(records)
}

pub(crate) async fn get_unset_slot_items_by_types_impl<C>(
    c: &C,
    profile_id: i64,
    type3: &[i64],
) -> Result<BTreeMap<i64, Vec<i64>>, GameplayError>
where
    C: ConnectionTrait,
{
    let mut records = slot_item::Entity::find()
        .filter(slot_item::Column::ProfileId.eq(profile_id))
        .filter(slot_item::Column::EquipOn.lte(0))
        .filter(slot_item::Column::Type3.is_in(type3.to_owned()))
        .all(c)
        .await?;
    retain_free_slot_items_impl(c, profile_id, &mut records).await?;

    let mut map: BTreeMap<i64, Vec<i64>> = BTreeMap::new();

    records.iter().for_each(|record| {
        map.entry(record.type3).or_default().push(record.id);
    });

    Ok(map)
}

pub(crate) async fn toggle_slot_item_locked_impl<C>(
    c: &C,
    profile_id: i64,
    item_id: i64,
) -> Result<slot_item::Model, GameplayError>
where
    C: ConnectionTrait,
{
    let record = slot_item::Entity::find()
        .filter(slot_item::Column::Id.eq(item_id))
        .one(c)
        .await?
        .ok_or_else(|| GameplayError::EntryNotFound(format!("slot item {item_id} not found")))?;

    if record.profile_id != profile_id {
        return Err(GameplayError::EntryNotFound(format!(
            "slot item {item_id} does not belong to profile {profile_id}"
        )));
    }

    let locked = record.locked;
    let mut am: slot_item::ActiveModel = record.into();

    am.locked = ActiveValue::Set(!locked);

    let m = am.update(c).await?;

    Ok(m)
}

/// Destroy slot items and refund their scrap materials.
///
/// Returns the refunded materials plus one [`GameplayOutcome::SlotItemScrapped`]
/// per item actually destroyed, in destruction order, for the caller to observe.
pub(crate) async fn destroy_items_impl<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
    item_ids: &[i64],
) -> Result<(Vec<(MaterialCategory, i64)>, Vec<GameplayOutcome>), GameplayError>
where
    C: ConnectionTrait,
{
    let mut outcomes: Vec<GameplayOutcome> = Vec::new();
    let mut scrap_materials = vec![
        (MaterialCategory::Fuel, 0),
        (MaterialCategory::Ammo, 0),
        (MaterialCategory::Steel, 0),
        (MaterialCategory::Bauxite, 0),
    ];

    let items = slot_item::Entity::find()
        .filter(slot_item::Column::ProfileId.eq(profile_id))
        .filter(slot_item::Column::Id.is_in(item_ids.to_owned()))
        .all(c)
        .await?;

    for item in items {
        let mst = codex.find::<ApiMstSlotitem>(&item.mst_id)?;

        mst.api_broken.iter().enumerate().for_each(|(i, v)| {
            scrap_materials[i].1 += v;
        });

        let item_mst_id = item.mst_id;
        let item_level = item.level;
        item.delete(c).await?;

        outcomes.push(GameplayOutcome::SlotItemScrapped {
            item_mst_id,
            stars: item_level,
        });
    }

    add_material_impl(c, codex, profile_id, &scrap_materials).await?;

    Ok((scrap_materials, outcomes))
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
    slot_item::Entity::delete_many()
        .filter(slot_item::Column::ProfileId.eq(profile_id))
        .exec(c)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_bool_to_i64() {
        assert_eq!(true as i64, 1);
        assert_eq!(false as i64, 0);
    }
}
