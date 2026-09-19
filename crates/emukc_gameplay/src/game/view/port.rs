//! The `port` view.

use emukc_db::sea_orm::TransactionTrait;
use emukc_model::{
    kc2::{KcApiGameSetting, KcApiShip, KcApiUserBasic},
    profile::{fleet::Fleet, material::Material, ndock::RepairDock},
};

use crate::{
    err::GameplayError,
    game::{
        basic::{find_profile, get_user_basic_impl},
        fleet::get_fleets_impl,
        material::{get_mat_impl, update_materials_impl},
        ndock::get_ndocks_impl,
        settings::game::get_game_settings_impl,
    },
    gameplay::Ctx,
};

/// Everything the `port` view shows.
#[derive(Debug)]
pub struct PortView {
    /// User basics.
    pub basic: KcApiUserBasic,

    /// Materials, after self-replenish settlement.
    pub materials: Material,

    /// Fleets.
    pub fleets: Vec<Fleet>,

    /// Repair docks.
    pub ndocks: Vec<RepairDock>,

    /// Owned ships.
    pub ships: Vec<KcApiShip>,

    /// Port BGM ID.
    pub port_bgm_id: i64,

    /// Current combined fleet type.
    pub combined_type: i64,
}

impl Ctx {
    /// Settle the port and read everything the `port` view shows.
    ///
    /// The order is fixed (plan 002 KTD5): drop any stale sortie state left by a
    /// mid-sortie disconnect, settle material self-replenish, then read.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    pub async fn port_view(&self, profile_id: i64) -> Result<PortView, GameplayError> {
        // Clear stale sortie state from mid-sortie disconnects.
        self.clear_sortie_state_if_any(profile_id).await;

        let codex = self.codex.as_ref();
        let tx = self.db.begin().await?;

        let profile = find_profile(&tx, profile_id).await?;
        update_materials_impl(&tx, codex, profile_id, profile.hq_level).await?;

        // TODO(#0): update quests here

        let (_, basic) = get_user_basic_impl(&tx, profile_id).await?;
        let materials = get_mat_impl(&tx, profile_id).await?;
        let fleets = get_fleets_impl(&tx, profile_id).await?;
        let ndocks = get_ndocks_impl(&tx, codex, profile_id).await?;
        let game_settings: KcApiGameSetting = get_game_settings_impl(&tx, profile_id).await?.into();

        tx.commit().await?;

        // `Ctx::get_ships` owns the `api_onslot_max` and `api_sp_effect_items`
        // filling, which is private to the `ship` module, so ships keep their
        // own read.
        let ships = self.get_ships(profile_id).await?;

        Ok(PortView {
            basic,
            materials: materials.into(),
            fleets: fleets.into_iter().map(std::convert::Into::into).collect(),
            ndocks: ndocks.into_iter().map(std::convert::Into::into).collect(),
            ships,
            port_bgm_id: game_settings.api_p_bgm_id,
            combined_type: profile.combined_type,
        })
    }
}
