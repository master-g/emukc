use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use emukc_internal::prelude::*;

use crate::net::{
    AppState,
    auth::GameSession,
    resp::{KcApiResponse, KcApiResult},
};

#[derive(Deserialize, Debug)]
pub(super) struct Params {
    /// ship id(s) to destroy, comma-separated for batch
    api_ship_id: String,

    /// 0: keep equipment, 1: destroy equipment
    api_slot_dest_flag: i64,
}

#[derive(Serialize)]
struct Resp {
    api_material: Vec<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_unset_list: Option<KcApiUnsetSlot>,
}

pub(super) async fn handler(
    state: AppState,
    Extension(session): Extension<GameSession>,
    Form(params): Form<Params>,
) -> KcApiResult {
    let pid = session.profile.id;
    let keep_equipment = params.api_slot_dest_flag == 0;

    for ship_id_str in params.api_ship_id.split(',') {
        let ship_id: i64 = ship_id_str
            .parse()
            .map_err(|_| GameplayError::WrongType(format!("invalid ship id: {ship_id_str}")))?;
        state.destroy_ship(pid, ship_id, keep_equipment).await?;
    }

    let materials = state.get_materials(pid).await?;

    let api_unset_list = if keep_equipment {
        let codex = state.codex();
        let unset_slots = state.get_unset_slot_items(pid).await?;
        Some(codex.convert_unused_slot_items_to_api(&unset_slots)?)
    } else {
        None
    };

    Ok(KcApiResponse::success(&Resp {
        api_material: vec![materials.fuel, materials.ammo, materials.steel, materials.bauxite],
        api_unset_list,
    }))
}
