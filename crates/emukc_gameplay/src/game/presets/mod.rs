use deck::{
    apply_preset_deck_impl, delete_preset_deck_impl, expand_preset_deck_capacity_impl,
    find_preset_deck_impl, get_preset_decks_impl, register_preset_deck_impl,
};
use dev_item::{
    delete_preset_dev_item_impl, expand_preset_dev_item_capacity_impl, get_preset_dev_items_impl,
    register_preset_dev_item_impl, update_preset_dev_item_name_impl,
};
use emukc_db::{
    entity::profile::{
        fleet,
        preset::{
            preset_caps, preset_deck, preset_dev_item,
            preset_slot::{self},
        },
    },
    sea_orm::{ActiveValue, TransactionTrait, entity::prelude::*},
};
use emukc_model::profile::{
    preset_deck::{PresetDeck, PresetDeckItem},
    preset_dev_item::{PresetDevItem, PresetDevItemElement},
    preset_slot::PresetSlot,
};
use slot::{
    apply_preset_slot_impl, delete_preset_slot_impl, expand_preset_slot_capacity_impl,
    get_preset_slots_impl, register_preset_slot_impl, toggle_preset_slot_ex_flag_impl,
    toggle_preset_slot_locked_impl, update_preset_slot_name_impl,
};

use crate::{err::GameplayError, gameplay::Ctx};

use super::{
    fleet::{get_fleets_impl, update_fleet_ships_impl},
    slot_item::get_unset_slot_items_impl,
    use_item::deduct_use_item_impl,
};

pub(crate) mod deck;
pub(crate) mod dev_item;
pub(crate) mod slot;

impl Ctx {
    /// Get preset deck
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    pub async fn get_preset_decks(&self, profile_id: i64) -> Result<PresetDeck, GameplayError> {
        let db = self.db.as_ref();

        let (caps, decks) = get_preset_decks_impl(db, profile_id).await?;

        Ok(PresetDeck {
            max_num: caps.deck_limit,
            records: decks.into_iter().map(Into::into).collect(),
        })
    }

    /// Find preset deck
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `preset_no`: The preset number.
    pub async fn find_preset_deck(
        &self,
        profile_id: i64,
        preset_no: i64,
    ) -> Result<PresetDeckItem, GameplayError> {
        let db = self.db.as_ref();

        let model = find_preset_deck_impl(db, profile_id, preset_no).await?;

        Ok(model.into())
    }

    /// Get preset slot
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    pub async fn get_preset_slots(&self, profile_id: i64) -> Result<PresetSlot, GameplayError> {
        let db = self.db.as_ref();

        let (caps, slots) = get_preset_slots_impl(db, profile_id).await?;

        Ok(PresetSlot {
            profile_id,
            max_num: caps.slot_limit,
            records: slots.into_iter().map(Into::into).collect(),
        })
    }

    /// Register preset deck
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `preset`: The preset deck item.
    pub async fn register_preset_deck(
        &self,
        profile_id: i64,
        preset: &PresetDeckItem,
    ) -> Result<preset_deck::Model, GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        let m = register_preset_deck_impl(&tx, profile_id, preset).await?;

        tx.commit().await?;

