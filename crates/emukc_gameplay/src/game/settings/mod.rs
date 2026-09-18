use emukc_db::sea_orm::{TransactionTrait, entity::prelude::*};
use emukc_model::kc2::{KcApiGameSetting, KcApiOptionSetting, KcApiOssSetting};
use game::{
    get_game_settings_impl, update_flagship_position_impl, update_friendly_fleet_settings_impl,
    update_port_bgm_impl,
};
use option::update_options_settings_impl;
use oss::update_oss_settings_impl;

use crate::{err::GameplayError, gameplay::Ctx};

pub(crate) mod game;
pub(crate) mod option;
pub(crate) mod oss;

impl Ctx {
    /// Get user game settings.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    pub async fn get_game_settings(
        &self,
        profile_id: i64,
    ) -> Result<KcApiGameSetting, GameplayError> {
        let db = self.db.as_ref();
        let settings = get_game_settings_impl(db, profile_id).await?;

        Ok(settings.into())
    }

    /// Get user option settings.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    pub async fn get_option_settings(
        &self,
        profile_id: i64,
    ) -> Result<Option<KcApiOptionSetting>, GameplayError> {
        let db = self.db.as_ref();
        let settings = option::get_option_settings_impl(db, profile_id).await?;

        Ok(settings.map(std::convert::Into::into))
    }

    /// Get oss settings.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    pub async fn get_oss_settings(
        &self,
        profile_id: i64,
    ) -> Result<KcApiOssSetting, GameplayError> {
        let db = self.db.as_ref();
        let settings = oss::get_oss_settings_impl(db, profile_id).await?;

        Ok(settings.into())
    }

    /// Update oss settings.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `lan_type`: The language type.
    /// - `oss_items`: The OSS items.
    pub async fn update_oss_settings(
        &self,
        profile_id: i64,
        lan_type: i64,
        oss_items: &[i64],
    ) -> Result<(), GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        update_oss_settings_impl(&tx, profile_id, lan_type, oss_items).await?;

        tx.commit().await?;

        Ok(())
    }

    /// Update options settings.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `settings`: The option settings.
    pub async fn update_options_settings(
        &self,
        profile_id: i64,
        settings: &KcApiOptionSetting,
    ) -> Result<(), GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        update_options_settings_impl(&tx, profile_id, settings).await?;

        tx.commit().await?;

        Ok(())
    }

    /// Update port BGM.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `bgm_id`: The BGM ID.
    pub async fn update_port_bgm(&self, profile_id: i64, bgm_id: i64) -> Result<(), GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        update_port_bgm_impl(&tx, profile_id, bgm_id).await?;

        tx.commit().await?;

        Ok(())
    }

    /// Update flagship position.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `position_id`: The position ID.
    pub async fn update_flagship_position(
        &self,
        profile_id: i64,
        position_id: i64,
    ) -> Result<(), GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        update_flagship_position_impl(&tx, profile_id, position_id).await?;

        tx.commit().await?;

        Ok(())
    }

    /// Update friendly fleet settings.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `request`: The request flag.
    /// - `typ`: The type.
    pub async fn update_friendly_fleet_settings(
        &self,
        profile_id: i64,
        request: bool,
        typ: i64,
    ) -> Result<(), GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        update_friendly_fleet_settings_impl(&tx, profile_id, request, typ).await?;

        tx.commit().await?;

        Ok(())
    }
}

pub(super) async fn init<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    game::init(c, profile_id).await?;
    option::init(c, profile_id).await?;
    oss::init(c, profile_id).await?;

    Ok(())
}

pub(super) async fn wipe<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    game::wipe(c, profile_id).await?;
    option::wipe(c, profile_id).await?;
    oss::wipe(c, profile_id).await?;

    Ok(())
}
