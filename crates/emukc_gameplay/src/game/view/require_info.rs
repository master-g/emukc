//! The `require_info` view.

use emukc_db::{entity::profile::item::slot_item, sea_orm::TransactionTrait};
use emukc_model::{
    kc2::{
        KcApiFurniture, KcApiGameSetting, KcApiOptionSetting, KcApiOssSetting, KcApiSlotItem,
        KcApiUnsetSlot, KcApiUserItem,
    },
    profile::{kdock::ConstructionDock, slot_item::SlotItem, user_item::UserItem},
};

use crate::{
    err::GameplayError,
    game::{
        basic::get_user_basic_impl,
        kdock::get_kdocks_impl,
        settings::{
            game::get_game_settings_impl, option::get_option_settings_impl,
            oss::get_oss_settings_impl,
        },
        slot_item::{get_slot_items_impl, get_unset_slot_items_impl},
        use_item::get_use_items_impl,
    },
    gameplay::Ctx,
};

/// Port skin used when the profile has no option settings yet.
const DEFAULT_SKIN_ID: i64 = 101;

/// Everything the `require_info` view shows.
#[derive(Debug)]
pub struct RequireInfoView {
    /// Profile ID.
    pub member_id: i64,

    /// First-launch flag of the user basics.
    pub firstflag: i64,

    /// Extra supply state of the user basics.
    pub extra_supply: [i64; 2],

    /// Owned furniture.
    pub furnitures: Vec<KcApiFurniture>,

    /// Construction docks.
    pub kdocks: Vec<ConstructionDock>,

    /// OSS settings.
    pub oss_settings: KcApiOssSetting,

    /// Secretary ship position.
    pub position_id: i64,

    /// Port skin ID.
    pub skin_id: i64,

    /// Owned slot items.
    pub slot_items: Vec<KcApiSlotItem>,

    /// Unequipped slot items, grouped by equipment type.
    pub unset_slots: KcApiUnsetSlot,

    /// Owned use items.
    pub use_items: Vec<KcApiUserItem>,
}

impl Ctx {
    /// Read everything the `require_info` view shows.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    pub async fn require_info_view(
        &self,
        profile_id: i64,
    ) -> Result<RequireInfoView, GameplayError> {
        // `Ctx::get_furnitures` owns the codex to `KcApiFurniture` mapping and has
        // no `_impl`, so furniture keeps its own read.
        let furnitures = self.get_furnitures(profile_id).await?;

        let codex = self.codex.as_ref();
        let tx = self.db.begin().await?;

        let (_, basic) = get_user_basic_impl(&tx, profile_id).await?;
        let kdocks = get_kdocks_impl(&tx, profile_id).await?;
        let oss_settings = get_oss_settings_impl(&tx, profile_id).await?;
        let game_settings = get_game_settings_impl(&tx, profile_id).await?;
        let option_settings = get_option_settings_impl(&tx, profile_id).await?;
        let slot_items = get_slot_items_impl(&tx, profile_id).await?;
        let use_items = get_use_items_impl(&tx, profile_id).await?;
        let unset_slot_items = get_unset_slot_items_impl(&tx, profile_id).await?;

        tx.commit().await?;

        let game_settings: KcApiGameSetting = game_settings.into();
        let skin_id = option_settings
            .map(|m| KcApiOptionSetting::from(m).api_skin_id)
            .unwrap_or(DEFAULT_SKIN_ID);

        Ok(RequireInfoView {
            member_id: basic.api_member_id,
            firstflag: basic.api_firstflag,
            extra_supply: basic.api_extra_supply,
            furnitures,
            kdocks: kdocks.into_iter().map(std::convert::Into::into).collect(),
            oss_settings: oss_settings.into(),
            position_id: game_settings.api_position_id,
            skin_id,
            slot_items: to_api_slot_items(slot_items),
            unset_slots: codex
                .convert_unused_slot_items_to_api(&to_api_slot_items(unset_slot_items))?,
            use_items: use_items
                .into_iter()
                .map(|m| UserItem::from(m).into())
                .collect::<Vec<KcApiUserItem>>(),
        })
    }
}

fn to_api_slot_items(models: Vec<slot_item::Model>) -> Vec<KcApiSlotItem> {
    models.into_iter().map(|m| SlotItem::from(m).into()).collect()
}