        Ok(m)
    }

    /// Register preset slot
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `preset_no`: The preset number.
    /// - `ship_id`: The ship ID.
    pub async fn register_preset_slot(
        &self,
        profile_id: i64,
        preset_no: i64,
        ship_id: i64,
    ) -> Result<preset_slot::Model, GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        let m = register_preset_slot_impl(&tx, profile_id, preset_no, ship_id).await?;

        tx.commit().await?;

        Ok(m)
    }

    /// Delete preset deck
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `preset_no`: The preset number.
    pub async fn delete_preset_deck(
        &self,
        profile_id: i64,
        preset_no: i64,
    ) -> Result<(), GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        delete_preset_deck_impl(&tx, profile_id, preset_no).await?;

        tx.commit().await?;

        Ok(())
    }

    /// Expand preset deck capacity
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    pub async fn expand_preset_deck_capacity(&self, profile_id: i64) -> Result<(), GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        expand_preset_deck_capacity_impl(&tx, profile_id).await?;

        tx.commit().await?;

        Ok(())
    }

    /// Expand preset slot capacity
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    pub async fn expand_preset_slot_capacity(&self, profile_id: i64) -> Result<i64, GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        let new_cap = expand_preset_slot_capacity_impl(&tx, profile_id).await?;

        tx.commit().await?;

        Ok(new_cap)
    }

    /// Apply preset deck
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `deck_id`: The deck ID.
    /// - `preset_no`: The preset number.
    pub async fn apply_preset_deck(
        &self,
        profile_id: i64,
        deck_id: i64,
        preset_no: i64,
    ) -> Result<fleet::Model, GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        let m = apply_preset_deck_impl(&tx, profile_id, deck_id, preset_no).await?;

        tx.commit().await?;

        Ok(m)
    }

    /// Apply preset slot
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `preset_no`: The preset number.
    /// - `ship_id`: The ship ID.
    /// - `mode`: The mode.
    pub async fn apply_preset_slot(
        &self,
        profile_id: i64,
        preset_no: i64,
        ship_id: i64,
        mode: i64,
    ) -> Result<i64, GameplayError> {
        let codex = self.codex.as_ref();
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        let bauxite =
            apply_preset_slot_impl(&tx, codex, profile_id, preset_no, ship_id, mode).await?;

        tx.commit().await?;

        Ok(bauxite)
    }

    /// Delete preset slot
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `preset_no`: The preset number.
    pub async fn delete_preset_slot(
        &self,
        profile_id: i64,
        preset_no: i64,
    ) -> Result<(), GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        delete_preset_slot_impl(&tx, profile_id, preset_no).await?;

        tx.commit().await?;

        Ok(())
    }

    /// Toggle preset slot ex flag
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `preset_no`: The preset number.
    pub async fn toggle_preset_slot_ex_flag(
        &self,
        profile_id: i64,
        preset_no: i64,
    ) -> Result<(), GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        toggle_preset_slot_ex_flag_impl(&tx, profile_id, preset_no).await?;

        tx.commit().await?;

        Ok(())
    }

    /// Toggle preset slot locked
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `preset_no`: The preset number.
    pub async fn toggle_preset_slot_locked(
        &self,
        profile_id: i64,
        preset_no: i64,
    ) -> Result<(), GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        toggle_preset_slot_locked_impl(&tx, profile_id, preset_no).await?;

        tx.commit().await?;

        Ok(())
    }

    /// Update preset slot name
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `preset_no`: The preset number.
    /// - `name`: The new name.
    pub async fn update_preset_slot_name(
        &self,
        profile_id: i64,
        preset_no: i64,
        name: &str,
    ) -> Result<(), GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        update_preset_slot_name_impl(&tx, profile_id, preset_no, name).await?;

        tx.commit().await?;

        Ok(())
    }

    /// Get preset dev items
    pub async fn get_preset_dev_items(
        &self,
        profile_id: i64,
    ) -> Result<PresetDevItem, GameplayError> {
        let db = self.db.as_ref();

        let (caps, items) = get_preset_dev_items_impl(db, profile_id).await?;

        Ok(PresetDevItem {
            max_num: caps.dev_item_limit,
            records: items
                .into_iter()
                .map(|m| PresetDevItemElement {
                    index: m.index,
                    name: m.name,
                    item1: m.item1,
                    item2: m.item2,
                    item3: m.item3,
                    item4: m.item4,
                })
                .collect(),
        })
    }

    /// Register preset dev item
    pub async fn register_preset_dev_item(
        &self,
        profile_id: i64,
        preset: &PresetDevItemElement,
    ) -> Result<preset_dev_item::Model, GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        let m = register_preset_dev_item_impl(&tx, profile_id, preset).await?;

        tx.commit().await?;

        Ok(m)
    }

    /// Delete preset dev item
    pub async fn delete_preset_dev_item(
        &self,
        profile_id: i64,
        preset_no: i64,
    ) -> Result<(), GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        delete_preset_dev_item_impl(&tx, profile_id, preset_no).await?;

        tx.commit().await?;

        Ok(())
    }

    /// Update preset dev item name
    pub async fn update_preset_dev_item_name(
        &self,
        profile_id: i64,
        preset_no: i64,
        name: String,
    ) -> Result<(), GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        update_preset_dev_item_name_impl(&tx, profile_id, preset_no, name).await?;

        tx.commit().await?;

        Ok(())
    }

    /// Expand preset dev item capacity
    pub async fn expand_preset_dev_item_capacity(
        &self,
        profile_id: i64,
    ) -> Result<i64, GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        let new_cap = expand_preset_dev_item_capacity_impl(&tx, profile_id).await?;

        tx.commit().await?;

        Ok(new_cap)
    }
}

pub(super) async fn init<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    let caps_am: preset_caps::ActiveModel = preset_caps::ActiveModel {
        id: ActiveValue::set(profile_id),
        deck_limit: ActiveValue::set(3),
        slot_limit: ActiveValue::set(4),
        dev_item_limit: ActiveValue::set(3),
    };

    caps_am.insert(c).await?;
    Ok(())
}

pub(super) async fn wipe<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    preset_caps::Entity::delete_by_id(profile_id).exec(c).await?;
    preset_deck::Entity::delete_many()
        .filter(preset_deck::Column::ProfileId.eq(profile_id))
        .exec(c)
        .await?;
    preset_slot::Entity::delete_many()
        .filter(preset_slot::Column::ProfileId.eq(profile_id))
        .exec(c)
        .await?;
    preset_dev_item::Entity::delete_many()
        .filter(preset_dev_item::Column::ProfileId.eq(profile_id))
        .exec(c)
        .await?;

    Ok(())
}
